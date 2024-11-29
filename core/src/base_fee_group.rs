use bytemuck::{Pod, Zeroable};
use shank::ShankType;

use crate::error::TipRouterError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BaseFeeGroupType {
    DAO = 0x0, // 270
    Reserved1 = 0x1,
    Reserved2 = 0x2,
    Reserved3 = 0x3,
    Reserved4 = 0x4,
    Reserved5 = 0x5,
    Reserved6 = 0x6,
    Reserved7 = 0x7,
}

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct BaseFeeGroup {
    pub group: u8,
}

impl Default for BaseFeeGroup {
    fn default() -> Self {
        Self {
            group: BaseFeeGroupType::DAO as u8,
        }
    }
}

impl TryFrom<u8> for BaseFeeGroup {
    type Error = TipRouterError;

    fn try_from(group: u8) -> Result<Self, Self::Error> {
        match group {
            0x0 => Ok(Self::new(BaseFeeGroupType::DAO)),
            0x1 => Ok(Self::new(BaseFeeGroupType::Reserved1)),
            0x2 => Ok(Self::new(BaseFeeGroupType::Reserved2)),
            0x3 => Ok(Self::new(BaseFeeGroupType::Reserved3)),
            0x4 => Ok(Self::new(BaseFeeGroupType::Reserved4)),
            0x5 => Ok(Self::new(BaseFeeGroupType::Reserved5)),
            0x6 => Ok(Self::new(BaseFeeGroupType::Reserved6)),
            0x7 => Ok(Self::new(BaseFeeGroupType::Reserved7)),
            _ => Err(TipRouterError::InvalidBaseFeeGroup),
        }
    }
}

impl BaseFeeGroup {
    pub const FEE_GROUP_COUNT: usize = 8;

    pub const fn new(group: BaseFeeGroupType) -> Self {
        // So compiler will yell at us if we miss a group
        match group {
            BaseFeeGroupType::DAO => Self { group: group as u8 },
            BaseFeeGroupType::Reserved1 => Self { group: group as u8 },
            BaseFeeGroupType::Reserved2 => Self { group: group as u8 },
            BaseFeeGroupType::Reserved3 => Self { group: group as u8 },
            BaseFeeGroupType::Reserved4 => Self { group: group as u8 },
            BaseFeeGroupType::Reserved5 => Self { group: group as u8 },
            BaseFeeGroupType::Reserved6 => Self { group: group as u8 },
            BaseFeeGroupType::Reserved7 => Self { group: group as u8 },
        }
    }

    pub const fn group_type(&self) -> Result<BaseFeeGroupType, TipRouterError> {
        match self.group {
            0x0 => Ok(BaseFeeGroupType::DAO),
            0x1 => Ok(BaseFeeGroupType::Reserved1),
            0x2 => Ok(BaseFeeGroupType::Reserved2),
            0x3 => Ok(BaseFeeGroupType::Reserved3),
            0x4 => Ok(BaseFeeGroupType::Reserved4),
            0x5 => Ok(BaseFeeGroupType::Reserved5),
            0x6 => Ok(BaseFeeGroupType::Reserved6),
            0x7 => Ok(BaseFeeGroupType::Reserved7),
            _ => Err(TipRouterError::InvalidNcnFeeGroup),
        }
    }

    pub fn group_index(&self) -> Result<usize, TipRouterError> {
        let group = self.group_type()?;
        Ok(group as usize)
    }

    pub fn all_groups() -> Vec<Self> {
        vec![
            Self::new(BaseFeeGroupType::DAO),
            Self::new(BaseFeeGroupType::Reserved1),
            Self::new(BaseFeeGroupType::Reserved2),
            Self::new(BaseFeeGroupType::Reserved3),
            Self::new(BaseFeeGroupType::Reserved4),
            Self::new(BaseFeeGroupType::Reserved5),
            Self::new(BaseFeeGroupType::Reserved6),
            Self::new(BaseFeeGroupType::Reserved7),
        ]
    }
}
