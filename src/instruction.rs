use crate::error::CustomError;
use solana_program::program_error::ProgramError;

use std::mem::size_of;

#[repr(C)]
#[derive(Debug, PartialEq)]
pub enum InstructionType {
    CreatePool,
    Stake,
    Unstake,
    Claim,
}

impl InstructionType {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&tag, _rest) = input
            .split_first()
            .ok_or(CustomError::IncorrectInstruction)?;

        Ok(match tag {
            0x1 => Self::CreatePool,
            0x2 => Self::Stake,
            0x3 => Self::Unstake,
            0x4 => Self::Claim,
            _ => return Err(CustomError::IncorrectInstruction.into()),
        })
    }

    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(size_of::<Self>());
        match *self {
            Self::CreatePool => {
                buf.push(0x1);
            }
            Self::Stake => {
                buf.push(0x2);
            }
            Self::Unstake => {
                buf.push(0x3);
            }
            Self::Claim => {
                buf.push(0x4);
            }
        }
        buf
    }
}
