use crate::error::TipRouterError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RewardType {
    DAO = 0x0,
    NCN = 0x1,
    Operator = 0x2,
    Vault = 0x3,
}

impl TryFrom<u8> for RewardType {
    type Error = TipRouterError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x0 => Ok(RewardType::DAO),
            0x1 => Ok(RewardType::NCN),
            0x2 => Ok(RewardType::Operator),
            0x3 => Ok(RewardType::Vault),
            _ => Err(TipRouterError::InvalidNcnFeeGroup),
        }
    }
}
