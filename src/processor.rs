use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::instruction::{InitData, InstructionType, StakeData};

pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
    let instruction = InstructionType::unpack(input)?;

    match instruction {
        InstructionType::CreatePool(init_data) => {
            process_create_stake_pool(program_id, accounts, init_data)
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

pub fn process_create_stake_pool(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    init_data: InitData,
) -> ProgramResult {
    Ok(())
}

pub fn process_create_stake_user(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    Ok(())
}

pub fn process_stake(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    Ok(())
}

pub fn process_unstake(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    Ok(())
}

pub fn process_claim(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    Ok(())
}

pub fn process_refresh(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    Ok(())
}
