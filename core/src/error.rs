use solana_program::{decode_error::DecodeError, program_error::ProgramError};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TipRouterError {
    #[error("Zero in the denominator")]
    DenominatorIsZero = 0x2100,
    #[error("Overflow")]
    ArithmeticOverflow,
    #[error("Modulo Overflow")]
    ModuloOverflow,
    #[error("New precise number error")]
    NewPreciseNumberError,
    #[error("Cast to imprecise number error")]
    CastToImpreciseNumberError,

    #[error("Incorrect weight table admin")]
    IncorrectWeightTableAdmin = 0x2200,
    #[error("Duplicate mints in table")]
    DuplicateMintsInTable,
    #[error("There are no mints in the table")]
    NoMintsInTable,
    #[error("Too many mints for table")]
    TooManyMintsForTable,
    #[error("Weight table already initialized")]
    WeightTableAlreadyInitialized,
    #[error("Cannnot create future weight tables")]
    CannotCreateFutureWeightTables,
    #[error("Weight mints do not match - length")]
    WeightMintsDoNotMatchLength,
    #[error("Weight mints do not match - mint hash")]
    WeightMintsDoNotMatchMintHash,
    #[error("Invalid mint for weight table")]
    InvalidMintForWeightTable,
    #[error("Config supported mints do not match NCN Vault Count")]
    ConfigMintsNotUpdated,
    #[error("NCN config vaults are at capacity")]
    ConfigMintListFull,
    #[error("Tracked mints are at capacity")]
    TrackedMintListFull,
    #[error("Tracked mints are locked for the epoch")]
    TrackedMintsLocked,
    #[error("Vault index already in use by a different mint")]
    VaultIndexAlreadyInUse,
    #[error("Fee cap exceeded")]
    FeeCapExceeded,
    #[error("Incorrect NCN Admin")]
    IncorrectNcnAdmin,
    #[error("Incorrect NCN")]
    IncorrectNcn,
    #[error("Incorrect fee admin")]
    IncorrectFeeAdmin,
    #[error("Weight table not finalized")]
    WeightTableNotFinalized,
    #[error("Weight not found")]
    WeightNotFound,
    #[error("No operators in ncn")]
    NoOperators,
    #[error("Vault operator delegation is already finalized - should not happen")]
    VaultOperatorDelegationFinalized,
    #[error("Operator is already finalized - should not happen")]
    OperatorFinalized,
    #[error("Too many vault operator delegations")]
    TooManyVaultOperatorDelegations,
    #[error("Duplicate vault operator delegation")]
    DuplicateVaultOperatorDelegation,
}

impl<T> DecodeError<T> for TipRouterError {
    fn type_of() -> &'static str {
        "jito::weight_table"
    }
}

impl From<TipRouterError> for ProgramError {
    fn from(e: TipRouterError) -> Self {
        Self::Custom(e as u32)
    }
}

impl From<TipRouterError> for u64 {
    fn from(e: TipRouterError) -> Self {
        e as Self
    }
}

impl From<TipRouterError> for u32 {
    fn from(e: TipRouterError) -> Self {
        e as Self
    }
}
