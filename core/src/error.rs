use solana_program::{decode_error::DecodeError, program_error::ProgramError};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TipRouterError {
    #[error("Zero in the denominator")]
    DenominatorIsZero = 0x2100,
    #[error("Overflow")]
    ArithmeticOverflow,
    #[error("Underflow")]
    ArithmeticUnderflowError,
    #[error("Floor Overflow")]
    ArithmeticFloorError,
    #[error("Modulo Overflow")]
    ModuloOverflow,
    #[error("New precise number error")]
    NewPreciseNumberError,
    #[error("Cast to imprecise number error")]
    CastToImpreciseNumberError,
    #[error("Cast to u64 error")]
    CastToU64Error,
    #[error("Cast to u128 error")]
    CastToU128Error,

    #[error("Incorrect weight table admin")]
    IncorrectWeightTableAdmin = 0x2200,
    #[error("Duplicate mints in table")]
    DuplicateMintsInTable,
    #[error("There are no mints in the table")]
    NoMintsInTable,
    #[error("Table not initialized")]
    TableNotInitialized,
    #[error("Registry not initialized")]
    RegistryNotInitialized,
    #[error("There are no vaults in the registry")]
    NoVaultsInRegistry,
    #[error("Vault not in weight table registry")]
    VaultNotInRegistry,
    #[error("Mint is already in the table")]
    MintInTable,
    #[error("Too many mints for table")]
    TooManyMintsForTable,
    #[error("Too many vaults for registry")]
    TooManyVaultsForRegistry,
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
    #[error("Vault Registry mints are at capacity")]
    VaultRegistryListFull,
    #[error("Vault registry are locked for the epoch")]
    VaultRegistryVaultLocked,
    #[error("Vault index already in use by a different mint")]
    VaultIndexAlreadyInUse,
    #[error("Mint Entry not found")]
    MintEntryNotFound,
    #[error("Fee cap exceeded")]
    FeeCapExceeded,
    #[error("DAO wallet cannot be default")]
    DefaultDaoWallet,
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
    #[error("Duplicate Vote Cast")]
    DuplicateVoteCast,
    #[error("Operator votes full")]
    OperatorVotesFull,
    #[error("Merkle root tally full")]
    BallotTallyFull,
    #[error("Ballot tally not found")]
    BallotTallyNotFoundFull,
    #[error("Ballot tally not empty")]
    BallotTallyNotEmpty,
    #[error("Consensus already reached, cannot change vote")]
    ConsensusAlreadyReached,
    #[error("Consensus not reached")]
    ConsensusNotReached,

    #[error("Epoch snapshot not finalized")]
    EpochSnapshotNotFinalized,
    #[error("Voting not valid, too many slots after consensus reached")]
    VotingNotValid,
    #[error("Tie breaker admin invalid")]
    TieBreakerAdminInvalid,
    #[error("Voting not finalized")]
    VotingNotFinalized,
    #[error("Tie breaking ballot must be one of the prior votes")]
    TieBreakerNotInPriorVotes,
    #[error("Invalid merkle proof")]
    InvalidMerkleProof,
    #[error("Operator admin needs to sign its vote")]
    OperatorAdminInvalid,
    #[error("Not a valid NCN fee group")]
    InvalidNcnFeeGroup,
    #[error("Not a valid base fee group")]
    InvalidBaseFeeGroup,
    #[error("Operator reward list full")]
    OperatorRewardListFull,
    #[error("Operator Reward not found")]
    OperatorRewardNotFound,
    #[error("Vault Reward not found")]
    VaultRewardNotFound,
    #[error("Destination mismatch")]
    DestinationMismatch,
    #[error("Ncn reward route not found")]
    NcnRewardRouteNotFound,
    #[error("Fee not active")]
    FeeNotActive,
    #[error("No rewards to distribute")]
    NoRewards,
    #[error("No Feed Weight not set")]
    NoFeedWeightNotSet,
    #[error("Switchboard not registered")]
    SwitchboardNotRegistered,
    #[error("Bad switchboard feed")]
    BadSwitchboardFeed,
    #[error("Bad switchboard value")]
    BadSwitchboardValue,
    #[error("Stale switchboard feed")]
    StaleSwitchboardFeed,
    #[error("Weight entry needs either a feed or a no feed weight")]
    NoFeedWeightOrSwitchboardFeed,
    #[error("Router still routing")]
    RouterStillRouting,
    #[error("Invalid epochs before stall")]
    InvalidEpochsBeforeStall,
    #[error("Invalid slots after consensus")]
    InvalidSlotsAfterConsensus,
    #[error("Vault needs to be updated")]
    VaultNeedsUpdate,
    #[error("Invalid Account Status")]
    InvalidAccountStatus,
    #[error("Account already initialized")]
    AccountAlreadyInitialized,
    #[error("Cannot vote with uninitialized account")]
    BadBallot,
    #[error("Cannot route until voting is over")]
    VotingIsNotOver,
    #[error("Operator is not in snapshot")]
    OperatorIsNotInSnapshot,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        // Test base error codes are correct
        assert_eq!(TipRouterError::DenominatorIsZero as u32, 0x2100);
        assert_eq!(TipRouterError::IncorrectWeightTableAdmin as u32, 0x2200);

        // Test sequential error codes
        assert_eq!(
            TipRouterError::ArithmeticOverflow as u32,
            TipRouterError::DenominatorIsZero as u32 + 1
        );
        assert_eq!(
            TipRouterError::ArithmeticUnderflowError as u32,
            TipRouterError::ArithmeticOverflow as u32 + 1
        );
    }

    #[test]
    fn test_error_messages() {
        // Test error messages match their definitions
        assert_eq!(
            TipRouterError::DenominatorIsZero.to_string(),
            "Zero in the denominator"
        );
        assert_eq!(TipRouterError::ArithmeticOverflow.to_string(), "Overflow");
        assert_eq!(
            TipRouterError::WeightTableNotFinalized.to_string(),
            "Weight table not finalized"
        );
        assert_eq!(
            TipRouterError::InvalidMerkleProof.to_string(),
            "Invalid merkle proof"
        );
    }

    #[test]
    fn test_program_error_conversion() {
        // Test conversion to ProgramError
        let program_error: ProgramError = TipRouterError::DenominatorIsZero.into();
        assert_eq!(
            program_error,
            ProgramError::Custom(TipRouterError::DenominatorIsZero as u32)
        );

        let program_error: ProgramError = TipRouterError::WeightTableNotFinalized.into();
        assert_eq!(
            program_error,
            ProgramError::Custom(TipRouterError::WeightTableNotFinalized as u32)
        );
    }

    #[test]
    fn test_numeric_conversions() {
        // Test conversion to u64
        let error_u64: u64 = TipRouterError::DenominatorIsZero.into();
        assert_eq!(error_u64, 0x2100);

        // Test conversion to u32
        let error_u32: u32 = TipRouterError::DenominatorIsZero.into();
        assert_eq!(error_u32, 0x2100);

        // Test conversion for different error types
        assert_eq!(u64::from(TipRouterError::IncorrectWeightTableAdmin), 0x2200);
        assert_eq!(u32::from(TipRouterError::IncorrectWeightTableAdmin), 0x2200);
    }

    #[test]
    fn test_decode_error_type() {
        assert_eq!(
            <TipRouterError as DecodeError<ProgramError>>::type_of(),
            "jito::weight_table"
        );
    }

    #[test]
    fn test_error_equality() {
        // Test PartialEq implementation
        assert_eq!(
            TipRouterError::DenominatorIsZero,
            TipRouterError::DenominatorIsZero
        );
        assert_ne!(
            TipRouterError::DenominatorIsZero,
            TipRouterError::ArithmeticOverflow
        );

        // Test with different error variants
        assert_eq!(
            TipRouterError::WeightTableNotFinalized,
            TipRouterError::WeightTableNotFinalized
        );
        assert_ne!(
            TipRouterError::WeightTableNotFinalized,
            TipRouterError::InvalidMerkleProof
        );
    }
}
