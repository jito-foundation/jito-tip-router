use bytemuck::{Pod, Zeroable};
use jito_bytemuck::{types::PodU64, AccountDeserialize, Discriminator};
use jito_vault_core::MAX_BPS;
use shank::{ShankAccount, ShankType};
use solana_program::pubkey::Pubkey;
use spl_math::precise_number::PreciseNumber;

use crate::{
    ballot_box::BallotBox,
    discriminators::Discriminators,
    error::TipRouterError,
    fees::{FeeConfig, Fees},
};

#[derive(Debug, Clone, PartialEq, Eq, Copy, Zeroable, ShankType, Pod, ShankType)]
#[repr(C)]
pub struct OperatorReward {
    operator: Pubkey,
    rewards: PodU64,
    reserved: [u8; 128],
}

impl OperatorReward {
    pub const fn operator(&self) -> Pubkey {
        self.operator
    }

    pub fn rewards(&self) -> u64 {
        self.rewards.into()
    }

    pub fn increment_rewards(&mut self, rewards: u64) -> Result<(), TipRouterError> {
        self.rewards = PodU64::from(
            self.rewards()
                .checked_add(rewards)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );
        Ok(())
    }

    pub fn decrement_rewards(&mut self, rewards: u64) -> Result<(), TipRouterError> {
        self.rewards = PodU64::from(
            self.rewards()
                .checked_sub(rewards)
                .ok_or(TipRouterError::ArithmeticUnderflowError)?,
        );
        Ok(())
    }
}

// PDA'd ["epoch_reward_router", NCN, NCN_EPOCH_SLOT]
#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, AccountDeserialize, ShankAccount)]
#[repr(C)]
pub struct EpochRewardRouter {
    ncn: Pubkey,

    ncn_epoch: PodU64,

    bump: u8,

    slot_created: PodU64,

    reserved: [u8; 128],

    reward_pool: PodU64,

    dao_rewards: PodU64,

    operator_rewards: [OperatorReward; 32],
}

impl Discriminator for EpochRewardRouter {
    const DISCRIMINATOR: u8 = Discriminators::EpochSnapshot as u8;
}

impl EpochRewardRouter {
    pub fn process_reward_pool(
        &mut self,
        fee_config: &FeeConfig,
        ballot_box: &BallotBox,
        current_epoch: u64,
    ) -> Result<(), TipRouterError> {
        let rewards_to_process: u64 = self.reward_pool();

        let mut precise_rewards_to_process = PreciseNumber::new(rewards_to_process as u128)
            .ok_or(TipRouterError::NewPreciseNumberError)?;

        // Dao Rewards
        {
            let adjusted_precise_dao_fee =
                fee_config.adjusted_precise_dao_fee_bps(current_epoch)?;

            let precise_max_bps =
                PreciseNumber::new(MAX_BPS as u128).ok_or(TipRouterError::NewPreciseNumberError)?;

            let precise_dao_rewards = precise_rewards_to_process
                .checked_mul(&adjusted_precise_dao_fee)
                .and_then(|x| x.checked_div(&precise_max_bps))
                .ok_or(TipRouterError::ArithmeticOverflow)?;

            let floored_precise_dao_rewards = precise_dao_rewards
                .floor()
                .ok_or(TipRouterError::ArithmeticFloorError)?;

            let dao_rewards_u128: u128 = floored_precise_dao_rewards
                .to_imprecise()
                .ok_or(TipRouterError::CastToImpreciseNumberError)?;

            let dao_rewards: u64 = dao_rewards_u128
                .try_into()
                .map_err(|_| TipRouterError::CastToU64Error)?;

            self.increment_dao_rewards(dao_rewards)?;

            self.decrement_reward_pool(dao_rewards)?;

            precise_rewards_to_process = precise_rewards_to_process
                .checked_sub(&floored_precise_dao_rewards)
                .ok_or(TipRouterError::ArithmeticUnderflowError)?;
        }

        // Operator Rewards
        {
            let total_reward_stake_weight =
                ballot_box.get_winning_ballot_tally()?.reward_stake_weight();
            let precise_total_reward_stake_weight = PreciseNumber::new(total_reward_stake_weight)
                .ok_or(TipRouterError::NewPreciseNumberError)?;

            for ballot in ballot_box.operator_votes().iter() {
                let operator_reward_stake_weight = ballot.reward_stake_weight();
                let precise_operator_reward_stake_weight =
                    PreciseNumber::new(operator_reward_stake_weight)
                        .ok_or(TipRouterError::NewPreciseNumberError)?;

                let precise_reward_split = precise_operator_reward_stake_weight
                    .checked_div(&precise_total_reward_stake_weight)
                    .ok_or(TipRouterError::DenominatorIsZero)?;

                let precise_rewards = precise_rewards_to_process
                    .checked_div(&precise_reward_split)
                    .ok_or(TipRouterError::DenominatorIsZero)?;

                let floored_precise_rewards = precise_rewards
                    .floor()
                    .ok_or(TipRouterError::ArithmeticFloorError)?;

                let operator_rewards_u128: u128 = floored_precise_rewards
                    .to_imprecise()
                    .ok_or(TipRouterError::CastToImpreciseNumberError)?;

                let operator_rewards: u64 = operator_rewards_u128
                    .try_into()
                    .map_err(|_| TipRouterError::CastToU64Error)?;

                self.insert_or_increment_operator_rewards(ballot.operator(), operator_rewards)?;
                self.decrement_reward_pool(operator_rewards)?;
            }
        }

        // Any Leftovers go to DAO
        {
            let leftover_rewards = self.reward_pool();

            self.increment_dao_rewards(leftover_rewards)?;
            self.decrement_reward_pool(leftover_rewards)?;
        }

        Ok(())
    }

    pub fn insert_or_increment_operator_rewards(
        &mut self,
        operator: Pubkey,
        rewards: u64,
    ) -> Result<(), TipRouterError> {
        for operator_reward in self.operator_rewards.iter_mut() {
            if operator_reward.operator == operator {
                operator_reward.increment_rewards(rewards)?;
                return Ok(());
            }
        }

        for operator_reward in self.operator_rewards.iter_mut() {
            if operator_reward.operator == Pubkey::default() {
                operator_reward.operator = operator;
                operator_reward.rewards = PodU64::from(rewards);
                return Ok(());
            }
        }

        Err(TipRouterError::OperatorRewardListFull.into())
    }

    pub fn decrement_operator_rewards(
        &mut self,
        operator: Pubkey,
        rewards: u64,
    ) -> Result<(), TipRouterError> {
        for operator_reward in self.operator_rewards.iter_mut() {
            if operator_reward.operator == operator {
                operator_reward.decrement_rewards(rewards)?;
                return Ok(());
            }
        }

        Err(TipRouterError::OperatorRewardNotFound.into())
    }

    pub fn reward_pool(&self) -> u64 {
        self.reward_pool.into()
    }

    pub fn increment_reward_pool(&mut self, rewards: u64) -> Result<(), TipRouterError> {
        self.reward_pool = PodU64::from(
            self.reward_pool()
                .checked_add(rewards)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );
        Ok(())
    }

    pub fn decrement_reward_pool(&mut self, rewards: u64) -> Result<(), TipRouterError> {
        self.reward_pool = PodU64::from(
            self.reward_pool()
                .checked_sub(rewards)
                .ok_or(TipRouterError::ArithmeticUnderflowError)?,
        );
        Ok(())
    }

    pub fn dao_rewards(&self) -> u64 {
        self.dao_rewards.into()
    }

    pub fn increment_dao_rewards(&mut self, rewards: u64) -> Result<(), TipRouterError> {
        self.dao_rewards = PodU64::from(
            self.dao_rewards()
                .checked_add(rewards)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );
        Ok(())
    }

    pub fn decrement_dao_rewards(&mut self, rewards: u64) -> Result<(), TipRouterError> {
        self.dao_rewards = PodU64::from(
            self.dao_rewards()
                .checked_sub(rewards)
                .ok_or(TipRouterError::ArithmeticUnderflowError)?,
        );
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy, Zeroable, ShankType, Pod, ShankType)]
#[repr(C)]
pub struct VaultReward {
    vault: Pubkey,
    reward: PodU64,
    reserved: [u8; 128],
}

// PDA'd ["operator_reward_router", NCN, NCN_EPOCH_SLOT]
#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, AccountDeserialize, ShankAccount)]
#[repr(C)]
pub struct OperatorRewardRouter {
    ncn: Pubkey,

    ncn_epoch: PodU64,

    bump: u8,

    slot_created: PodU64,

    reserved: [u8; 128],

    reward_pool: PodU64,

    vault_rewards: [VaultReward; 32],
}

impl Discriminator for OperatorRewardRouter {
    const DISCRIMINATOR: u8 = Discriminators::EpochSnapshot as u8;
}

impl EpochRewardRouter {}
