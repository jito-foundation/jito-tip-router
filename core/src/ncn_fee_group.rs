use bytemuck::{Pod, Zeroable};
use shank::ShankType;

use crate::error::TipRouterError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NcnFeeGroupType {
    Default = 0x0, //0.15
    JTO = 0x1,     //0.15
    Reserved2 = 0x2,
    Reserved3 = 0x3,
    Reserved4 = 0x4,
    Reserved5 = 0x5,
    Reserved6 = 0x6,
    Reserved7 = 0x7,
}

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, PartialEq, Eq)]
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

impl TryFrom<u8> for NcnFeeGroup {
    type Error = TipRouterError;

    fn try_from(group: u8) -> Result<Self, Self::Error> {
        match group {
            0x0 => Ok(Self::new(NcnFeeGroupType::Default)),
            0x1 => Ok(Self::new(NcnFeeGroupType::JTO)),
            0x2 => Ok(Self::new(NcnFeeGroupType::Reserved2)),
            0x3 => Ok(Self::new(NcnFeeGroupType::Reserved3)),
            0x4 => Ok(Self::new(NcnFeeGroupType::Reserved4)),
            0x5 => Ok(Self::new(NcnFeeGroupType::Reserved5)),
            0x6 => Ok(Self::new(NcnFeeGroupType::Reserved6)),
            0x7 => Ok(Self::new(NcnFeeGroupType::Reserved7)),
            _ => Err(TipRouterError::InvalidNcnFeeGroup),
        }
    }
}

impl NcnFeeGroup {
    pub const FEE_GROUP_COUNT: usize = 8;

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
        }
    }

    pub const fn lst() -> Self {
        Self::new(NcnFeeGroupType::Default)
    }

    pub const fn jto() -> Self {
        Self::new(NcnFeeGroupType::JTO)
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
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ncn_fee_group_type_values() {
        // Verify enum values match expected u8 values
        assert_eq!(NcnFeeGroupType::Default as u8, 0x0);
        assert_eq!(NcnFeeGroupType::JTO as u8, 0x1);
        assert_eq!(NcnFeeGroupType::Reserved2 as u8, 0x2);
        assert_eq!(NcnFeeGroupType::Reserved3 as u8, 0x3);
        assert_eq!(NcnFeeGroupType::Reserved4 as u8, 0x4);
        assert_eq!(NcnFeeGroupType::Reserved5 as u8, 0x5);
        assert_eq!(NcnFeeGroupType::Reserved6 as u8, 0x6);
        assert_eq!(NcnFeeGroupType::Reserved7 as u8, 0x7);
    }

    #[test]
    fn test_ncn_fee_group_default() {
        let default_group = NcnFeeGroup::default();
        assert_eq!(default_group.group, NcnFeeGroupType::Default as u8);

        // Verify default group type conversion
        assert!(matches!(
            default_group.group_type().unwrap(),
            NcnFeeGroupType::Default
        ));
    }

    #[test]
    fn test_ncn_fee_group_try_from() {
        // Test valid conversions
        assert!(matches!(
            NcnFeeGroup::try_from(0x0).unwrap().group_type().unwrap(),
            NcnFeeGroupType::Default
        ));
        assert!(matches!(
            NcnFeeGroup::try_from(0x1).unwrap().group_type().unwrap(),
            NcnFeeGroupType::JTO
        ));

        // Test all reserved groups
        for i in 2..=7 {
            assert!(NcnFeeGroup::try_from(i).is_ok());
        }

        // Test invalid values
        assert!(matches!(
            NcnFeeGroup::try_from(8),
            Err(TipRouterError::InvalidNcnFeeGroup)
        ));
        assert!(matches!(
            NcnFeeGroup::try_from(255),
            Err(TipRouterError::InvalidNcnFeeGroup)
        ));
    }

    #[test]
    fn test_ncn_fee_group_new() {
        // Test creation of all group types
        let default_group = NcnFeeGroup::new(NcnFeeGroupType::Default);
        assert_eq!(default_group.group, 0x0);

        let jto_group = NcnFeeGroup::new(NcnFeeGroupType::JTO);
        assert_eq!(jto_group.group, 0x1);

        // Test all reserved groups
        let reserved_groups = [
            NcnFeeGroup::new(NcnFeeGroupType::Reserved2),
            NcnFeeGroup::new(NcnFeeGroupType::Reserved3),
            NcnFeeGroup::new(NcnFeeGroupType::Reserved4),
            NcnFeeGroup::new(NcnFeeGroupType::Reserved5),
            NcnFeeGroup::new(NcnFeeGroupType::Reserved6),
            NcnFeeGroup::new(NcnFeeGroupType::Reserved7),
        ];

        for (i, group) in reserved_groups.iter().enumerate() {
            assert_eq!(group.group as usize, i + 2);
        }
    }

    #[test]
    fn test_group_type_conversion() {
        // Test valid conversions
        let test_cases = [
            (0x0, NcnFeeGroupType::Default),
            (0x1, NcnFeeGroupType::JTO),
            (0x2, NcnFeeGroupType::Reserved2),
            (0x3, NcnFeeGroupType::Reserved3),
            (0x4, NcnFeeGroupType::Reserved4),
            (0x5, NcnFeeGroupType::Reserved5),
            (0x6, NcnFeeGroupType::Reserved6),
            (0x7, NcnFeeGroupType::Reserved7),
        ];

        for (value, expected_type) in test_cases {
            let group = NcnFeeGroup { group: value };
            assert_eq!(group.group_type().unwrap(), expected_type);
        }

        // Test invalid conversion
        let invalid_group = NcnFeeGroup { group: 8 };
        assert!(matches!(
            invalid_group.group_type(),
            Err(TipRouterError::InvalidNcnFeeGroup)
        ));
    }

    #[test]
    fn test_group_index() {
        // Test all valid indices
        for i in 0..NcnFeeGroup::FEE_GROUP_COUNT {
            let group = NcnFeeGroup { group: i as u8 };
            assert_eq!(group.group_index().unwrap(), i);
        }

        // Test invalid index
        let invalid_group = NcnFeeGroup { group: 8 };
        assert!(matches!(
            invalid_group.group_index(),
            Err(TipRouterError::InvalidNcnFeeGroup)
        ));
    }
    #[test]
    fn test_all_groups() {
        let all_groups = NcnFeeGroup::all_groups();

        // Verify count matches FEE_GROUP_COUNT
        assert_eq!(all_groups.len(), NcnFeeGroup::FEE_GROUP_COUNT);

        // Verify groups are exactly as expected
        let expected_groups = vec![
            NcnFeeGroup::new(NcnFeeGroupType::Default),
            NcnFeeGroup::new(NcnFeeGroupType::JTO),
            NcnFeeGroup::new(NcnFeeGroupType::Reserved2),
            NcnFeeGroup::new(NcnFeeGroupType::Reserved3),
            NcnFeeGroup::new(NcnFeeGroupType::Reserved4),
            NcnFeeGroup::new(NcnFeeGroupType::Reserved5),
            NcnFeeGroup::new(NcnFeeGroupType::Reserved6),
            NcnFeeGroup::new(NcnFeeGroupType::Reserved7),
        ];

        assert_eq!(all_groups, expected_groups);
    }

    #[test]
    fn test_fee_group_count_constant() {
        // Verify FEE_GROUP_COUNT matches number of enum variants
        assert_eq!(NcnFeeGroup::FEE_GROUP_COUNT, 8);

        // Verify all_groups() returns exactly FEE_GROUP_COUNT items
        assert_eq!(
            NcnFeeGroup::all_groups().len(),
            NcnFeeGroup::FEE_GROUP_COUNT
        );
    }
}
