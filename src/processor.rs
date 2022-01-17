use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    instruction::Instruction,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};
use spl_token::state::{Account, Mint};

use crate::{
    error::CustomError,
    instruction::{InitData, InstructionType, StakeData},
    state::{InitStakeUserParams, Pool, StakeUser},
};

pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
    let instruction = InstructionType::unpack(input)?;

    match instruction {
        InstructionType::CreatePool(init_data) => {
            process_create_stake_pool(program_id, accounts, init_data.clone())
        }
        InstructionType::CreateStakeUser => process_create_stake_user(program_id, accounts),
        InstructionType::Stake(StakeData { amount }) => process_stake(program_id, accounts, amount),
        InstructionType::Unstake(StakeData { amount }) => {
            process_unstake(program_id, accounts, amount)
        }

        InstructionType::Claim => process_claim(program_id, accounts),
        InstructionType::Refresh => process_refresh(program_id, accounts),
    }
}

/// For Task 1: create stake pool
pub fn process_create_stake_pool(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    init_data: InitData,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let stake_pool_info = next_account_info(account_info_iter)?;
    let stake_pool_authority_info = next_account_info(account_info_iter)?;
    let staking_token_mint_info = next_account_info(account_info_iter)?;
    let staking_token_reserve_info = next_account_info(account_info_iter)?;
    let reward_token_mint_info = next_account_info(account_info_iter)?;
    let rent_info = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(rent_info)?;
    let token_program_info = next_account_info(account_info_iter)?;

    if stake_pool_info.owner != program_id {
        return Err(CustomError::InvalidAccountOwner.into());
    }

    assert_rent_exempt(rent, stake_pool_info)?;
    let mut stake_pool = assert_uninitialized::<Pool>(stake_pool_info)?;

    let authority_signer_seeds = &[stake_pool_info.key.as_ref(), &[init_data.bump_seed]];
    if *stake_pool_authority_info.key
        != Pubkey::create_program_address(authority_signer_seeds, program_id)?
    {
        return Err(CustomError::InvalidPoolAuthority.into());
    }

    stake_pool.is_initialized = true;
    stake_pool.bump_seed = init_data.bump_seed;
    stake_pool.stake_token_mint = *staking_token_mint_info.key;
    stake_pool.reserved = *staking_token_reserve_info.key;
    stake_pool.reward_mint = *reward_token_mint_info.key;
    stake_pool.reward_numerator = init_data.reward_numerator;
    stake_pool.reward_denominator = init_data.reward_denominator;
    Pool::pack(stake_pool, &mut stake_pool_info.data.borrow_mut())?;

    spl_token_init_account(TokenInitializeAccountParams {
        account: staking_token_reserve_info.clone(),
        mint: staking_token_mint_info.clone(),
        owner: stake_pool_authority_info.clone(),
        rent: rent_info.clone(),
        token_program: token_program_info.clone(),
    })?;

    spl_token_init_mint(TokenInitializeMintParams {
        mint: reward_token_mint_info.clone(),
        authority: stake_pool_authority_info.key,
        rent: rent_info.clone(),
        decimals: 9,
        token_program: token_program_info.clone(),
    })?;

    Ok(())
}

/// For Task 1: create stake user
pub fn process_create_stake_user(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let stake_pool_info = next_account_info(account_info_iter)?;
    let stake_user_info = next_account_info(account_info_iter)?;
    let stake_owner_info = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(next_account_info(account_info_iter)?)?;

    if stake_pool_info.owner != program_id || stake_user_info.owner != program_id {
        return Err(CustomError::InvalidAccountOwner.into());
    }

    assert_rent_exempt(rent, stake_user_info)?;
    let mut stake_user = assert_uninitialized::<StakeUser>(stake_user_info)?;

    if !stake_owner_info.is_signer {
        return Err(CustomError::InvalidSigner.into());
    }

    stake_user.init(InitStakeUserParams {
        pool_pubkey: *stake_pool_info.key,
        owner: *stake_owner_info.key,
    });
    StakeUser::pack(stake_user, &mut stake_user_info.data.borrow_mut())?;

    Ok(())
}

/// For Task 1: do stake
pub fn process_stake(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let stake_pool_info = next_account_info(account_info_iter)?;
    let stake_user_info = next_account_info(account_info_iter)?;
    let user_transfer_authority_info = next_account_info(account_info_iter)?;
    let stake_owner_info = next_account_info(account_info_iter)?;
    let source_info = next_account_info(account_info_iter)?;
    let destination_info = next_account_info(account_info_iter)?;
    let clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;
    let token_program_info = next_account_info(account_info_iter)?;

    if stake_pool_info.owner != program_id || stake_user_info.owner != program_id {
        return Err(CustomError::InvalidAccountOwner.into());
    }

    let mut stake_user = StakeUser::unpack(&stake_user_info.data.borrow_mut())?;
    if !stake_owner_info.is_signer {
        return Err(CustomError::InvalidSigner.into());
    }
    if stake_user.owner != *stake_owner_info.key {
        return Err(CustomError::InvalidStakeOwner.into());
    }
    let stake_pool = Pool::unpack(&stake_pool_info.data.borrow_mut())?;
    if stake_pool.reserved != *destination_info.key {
        return Err(CustomError::InvalidTokenAccount.into());
    }
    let source_token = unpack_token_account(source_info, token_program_info.key)?;
    let destination_token = unpack_token_account(destination_info, token_program_info.key)?;
    if source_token.mint != destination_token.mint {
        return Err(CustomError::InvalidTokenMint.into());
    }
    if source_token.mint != stake_pool.stake_token_mint {
        return Err(CustomError::InvalidTokenMint.into());
    }
    if source_token.amount < amount {
        return Err(CustomError::InsufficientFunds.into());
    }

    if stake_user.stake_amount != 0 {
        stake_user.update_reward_owed(
            stake_pool.reward_numerator,
            stake_pool.reward_denominator,
            clock.unix_timestamp,
        )?;
    } else {
        stake_user.last_update = clock.unix_timestamp
    }

    stake_user.stake(amount)?;
    StakeUser::pack(stake_user, &mut stake_user_info.data.borrow_mut())?;

    spl_token_transfer(TokenTransferParams {
        source: source_info.clone(),
        destination: destination_info.clone(),
        amount,
        authority: user_transfer_authority_info.clone(),
        authority_signer_seeds: &[],
        token_program: token_program_info.clone(),
    })?;

    Ok(())
}

/// For Task 1: do unstake
pub fn process_unstake(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let stake_pool_info = next_account_info(account_info_iter)?;
    let stake_user_info = next_account_info(account_info_iter)?;
    let stake_pool_authority_info = next_account_info(account_info_iter)?;
    let stake_owner_info = next_account_info(account_info_iter)?;
    let source_info = next_account_info(account_info_iter)?;
    let destination_info = next_account_info(account_info_iter)?;
    let clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;
    let token_program_info = next_account_info(account_info_iter)?;

    if stake_pool_info.owner != program_id || stake_user_info.owner != program_id {
        return Err(CustomError::InvalidAccountOwner.into());
    }

    let mut stake_user = StakeUser::unpack(&stake_user_info.data.borrow_mut())?;
    if !stake_owner_info.is_signer {
        return Err(CustomError::InvalidSigner.into());
    }
    if stake_user.owner != *stake_owner_info.key {
        return Err(CustomError::InvalidStakeOwner.into());
    }

    let stake_pool = Pool::unpack(&stake_pool_info.data.borrow_mut())?;
    let stake_pool_authority_signer_seeds =
        &[stake_pool_info.key.as_ref(), &[stake_pool.bump_seed]];
    if *stake_pool_authority_info.key
        != Pubkey::create_program_address(stake_pool_authority_signer_seeds, program_id)?
    {
        return Err(CustomError::InvalidPoolAuthority.into());
    }
    let source_token = unpack_token_account(source_info, token_program_info.key)?;
    let destination_token = unpack_token_account(destination_info, token_program_info.key)?;
    if source_token.mint != destination_token.mint {
        return Err(CustomError::InvalidTokenMint.into());
    }
    if destination_token.mint != stake_pool.stake_token_mint {
        return Err(CustomError::InvalidTokenMint.into());
    }
    if source_token.amount < amount {
        return Err(CustomError::InsufficientLiquidity.into());
    }

    if stake_user.stake_amount != 0 {
        stake_user.update_reward_owed(
            stake_pool.reward_numerator,
            stake_pool.reward_denominator,
            clock.unix_timestamp,
        )?;
    }

    stake_user.unstake(amount)?;
    StakeUser::pack(stake_user, &mut stake_user_info.data.borrow_mut())?;

    spl_token_transfer(TokenTransferParams {
        source: source_info.clone(),
        destination: destination_info.clone(),
        amount,
        authority: stake_pool_authority_info.clone(),
        authority_signer_seeds: stake_pool_authority_signer_seeds,
        token_program: token_program_info.clone(),
    })?;

    Ok(())
}

/// For task 2: Claim rewards owed
pub fn process_claim(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let stake_pool_info = next_account_info(account_info_iter)?;
    let stake_user_info = next_account_info(account_info_iter)?;
    let stake_owner_info = next_account_info(account_info_iter)?;
    let stake_pool_authority_info = next_account_info(account_info_iter)?;
    let reward_mint_info = next_account_info(account_info_iter)?;
    let reward_token_info = next_account_info(account_info_iter)?;
    let clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;
    let token_program_info = next_account_info(account_info_iter)?;

    if stake_pool_info.owner != program_id || stake_user_info.owner != program_id {
        return Err(CustomError::InvalidAccountOwner.into());
    }

    let mut stake_user = StakeUser::unpack(&stake_user_info.data.borrow_mut())?;
    if stake_user.pool_pubkey != *stake_pool_info.key {
        return Err(CustomError::InvalidStakeOwner.into());
    }
    if stake_user.owner != *stake_owner_info.key {
        return Err(CustomError::InvalidStakeOwner.into());
    }
    if !stake_owner_info.is_signer {
        return Err(CustomError::InvalidSigner.into());
    }
    let stake_pool = Pool::unpack(&stake_pool_info.data.borrow())?;
    let reward_token = unpack_token_account(reward_token_info, token_program_info.key)?;
    if stake_pool.reward_mint != *reward_mint_info.key {
        return Err(CustomError::InvalidTokenMint.into());
    }
    if reward_token_info.owner == stake_pool_authority_info.key {
        return Err(CustomError::InvalidAccountOwner.into());
    }
    if reward_token.mint != *reward_mint_info.key {
        return Err(CustomError::InvalidTokenMint.into());
    }
    let stake_pool_authority_signer_seeds =
        &[stake_pool_info.key.as_ref(), &[stake_pool.bump_seed]];
    if *stake_pool_authority_info.key
        != Pubkey::create_program_address(stake_pool_authority_signer_seeds, program_id)?
    {
        return Err(CustomError::InvalidPoolAuthority.into());
    }

    if stake_user.stake_amount != 0 {
        stake_user.update_reward_owed(
            stake_pool.reward_numerator,
            stake_pool.reward_denominator,
            clock.unix_timestamp,
        )?;
    }

    let amount = stake_user.claim()?;
    StakeUser::pack(stake_user, &mut stake_user_info.data.borrow_mut())?;

    spl_token_mint_to(TokenMintToParams {
        mint: reward_mint_info.clone(),
        destination: reward_token_info.clone(),
        amount,
        authority: stake_pool_authority_info.clone(),
        authority_signer_seeds: stake_pool_authority_signer_seeds,
        token_program: token_program_info.clone(),
    })?;

    Ok(())
}

pub fn process_refresh(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let stake_pool_info = next_account_info(account_info_iter)?;
    let clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;

    if stake_pool_info.owner != program_id {
        return Err(CustomError::InvalidAccountOwner.into());
    }

    let stake_pool = Pool::unpack(&stake_pool_info.data.borrow())?;

    for stake_user_info in account_info_iter {
        if stake_user_info.owner != program_id {
            continue;
        }
        let mut stake_user = StakeUser::unpack(&stake_user_info.data.borrow_mut())?;
        if stake_user.pool_pubkey != *stake_pool_info.key {
            continue;
        }
        if stake_user.stake_amount != 0 {
            stake_user.update_reward_owed(
                stake_pool.reward_numerator,
                stake_pool.reward_denominator,
                clock.unix_timestamp,
            )?;
            StakeUser::pack(stake_user, &mut stake_user_info.data.borrow_mut())?;
        }
    }

    Ok(())
}

pub fn assert_rent_exempt(rent: &Rent, account_info: &AccountInfo) -> ProgramResult {
    if !rent.is_exempt(account_info.lamports(), account_info.data_len()) {
        Err(CustomError::NotRentExempt.into())
    } else {
        Ok(())
    }
}

pub fn assert_uninitialized<T: Pack + IsInitialized>(
    account_info: &AccountInfo,
) -> Result<T, ProgramError> {
    let account: T = T::unpack_unchecked(&account_info.data.borrow())?;
    if account.is_initialized() {
        Err(CustomError::AlreadyInUse.into())
    } else {
        Ok(account)
    }
}

pub fn unpack_mint(
    account_info: &AccountInfo,
    token_program_id: &Pubkey,
) -> Result<Mint, ProgramError> {
    if account_info.owner != token_program_id {
        Err(CustomError::InvalidAccountOwner.into())
    } else {
        Mint::unpack(&account_info.data.borrow()).map_err(|_| CustomError::InvalidTokenMint.into())
    }
}

pub fn unpack_token_account(
    account_info: &AccountInfo,
    token_program_id: &Pubkey,
) -> Result<Account, ProgramError> {
    if account_info.owner != token_program_id {
        Err(CustomError::InvalidAccountOwner.into())
    } else {
        spl_token::state::Account::unpack(&account_info.data.borrow())
            .map_err(|_| CustomError::InvalidTokenAccount.into())
    }
}

struct TokenInitializeAccountParams<'a> {
    account: AccountInfo<'a>,
    mint: AccountInfo<'a>,
    owner: AccountInfo<'a>,
    rent: AccountInfo<'a>,
    token_program: AccountInfo<'a>,
}

struct TokenInitializeMintParams<'a: 'b, 'b> {
    mint: AccountInfo<'a>,
    rent: AccountInfo<'a>,
    authority: &'b Pubkey,
    decimals: u8,
    token_program: AccountInfo<'a>,
}

struct TokenTransferParams<'a: 'b, 'b> {
    source: AccountInfo<'a>,
    destination: AccountInfo<'a>,
    amount: u64,
    authority: AccountInfo<'a>,
    authority_signer_seeds: &'b [&'b [u8]],
    token_program: AccountInfo<'a>,
}

struct TokenMintToParams<'a: 'b, 'b> {
    mint: AccountInfo<'a>,
    destination: AccountInfo<'a>,
    amount: u64,
    authority: AccountInfo<'a>,
    authority_signer_seeds: &'b [&'b [u8]],
    token_program: AccountInfo<'a>,
}

fn spl_token_init_account(params: TokenInitializeAccountParams<'_>) -> ProgramResult {
    let TokenInitializeAccountParams {
        account,
        mint,
        owner,
        rent,
        token_program,
    } = params;
    let ix = spl_token::instruction::initialize_account(
        token_program.key,
        account.key,
        mint.key,
        owner.key,
    )?;
    let result = invoke(&ix, &[account, mint, owner, rent, token_program]);
    result.map_err(|_| CustomError::TokenInitializeAccountFailed.into())
}

fn spl_token_init_mint(params: TokenInitializeMintParams<'_, '_>) -> ProgramResult {
    let TokenInitializeMintParams {
        mint,
        rent,
        authority,
        token_program,
        decimals,
    } = params;
    let ix = spl_token::instruction::initialize_mint(
        token_program.key,
        mint.key,
        authority,
        None,
        decimals,
    )?;
    let result = invoke(&ix, &[mint, rent, token_program]);
    result.map_err(|_| CustomError::TokenInitializeMintFailed.into())
}

fn spl_token_transfer(params: TokenTransferParams<'_, '_>) -> ProgramResult {
    let TokenTransferParams {
        source,
        destination,
        authority,
        token_program,
        amount,
        authority_signer_seeds,
    } = params;
    let result = invoke_optionally_signed(
        &spl_token::instruction::transfer(
            token_program.key,
            source.key,
            destination.key,
            authority.key,
            &[],
            amount,
        )?,
        &[source, destination, authority, token_program],
        authority_signer_seeds,
    );
    result.map_err(|_| CustomError::TokenTransferFailed.into())
}

fn spl_token_mint_to(params: TokenMintToParams<'_, '_>) -> ProgramResult {
    let TokenMintToParams {
        mint,
        destination,
        authority,
        token_program,
        amount,
        authority_signer_seeds,
    } = params;
    let result = invoke_optionally_signed(
        &spl_token::instruction::mint_to(
            token_program.key,
            mint.key,
            destination.key,
            authority.key,
            &[],
            amount,
        )?,
        &[mint, destination, authority, token_program],
        authority_signer_seeds,
    );
    result.map_err(|_| CustomError::TokenMintToFailed.into())
}

fn invoke_optionally_signed(
    instruction: &Instruction,
    account_infos: &[AccountInfo],
    authority_signer_seeds: &[&[u8]],
) -> ProgramResult {
    if authority_signer_seeds.is_empty() {
        invoke(instruction, account_infos)
    } else {
        invoke_signed(instruction, account_infos, &[authority_signer_seeds])
    }
}
