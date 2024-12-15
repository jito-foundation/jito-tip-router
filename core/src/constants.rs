use solana_program::{entrypoint::MAX_PERMITTED_DATA_INCREASE, pubkey, pubkey::Pubkey};
use spl_math::precise_number::PreciseNumber;

use crate::error::TipRouterError;

pub const MAX_FEE_BPS: u64 = 10_000;
pub const MAX_VAULT_OPERATOR_DELEGATIONS: usize = 64;
pub const MAX_OPERATORS: usize = 256;
const PRECISE_CONSENSUS_NUMERATOR: u128 = 2;
const PRECISE_CONSENSUS_DENOMINATOR: u128 = 3;
pub fn precise_consensus() -> Result<PreciseNumber, TipRouterError> {
    PreciseNumber::new(PRECISE_CONSENSUS_NUMERATOR)
        .ok_or(TipRouterError::NewPreciseNumberError)?
        .checked_div(
            &PreciseNumber::new(PRECISE_CONSENSUS_DENOMINATOR)
                .ok_or(TipRouterError::NewPreciseNumberError)?,
        )
        .ok_or(TipRouterError::DenominatorIsZero)
}

pub const DEFAULT_CONSENSUS_REACHED_SLOT: u64 = u64::MAX;
pub const MAX_REALLOC_BYTES: u64 = MAX_PERMITTED_DATA_INCREASE as u64; // TODO just use this?

pub const WEIGHT_PRECISION: u128 = 1_000_000;
pub const DEFAULT_LST_WEIGHT: u128 = 1;

pub const DEFAULT_REWARD_MULTIPLIER_BPS: u64 = 10_000;
pub const JITO_SOL_REWARD_MULTIPLIER_BPS: u64 = 20_000;
pub const JTO_MINT: Pubkey = pubkey!("jtojtomepa8beP8AuQc6eXt5FriJwfFMwQx2v2f9mCL");
pub const JITO_SOL_MINT: Pubkey = pubkey!("J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn");

pub const MAX_STALE_SLOTS: u64 = 100;
pub const MIN_SAMPLES: u32 = 3;
pub const JTO_USD_FEED: Pubkey = pubkey!("E9fHVUZnvT4i8H3jQLb6g2tSpcagunJpjwCGNTYxwKSE");
