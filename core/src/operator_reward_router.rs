use bytemuck::{Pod, Zeroable};
use jito_bytemuck::{types::PodU64, AccountDeserialize, Discriminator};
use jito_vault_core::MAX_BPS;
use shank::{ShankAccount, ShankType};
use solana_program::{account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey};
use spl_math::precise_number::PreciseNumber;

use crate::{
    discriminators::Discriminators, epoch_reward_router::RewardRoutes,
    epoch_snapshot::OperatorSnapshot, error::TipRouterError,
};

// PDA'd ["epoch_reward_router", NCN, NCN_EPOCH_SLOT]
#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, AccountDeserialize, ShankAccount)]
#[repr(C)]
pub struct OperatorEpochRewardRouter {
    operator: Pubkey,

    ncn: Pubkey,

    ncn_epoch: PodU64,

    bump: u8,

    slot_created: PodU64,

    reward_pool: PodU64,

    operator_rewards: PodU64,

    reserved: [u8; 128],

    //TODO change to 64
    vault_rewards: [RewardRoutes; 32],
}

impl Discriminator for OperatorEpochRewardRouter {
    const DISCRIMINATOR: u8 = Discriminators::EpochRewardRouter as u8;
}

impl OperatorEpochRewardRouter {
    pub fn new(operator: Pubkey, ncn: Pubkey, ncn_epoch: u64, bump: u8, slot_created: u64) -> Self {
        Self {
            operator,
            ncn,
            ncn_epoch: PodU64::from(ncn_epoch),
            bump,
            slot_created: PodU64::from(slot_created),
            vault_rewards: [RewardRoutes::default(); 32],
            reward_pool: PodU64::from(0),
            operator_rewards: PodU64::from(0),
            reserved: [0; 128],
        }
    }

    pub fn seeds(operator: &Pubkey, ncn: &Pubkey, ncn_epoch: u64) -> Vec<Vec<u8>> {
        Vec::from_iter(
            [
                b"operator_reward_router".to_vec(),
                operator.to_bytes().to_vec(),
                ncn.to_bytes().to_vec(),
                ncn_epoch.to_le_bytes().to_vec(),
            ]
            .iter()
            .cloned(),
        )
    }

    pub fn find_program_address(
        program_id: &Pubkey,
        operator: &Pubkey,
        ncn: &Pubkey,
        ncn_epoch: u64,
    ) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(operator, ncn, ncn_epoch);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
    }

    pub fn load(
        program_id: &Pubkey,
        operator: &Pubkey,
        ncn: &Pubkey,
        ncn_epoch: u64,
        account: &AccountInfo,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        if account.owner.ne(program_id) {
            msg!("Epoch Reward Router account has an invalid owner");
            return Err(ProgramError::InvalidAccountOwner);
        }
        if account.data_is_empty() {
            msg!("Epoch Reward Router account data is empty");
            return Err(ProgramError::InvalidAccountData);
        }
        if expect_writable && !account.is_writable {
            msg!("Epoch Reward Router account is not writable");
            return Err(ProgramError::InvalidAccountData);
        }
        if account.data.borrow()[0].ne(&Self::DISCRIMINATOR) {
            msg!("Epoch Reward Router account discriminator is invalid");
            return Err(ProgramError::InvalidAccountData);
        }
        if account
            .key
            .ne(&Self::find_program_address(program_id, operator, ncn, ncn_epoch).0)
        {
            msg!("Epoch Reward Router account is not at the correct PDA");
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }

    pub fn process_reward_pool(
        &mut self,
        operator_snapshot: &OperatorSnapshot,
    ) -> Result<(), TipRouterError> {
        let rewards_to_process: u64 = self.reward_pool();

        // Operator Fee Rewards
        {
            let operator_fee_bps = operator_snapshot.operator_fee_bps();
            let operator_rewards =
                Self::calculate_operator_reward(operator_fee_bps as u64, rewards_to_process)?;

            self.increment_operator_rewards(operator_rewards)?;
            self.decrement_reward_pool(operator_rewards)?;
        }

        // Vault Rewards
        {
            let operator_stake_weight = operator_snapshot.stake_weight();
            let rewards_to_process: u64 = self.reward_pool();

            for vault_operator_delegation in operator_snapshot.vault_operator_stake_weight().iter()
            {
                let vault = vault_operator_delegation.vault();
                let vault_ncn_fee_group = vault_operator_delegation.ncn_fee_group();

                let vault_reward_stake_weight = vault_operator_delegation
                    .stake_weight()
                    .reward_stake_weight(vault_ncn_fee_group)?;

                let operator_reward_stake_weight =
                    operator_stake_weight.reward_stake_weight(vault_ncn_fee_group)?;

                let vault_reward = Self::calculate_vault_reward(
                    vault_reward_stake_weight,
                    operator_reward_stake_weight,
                    rewards_to_process,
                )?;

                self.insert_or_increment_vault_rewards(vault, vault_reward)?;
                self.decrement_reward_pool(vault_reward)?;
            }
        }

        // Operator gets any remainder
        {
            let leftover_rewards = self.reward_pool();

            self.increment_operator_rewards(leftover_rewards)?;
            self.decrement_reward_pool(leftover_rewards)?;
        }

        Ok(())
    }

    fn calculate_operator_reward(
        fee_bps: u64,
        rewards_to_process: u64,
    ) -> Result<u64, TipRouterError> {
        if fee_bps == 0 || rewards_to_process == 0 {
            return Ok(0);
        }

        let precise_dao_fee_bps =
            PreciseNumber::new(fee_bps as u128).ok_or(TipRouterError::NewPreciseNumberError)?;

        let precise_max_bps =
            PreciseNumber::new(MAX_BPS as u128).ok_or(TipRouterError::NewPreciseNumberError)?;

        let precise_rewards_to_process = PreciseNumber::new(rewards_to_process as u128)
            .ok_or(TipRouterError::NewPreciseNumberError)?;

        let precise_dao_rewards = precise_rewards_to_process
            .checked_mul(&precise_dao_fee_bps)
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

        Ok(dao_rewards)
    }

    fn calculate_vault_reward(
        vault_reward_stake_weight: u128,
        operator_reward_stake_weight: u128,
        rewards_to_process: u64,
    ) -> Result<u64, TipRouterError> {
        if operator_reward_stake_weight == 0 || rewards_to_process == 0 {
            return Ok(0);
        }

        let precise_rewards_to_process = PreciseNumber::new(rewards_to_process as u128)
            .ok_or(TipRouterError::NewPreciseNumberError)?;

        let precise_vault_reward_stake_weight = PreciseNumber::new(vault_reward_stake_weight)
            .ok_or(TipRouterError::NewPreciseNumberError)?;

        let precise_operator_reward_stake_weight = PreciseNumber::new(operator_reward_stake_weight)
            .ok_or(TipRouterError::NewPreciseNumberError)?;

        let precise_operator_reward = precise_rewards_to_process
            .checked_mul(&precise_vault_reward_stake_weight)
            .and_then(|x| x.checked_div(&precise_operator_reward_stake_weight))
            .ok_or(TipRouterError::ArithmeticOverflow)?;

        let floored_precise_operator_reward = precise_operator_reward
            .floor()
            .ok_or(TipRouterError::ArithmeticFloorError)?;

        let operator_reward_u128: u128 = floored_precise_operator_reward
            .to_imprecise()
            .ok_or(TipRouterError::CastToImpreciseNumberError)?;

        let operator_reward: u64 = operator_reward_u128
            .try_into()
            .map_err(|_| TipRouterError::CastToU64Error)?;

        Ok(operator_reward)
    }

    pub fn insert_or_increment_vault_rewards(
        &mut self,
        vault: Pubkey,
        rewards: u64,
    ) -> Result<(), TipRouterError> {
        if rewards == 0 {
            return Ok(());
        }

        for vault_reward in self.vault_rewards.iter_mut() {
            if vault_reward.destination() == vault {
                vault_reward.increment_rewards(rewards)?;
                return Ok(());
            }
        }

        for vault_reward in self.vault_rewards.iter_mut() {
            if vault_reward.destination() == Pubkey::default() {
                vault_reward.set_destination(vault);
                vault_reward.increment_rewards(rewards)?;
                return Ok(());
            }
        }

        Err(TipRouterError::OperatorRewardListFull)
    }

    pub fn decrement_vault_rewards(
        &mut self,
        vault: Pubkey,
        rewards: u64,
    ) -> Result<(), TipRouterError> {
        if rewards == 0 {
            return Ok(());
        }

        for operator_reward in self.vault_rewards.iter_mut() {
            if operator_reward.destination() == vault {
                operator_reward.decrement_rewards(rewards)?;
                return Ok(());
            }
        }

        Err(TipRouterError::OperatorRewardNotFound)
    }

    pub fn reward_pool(&self) -> u64 {
        self.reward_pool.into()
    }

    pub fn increment_reward_pool(&mut self, rewards: u64) -> Result<(), TipRouterError> {
        if rewards == 0 {
            return Ok(());
        }

        self.reward_pool = PodU64::from(
            self.reward_pool()
                .checked_add(rewards)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );
        Ok(())
    }

    pub fn decrement_reward_pool(&mut self, rewards: u64) -> Result<(), TipRouterError> {
        if rewards == 0 {
            return Ok(());
        }

        self.reward_pool = PodU64::from(
            self.reward_pool()
                .checked_sub(rewards)
                .ok_or(TipRouterError::ArithmeticUnderflowError)?,
        );
        Ok(())
    }

    pub fn operator_rewards(&self) -> u64 {
        self.operator_rewards.into()
    }

    pub fn increment_operator_rewards(&mut self, rewards: u64) -> Result<(), TipRouterError> {
        if rewards == 0 {
            return Ok(());
        }

        self.operator_rewards = PodU64::from(
            self.operator_rewards()
                .checked_add(rewards)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );
        Ok(())
    }

    pub fn decrement_operator_rewards(&mut self, rewards: u64) -> Result<(), TipRouterError> {
        if rewards == 0 {
            return Ok(());
        }

        self.operator_rewards = PodU64::from(
            self.operator_rewards()
                .checked_sub(rewards)
                .ok_or(TipRouterError::ArithmeticUnderflowError)?,
        );
        Ok(())
    }
}
