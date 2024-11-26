//! This code was AUTOGENERATED using the kinobi library.
//! Please DO NOT EDIT THIS FILE, instead use visitors
//! to add features, then rerun kinobi to update it.
//!
//! <https://github.com/kinobi-so/kinobi>

use num_derive::FromPrimitive;
use thiserror::Error;

#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum JitoTipRouterError {
    /// 8448 - Zero in the denominator
    #[error("Zero in the denominator")]
    DenominatorIsZero = 0x2100,
    /// 8449 - Overflow
    #[error("Overflow")]
    ArithmeticOverflow = 0x2101,
    /// 8450 - Underflow
    #[error("Underflow")]
    ArithmeticUnderflowError = 0x2102,
    /// 8451 - Floor Overflow
    #[error("Floor Overflow")]
    ArithmeticFloorError = 0x2103,
    /// 8452 - Modulo Overflow
    #[error("Modulo Overflow")]
    ModuloOverflow = 0x2104,
    /// 8453 - New precise number error
    #[error("New precise number error")]
    NewPreciseNumberError = 0x2105,
    /// 8454 - Cast to imprecise number error
    #[error("Cast to imprecise number error")]
    CastToImpreciseNumberError = 0x2106,
    /// 8455 - Cast to u64 error
    #[error("Cast to u64 error")]
    CastToU64Error = 0x2107,
    /// 8704 - Incorrect weight table admin
    #[error("Incorrect weight table admin")]
    IncorrectWeightTableAdmin = 0x2200,
    /// 8705 - Duplicate mints in table
    #[error("Duplicate mints in table")]
    DuplicateMintsInTable = 0x2201,
    /// 8706 - There are no mints in the table
    #[error("There are no mints in the table")]
    NoMintsInTable = 0x2202,
    /// 8707 - Too many mints for table
    #[error("Too many mints for table")]
    TooManyMintsForTable = 0x2203,
    /// 8708 - Weight table already initialized
    #[error("Weight table already initialized")]
    WeightTableAlreadyInitialized = 0x2204,
    /// 8709 - Cannnot create future weight tables
    #[error("Cannnot create future weight tables")]
    CannotCreateFutureWeightTables = 0x2205,
    /// 8710 - Weight mints do not match - length
    #[error("Weight mints do not match - length")]
    WeightMintsDoNotMatchLength = 0x2206,
    /// 8711 - Weight mints do not match - mint hash
    #[error("Weight mints do not match - mint hash")]
    WeightMintsDoNotMatchMintHash = 0x2207,
    /// 8712 - Invalid mint for weight table
    #[error("Invalid mint for weight table")]
    InvalidMintForWeightTable = 0x2208,
    /// 8713 - Config supported mints do not match NCN Vault Count
    #[error("Config supported mints do not match NCN Vault Count")]
    ConfigMintsNotUpdated = 0x2209,
    /// 8714 - NCN config vaults are at capacity
    #[error("NCN config vaults are at capacity")]
    ConfigMintListFull = 0x220A,
    /// 8715 - Tracked mints are at capacity
    #[error("Tracked mints are at capacity")]
    TrackedMintListFull = 0x220B,
    /// 8716 - Tracked mints are locked for the epoch
    #[error("Tracked mints are locked for the epoch")]
    TrackedMintsLocked = 0x220C,
    /// 8717 - Vault index already in use by a different mint
    #[error("Vault index already in use by a different mint")]
    VaultIndexAlreadyInUse = 0x220D,
    /// 8718 - Mint Entry not found
    #[error("Mint Entry not found")]
    MintEntryNotFound = 0x220E,
    /// 8719 - Fee cap exceeded
    #[error("Fee cap exceeded")]
    FeeCapExceeded = 0x220F,
    /// 8720 - Incorrect NCN Admin
    #[error("Incorrect NCN Admin")]
    IncorrectNcnAdmin = 0x2210,
    /// 8721 - Incorrect NCN
    #[error("Incorrect NCN")]
    IncorrectNcn = 0x2211,
    /// 8722 - Incorrect fee admin
    #[error("Incorrect fee admin")]
    IncorrectFeeAdmin = 0x2212,
    /// 8723 - Weight table not finalized
    #[error("Weight table not finalized")]
    WeightTableNotFinalized = 0x2213,
    /// 8724 - Weight not found
    #[error("Weight not found")]
    WeightNotFound = 0x2214,
    /// 8725 - No operators in ncn
    #[error("No operators in ncn")]
    NoOperators = 0x2215,
    /// 8726 - Vault operator delegation is already finalized - should not happen
    #[error("Vault operator delegation is already finalized - should not happen")]
    VaultOperatorDelegationFinalized = 0x2216,
    /// 8727 - Operator is already finalized - should not happen
    #[error("Operator is already finalized - should not happen")]
    OperatorFinalized = 0x2217,
    /// 8728 - Too many vault operator delegations
    #[error("Too many vault operator delegations")]
    TooManyVaultOperatorDelegations = 0x2218,
    /// 8729 - Duplicate vault operator delegation
    #[error("Duplicate vault operator delegation")]
    DuplicateVaultOperatorDelegation = 0x2219,
    /// 8730 - Duplicate Vote Cast
    #[error("Duplicate Vote Cast")]
    DuplicateVoteCast = 0x221A,
    /// 8731 - Operator votes full
    #[error("Operator votes full")]
    OperatorVotesFull = 0x221B,
    /// 8732 - Merkle root tally full
    #[error("Merkle root tally full")]
    BallotTallyFull = 0x221C,
    /// 8733 - Consensus already reached
    #[error("Consensus already reached")]
    ConsensusAlreadyReached = 0x221D,
    /// 8734 - Consensus not reached
    #[error("Consensus not reached")]
    ConsensusNotReached = 0x221E,
    /// 8735 - Not a valid NCN fee group
    #[error("Not a valid NCN fee group")]
    InvalidNcnFeeGroup = 0x221F,
    /// 8736 - Operator reward list full
    #[error("Operator reward list full")]
    OperatorRewardListFull = 0x2220,
    /// 8737 - Operator Reward not found
    #[error("Operator Reward not found")]
    OperatorRewardNotFound = 0x2221,
}

impl solana_program::program_error::PrintProgramError for JitoTipRouterError {
    fn print<E>(&self) {
        solana_program::msg!(&self.to_string());
    }
}
