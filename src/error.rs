use num_derive::FromPrimitive;
use solana_program::{
    decode_error::DecodeError,
    msg,
    program_error::{PrintProgramError, ProgramError},
};
use thiserror::Error;

#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum CustomError {
    #[error("Incorrect program instruction")]
    IncorrectInstruction,
    #[error("Instruction unpack is failed")]
    InstructionUnpackError,
    #[error("CalculationFailure")]
    CalculationFailure,
    #[error("Insufficient liquidity available")]
    InsufficientLiquidity,
    #[error("Insufficient claim amount")]
    InsufficientClaimAmount,
    #[error("Lamport balance below rent-exempt threshold")]
    NotRentExempt,
    #[error("Stake pool is already initialized")]
    AlreadyInUse,
    #[error("Input account owner is not the program address")]
    InvalidAccountOwner,
    #[error("Token initialize account failed")]
    TokenInitializeAccountFailed,
    #[error("Token initialize mint failed")]
    TokenInitializeMintFailed,
    #[error("Pool authority is invalid")]
    InvalidPoolAuthority,
    #[error("Input token mint account is not valid")]
    InvalidTokenMint,
    #[error("Input token account is not valid")]
    InvalidTokenAccount,
    #[error("Token account has a close authority")]
    InvalidCloseAuthority,
    #[error("Pool token mint has a freeze authority")]
    InvalidFreezeAuthority,
    #[error("Token account has a delegate")]
    InvalidDelegate,
    #[error("Input account must be signer")]
    InvalidSigner,
    #[error("Invalid stake user owner")]
    InvalidStakeOwner,
    #[error("Insufficient funds")]
    InsufficientFunds,
    #[error("Token transfer failed")]
    TokenTransferFailed,
    #[error("Token mint to failed")]
    TokenMintToFailed,
}

impl From<CustomError> for ProgramError {
    fn from(e: CustomError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for CustomError {
    fn type_of() -> &'static str {
        "Reward pool error"
    }
}

impl PrintProgramError for CustomError {
    fn print<E>(&self)
    where
        E: 'static
            + std::error::Error
            + DecodeError<E>
            + PrintProgramError
            + num_traits::FromPrimitive,
    {
        match self {
            CustomError::IncorrectInstruction => msg!("Error: Incorrect program instruction"),
            CustomError::InstructionUnpackError => msg!("Error: Instruction unpacking is failed"),
            CustomError::CalculationFailure => msg!("Error: Failed calculation"),
            CustomError::InsufficientLiquidity => msg!("Error: Insufficient liquidity available"),
            CustomError::InsufficientClaimAmount => msg!("Error: No rewards to claim"),
            CustomError::NotRentExempt => {
                msg!("Error: Lamport balance below rent-exempt threshold")
            }
            CustomError::AlreadyInUse => msg!("Error: Stake pool is already initialized"),
            CustomError::InvalidAccountOwner => {
                msg!("Error: Input account owner is not the program address")
            }
            CustomError::TokenInitializeAccountFailed => {
                msg!("Error: Token initialize account failed")
            }
            CustomError::TokenInitializeMintFailed => msg!("Error: Token initialize mint failed"),
            CustomError::InvalidPoolAuthority => msg!("Error: Pool authority is invalid"),
            CustomError::InvalidTokenMint => msg!("Error: Input token mint account is not valid"),
            CustomError::InvalidTokenAccount => msg!("Error: Input token account is not valid"),
            CustomError::InvalidDelegate => msg!("Error: Token account has a delegate"),
            CustomError::InvalidCloseAuthority => {
                msg!("Error: Token account has a close authority")
            }
            CustomError::InvalidFreezeAuthority => {
                msg!("Error: Pool token mint has a freeze authority")
            }
            CustomError::InvalidSigner => {
                msg!("Error: Ivanlid signer account provided")
            }
            CustomError::InvalidStakeOwner => msg!("Error: Invalid stake user owner"),
            CustomError::InsufficientFunds => msg!("Error: Insufficient funds"),
            CustomError::TokenTransferFailed => msg!("Error: Token transfer failed"),
            CustomError::TokenMintToFailed => msg!("Error: Token mint to failed"),
        }
    }
}
