#[repr(u8)]
pub enum Discriminators {
    // Configs
    Config = 0x01,
    VaultRegistry = 0x02,

    // Snapshots
    WeightTable = 0x10,
    EpochSnapshot = 0x11,
    OperatorSnapshot = 0x12,

    // Voting
    BallotBox = 0x20,

    // Validation and Consensus
    // - Reserved for future use

    // Distribution
    BaseRewardRouter = 0x40,
    NcnRewardRouter = 0x41,

    // State Tracking
    EpochState = 0x50,
    EpochMarker = 0x51,
}
