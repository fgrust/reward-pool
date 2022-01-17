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
    #[error("CalculationFailure")]
    CalculationFailure,
    #[error("Insufficient liquidity available")]
    InsufficientLiquidity,
    /// Insufficient claim amount
    #[error("Insufficient claim amount")]
    InsufficientClaimAmount,
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
            CustomError::CalculationFailure => msg!("Error: Failed calculation"),
            CustomError::InsufficientLiquidity => msg!("Error: Insufficient liquidity available"),
            CustomError::InsufficientClaimAmount => msg!("Error: No rewards to claim"),
        }
    }
}
