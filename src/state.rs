use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    clock::UnixTimestamp,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::{Pubkey, PUBKEY_BYTES},
};

use std::convert::TryFrom;

use crate::error::CustomError;

#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pool {
    /// Initialization state
    pub is_initialized: bool,
    /// bump_seed to generate program authority
    pub bump_seed: u8,
    /// spl token mint to be staked
    pub stake_token_mint: Pubkey,
    /// Reserved token account
    pub reserved: Pubkey,
    /// spl token mint to be minted
    pub reward_mint: Pubkey,
    /// Daily reward ratio numerator
    pub reward_numerator: u64,
    /// Daily reward ratio denominator
    pub reward_denominator: u64,
}

impl Sealed for Pool {}
impl IsInitialized for Pool {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

const POOL_SIZE: usize = 114; // 1 + 1 + 32 + 32 + 32 + 8 + 8

impl Pack for Pool {
    const LEN: usize = POOL_SIZE;

    fn unpack_from_slice(src: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
        let input = array_ref![src, 0, POOL_SIZE];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            is_initialized,
            bump_seed,
            stake_token_mint,
            reserved,
            reward_mint,
            reward_numerator,
            reward_denominator,
        ) = array_refs![input, 1, 1, PUBKEY_BYTES, PUBKEY_BYTES, PUBKEY_BYTES, 8, 8];

        Ok(Self {
            is_initialized: unpack_bool(is_initialized)?,
            bump_seed: u8::from_le_bytes(*bump_seed),
            stake_token_mint: Pubkey::new_from_array(*stake_token_mint),
            reserved: Pubkey::new_from_array(*reserved),
            reward_mint: Pubkey::new_from_array(*reward_mint),
            reward_numerator: u64::from_le_bytes(*reward_numerator),
            reward_denominator: u64::from_le_bytes(*reward_denominator),
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let output = array_mut_ref![dst, 0, POOL_SIZE];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            is_initialized,
            bump_seed,
            stake_token_mint,
            reserved,
            reward_mint,
            reward_numerator,
            reward_denominator,
        ) = mut_array_refs![output, 1, 1, PUBKEY_BYTES, PUBKEY_BYTES, PUBKEY_BYTES, 8, 8];

        pack_bool(self.is_initialized, is_initialized);
        *bump_seed = self.bump_seed.to_le_bytes();
        stake_token_mint.copy_from_slice(self.stake_token_mint.as_ref());
        reserved.copy_from_slice(self.reserved.as_ref());
        reward_mint.copy_from_slice(self.reward_mint.as_ref());
        *reward_numerator = self.reward_numerator.to_le_bytes();
        *reward_denominator = self.reward_denominator.to_le_bytes();
    }
}

#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StakeUser {
    /// Initialization state
    pub is_initialized: bool,
    /// Owner pubkey related to user's wallet
    pub owner: Pubkey,
    /// Stake Pool pubkey
    pub pool_pubkey: Pubkey,
    /// Amount staked
    pub stake_amount: u64,
    /// Reward amount owed
    pub reward_owed: u64,
    /// Last update timestamp
    pub last_update: UnixTimestamp,
}

impl Sealed for StakeUser {}
impl IsInitialized for StakeUser {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

const STAKE_USER_SIZE: usize = 89; // 1 + 32 + 32 + 8 + 8 + 8

impl Pack for StakeUser {
    const LEN: usize = STAKE_USER_SIZE;

    fn unpack_from_slice(src: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
        let input = array_ref![src, 0, STAKE_USER_SIZE];
        #[allow(clippy::ptr_offset_with_cast)]
        let (is_initialized, owner, pool_pubkey, stake_amount, reward_owed, last_update) =
            array_refs![input, 1, PUBKEY_BYTES, PUBKEY_BYTES, 8, 8, 8];

        Ok(Self {
            is_initialized: unpack_bool(is_initialized)?,
            owner: Pubkey::new_from_array(*owner),
            pool_pubkey: Pubkey::new_from_array(*pool_pubkey),
            stake_amount: u64::from_le_bytes(*stake_amount),
            reward_owed: u64::from_le_bytes(*reward_owed),
            last_update: i64::from_le_bytes(*last_update),
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let output = array_mut_ref![dst, 0, STAKE_USER_SIZE];
        #[allow(clippy::ptr_offset_with_cast)]
        let (is_initialized, owner, pool_pubkey, stake_amount, reward_owed, last_update) =
            mut_array_refs![output, 1, PUBKEY_BYTES, PUBKEY_BYTES, 8, 8, 8];

        pack_bool(self.is_initialized, is_initialized);
        owner.copy_from_slice(self.owner.as_ref());
        pool_pubkey.copy_from_slice(self.pool_pubkey.as_ref());
        *stake_amount = self.stake_amount.to_le_bytes();
        *reward_owed = self.reward_owed.to_le_bytes();
        *last_update = self.last_update.to_le_bytes();
    }
}

const DAILY_TS: i64 = 86_400;

pub struct InitStakeUserParams {
    pub pool_pubkey: Pubkey,
    pub owner: Pubkey,
}

impl StakeUser {
    pub fn init(&mut self, params: InitStakeUserParams) {
        self.is_initialized = true;
        self.pool_pubkey = params.pool_pubkey;
        self.owner = params.owner;
    }

    pub fn stake(&mut self, amount: u64) -> ProgramResult {
        self.stake_amount = self
            .stake_amount
            .checked_add(amount)
            .ok_or(CustomError::CalculationFailure)?;
        Ok(())
    }

    pub fn unstake(&mut self, amount: u64) -> ProgramResult {
        if amount > self.stake_amount {
            return Err(CustomError::InsufficientLiquidity.into());
        }
        self.stake_amount = self
            .stake_amount
            .checked_sub(amount)
            .ok_or(CustomError::CalculationFailure)?;
        Ok(())
    }

    pub fn update_reward_owed(
        &mut self,
        numerator: u64,
        denominator: u64,
        current_ts: UnixTimestamp,
    ) -> ProgramResult {
        let calc_period = current_ts
            .checked_sub(self.last_update)
            .ok_or(CustomError::CalculationFailure)?;
        if calc_period > 0 {
            self.reward_owed = numerator
                .checked_mul(self.stake_amount)
                .ok_or(CustomError::CalculationFailure)?
                .checked_div(denominator)
                .ok_or(CustomError::CalculationFailure)?
                .checked_mul(u64::try_from(calc_period).unwrap())
                .ok_or(CustomError::CalculationFailure)?
                .checked_div(u64::try_from(DAILY_TS).unwrap())
                .ok_or(CustomError::CalculationFailure)?
                .checked_add(self.reward_owed)
                .ok_or(CustomError::CalculationFailure)?;

            self.last_update = current_ts;
        }
        Ok(())
    }

    pub fn claim(&mut self) -> Result<u64, ProgramError> {
        if self.reward_owed == 0 {
            return Err(CustomError::InsufficientClaimAmount.into());
        }
        let ret = self.reward_owed;
        self.reward_owed = 0;
        Ok(ret)
    }
}

pub fn pack_bool(boolean: bool, dst: &mut [u8; 1]) {
    *dst = (boolean as u8).to_le_bytes()
}

pub fn unpack_bool(src: &[u8; 1]) -> Result<bool, ProgramError> {
    match u8::from_le_bytes(*src) {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(ProgramError::InvalidAccountData),
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_pool_packing() {
        let is_initialized = true;
        let bump_seed: u8 = 255;
        let stake_token_mint_key_raw = [1u8; 32];
        let reserved_key_raw = [2u8; 32];
        let reward_mint_key_raw = [3u8; 32];
        let stake_token_mint = Pubkey::new_from_array(stake_token_mint_key_raw);
        let reserved = Pubkey::new_from_array(reserved_key_raw);
        let reward_mint = Pubkey::new_from_array(reward_mint_key_raw);
        let reward_numerator: u64 = 1;
        let reward_denominator: u64 = 1_000;

        let pool = Pool {
            is_initialized,
            bump_seed,
            stake_token_mint,
            reserved,
            reward_mint,
            reward_numerator,
            reward_denominator,
        };

        let mut packed = [0u8; Pool::LEN];
        Pool::pack_into_slice(&pool, &mut packed);
        let unpacked = Pool::unpack(&packed).unwrap();
        assert_eq!(pool, unpacked);
    }

    #[test]
    fn test_stake_user_packing() {
        let is_initialized = true;
        let owner_key_raw = [1u8; 32];
        let pool_pubkey_raw = [2u8; 32];
        let owner = Pubkey::new_from_array(owner_key_raw);
        let pool_pubkey = Pubkey::new_from_array(pool_pubkey_raw);
        let stake_amount: u64 = 10_000_000_000; // Decimal = 9
        let reward_owed: u64 = 100_000_000;
        let last_update: UnixTimestamp = 100;

        let stake_user = StakeUser {
            is_initialized,
            owner,
            pool_pubkey,
            stake_amount,
            reward_owed,
            last_update,
        };

        let mut packed = [0u8; StakeUser::LEN];
        StakeUser::pack_into_slice(&stake_user, &mut packed);
        let unpacked = StakeUser::unpack(&packed).unwrap();
        assert_eq!(stake_user, unpacked);
    }
}
