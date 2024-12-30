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
    // Distribution
    BaseRewardRouter = 0x30,
    NcnRewardRouter = 0x31,
}
