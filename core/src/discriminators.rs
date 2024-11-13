#[repr(u8)]
pub enum Discriminators {
    NCNConfig = 1,
    WeightTable = 2,
    TrackedMints = 3,
    EpochSnapshot = 4,
    OperatorSnapshot = 5,
    VaultOperatorDelegationSnapshot = 6,
}
