use bytemuck::{Pod, Zeroable};
use jito_bytemuck::types::PodU128;
use shank::ShankType;

use crate::{error::TipRouterError, ncn_fee_group::NcnFeeGroup};

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct StakeWeights {
    /// The total stake weight - used for voting
    stake_weight: PodU128,
    /// The components that make up the total stake weight - used for rewards
    ncn_fee_group_stake_weights: [NcnFeeGroupWeight; 8],
}

impl Default for StakeWeights {
    fn default() -> Self {
        Self {
            stake_weight: PodU128::from(0),
            ncn_fee_group_stake_weights: [NcnFeeGroupWeight::default();
                NcnFeeGroup::FEE_GROUP_COUNT],
        }
    }
}

impl StakeWeights {
    pub fn new(stake_weight: u128) -> Self {
        Self {
            stake_weight: PodU128::from(stake_weight),
            ncn_fee_group_stake_weights: [NcnFeeGroupWeight::default();
                NcnFeeGroup::FEE_GROUP_COUNT],
        }
    }

    pub fn snapshot(
        ncn_fee_group: NcnFeeGroup,
        stake_weight: u128,
        reward_multiplier_bps: u64,
    ) -> Result<Self, TipRouterError> {
        let mut stake_weights = Self::default();

        let reward_stake_weight = (reward_multiplier_bps as u128)
            .checked_mul(stake_weight)
            .ok_or(TipRouterError::ArithmeticOverflow)?;

        stake_weights.increment_stake_weight(stake_weight)?;
        stake_weights.increment_ncn_fee_group_stake_weight(ncn_fee_group, reward_stake_weight)?;

        Ok(stake_weights)
    }

    pub fn stake_weight(&self) -> u128 {
        self.stake_weight.into()
    }

    pub fn ncn_fee_group_stake_weight(
        &self,
        ncn_fee_group: NcnFeeGroup,
    ) -> Result<u128, TipRouterError> {
        let group_index = ncn_fee_group.group_index()?;

        Ok(self.ncn_fee_group_stake_weights[group_index].weight())
    }

    pub fn increment(&mut self, stake_weight: &Self) -> Result<(), TipRouterError> {
        self.increment_stake_weight(stake_weight.stake_weight())?;

        for group in NcnFeeGroup::all_groups().iter() {
            self.increment_ncn_fee_group_stake_weight(
                *group,
                stake_weight.ncn_fee_group_stake_weight(*group)?,
            )?;
        }

        Ok(())
    }

    fn increment_stake_weight(&mut self, stake_weight: u128) -> Result<(), TipRouterError> {
        self.stake_weight = PodU128::from(
            self.stake_weight()
                .checked_add(stake_weight)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );

        Ok(())
    }

    fn increment_ncn_fee_group_stake_weight(
        &mut self,
        ncn_fee_group: NcnFeeGroup,
        stake_weight: u128,
    ) -> Result<(), TipRouterError> {
        let group_index = ncn_fee_group.group_index()?;

        self.ncn_fee_group_stake_weights[group_index].weight = PodU128::from(
            self.ncn_fee_group_stake_weight(ncn_fee_group)?
                .checked_add(stake_weight)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );

        Ok(())
    }

    pub fn decrement(&mut self, other: &Self) -> Result<(), TipRouterError> {
        self.decrement_stake_weight(other.stake_weight())?;

        for group in NcnFeeGroup::all_groups().iter() {
            self.decrement_ncn_fee_group_stake_weight(
                *group,
                other.ncn_fee_group_stake_weight(*group)?,
            )?;
        }

        Ok(())
    }

    fn decrement_stake_weight(&mut self, stake_weight: u128) -> Result<(), TipRouterError> {
        self.stake_weight = PodU128::from(
            self.stake_weight()
                .checked_sub(stake_weight)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );

        Ok(())
    }

    fn decrement_ncn_fee_group_stake_weight(
        &mut self,
        ncn_fee_group: NcnFeeGroup,
        stake_weight: u128,
    ) -> Result<(), TipRouterError> {
        let group_index = ncn_fee_group.group_index()?;

        self.ncn_fee_group_stake_weights[group_index].weight = PodU128::from(
            self.ncn_fee_group_stake_weight(ncn_fee_group)?
                .checked_sub(stake_weight)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct NcnFeeGroupWeight {
    /// The weight
    weight: PodU128,
}

impl Default for NcnFeeGroupWeight {
    fn default() -> Self {
        Self {
            weight: PodU128::from(0),
        }
    }
}

impl NcnFeeGroupWeight {
    pub fn new(weight: u128) -> Self {
        Self {
            weight: PodU128::from(weight),
        }
    }

    pub fn weight(&self) -> u128 {
        self.weight.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ncn_fee_group::NcnFeeGroupType;

    #[test]
    fn test_stake_weights_default() {
        let stake_weights = StakeWeights::default();
        assert_eq!(stake_weights.stake_weight(), 0);

        // Check all NCN fee group weights are zero
        for group in NcnFeeGroup::all_groups() {
            assert_eq!(stake_weights.ncn_fee_group_stake_weight(group).unwrap(), 0);
        }
    }

    #[test]
    fn test_stake_weights_snapshot() {
        let ncn_fee_group = NcnFeeGroup::default();
        let stake_weight = 1000u128;
        let reward_multiplier_bps = 15000u64; // 150%

        let stake_weights =
            StakeWeights::snapshot(ncn_fee_group, stake_weight, reward_multiplier_bps).unwrap();

        // Check base stake weight
        assert_eq!(stake_weights.stake_weight(), stake_weight);

        // Check reward-adjusted NCN fee group weight
        let expected_reward_weight = (reward_multiplier_bps as u128) * stake_weight;
        assert_eq!(
            stake_weights
                .ncn_fee_group_stake_weight(ncn_fee_group)
                .unwrap(),
            expected_reward_weight
        );

        // Other groups should be zero
        for group in NcnFeeGroup::all_groups() {
            if group.group != ncn_fee_group.group {
                assert_eq!(stake_weights.ncn_fee_group_stake_weight(group).unwrap(), 0);
            }
        }
    }

    #[test]
    fn test_stake_weights_increment() {
        let mut base_weights = StakeWeights::default();

        // Create first snapshot
        let weights1 = StakeWeights::snapshot(
            NcnFeeGroup::new(NcnFeeGroupType::Default),
            1000u128,
            15000u64,
        )
        .unwrap();

        // Create second snapshot with different group
        let weights2 =
            StakeWeights::snapshot(NcnFeeGroup::new(NcnFeeGroupType::JTO), 2000u128, 12000u64)
                .unwrap();

        // Increment with first weights
        base_weights.increment(&weights1).unwrap();
        assert_eq!(base_weights.stake_weight(), 1000u128);
        assert_eq!(
            base_weights
                .ncn_fee_group_stake_weight(NcnFeeGroup::new(NcnFeeGroupType::Default))
                .unwrap(),
            15_000_000u128
        );

        // Increment with second weights
        base_weights.increment(&weights2).unwrap();
        assert_eq!(base_weights.stake_weight(), 3000u128);
        assert_eq!(
            base_weights
                .ncn_fee_group_stake_weight(NcnFeeGroup::new(NcnFeeGroupType::JTO))
                .unwrap(),
            24_000_000u128
        );
    }

    #[test]
    fn test_stake_weights_overflow() {
        // Test stake weight overflow
        let mut base_weights = StakeWeights::default();
        let max_weight = StakeWeights::snapshot(NcnFeeGroup::default(), u128::MAX, 1u64).unwrap();

        base_weights.increment(&max_weight).unwrap();

        // Adding any more should overflow
        let additional_weight =
            StakeWeights::snapshot(NcnFeeGroup::default(), 1u128, 1u64).unwrap();

        assert!(base_weights.increment(&additional_weight).is_err());

        // Test NCN fee group weight overflow
        assert!(StakeWeights::snapshot(
            NcnFeeGroup::default(),
            u128::MAX / 2,
            20000u64, // 200%
        )
        .is_err());
    }

    #[test]
    fn test_stake_weights_increment_overflow() {
        // Test stake weight overflow
        let mut base_weights = StakeWeights::default();
        let max_weight = StakeWeights::snapshot(NcnFeeGroup::default(), u128::MAX, 1u64).unwrap();

        base_weights.increment(&max_weight).unwrap();

        // Adding any more should overflow
        let additional_weight =
            StakeWeights::snapshot(NcnFeeGroup::default(), 1u128, 1u64).unwrap();

        assert!(base_weights.increment(&additional_weight).is_err());

        // Test NCN fee group weight overflow
        let mut base_weights = StakeWeights::default();

        // Use smaller numbers that won't overflow in the initial calculation
        // but will overflow when incremented twice
        let max_reward = StakeWeights::snapshot(
            NcnFeeGroup::default(),
            u128::MAX / 20_000, // Divide by reward multiplier to avoid initial overflow
            20000u64,           // 200%
        )
        .unwrap();

        base_weights.increment(&max_reward).unwrap();
        assert!(base_weights.increment(&max_reward).is_err());
    }

    #[test]
    fn test_ncn_fee_group_weight() {
        let weight = NcnFeeGroupWeight::new(1000u128);
        assert_eq!(weight.weight(), 1000u128);

        let default_weight = NcnFeeGroupWeight::default();
        assert_eq!(default_weight.weight(), 0u128);
    }

    #[test]
    fn test_invalid_ncn_fee_group() {
        let invalid_group = NcnFeeGroup { group: 8 }; // Only 0-7 are valid
        let stake_weights = StakeWeights::default();

        assert!(stake_weights
            .ncn_fee_group_stake_weight(invalid_group)
            .is_err());
    }

    #[test]
    fn test_multiple_group_increments() {
        let mut base_weights = StakeWeights::default();

        // Create snapshots for all possible groups
        let mut snapshots = Vec::new();
        for group in NcnFeeGroup::all_groups() {
            let snapshot = StakeWeights::snapshot(group, 1000u128, 15000u64).unwrap();
            snapshots.push(snapshot);
        }

        // Increment with all snapshots
        for snapshot in &snapshots {
            base_weights.increment(snapshot).unwrap();
        }

        // Verify total stake weight
        assert_eq!(
            base_weights.stake_weight(),
            1000u128 * (snapshots.len() as u128)
        );

        // Verify each group's weight
        for group in NcnFeeGroup::all_groups() {
            assert_eq!(
                base_weights.ncn_fee_group_stake_weight(group).unwrap(),
                15_000_000u128
            );
        }
    }

    #[test]
    fn test_multiple_snapshots_same_group() {
        let group = NcnFeeGroup::default();
        let mut base_weights = StakeWeights::default();

        // Add multiple snapshots to the same group
        for i in 1..=3 {
            let snapshot = StakeWeights::snapshot(group, 1000u128, (1000 * i) as u64).unwrap();
            base_weights.increment(&snapshot).unwrap();
        }

        // Verify total stake weight
        assert_eq!(base_weights.stake_weight(), 3000u128);

        // Verify accumulated group weight (1000 * 1000 + 1000 * 2000 + 1000 * 3000)
        assert_eq!(
            base_weights.ncn_fee_group_stake_weight(group).unwrap(),
            6_000_000u128
        );
    }

    #[test]
    fn test_array_bounds() {
        let mut base_weights = StakeWeights::default();

        // Test all valid indices
        for i in 0..NcnFeeGroup::FEE_GROUP_COUNT {
            let group = NcnFeeGroup { group: i as u8 };
            let snapshot = StakeWeights::snapshot(group, 1000u128, 10000u64).unwrap();
            base_weights.increment(&snapshot).unwrap();
        }

        // Verify we can read all valid indices
        for i in 0..NcnFeeGroup::FEE_GROUP_COUNT {
            let group = NcnFeeGroup { group: i as u8 };
            assert!(base_weights.ncn_fee_group_stake_weight(group).is_ok());
        }
    }

    #[test]
    fn test_reward_calculation_overflow() {
        // Test overflow in reward calculation
        let stake_weight = u128::MAX / 20000f64 as u128; // Divide by reward multiplier to avoid initial overflow
        let reward_multiplier_bps = 20000u64; // 200%

        // This should succeed as it's just at the limit
        assert!(StakeWeights::snapshot(
            NcnFeeGroup::default(),
            stake_weight,
            reward_multiplier_bps
        )
        .is_ok());

        // This should fail due to overflow in the reward calculation
        assert!(StakeWeights::snapshot(
            NcnFeeGroup::default(),
            stake_weight + 1,
            reward_multiplier_bps
        )
        .is_err());
    }
}
