use bytemuck::{Pod, Zeroable};
use jito_bytemuck::{types::PodU64, AccountDeserialize, Discriminator};
use shank::{ShankAccount, ShankType};
use solana_program::{account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey};
use spl_math::precise_number::PreciseNumber;

use crate::{
    ballot_box::BallotBox,
    discriminators::Discriminators,
    error::TipRouterError,
    fees::{FeeConfig, Fees},
    ncn_fee_group::NcnFeeGroup,
    operator_epoch_reward_router::OperatorEpochRewardRouter,
};

#[derive(Default, Debug, Clone, PartialEq, Eq, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct OperatorRewards {
    rewards: PodU64,
}

impl OperatorRewards {
    pub fn rewards(self) -> u64 {
        self.rewards.into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct RewardRoutes {
    operator: Pubkey,
    rewards: [OperatorRewards; 8],
}

impl Default for RewardRoutes {
    fn default() -> Self {
        Self {
            operator: Pubkey::default(),
            rewards: [OperatorRewards::default(); NcnFeeGroup::FEE_GROUP_COUNT],
        }
    }
}

impl RewardRoutes {
    pub const fn destination(&self) -> Pubkey {
        self.operator
    }

    pub fn set_destination(&mut self, destination: Pubkey) {
        self.operator = destination;
    }

    pub fn rewards(&self, ncn_fee_group: NcnFeeGroup) -> Result<u64, TipRouterError> {
        let group_index = ncn_fee_group.group_index()?;
        Ok(self.rewards[group_index].rewards())
    }

    pub fn set_rewards(
        &mut self,
        ncn_fee_group: NcnFeeGroup,
        rewards: u64,
    ) -> Result<(), TipRouterError> {
        let group_index = ncn_fee_group.group_index()?;
        self.rewards[group_index].rewards = PodU64::from(rewards);

        Ok(())
    }

    pub fn increment_rewards(
        &mut self,
        ncn_fee_group: NcnFeeGroup,
        rewards: u64,
    ) -> Result<(), TipRouterError> {
        let current_rewards = self.rewards(ncn_fee_group)?;

        let new_rewards = current_rewards
            .checked_add(rewards)
            .ok_or(TipRouterError::ArithmeticOverflow)?;

        self.set_rewards(ncn_fee_group, new_rewards)
    }

    pub fn decrement_rewards(
        &mut self,
        ncn_fee_group: NcnFeeGroup,
        rewards: u64,
    ) -> Result<(), TipRouterError> {
        let current_rewards = self.rewards(ncn_fee_group)?;

        let new_rewards = current_rewards
            .checked_sub(rewards)
            .ok_or(TipRouterError::ArithmeticOverflow)?;

        self.set_rewards(ncn_fee_group, new_rewards)
    }
}

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct RewardBucket {
    rewards: PodU64,
    reserved: [u8; 64],
}

impl Default for RewardBucket {
    fn default() -> Self {
        Self {
            rewards: PodU64::from(0),
            reserved: [0; 64],
        }
    }
}

impl RewardBucket {
    pub fn rewards(&self) -> u64 {
        self.rewards.into()
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

    reward_pool: PodU64,

    rewards_processed: PodU64,

    doa_rewards: PodU64,

    reserved: [u8; 128],

    ncn_reward_buckets: [RewardBucket; 8],

    //TODO change to 256
    reward_routes: [RewardRoutes; 32],
}

impl Discriminator for EpochRewardRouter {
    const DISCRIMINATOR: u8 = Discriminators::EpochRewardRouter as u8;
}

impl EpochRewardRouter {
    pub fn new(ncn: Pubkey, ncn_epoch: u64, bump: u8, slot_created: u64) -> Self {
        Self {
            ncn,
            ncn_epoch: PodU64::from(ncn_epoch),
            bump,
            slot_created: PodU64::from(slot_created),
            ncn_reward_buckets: [RewardBucket::default(); NcnFeeGroup::FEE_GROUP_COUNT],
            reward_routes: [RewardRoutes::default(); 32],
            reward_pool: PodU64::from(0),
            rewards_processed: PodU64::from(0),
            doa_rewards: PodU64::from(0),
            reserved: [0; 128],
        }
    }

    pub fn seeds(ncn: &Pubkey, ncn_epoch: u64) -> Vec<Vec<u8>> {
        Vec::from_iter(
            [
                b"epoch_reward_router".to_vec(),
                ncn.to_bytes().to_vec(),
                ncn_epoch.to_le_bytes().to_vec(),
            ]
            .iter()
            .cloned(),
        )
    }

    pub fn find_program_address(
        program_id: &Pubkey,
        ncn: &Pubkey,
        ncn_epoch: u64,
    ) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(ncn, ncn_epoch);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
    }

    pub fn load(
        program_id: &Pubkey,
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
            .ne(&Self::find_program_address(program_id, ncn, ncn_epoch).0)
        {
            msg!("Epoch Reward Router account is not at the correct PDA");
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }

    pub fn distribute_dao_rewards(
        &mut self,
        fee_config: &FeeConfig,
        destination: &Pubkey,
    ) -> Result<u64, TipRouterError> {
        if destination.ne(&fee_config.fee_wallet()) {
            return Err(TipRouterError::DestinationMismatch);
        }

        let dao_rewards = self.doa_rewards();

        self.decrement_dao_rewards(dao_rewards)?;

        Ok(self.doa_rewards())
    }

    // pub fn distribute_ncn_rewards(
    //     &mut self,
    //     destination: &Pubkey,
    // ) -> Result<u64, TipRouterError> {

    //     for route in

    //     let group_rewards = self.ncn_rewards(group)?;

    //     self.decrement_ncn_rewards(group, group_rewards)?;

    //     Ok(self.ncn_rewards(group)?)
    // }

    pub fn process_incoming_rewards(&mut self, account_balance: u64) -> Result<(), TipRouterError> {
        let total_rewards = self.total_rewards()?;

        let incoming_rewards = account_balance
            .checked_sub(total_rewards)
            .ok_or(TipRouterError::ArithmeticUnderflowError)?;

        self.increment_reward_pool(incoming_rewards)?;

        Ok(())
    }

    pub fn process_reward_pool(&mut self, fee: &Fees) -> Result<(), TipRouterError> {
        let rewards_to_process: u64 = self.reward_pool();

        let total_fee_bps = fee.total_fees_bps()?;

        // DAO Rewards
        {
            let dao_fee = fee.dao_fee_bps();
            let dao_rewards =
                Self::calculate_bucket_reward(dao_fee, total_fee_bps, rewards_to_process)?;

            self.increment_dao_rewards(dao_rewards)?;
            self.decrement_reward_pool(dao_rewards)?;
        }

        // Fee Buckets
        {
            for group in NcnFeeGroup::all_groups().iter() {
                let ncn_group_fee = fee.ncn_fee_bps(*group)?;

                let group_reward = Self::calculate_bucket_reward(
                    ncn_group_fee,
                    total_fee_bps,
                    rewards_to_process,
                )?;

                self.increment_ncn_rewards(*group, group_reward)?;
                self.decrement_reward_pool(group_reward)?;
            }
        }

        // DAO gets any remainder
        {
            let leftover_rewards = self.reward_pool();

            self.increment_dao_rewards(leftover_rewards)?;
            self.decrement_reward_pool(leftover_rewards)?;
        }

        Ok(())
    }

    pub fn process_buckets(
        &mut self,
        ballot_box: &BallotBox,
        program_id: &Pubkey,
    ) -> Result<(), TipRouterError> {
        let winning_ballot = ballot_box.get_winning_ballot()?;
        let winning_stake_weight = winning_ballot.stake_weight();

        for votes in ballot_box.operator_votes().iter() {
            if votes.ballot_index() == winning_ballot.index() {
                let operator = votes.operator();

                for group in NcnFeeGroup::all_groups().iter() {
                    let rewards_to_process = self.ncn_rewards(*group)?;
                    let winning_reward_stake_weight =
                        winning_stake_weight.reward_stake_weight(*group)?;
                    let operator_reward_stake_weight =
                        votes.stake_weight().reward_stake_weight(*group)?;

                    let operator_rewards = Self::calculate_operator_reward(
                        operator_reward_stake_weight,
                        winning_reward_stake_weight,
                        rewards_to_process,
                    )?;

                    let operator_epoch_reward_router =
                        OperatorEpochRewardRouter::find_program_address(
                            program_id,
                            *group,
                            &operator,
                            &self.ncn,
                            self.ncn_epoch.into(),
                        )
                        .0;

                    self.insert_or_increment_operator_rewards(
                        *group,
                        operator_epoch_reward_router,
                        operator_rewards,
                    )?;
                    self.decrement_ncn_rewards(*group, operator_rewards)?;
                }
            }
        }

        Ok(())
    }

    fn calculate_bucket_reward(
        fee_bps: u64,
        total_fee_bps: u64,
        rewards_to_process: u64,
    ) -> Result<u64, TipRouterError> {
        if fee_bps == 0 || rewards_to_process == 0 {
            return Ok(0);
        }

        let precise_dao_fee_bps =
            PreciseNumber::new(fee_bps as u128).ok_or(TipRouterError::NewPreciseNumberError)?;

        let precise_total_fee_bps = PreciseNumber::new(total_fee_bps as u128)
            .ok_or(TipRouterError::NewPreciseNumberError)?;

        let precise_rewards_to_process = PreciseNumber::new(rewards_to_process as u128)
            .ok_or(TipRouterError::NewPreciseNumberError)?;

        let precise_dao_rewards = precise_rewards_to_process
            .checked_mul(&precise_dao_fee_bps)
            .and_then(|x| x.checked_div(&precise_total_fee_bps))
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

    fn calculate_operator_reward(
        operator_reward_stake_weight: u128,
        winning_reward_stake_weight: u128,
        rewards_to_process: u64,
    ) -> Result<u64, TipRouterError> {
        if operator_reward_stake_weight == 0 || rewards_to_process == 0 {
            return Ok(0);
        }

        let precise_rewards_to_process = PreciseNumber::new(rewards_to_process as u128)
            .ok_or(TipRouterError::NewPreciseNumberError)?;

        let precise_operator_reward_stake_weight = PreciseNumber::new(operator_reward_stake_weight)
            .ok_or(TipRouterError::NewPreciseNumberError)?;

        let precise_winning_reward_stake_weight = PreciseNumber::new(winning_reward_stake_weight)
            .ok_or(TipRouterError::NewPreciseNumberError)?;

        let precise_operator_reward = precise_rewards_to_process
            .checked_mul(&precise_operator_reward_stake_weight)
            .and_then(|x| x.checked_div(&precise_winning_reward_stake_weight))
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

    pub fn insert_or_increment_operator_rewards(
        &mut self,
        ncn_fee_group: NcnFeeGroup,
        operator: Pubkey,
        rewards: u64,
    ) -> Result<(), TipRouterError> {
        if rewards == 0 {
            return Ok(());
        }

        for operator_reward in self.reward_routes.iter_mut() {
            if operator_reward.operator == operator {
                operator_reward.increment_rewards(ncn_fee_group, rewards)?;
                return Ok(());
            }
        }

        for operator_reward in self.reward_routes.iter_mut() {
            if operator_reward.operator == Pubkey::default() {
                operator_reward.operator = operator;
                operator_reward.increment_rewards(ncn_fee_group, rewards)?;
                return Ok(());
            }
        }

        Err(TipRouterError::OperatorRewardListFull)
    }

    pub fn decrement_operator_rewards(
        &mut self,
        ncn_fee_group: NcnFeeGroup,
        operator: Pubkey,
        rewards: u64,
    ) -> Result<(), TipRouterError> {
        if rewards == 0 {
            return Ok(());
        }

        for operator_reward in self.reward_routes.iter_mut() {
            if operator_reward.operator == operator {
                operator_reward.decrement_rewards(ncn_fee_group, rewards)?;
                return Ok(());
            }
        }

        Err(TipRouterError::OperatorRewardNotFound)
    }

    pub fn total_rewards(&self) -> Result<u64, TipRouterError> {
        let total_rewards = self
            .reward_pool()
            .checked_add(self.rewards_processed())
            .ok_or(TipRouterError::ArithmeticOverflow)?;

        Ok(total_rewards)
    }

    pub fn rewards_processed(&self) -> u64 {
        self.rewards_processed.into()
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

        self.rewards_processed = PodU64::from(
            self.rewards_processed()
                .checked_add(rewards)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );
        Ok(())
    }

    pub fn doa_rewards(&self) -> u64 {
        self.doa_rewards.into()
    }

    pub fn increment_dao_rewards(&mut self, rewards: u64) -> Result<(), TipRouterError> {
        if rewards == 0 {
            return Ok(());
        }

        self.doa_rewards = PodU64::from(
            self.doa_rewards()
                .checked_add(rewards)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );
        Ok(())
    }

    pub fn decrement_dao_rewards(&mut self, rewards: u64) -> Result<(), TipRouterError> {
        if rewards == 0 {
            return Ok(());
        }

        self.doa_rewards = PodU64::from(
            self.doa_rewards()
                .checked_sub(rewards)
                .ok_or(TipRouterError::ArithmeticUnderflowError)?,
        );

        self.rewards_processed = PodU64::from(
            self.rewards_processed()
                .checked_sub(rewards)
                .ok_or(TipRouterError::ArithmeticUnderflowError)?,
        );
        Ok(())
    }

    pub const fn ncn_reward_buckets(&self) -> &[RewardBucket; 8] {
        &self.ncn_reward_buckets
    }

    pub fn ncn_rewards(&self, group: NcnFeeGroup) -> Result<u64, TipRouterError> {
        let group_index = group.group_index()?;
        Ok(self.ncn_reward_buckets[group_index].rewards())
    }

    pub fn increment_ncn_rewards(
        &mut self,
        group: NcnFeeGroup,
        rewards: u64,
    ) -> Result<(), TipRouterError> {
        if rewards == 0 {
            return Ok(());
        }

        let group_index = group.group_index()?;
        self.ncn_reward_buckets[group_index].rewards = PodU64::from(
            self.ncn_rewards(group)?
                .checked_add(rewards)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );
        Ok(())
    }

    pub fn decrement_ncn_rewards(
        &mut self,
        group: NcnFeeGroup,
        rewards: u64,
    ) -> Result<(), TipRouterError> {
        if rewards == 0 {
            return Ok(());
        }

        let group_index = group.group_index()?;
        self.ncn_reward_buckets[group_index].rewards = PodU64::from(
            self.ncn_rewards(group)?
                .checked_sub(rewards)
                .ok_or(TipRouterError::ArithmeticUnderflowError)?,
        );

        self.rewards_processed = PodU64::from(
            self.rewards_processed()
                .checked_sub(rewards)
                .ok_or(TipRouterError::ArithmeticUnderflowError)?,
        );
        Ok(())
    }
}
