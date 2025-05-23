//! This code was AUTOGENERATED using the kinobi library.
//! Please DO NOT EDIT THIS FILE, instead use visitors
//! to add features, then rerun kinobi to update it.
//!
//! <https://github.com/kinobi-so/kinobi>
//!

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
    /// 8456 - Cast to u128 error
    #[error("Cast to u128 error")]
    CastToU128Error = 0x2108,
    /// 8704 - Incorrect weight table admin
    #[error("Incorrect weight table admin")]
    IncorrectWeightTableAdmin = 0x2200,
    /// 8705 - Duplicate mints in table
    #[error("Duplicate mints in table")]
    DuplicateMintsInTable = 0x2201,
    /// 8706 - There are no mints in the table
    #[error("There are no mints in the table")]
    NoMintsInTable = 0x2202,
    /// 8707 - Table not initialized
    #[error("Table not initialized")]
    TableNotInitialized = 0x2203,
    /// 8708 - Registry not initialized
    #[error("Registry not initialized")]
    RegistryNotInitialized = 0x2204,
    /// 8709 - There are no vaults in the registry
    #[error("There are no vaults in the registry")]
    NoVaultsInRegistry = 0x2205,
    /// 8710 - Vault not in weight table registry
    #[error("Vault not in weight table registry")]
    VaultNotInRegistry = 0x2206,
    /// 8711 - Mint is already in the table
    #[error("Mint is already in the table")]
    MintInTable = 0x2207,
    /// 8712 - Too many mints for table
    #[error("Too many mints for table")]
    TooManyMintsForTable = 0x2208,
    /// 8713 - Too many vaults for registry
    #[error("Too many vaults for registry")]
    TooManyVaultsForRegistry = 0x2209,
    /// 8714 - Weight table already initialized
    #[error("Weight table already initialized")]
    WeightTableAlreadyInitialized = 0x220A,
    /// 8715 - Cannnot create future weight tables
    #[error("Cannnot create future weight tables")]
    CannotCreateFutureWeightTables = 0x220B,
    /// 8716 - Weight mints do not match - length
    #[error("Weight mints do not match - length")]
    WeightMintsDoNotMatchLength = 0x220C,
    /// 8717 - Weight mints do not match - mint hash
    #[error("Weight mints do not match - mint hash")]
    WeightMintsDoNotMatchMintHash = 0x220D,
    /// 8718 - Invalid mint for weight table
    #[error("Invalid mint for weight table")]
    InvalidMintForWeightTable = 0x220E,
    /// 8719 - Config supported mints do not match NCN Vault Count
    #[error("Config supported mints do not match NCN Vault Count")]
    ConfigMintsNotUpdated = 0x220F,
    /// 8720 - NCN config vaults are at capacity
    #[error("NCN config vaults are at capacity")]
    ConfigMintListFull = 0x2210,
    /// 8721 - Vault Registry mints are at capacity
    #[error("Vault Registry mints are at capacity")]
    VaultRegistryListFull = 0x2211,
    /// 8722 - Vault registry are locked for the epoch
    #[error("Vault registry are locked for the epoch")]
    VaultRegistryVaultLocked = 0x2212,
    /// 8723 - Vault index already in use by a different mint
    #[error("Vault index already in use by a different mint")]
    VaultIndexAlreadyInUse = 0x2213,
    /// 8724 - Mint Entry not found
    #[error("Mint Entry not found")]
    MintEntryNotFound = 0x2214,
    /// 8725 - Fee cap exceeded
    #[error("Fee cap exceeded")]
    FeeCapExceeded = 0x2215,
    /// 8726 - Total fees cannot be 0
    #[error("Total fees cannot be 0")]
    TotalFeesCannotBeZero = 0x2216,
    /// 8727 - DAO wallet cannot be default
    #[error("DAO wallet cannot be default")]
    DefaultDaoWallet = 0x2217,
    /// 8728 - Incorrect NCN Admin
    #[error("Incorrect NCN Admin")]
    IncorrectNcnAdmin = 0x2218,
    /// 8729 - Incorrect NCN
    #[error("Incorrect NCN")]
    IncorrectNcn = 0x2219,
    /// 8730 - Incorrect fee admin
    #[error("Incorrect fee admin")]
    IncorrectFeeAdmin = 0x221A,
    /// 8731 - Weight table not finalized
    #[error("Weight table not finalized")]
    WeightTableNotFinalized = 0x221B,
    /// 8732 - Weight not found
    #[error("Weight not found")]
    WeightNotFound = 0x221C,
    /// 8733 - No operators in ncn
    #[error("No operators in ncn")]
    NoOperators = 0x221D,
    /// 8734 - Vault operator delegation is already finalized - should not happen
    #[error("Vault operator delegation is already finalized - should not happen")]
    VaultOperatorDelegationFinalized = 0x221E,
    /// 8735 - Operator is already finalized - should not happen
    #[error("Operator is already finalized - should not happen")]
    OperatorFinalized = 0x221F,
    /// 8736 - Too many vault operator delegations
    #[error("Too many vault operator delegations")]
    TooManyVaultOperatorDelegations = 0x2220,
    /// 8737 - Duplicate vault operator delegation
    #[error("Duplicate vault operator delegation")]
    DuplicateVaultOperatorDelegation = 0x2221,
    /// 8738 - Duplicate Vote Cast
    #[error("Duplicate Vote Cast")]
    DuplicateVoteCast = 0x2222,
    /// 8739 - Operator votes full
    #[error("Operator votes full")]
    OperatorVotesFull = 0x2223,
    /// 8740 - Merkle root tally full
    #[error("Merkle root tally full")]
    BallotTallyFull = 0x2224,
    /// 8741 - Ballot tally not found
    #[error("Ballot tally not found")]
    BallotTallyNotFoundFull = 0x2225,
    /// 8742 - Ballot tally not empty
    #[error("Ballot tally not empty")]
    BallotTallyNotEmpty = 0x2226,
    /// 8743 - Consensus already reached, cannot change vote
    #[error("Consensus already reached, cannot change vote")]
    ConsensusAlreadyReached = 0x2227,
    /// 8744 - Consensus not reached
    #[error("Consensus not reached")]
    ConsensusNotReached = 0x2228,
    /// 8745 - Epoch snapshot not finalized
    #[error("Epoch snapshot not finalized")]
    EpochSnapshotNotFinalized = 0x2229,
    /// 8746 - Voting not valid, too many slots after consensus reached
    #[error("Voting not valid, too many slots after consensus reached")]
    VotingNotValid = 0x222A,
    /// 8747 - Tie breaker admin invalid
    #[error("Tie breaker admin invalid")]
    TieBreakerAdminInvalid = 0x222B,
    /// 8748 - Voting not finalized
    #[error("Voting not finalized")]
    VotingNotFinalized = 0x222C,
    /// 8749 - Tie breaking ballot must be one of the prior votes
    #[error("Tie breaking ballot must be one of the prior votes")]
    TieBreakerNotInPriorVotes = 0x222D,
    /// 8750 - Invalid merkle proof
    #[error("Invalid merkle proof")]
    InvalidMerkleProof = 0x222E,
    /// 8751 - Operator voter needs to sign its vote
    #[error("Operator voter needs to sign its vote")]
    InvalidOperatorVoter = 0x222F,
    /// 8752 - Not a valid NCN fee group
    #[error("Not a valid NCN fee group")]
    InvalidNcnFeeGroup = 0x2230,
    /// 8753 - Not a valid base fee group
    #[error("Not a valid base fee group")]
    InvalidBaseFeeGroup = 0x2231,
    /// 8754 - Operator reward list full
    #[error("Operator reward list full")]
    OperatorRewardListFull = 0x2232,
    /// 8755 - Operator Reward not found
    #[error("Operator Reward not found")]
    OperatorRewardNotFound = 0x2233,
    /// 8756 - Vault Reward not found
    #[error("Vault Reward not found")]
    VaultRewardNotFound = 0x2234,
    /// 8757 - Destination mismatch
    #[error("Destination mismatch")]
    DestinationMismatch = 0x2235,
    /// 8758 - Ncn reward route not found
    #[error("Ncn reward route not found")]
    NcnRewardRouteNotFound = 0x2236,
    /// 8759 - Fee not active
    #[error("Fee not active")]
    FeeNotActive = 0x2237,
    /// 8760 - No rewards to distribute
    #[error("No rewards to distribute")]
    NoRewards = 0x2238,
    /// 8761 - No Feed Weight not set
    #[error("No Feed Weight not set")]
    NoFeedWeightNotSet = 0x2239,
    /// 8762 - Switchboard not registered
    #[error("Switchboard not registered")]
    SwitchboardNotRegistered = 0x223A,
    /// 8763 - Bad switchboard feed
    #[error("Bad switchboard feed")]
    BadSwitchboardFeed = 0x223B,
    /// 8764 - Bad switchboard value
    #[error("Bad switchboard value")]
    BadSwitchboardValue = 0x223C,
    /// 8765 - Stale switchboard feed
    #[error("Stale switchboard feed")]
    StaleSwitchboardFeed = 0x223D,
    /// 8766 - Weight entry needs either a feed or a no feed weight
    #[error("Weight entry needs either a feed or a no feed weight")]
    NoFeedWeightOrSwitchboardFeed = 0x223E,
    /// 8767 - Router still routing
    #[error("Router still routing")]
    RouterStillRouting = 0x223F,
    /// 8768 - Invalid epochs before stall
    #[error("Invalid epochs before stall")]
    InvalidEpochsBeforeStall = 0x2240,
    /// 8769 - Invalid epochs before accounts can close
    #[error("Invalid epochs before accounts can close")]
    InvalidEpochsBeforeClose = 0x2241,
    /// 8770 - Invalid slots after consensus
    #[error("Invalid slots after consensus")]
    InvalidSlotsAfterConsensus = 0x2242,
    /// 8771 - Vault needs to be updated
    #[error("Vault needs to be updated")]
    VaultNeedsUpdate = 0x2243,
    /// 8772 - Invalid Account Status
    #[error("Invalid Account Status")]
    InvalidAccountStatus = 0x2244,
    /// 8773 - Account already initialized
    #[error("Account already initialized")]
    AccountAlreadyInitialized = 0x2245,
    /// 8774 - Cannot vote with uninitialized account
    #[error("Cannot vote with uninitialized account")]
    BadBallot = 0x2246,
    /// 8775 - Cannot route until voting is over
    #[error("Cannot route until voting is over")]
    VotingIsNotOver = 0x2247,
    /// 8776 - Operator is not in snapshot
    #[error("Operator is not in snapshot")]
    OperatorIsNotInSnapshot = 0x2248,
    /// 8777 - Invalid account_to_close Discriminator
    #[error("Invalid account_to_close Discriminator")]
    InvalidAccountToCloseDiscriminator = 0x2249,
    /// 8778 - Cannot close account
    #[error("Cannot close account")]
    CannotCloseAccount = 0x224A,
    /// 8779 - Cannot close account - Already closed
    #[error("Cannot close account - Already closed")]
    CannotCloseAccountAlreadyClosed = 0x224B,
    /// 8780 - Cannot close account - Not enough epochs have passed since consensus reached
    #[error("Cannot close account - Not enough epochs have passed since consensus reached")]
    CannotCloseAccountNotEnoughEpochs = 0x224C,
    /// 8781 - Cannot close account - No receiver provided
    #[error("Cannot close account - No receiver provided")]
    CannotCloseAccountNoReceiverProvided = 0x224D,
    /// 8782 - Cannot close epoch state account - Epoch state needs all other accounts to be closed first
    #[error("Cannot close epoch state account - Epoch state needs all other accounts to be closed first")]
    CannotCloseEpochStateAccount = 0x224E,
    /// 8783 - Invalid DAO wallet
    #[error("Invalid DAO wallet")]
    InvalidDaoWallet = 0x224F,
    /// 8784 - Epoch is closing down
    #[error("Epoch is closing down")]
    EpochIsClosingDown = 0x2250,
    /// 8785 - Marker exists
    #[error("Marker exists")]
    MarkerExists = 0x2251,
}

impl solana_program::program_error::PrintProgramError for JitoTipRouterError {
    fn print<E>(&self) {
        solana_program::msg!(&self.to_string());
    }
}
