use bytemuck::{Pod, Zeroable};
use jito_bytemuck::types::{PodU128, PodU64};
use shank::ShankType;

use crate::{error::TipRouterError, ncn_fee_group::NcnFeeGroup};

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct StakeWeight {
    stake_weight: PodU128,
    reward_stake_weights: [RewardStakeWeight; NcnFeeGroup::FEE_GROUP_COUNT],
    // Reserves
    reserved: [u8; 64],
}

impl Default for StakeWeight {
    fn default() -> Self {
        Self {
            stake_weight: PodU128::from(0),
            reward_stake_weights: [RewardStakeWeight::default(); NcnFeeGroup::FEE_GROUP_COUNT],
            reserved: [0; 64],
        }
    }
}

impl StakeWeight {
    pub fn stake_weight(&self) -> u128 {
        self.stake_weight.into()
    }

    pub fn reward_stake_weight(&self, ncn_fee_group: NcnFeeGroup) -> Result<u64, TipRouterError> {
        let group_index = ncn_fee_group.group_index()?;

        Ok(self.reward_stake_weights[group_index].reward_stake_weight())
    }

    pub fn increment(&mut self, stake_weight: &StakeWeight) -> Result<(), TipRouterError> {
        self.increment_stake_weight(stake_weight.stake_weight())?;

        for group in NcnFeeGroup::all_groups().iter() {
            self.increment_reward_stake_weight(*group, stake_weight.reward_stake_weight(*group)?)?;
        }

        Ok(())
    }

    pub fn increment_stake_weight(&mut self, stake_weight: u128) -> Result<(), TipRouterError> {
        self.stake_weight = PodU128::from(
            self.stake_weight()
                .checked_add(stake_weight)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );

        Ok(())
    }

    pub fn increment_reward_stake_weight(
        &mut self,
        ncn_fee_group: NcnFeeGroup,
        stake_weight: u64,
    ) -> Result<(), TipRouterError> {
        let group_index = ncn_fee_group.group_index()?;

        self.reward_stake_weights[group_index].reward_stake_weight = PodU64::from(
            self.reward_stake_weight(ncn_fee_group)?
                .checked_add(stake_weight)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct RewardStakeWeight {
    reward_stake_weight: PodU64,
    reserved: [u8; 64],
}

impl Default for RewardStakeWeight {
    fn default() -> Self {
        Self {
            reward_stake_weight: PodU64::from(0),
            reserved: [0; 64],
        }
    }
}

impl RewardStakeWeight {
    pub fn new(reward_stake_weight: u64) -> Self {
        Self {
            reward_stake_weight: PodU64::from(reward_stake_weight),
            reserved: [0; 64],
        }
    }

    pub fn reward_stake_weight(&self) -> u64 {
        self.reward_stake_weight.into()
    }
}
