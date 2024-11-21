#[repr(u8)]
pub enum Discriminators {
    // Configs
    NCNConfig = 0x01,
    TrackedMints = 0x02,
    // Snapshots
    WeightTable = 0x10,
    EpochSnapshot = 0x11,
    OperatorSnapshot = 0x12,
    // Voting
    BallotBox = 0x20,
    // Distribution
}
