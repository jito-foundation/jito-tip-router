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
    /// 8450 - Modulo Overflow
    #[error("Modulo Overflow")]
    ModuloOverflow = 0x2102,
    /// 8451 - New precise number error
    #[error("New precise number error")]
    NewPreciseNumberError = 0x2103,
    /// 8452 - Cast to imprecise number error
    #[error("Cast to imprecise number error")]
    CastToImpreciseNumberError = 0x2104,
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
    /// 8718 - Fee cap exceeded
    #[error("Fee cap exceeded")]
    FeeCapExceeded = 0x220E,
    /// 8719 - Incorrect NCN Admin
    #[error("Incorrect NCN Admin")]
    IncorrectNcnAdmin = 0x220F,
    /// 8720 - Incorrect NCN
    #[error("Incorrect NCN")]
    IncorrectNcn = 0x2210,
    /// 8721 - Incorrect fee admin
    #[error("Incorrect fee admin")]
    IncorrectFeeAdmin = 0x2211,
    /// 8722 - Weight table not finalized
    #[error("Weight table not finalized")]
    WeightTableNotFinalized = 0x2212,
    /// 8723 - Weight not found
    #[error("Weight not found")]
    WeightNotFound = 0x2213,
    /// 8724 - No operators in ncn
    #[error("No operators in ncn")]
    NoOperators = 0x2214,
    /// 8725 - Vault operator delegation is already finalized - should not happen
    #[error("Vault operator delegation is already finalized - should not happen")]
    VaultOperatorDelegationFinalized = 0x2215,
    /// 8726 - Operator is already finalized - should not happen
    #[error("Operator is already finalized - should not happen")]
    OperatorFinalized = 0x2216,
}

impl solana_program::program_error::PrintProgramError for JitoTipRouterError {
    fn print<E>(&self) {
        solana_program::msg!(&self.to_string());
    }
}
