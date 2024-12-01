use bytemuck::{Pod, Zeroable};
use jito_bytemuck::types::PodU128;
use shank::ShankType;

use crate::{error::TipRouterError, ncn_fee_group::NcnFeeGroup};

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct StakeWeights {
    stake_weight: PodU128,
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
    pub fn new(ncn_fee_group: NcnFeeGroup, stake_weight: u128) -> Result<Self, TipRouterError> {
        let mut stake_weights = StakeWeights::default();

        stake_weights.increment_stake_weight(stake_weight)?;
        stake_weights.increment_ncn_fee_group_stake_weight(ncn_fee_group, stake_weight)?;

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
}

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct NcnFeeGroupWeight {
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
