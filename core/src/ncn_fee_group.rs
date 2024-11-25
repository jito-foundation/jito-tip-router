use bytemuck::{Pod, Zeroable};
use shank::ShankType;

use crate::error::TipRouterError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NcnFeeGroupType {
    Default = 0x0,
    JTO = 0x1,
    Reserved2 = 0x2,
    Reserved3 = 0x3,
    Reserved4 = 0x4,
    Reserved5 = 0x5,
    Reserved6 = 0x6,
    Reserved7 = 0x7,
    Reserved8 = 0x8,
    Reserved9 = 0x9,
    ReservedA = 0xA,
    ReservedB = 0xB,
    ReservedC = 0xC,
    ReservedD = 0xD,
    ReservedE = 0xE,
    ReservedF = 0xF,
}

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct NcnFeeGroup {
    pub group: u8,
}

impl Default for NcnFeeGroup {
    fn default() -> Self {
        Self {
            group: NcnFeeGroupType::Default as u8,
        }
    }
}

impl NcnFeeGroup {
    pub const FEE_GROUP_COUNT: usize = 16;

    pub const fn new(group: NcnFeeGroupType) -> Self {
        // So compiler will yell at us if we miss a group
        match group {
            NcnFeeGroupType::Default => Self { group: group as u8 },
            NcnFeeGroupType::JTO => Self { group: group as u8 },
            NcnFeeGroupType::Reserved2 => Self { group: group as u8 },
            NcnFeeGroupType::Reserved3 => Self { group: group as u8 },
            NcnFeeGroupType::Reserved4 => Self { group: group as u8 },
            NcnFeeGroupType::Reserved5 => Self { group: group as u8 },
            NcnFeeGroupType::Reserved6 => Self { group: group as u8 },
            NcnFeeGroupType::Reserved7 => Self { group: group as u8 },
            NcnFeeGroupType::Reserved8 => Self { group: group as u8 },
            NcnFeeGroupType::Reserved9 => Self { group: group as u8 },
            NcnFeeGroupType::ReservedA => Self { group: group as u8 },
            NcnFeeGroupType::ReservedB => Self { group: group as u8 },
            NcnFeeGroupType::ReservedC => Self { group: group as u8 },
            NcnFeeGroupType::ReservedD => Self { group: group as u8 },
            NcnFeeGroupType::ReservedE => Self { group: group as u8 },
            NcnFeeGroupType::ReservedF => Self { group: group as u8 },
        }
    }

    pub const fn from_u8(group: u8) -> Result<Self, TipRouterError> {
        match group {
            0x0 => Ok(Self::new(NcnFeeGroupType::Default)),
            0x1 => Ok(Self::new(NcnFeeGroupType::JTO)),
            0x2 => Ok(Self::new(NcnFeeGroupType::Reserved2)),
            0x3 => Ok(Self::new(NcnFeeGroupType::Reserved3)),
            0x4 => Ok(Self::new(NcnFeeGroupType::Reserved4)),
            0x5 => Ok(Self::new(NcnFeeGroupType::Reserved5)),
            0x6 => Ok(Self::new(NcnFeeGroupType::Reserved6)),
            0x7 => Ok(Self::new(NcnFeeGroupType::Reserved7)),
            0x8 => Ok(Self::new(NcnFeeGroupType::Reserved8)),
            0x9 => Ok(Self::new(NcnFeeGroupType::Reserved9)),
            0xA => Ok(Self::new(NcnFeeGroupType::ReservedA)),
            0xB => Ok(Self::new(NcnFeeGroupType::ReservedB)),
            0xC => Ok(Self::new(NcnFeeGroupType::ReservedC)),
            0xD => Ok(Self::new(NcnFeeGroupType::ReservedD)),
            0xE => Ok(Self::new(NcnFeeGroupType::ReservedE)),
            0xF => Ok(Self::new(NcnFeeGroupType::ReservedF)),
            _ => Err(TipRouterError::InvalidNcnFeeGroup),
        }
    }

    pub const fn group_type(&self) -> Result<NcnFeeGroupType, TipRouterError> {
        match self.group {
            0x0 => Ok(NcnFeeGroupType::Default),
            0x1 => Ok(NcnFeeGroupType::JTO),
            0x2 => Ok(NcnFeeGroupType::Reserved2),
            0x3 => Ok(NcnFeeGroupType::Reserved3),
            0x4 => Ok(NcnFeeGroupType::Reserved4),
            0x5 => Ok(NcnFeeGroupType::Reserved5),
            0x6 => Ok(NcnFeeGroupType::Reserved6),
            0x7 => Ok(NcnFeeGroupType::Reserved7),
            0x8 => Ok(NcnFeeGroupType::Reserved8),
            0x9 => Ok(NcnFeeGroupType::Reserved9),
            0xA => Ok(NcnFeeGroupType::ReservedA),
            0xB => Ok(NcnFeeGroupType::ReservedB),
            0xC => Ok(NcnFeeGroupType::ReservedC),
            0xD => Ok(NcnFeeGroupType::ReservedD),
            0xE => Ok(NcnFeeGroupType::ReservedE),
            0xF => Ok(NcnFeeGroupType::ReservedF),
            _ => Err(TipRouterError::InvalidNcnFeeGroup),
        }
    }

    pub fn group_index(&self) -> Result<usize, TipRouterError> {
        let group = self.group_type()?;
        Ok(group as usize)
    }

    pub fn all_groups() -> Vec<Self> {
        vec![
            Self::new(NcnFeeGroupType::Default),
            Self::new(NcnFeeGroupType::JTO),
            Self::new(NcnFeeGroupType::Reserved2),
            Self::new(NcnFeeGroupType::Reserved3),
            Self::new(NcnFeeGroupType::Reserved4),
            Self::new(NcnFeeGroupType::Reserved5),
            Self::new(NcnFeeGroupType::Reserved6),
            Self::new(NcnFeeGroupType::Reserved7),
            Self::new(NcnFeeGroupType::Reserved8),
            Self::new(NcnFeeGroupType::Reserved9),
            Self::new(NcnFeeGroupType::ReservedA),
            Self::new(NcnFeeGroupType::ReservedB),
            Self::new(NcnFeeGroupType::ReservedC),
            Self::new(NcnFeeGroupType::ReservedD),
            Self::new(NcnFeeGroupType::ReservedE),
            Self::new(NcnFeeGroupType::ReservedF),
        ]
    }
}
