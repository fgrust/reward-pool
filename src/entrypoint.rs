#![cfg(all(target_arch = "bpf", not(feature = "no-entrypoint")))]

use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult,
    program_error::PrintProgramError, pubkey::Pubkey,
};

use crate::{error::CustomError, processor::process};

entrypoint!(process_instruction);
fn process_instruction<'a>(
    program_id: &Pubkey,
    accounts: &'a [AccountInfo<'a>],
    instruction_data: &[u8],
) -> ProgramResult {
    if let Err(error) = process(program_id, accounts, instruction_data) {
        // catch the error so we can print it
        error.print::<CustomError>();
        return Err(error);
    }
    Ok(())
}
