use solana_program::entrypoint::MAX_PERMITTED_DATA_INCREASE;
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
