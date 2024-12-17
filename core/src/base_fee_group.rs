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

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, PartialEq, Eq)]
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

    pub const fn dao() -> Self {
        Self::new(BaseFeeGroupType::DAO)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_fee_group_type_values() {
        // Verify enum values match expected u8 values
        assert_eq!(BaseFeeGroupType::DAO as u8, 0x0);
        assert_eq!(BaseFeeGroupType::Reserved1 as u8, 0x1);
        assert_eq!(BaseFeeGroupType::Reserved2 as u8, 0x2);
        assert_eq!(BaseFeeGroupType::Reserved3 as u8, 0x3);
        assert_eq!(BaseFeeGroupType::Reserved4 as u8, 0x4);
        assert_eq!(BaseFeeGroupType::Reserved5 as u8, 0x5);
        assert_eq!(BaseFeeGroupType::Reserved6 as u8, 0x6);
        assert_eq!(BaseFeeGroupType::Reserved7 as u8, 0x7);
    }

    #[test]
    fn test_base_fee_group_default() {
        let default_group = BaseFeeGroup::default();
        assert_eq!(default_group.group, BaseFeeGroupType::DAO as u8);

        // Verify default group type conversion
        assert_eq!(default_group.group_type().unwrap(), BaseFeeGroupType::DAO);
    }

    #[test]
    fn test_base_fee_group_try_from() {
        // Test valid conversions
        assert_eq!(
            BaseFeeGroup::try_from(0x0).unwrap().group_type().unwrap(),
            BaseFeeGroupType::DAO
        );
        assert_eq!(
            BaseFeeGroup::try_from(0x1).unwrap().group_type().unwrap(),
            BaseFeeGroupType::Reserved1
        );

        // Test all reserved groups
        for i in 2..=7 {
            assert!(BaseFeeGroup::try_from(i).is_ok());
        }

        // Test invalid values
        assert!(matches!(
            BaseFeeGroup::try_from(8),
            Err(TipRouterError::InvalidBaseFeeGroup)
        ));
        assert!(matches!(
            BaseFeeGroup::try_from(255),
            Err(TipRouterError::InvalidBaseFeeGroup)
        ));
    }

    #[test]
    fn test_base_fee_group_new() {
        // Test creation of all group types
        let dao_group = BaseFeeGroup::new(BaseFeeGroupType::DAO);
        assert_eq!(dao_group.group, 0x0);

        let reserved1_group = BaseFeeGroup::new(BaseFeeGroupType::Reserved1);
        assert_eq!(reserved1_group.group, 0x1);

        // Test all reserved groups
        let reserved_groups = [
            BaseFeeGroup::new(BaseFeeGroupType::Reserved2),
            BaseFeeGroup::new(BaseFeeGroupType::Reserved3),
            BaseFeeGroup::new(BaseFeeGroupType::Reserved4),
            BaseFeeGroup::new(BaseFeeGroupType::Reserved5),
            BaseFeeGroup::new(BaseFeeGroupType::Reserved6),
            BaseFeeGroup::new(BaseFeeGroupType::Reserved7),
        ];

        for (i, group) in reserved_groups.iter().enumerate() {
            assert_eq!(group.group as usize, i + 2);
        }
    }

    #[test]
    fn test_group_type_conversion() {
        // Test valid conversions
        let test_cases = [
            (0x0, BaseFeeGroupType::DAO),
            (0x1, BaseFeeGroupType::Reserved1),
            (0x2, BaseFeeGroupType::Reserved2),
            (0x3, BaseFeeGroupType::Reserved3),
            (0x4, BaseFeeGroupType::Reserved4),
            (0x5, BaseFeeGroupType::Reserved5),
            (0x6, BaseFeeGroupType::Reserved6),
            (0x7, BaseFeeGroupType::Reserved7),
        ];

        for (value, expected_type) in test_cases {
            let group = BaseFeeGroup { group: value };
            assert_eq!(group.group_type().unwrap(), expected_type);
        }

        // Test invalid conversion
        let invalid_group = BaseFeeGroup { group: 8 };
        assert!(matches!(
            invalid_group.group_type(),
            Err(TipRouterError::InvalidNcnFeeGroup)
        ));
    }

    #[test]
    fn test_group_index() {
        // Test all valid indices
        for i in 0..BaseFeeGroup::FEE_GROUP_COUNT {
            let group = BaseFeeGroup { group: i as u8 };
            assert_eq!(group.group_index().unwrap(), i);
        }

        // Test invalid index
        let invalid_group = BaseFeeGroup { group: 8 };
        assert!(matches!(
            invalid_group.group_index(),
            Err(TipRouterError::InvalidNcnFeeGroup)
        ));
    }
    #[test]
    fn test_all_groups() {
        let all_groups = BaseFeeGroup::all_groups();

        // Verify count matches FEE_GROUP_COUNT
        assert_eq!(all_groups.len(), BaseFeeGroup::FEE_GROUP_COUNT);

        // Verify groups are in correct order and have expected values
        let expected_groups = vec![
            BaseFeeGroup::new(BaseFeeGroupType::DAO),
            BaseFeeGroup::new(BaseFeeGroupType::Reserved1),
            BaseFeeGroup::new(BaseFeeGroupType::Reserved2),
            BaseFeeGroup::new(BaseFeeGroupType::Reserved3),
            BaseFeeGroup::new(BaseFeeGroupType::Reserved4),
            BaseFeeGroup::new(BaseFeeGroupType::Reserved5),
            BaseFeeGroup::new(BaseFeeGroupType::Reserved6),
            BaseFeeGroup::new(BaseFeeGroupType::Reserved7),
        ];

        assert_eq!(all_groups, expected_groups);
    }

    #[test]
    fn test_fee_group_count_constant() {
        // Verify FEE_GROUP_COUNT matches number of enum variants
        assert_eq!(BaseFeeGroup::FEE_GROUP_COUNT, 8);

        // Verify all_groups() returns exactly FEE_GROUP_COUNT items
        assert_eq!(
            BaseFeeGroup::all_groups().len(),
            BaseFeeGroup::FEE_GROUP_COUNT
        );
    }
}
