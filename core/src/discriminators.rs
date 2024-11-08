#[repr(u8)]
pub enum Discriminators {
    NCNConfig = 1,
    WeightTable = 2,
    EpochSnapshot = 3,
    OperatorSnapshot = 4,
}
