#![allow(clippy::too_many_arguments)]

use crate::error::CustomError;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::{clock, rent},
};

use std::{convert::TryInto, mem::size_of};

#[repr(C)]
#[derive(Debug, PartialEq)]
pub enum InstructionType {
    /// Create stake pool
    ///
    /// 0. `[writable]` stake pool account to create
    /// 1. `[]` authority generated from bump_seed to mint reward
    /// 2. `[]` staking token mint
    /// 3. `[writable]` staking token reserve account
    /// 4. `[wrtiable]` reward token mint
    /// 5. `[]` rent sysvar
    /// 6. `[]` token program id
    CreatePool(InitData),
    /// Create stake user
    ///
    /// 0. `[]` stake pool account
    /// 1. `[writable]` stake user account to create
    /// 2. `[signer]` stake user owner account
    /// 3. `[]` rent sysvar
    /// 4. `[]` clock sysvar
    CreateStakeUser,
    /// Stake token to the pool
    ///
    /// 0. `[]` stake pool account
    /// 1. `[writable]` stake user account
    /// 2. `[signer]` user transfer authority
    /// 3. `[signer]` stake user owner account
    /// 4. `[writable]` staking token user account
    /// 5. `[writable]` staking token reserve account
    /// 6. `[]` clock syavar
    /// 7. `[]` token program id
    Stake(StakeData),
    /// Unstake token to the pool
    ///
    /// 0. `[]` stake pool account
    /// 1. `[writable]` stake user account
    /// 2. `[]` authority generated from bump_seed to mint reward
    /// 3. `[signer]` stake user owner account
    /// 4. `[writable]` staking token reserve account
    /// 5. `[writable]` staking token user account
    /// 6. `[]` clock ssyavar
    /// 7. `[]` token program id
    Unstake(StakeData),
    /// Calculate and Claim reward token owed
    ///
    /// 0. `[]` stake pool account
    /// 1. `[writable]` stake user account
    /// 2. `[signer]` stake owner account
    /// 2. `[]` authorty generated from bump_seed to mint reward
    /// 2. `[writable]` reward token mint
    /// 3. `[writable]` reward token account
    /// 4. `[]` clock sysvar
    /// 5. `[]` token program id
    Claim,
    /// Calculate reward token for stake users
    ///
    /// 0. `[]` stake pool account
    /// 1. `[]` clock sysvar
    /// 2. `[writable]` array of staking user account
    Refresh,
}

#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct InitData {
    /// bump_seed to generate pool authority
    pub bump_seed: u8,
    /// Daily reward numerator
    pub reward_numerator: u64,
    /// Daily reward denominator
    pub reward_denominator: u64,
}

#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct StakeData {
    /// Amount to stake
    pub amount: u64,
}

impl InstructionType {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&tag, rest) = input
            .split_first()
            .ok_or(CustomError::IncorrectInstruction)?;

        Ok(match tag {
            0x1 => {
                let (bump_seed, rest) = unpack_u8(rest)?;
                let (reward_numerator, rest) = unpack_u64(rest)?;
                let (reward_denominator, _) = unpack_u64(rest)?;
                Self::CreatePool(InitData {
                    bump_seed,
                    reward_numerator,
                    reward_denominator,
                })
            }
            0x2 => Self::CreateStakeUser,
            0x3 => {
                let (amount, _) = unpack_u64(rest)?;
                Self::Stake(StakeData { amount })
            }
            0x4 => {
                let (amount, _) = unpack_u64(rest)?;
                Self::Unstake(StakeData { amount })
            }
            0x5 => Self::Claim,
            0x6 => Self::Refresh,
            _ => return Err(CustomError::IncorrectInstruction.into()),
        })
    }

    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(size_of::<Self>());
        match *self {
            Self::CreatePool(InitData {
                bump_seed,
                reward_numerator,
                reward_denominator,
            }) => {
                buf.push(0x1);
                buf.extend_from_slice(&bump_seed.to_le_bytes());
                buf.extend_from_slice(&reward_numerator.to_le_bytes());
                buf.extend_from_slice(&reward_denominator.to_le_bytes());
            }
            Self::CreateStakeUser => {
                buf.push(0x2);
            }
            Self::Stake(StakeData { amount }) => {
                buf.push(0x3);
                buf.extend_from_slice(&amount.to_le_bytes());
            }
            Self::Unstake(StakeData { amount }) => {
                buf.push(0x4);
                buf.extend_from_slice(&amount.to_le_bytes());
            }
            Self::Claim => {
                buf.push(0x5);
            }
            Self::Refresh => {
                buf.push(0x6);
            }
        }
        buf
    }
}

fn unpack_u8(input: &[u8]) -> Result<(u8, &[u8]), ProgramError> {
    if input.is_empty() {
        return Err(CustomError::InstructionUnpackError.into());
    }
    let (bytes, rest) = input.split_at(1);
    let value = bytes
        .get(..1)
        .and_then(|slice| slice.try_into().ok())
        .map(u8::from_le_bytes)
        .ok_or(CustomError::InstructionUnpackError)?;
    Ok((value, rest))
}

fn unpack_u64(input: &[u8]) -> Result<(u64, &[u8]), ProgramError> {
    if input.len() < 8 {
        return Err(CustomError::InstructionUnpackError.into());
    }
    let (amount, rest) = input.split_at(8);
    let amount = amount
        .get(..8)
        .and_then(|slice| slice.try_into().ok())
        .map(u64::from_le_bytes)
        .ok_or(CustomError::InstructionUnpackError)?;
    Ok((amount, rest))
}

pub fn create_stake_pool(
    program_id: Pubkey,
    stake_pool_pubkey: Pubkey,
    stake_pool_authority_pubkey: Pubkey,
    staking_token_mint_pubkey: Pubkey,
    staking_token_reserve_pubkey: Pubkey,
    reward_token_mint_pubkey: Pubkey,
    init_data: InitData,
) -> Result<Instruction, ProgramError> {
    let data = InstructionType::CreatePool(init_data).pack();

    let accounts = vec![
        AccountMeta::new(stake_pool_pubkey, true),
        AccountMeta::new_readonly(stake_pool_authority_pubkey, false),
        AccountMeta::new_readonly(staking_token_mint_pubkey, false),
        AccountMeta::new(staking_token_reserve_pubkey, false),
        AccountMeta::new(reward_token_mint_pubkey, false),
        AccountMeta::new_readonly(rent::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];

    Ok(Instruction {
        program_id,
        accounts,
        data,
    })
}

pub fn create_stake_user(
    program_id: Pubkey,
    stake_pool_pubkey: Pubkey,
    stake_user_pubkey: Pubkey,
    stake_owner_pubkey: Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = InstructionType::CreateStakeUser.pack();

    let accounts = vec![
        AccountMeta::new_readonly(stake_pool_pubkey, false),
        AccountMeta::new(stake_user_pubkey, false),
        AccountMeta::new(stake_owner_pubkey, true),
        AccountMeta::new_readonly(rent::id(), false),
    ];

    Ok(Instruction {
        program_id,
        accounts,
        data,
    })
}

pub fn stake(
    program_id: Pubkey,
    stake_pool_pubkey: Pubkey,
    stake_user_pubkey: Pubkey,
    user_transfer_authority_pubkey: Pubkey,
    stake_owner_pubkey: Pubkey,
    source_pubkey: Pubkey,
    destination_pubkey: Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let data = InstructionType::Stake(StakeData { amount }).pack();

    let accounts = vec![
        AccountMeta::new_readonly(stake_pool_pubkey, false),
        AccountMeta::new(stake_user_pubkey, false),
        AccountMeta::new_readonly(user_transfer_authority_pubkey, true),
        AccountMeta::new_readonly(stake_owner_pubkey, true),
        AccountMeta::new(source_pubkey, false),
        AccountMeta::new(destination_pubkey, false),
        AccountMeta::new_readonly(clock::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];

    Ok(Instruction {
        program_id,
        accounts,
        data,
    })
}

pub fn unstake(
    program_id: Pubkey,
    stake_pool_pubkey: Pubkey,
    stake_user_pubkey: Pubkey,
    authority_pubkey: Pubkey,
    stake_owner_pubkey: Pubkey,
    source_pubkey: Pubkey,
    destination_pubkey: Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let data = InstructionType::Unstake(StakeData { amount }).pack();

    let accounts = vec![
        AccountMeta::new(stake_pool_pubkey, false),
        AccountMeta::new(stake_user_pubkey, false),
        AccountMeta::new_readonly(authority_pubkey, false),
        AccountMeta::new_readonly(stake_owner_pubkey, true),
        AccountMeta::new(source_pubkey, false),
        AccountMeta::new(destination_pubkey, false),
        AccountMeta::new_readonly(clock::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];

    Ok(Instruction {
        program_id,
        accounts,
        data,
    })
}

pub fn claim(
    program_id: Pubkey,
    stake_pool_pubkey: Pubkey,
    stake_user_pubkey: Pubkey,
    authority_pubkey: Pubkey,
    reward_token_mint_pubkey: Pubkey,
    destination_pubkey: Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = InstructionType::Claim.pack();

    let accounts = vec![
        AccountMeta::new_readonly(stake_pool_pubkey, false),
        AccountMeta::new(stake_user_pubkey, false),
        AccountMeta::new_readonly(authority_pubkey, false),
        AccountMeta::new(reward_token_mint_pubkey, false),
        AccountMeta::new(destination_pubkey, false),
        AccountMeta::new_readonly(clock::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];

    Ok(Instruction {
        program_id,
        accounts,
        data,
    })
}

pub fn refresh(
    program_id: Pubkey,
    stake_pool_pubkey: Pubkey,
    stake_user_pubkeys: Vec<Pubkey>,
) -> Result<Instruction, ProgramError> {
    let data = InstructionType::Refresh.pack();

    let mut accounts = vec![
        AccountMeta::new_readonly(stake_pool_pubkey, false),
        AccountMeta::new_readonly(clock::id(), false),
    ];

    accounts.extend(
        stake_user_pubkeys
            .into_iter()
            .map(|pubkey| AccountMeta::new(pubkey, false)),
    );

    Ok(Instruction {
        program_id,
        accounts,
        data,
    })
}
