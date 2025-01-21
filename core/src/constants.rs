use solana_program::{
    clock::DEFAULT_SLOTS_PER_EPOCH, entrypoint::MAX_PERMITTED_DATA_INCREASE, pubkey, pubkey::Pubkey,
};
use spl_math::precise_number::PreciseNumber;

use crate::error::TipRouterError;

pub const MAX_FEE_BPS: u64 = 10_000;
pub const MAX_ST_MINTS: usize = 64;
pub const MAX_VAULTS: usize = 64;
pub const MAX_OPERATORS: usize = 256;
pub const MIN_EPOCHS_BEFORE_STALL: u64 = 1;
pub const MAX_EPOCHS_BEFORE_STALL: u64 = 50;
pub const MIN_EPOCHS_AFTER_CONSENSUS_BEFORE_CLOSE: u64 = 10;
pub const MAX_EPOCHS_AFTER_CONSENSUS_BEFORE_CLOSE: u64 = 100;
pub const MIN_SLOTS_AFTER_CONSENSUS: u64 = 1000;
pub const MAX_SLOTS_AFTER_CONSENSUS: u64 = 50 * DEFAULT_SLOTS_PER_EPOCH;
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
pub const MAX_REALLOC_BYTES: u64 = MAX_PERMITTED_DATA_INCREASE as u64;

pub const WEIGHT_PRECISION: u128 = 1_000_000_000;
pub const SWITCHBOARD_MAX_STALE_SLOTS: u64 = 100;
pub const JTO_SOL_FEED: Pubkey = pubkey!("5S7ErPSkFmyXuq2aE3rZ6ofwVyZpwzUt6w7m6kqekvMe");
pub const JITOSOL_SOL_FEED: Pubkey = pubkey!("4Z1SLH9g4ikNBV8uP2ZctEouqjYmVqB2Tz5SZxKYBN7z");

pub const JITOSOL_MINT: Pubkey = pubkey!("J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn");
pub const JITOSOL_POOL_ADDRESS: Pubkey = pubkey!("Jito4APyf642JPZPx3hGc6WWJ8zPKtRbRs4P815Awbb");
pub const JITOSOL_POOL_MANAGER: Pubkey = pubkey!("5eosrve6LktMZgVNszYzebgmmC7BjLK8NoWyRQtcmGTF");
pub const JITOSOL_POOL_FEE: Pubkey = pubkey!("5eosrve6LktMZgVNszYzebgmmC7BjLK8NoWyRQtcmGTF");
pub const JITOSOL_RESERVE_STAKE: Pubkey = pubkey!("BgKUXdS29YcHCFrPm5M8oLHiTzZaMDjsebggjoaQ6KFL");
