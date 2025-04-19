use bytemuck::{Pod, Zeroable};
use jito_bytemuck::types::PodU128;
use shank::ShankType;

use crate::error::TipRouterError;

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct StakeWeights {
    /// The total stake weight - used for voting
    stake_weight: PodU128,
    /// Reserved_space: The components that make up the total stake weight - used for rewards
    reserved_for_ncn_fee_group_stake_weights: [u8; 128],
}

impl Default for StakeWeights {
    fn default() -> Self {
        Self {
            stake_weight: PodU128::from(0),
            reserved_for_ncn_fee_group_stake_weights: [0; 128],
        }
    }
}

impl StakeWeights {
    pub fn new(stake_weight: u128) -> Self {
        Self {
            stake_weight: PodU128::from(stake_weight),
            reserved_for_ncn_fee_group_stake_weights: [0; 128],
        }
    }

    pub fn snapshot(stake_weight: u128) -> Result<Self, TipRouterError> {
        let mut stake_weights = Self::default();

        stake_weights.increment_stake_weight(stake_weight)?;

        Ok(stake_weights)
    }

    pub fn stake_weight(&self) -> u128 {
        self.stake_weight.into()
    }

    pub fn increment(&mut self, stake_weight: &Self) -> Result<(), TipRouterError> {
        self.increment_stake_weight(stake_weight.stake_weight())?;

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

    pub fn decrement(&mut self, other: &Self) -> Result<(), TipRouterError> {
        self.decrement_stake_weight(other.stake_weight())?;

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stake_weights_default() {
        let stake_weights = StakeWeights::default();
        assert_eq!(stake_weights.stake_weight(), 0);
    }

    #[test]
    fn test_stake_weights_snapshot() {
        let stake_weight = 1000u128;

        let stake_weights = StakeWeights::snapshot(stake_weight).unwrap();

        // Check base stake weight
        assert_eq!(stake_weights.stake_weight(), stake_weight);
    }

    #[test]
    fn test_stake_weights_increment() {
        let mut base_weights = StakeWeights::default();

        // Create first snapshot
        let weights1 = StakeWeights::snapshot(1000u128).unwrap();

        // Create second snapshot with different group
        let weights2 = StakeWeights::snapshot(2000u128).unwrap();

        // Increment with first weights
        base_weights.increment(&weights1).unwrap();
        assert_eq!(base_weights.stake_weight(), 1000u128);

        // Increment with second weights
        base_weights.increment(&weights2).unwrap();
        assert_eq!(base_weights.stake_weight(), 3000u128);
    }

    #[test]
    fn test_stake_weights_overflow() {
        // Test stake weight overflow
        let mut base_weights = StakeWeights::default();
        let max_weight = StakeWeights::snapshot(u128::MAX).unwrap();

        base_weights.increment(&max_weight).unwrap();

        // Adding any more should overflow
        let additional_weight = StakeWeights::snapshot(1u128).unwrap();

        assert!(base_weights.increment(&additional_weight).is_err());
    }

    #[test]
    fn test_stake_weights_increment_overflow() {
        // Test stake weight overflow
        let mut base_weights = StakeWeights::default();
        let max_weight = StakeWeights::snapshot(u128::MAX).unwrap();

        base_weights.increment(&max_weight).unwrap();

        // Adding any more should overflow
        let additional_weight = StakeWeights::snapshot(1u128).unwrap();

        assert!(base_weights.increment(&additional_weight).is_err());

        // Test NCN fee group weight overflow
        let mut base_weights = StakeWeights::default();

        // Use smaller numbers that won't overflow in the initial calculation
        // but will overflow when incremented twice
        let max_reward = StakeWeights::snapshot(u128::MAX).unwrap();

        base_weights.increment(&max_reward).unwrap();
        assert!(base_weights.increment(&max_reward).is_err());
    }
}
