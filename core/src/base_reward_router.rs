use core::mem::size_of;

use bytemuck::{Pod, Zeroable};
use jito_bytemuck::{
    types::{PodU16, PodU64},
    AccountDeserialize, Discriminator,
};
use shank::{ShankAccount, ShankType};
use solana_program::{
    account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey, rent::Rent,
};
use spl_math::precise_number::PreciseNumber;

use crate::{
    ballot_box::BallotBox, base_fee_group::BaseFeeGroup, constants::MAX_OPERATORS,
    discriminators::Discriminators, error::TipRouterError, fees::Fees, ncn_fee_group::NcnFeeGroup,
};

// PDA'd ["epoch_reward_router", NCN, NCN_EPOCH_SLOT]
#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, AccountDeserialize, ShankAccount)]
#[repr(C)]
pub struct BaseRewardRouter {
    /// NCN the account is associated with
    ncn: Pubkey,
    /// The epoch the account is associated with
    epoch: PodU64,
    /// Bump seed for the PDA
    bump: u8,
    /// Slot the account was created
    slot_created: PodU64,
    /// Total rewards routed ( in lamports )
    total_rewards: PodU64,
    /// Amount of rewards in the reward pool ( in lamports )
    reward_pool: PodU64,
    /// Amount of rewards processed ( in lamports )
    rewards_processed: PodU64,
    /// Reserved space
    reserved: [u8; 128],

    // route state tracking - to recover from unfinished routing
    /// Last NCN fee group index
    last_ncn_group_index: u8,
    /// Last vote index
    last_vote_index: PodU16,
    /// Last rewards to process
    last_rewards_to_process: PodU64,

    /// Base Fee Group Rewards
    base_fee_group_rewards: [BaseRewardRouterRewards; 8],
    /// NCN Fee Group Rewards
    ncn_fee_group_rewards: [BaseRewardRouterRewards; 8],
    /// NCN Fee Group Reward Routes
    ncn_fee_group_reward_routes: [NcnRewardRoute; 256],
}

impl Discriminator for BaseRewardRouter {
    const DISCRIMINATOR: u8 = Discriminators::BaseRewardRouter as u8;
}

impl BaseRewardRouter {
    pub const SIZE: usize = 8 + size_of::<Self>();

    pub const NO_LAST_NCN_GROUP_INDEX: u8 = u8::MAX;
    pub const NO_LAST_VOTE_INDEX: u16 = u16::MAX;
    pub const NO_LAST_REWARDS_TO_PROCESS: u64 = u64::MAX;
    pub const MAX_ROUTE_BASE_ITERATIONS: u16 = 30;

    pub fn new(ncn: &Pubkey, ncn_epoch: u64, bump: u8, slot_created: u64) -> Self {
        Self {
            ncn: *ncn,
            epoch: PodU64::from(ncn_epoch),
            bump,
            slot_created: PodU64::from(slot_created),
            total_rewards: PodU64::from(0),
            reward_pool: PodU64::from(0),
            rewards_processed: PodU64::from(0),
            reserved: [0; 128],
            last_ncn_group_index: Self::NO_LAST_NCN_GROUP_INDEX,
            last_vote_index: PodU16::from(Self::NO_LAST_VOTE_INDEX),
            last_rewards_to_process: PodU64::from(Self::NO_LAST_REWARDS_TO_PROCESS),
            base_fee_group_rewards: [BaseRewardRouterRewards::default();
                NcnFeeGroup::FEE_GROUP_COUNT],
            ncn_fee_group_rewards: [BaseRewardRouterRewards::default();
                NcnFeeGroup::FEE_GROUP_COUNT],
            ncn_fee_group_reward_routes: [NcnRewardRoute::default(); MAX_OPERATORS],
        }
    }

    pub fn initialize(&mut self, ncn: &Pubkey, ncn_epoch: u64, bump: u8, current_slot: u64) {
        // Initializes field by field to avoid overflowing stack
        self.ncn = *ncn;
        self.epoch = PodU64::from(ncn_epoch);
        self.bump = bump;
        self.slot_created = PodU64::from(current_slot);
        self.total_rewards = PodU64::from(0);
        self.reward_pool = PodU64::from(0);
        self.rewards_processed = PodU64::from(0);
        self.reserved = [0; 128];
        self.base_fee_group_rewards =
            [BaseRewardRouterRewards::default(); NcnFeeGroup::FEE_GROUP_COUNT];
        self.ncn_fee_group_rewards =
            [BaseRewardRouterRewards::default(); NcnFeeGroup::FEE_GROUP_COUNT];
        self.ncn_fee_group_reward_routes = [NcnRewardRoute::default(); MAX_OPERATORS];

        self.reset_routing_state();
    }

    pub fn seeds(ncn: &Pubkey, ncn_epoch: u64) -> Vec<Vec<u8>> {
        Vec::from_iter(
            [
                b"base_reward_router".to_vec(),
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
        let seeds: Vec<Vec<u8>> = Self::seeds(ncn, ncn_epoch);
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
            msg!("Base Reward Router account has an invalid owner");
            return Err(ProgramError::InvalidAccountOwner);
        }
        if account.data_is_empty() {
            msg!("Base Reward Router account data is empty");
            return Err(ProgramError::InvalidAccountData);
        }
        if expect_writable && !account.is_writable {
            msg!("Base Reward Router account is not writable");
            return Err(ProgramError::InvalidAccountData);
        }
        if account.data.borrow()[0].ne(&Self::DISCRIMINATOR) {
            msg!("Base Reward Router account discriminator is invalid");
            return Err(ProgramError::InvalidAccountData);
        }
        if account
            .key
            .ne(&Self::find_program_address(program_id, ncn, ncn_epoch).0)
        {
            msg!("Base Reward Router account is not at the correct PDA");
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }

    // ----------------- ROUTE STATE TRACKING --------------
    pub const fn last_ncn_group_index(&self) -> u8 {
        self.last_ncn_group_index
    }

    pub fn last_vote_index(&self) -> u16 {
        self.last_vote_index.into()
    }

    pub fn last_rewards_to_process(&self) -> u64 {
        self.last_rewards_to_process.into()
    }

    pub fn resume_routing_state(&mut self) -> (usize, usize, u64) {
        if !self.still_routing() {
            return (0, 0, 0);
        }

        (
            self.last_ncn_group_index() as usize,
            self.last_vote_index() as usize,
            self.last_rewards_to_process(),
        )
    }

    pub fn save_routing_state(
        &mut self,
        ncn_group_index: usize,
        vote_index: usize,
        rewards_to_process: u64,
    ) {
        self.last_ncn_group_index = ncn_group_index as u8;
        self.last_vote_index = PodU16::from(vote_index as u16);
        self.last_rewards_to_process = PodU64::from(rewards_to_process);
    }

    pub fn reset_routing_state(&mut self) {
        self.last_ncn_group_index = Self::NO_LAST_NCN_GROUP_INDEX;
        self.last_vote_index = PodU16::from(Self::NO_LAST_VOTE_INDEX);
        self.last_rewards_to_process = PodU64::from(Self::NO_LAST_REWARDS_TO_PROCESS);
    }

    pub fn still_routing(&self) -> bool {
        self.last_ncn_group_index() != Self::NO_LAST_NCN_GROUP_INDEX
            || self.last_vote_index() != Self::NO_LAST_VOTE_INDEX
            || self.last_rewards_to_process() != Self::NO_LAST_REWARDS_TO_PROCESS
    }

    // ----------------- ROUTE REWARDS ---------------------
    pub fn route_incoming_rewards(
        &mut self,
        rent_cost: u64,
        account_balance: u64,
    ) -> Result<(), TipRouterError> {
        let total_rewards = self.total_rewards_in_transit()?;

        let incoming_rewards = account_balance
            .checked_sub(total_rewards)
            .ok_or(TipRouterError::ArithmeticUnderflowError)?;

        let rewards_to_route = incoming_rewards
            .checked_sub(rent_cost)
            .ok_or(TipRouterError::ArithmeticUnderflowError)?;

        self.route_to_reward_pool(rewards_to_route)?;

        Ok(())
    }

    pub fn route_reward_pool(&mut self, fee: &Fees) -> Result<(), TipRouterError> {
        let rewards_to_process: u64 = self.reward_pool();

        let total_fee_bps = fee.total_fees_bps()?;

        // Base Fee Group Rewards
        for group in BaseFeeGroup::all_groups().iter() {
            let base_fee = fee.base_fee_bps(*group)?;

            let rewards =
                Self::calculate_reward_split(base_fee, total_fee_bps, rewards_to_process)?;

            self.route_from_reward_pool(rewards)?;
            self.route_to_base_fee_group_rewards(*group, rewards)?;
        }

        // NCN Fee Group Rewards
        for group in NcnFeeGroup::all_groups().iter() {
            let ncn_group_fee = fee.ncn_fee_bps(*group)?;

            let rewards =
                Self::calculate_reward_split(ncn_group_fee, total_fee_bps, rewards_to_process)?;

            self.route_from_reward_pool(rewards)?;
            self.route_to_ncn_fee_group_rewards(*group, rewards)?;
        }

        // DAO gets any remainder
        {
            let leftover_rewards = self.reward_pool();

            self.route_from_reward_pool(leftover_rewards)?;
            self.route_to_base_fee_group_rewards(BaseFeeGroup::default(), leftover_rewards)?;
        }

        Ok(())
    }

    pub fn route_ncn_fee_group_rewards(
        &mut self,
        ballot_box: &BallotBox,
        max_iterations: u16,
    ) -> Result<(), TipRouterError> {
        let winning_ballot = ballot_box.get_winning_ballot_tally()?;
        let winning_stake_weight = winning_ballot.stake_weights();

        let (starting_group_index, starting_vote_index, mut starting_rewards_to_process) =
            self.resume_routing_state();
        let mut iterations: u16 = 0;

        for group_index in starting_group_index..NcnFeeGroup::FEE_GROUP_COUNT {
            let group = NcnFeeGroup::all_groups()[group_index];
            let rewards_to_process = if starting_rewards_to_process > 0 {
                starting_rewards_to_process
            } else {
                self.ncn_fee_group_rewards(group)?
            };

            // Reset starting rewards to process
            starting_rewards_to_process = 0;

            if rewards_to_process == 0 {
                continue;
            }

            for vote_index in starting_vote_index..ballot_box.operator_votes().len() {
                let votes = ballot_box.operator_votes()[vote_index];

                if votes.ballot_index() == winning_ballot.index() {
                    // Update iteration state
                    {
                        iterations = iterations
                            .checked_add(1)
                            .ok_or(TipRouterError::ArithmeticOverflow)?;

                        if iterations >= max_iterations {
                            msg!(
                                "Reached max iterations, saving state and exiting {}/{}",
                                group_index,
                                vote_index
                            );
                            self.save_routing_state(group_index, vote_index, rewards_to_process);
                            return Ok(());
                        }
                    }

                    let operator = votes.operator();

                    let winning_reward_stake_weight =
                        winning_stake_weight.ncn_fee_group_stake_weight(group)?;
                    let ncn_route_reward_stake_weight =
                        votes.stake_weights().ncn_fee_group_stake_weight(group)?;

                    let ncn_fee_group_route_reward = Self::calculate_ncn_fee_group_route_reward(
                        ncn_route_reward_stake_weight,
                        winning_reward_stake_weight,
                        rewards_to_process,
                    )?;

                    self.route_from_ncn_fee_group_rewards(group, ncn_fee_group_route_reward)?;
                    self.route_to_ncn_fee_group_reward_route(
                        group,
                        operator,
                        ncn_fee_group_route_reward,
                    )?;
                }
            }
        }

        msg!("Finished routing NCN fee group rewards");
        self.reset_routing_state();

        Ok(())
    }

    // ------------------ CALCULATIONS ---------------------
    fn calculate_reward_split(
        fee_bps: u16,
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

    fn calculate_ncn_fee_group_route_reward(
        ncn_route_reward_stake_weight: u128,
        winning_reward_stake_weight: u128,
        rewards_to_process: u64,
    ) -> Result<u64, TipRouterError> {
        if ncn_route_reward_stake_weight == 0 || rewards_to_process == 0 {
            return Ok(0);
        }

        let precise_rewards_to_process = PreciseNumber::new(rewards_to_process as u128)
            .ok_or(TipRouterError::NewPreciseNumberError)?;

        let precise_ncn_route_reward_stake_weight =
            PreciseNumber::new(ncn_route_reward_stake_weight)
                .ok_or(TipRouterError::NewPreciseNumberError)?;

        let precise_winning_reward_stake_weight = PreciseNumber::new(winning_reward_stake_weight)
            .ok_or(TipRouterError::NewPreciseNumberError)?;

        let precise_ncn_route_reward = precise_rewards_to_process
            .checked_mul(&precise_ncn_route_reward_stake_weight)
            .and_then(|x| x.checked_div(&precise_winning_reward_stake_weight))
            .ok_or(TipRouterError::ArithmeticOverflow)?;

        let floored_precise_ncn_route_reward = precise_ncn_route_reward
            .floor()
            .ok_or(TipRouterError::ArithmeticFloorError)?;

        let ncn_route_reward_u128: u128 = floored_precise_ncn_route_reward
            .to_imprecise()
            .ok_or(TipRouterError::CastToImpreciseNumberError)?;

        let ncn_route_reward: u64 = ncn_route_reward_u128
            .try_into()
            .map_err(|_| TipRouterError::CastToU64Error)?;

        Ok(ncn_route_reward)
    }

    // ------------------ REWARD TALLIES ---------------------
    pub fn total_rewards_in_transit(&self) -> Result<u64, TipRouterError> {
        let total_rewards = self
            .reward_pool()
            .checked_add(self.rewards_processed())
            .ok_or(TipRouterError::ArithmeticOverflow)?;

        Ok(total_rewards)
    }

    pub fn rent_cost(&self, rent: &Rent) -> Result<u64, TipRouterError> {
        let size = 8_u64
            .checked_add(size_of::<Self>() as u64)
            .ok_or(TipRouterError::ArithmeticOverflow)?;

        Ok(rent.minimum_balance(size as usize))
    }

    pub fn total_rewards(&self) -> u64 {
        self.total_rewards.into()
    }

    pub fn reward_pool(&self) -> u64 {
        self.reward_pool.into()
    }

    pub const fn ncn(&self) -> &Pubkey {
        &self.ncn
    }

    pub fn ncn_epoch(&self) -> u64 {
        self.epoch.into()
    }

    pub fn slot_created(&self) -> u64 {
        self.slot_created.into()
    }

    pub fn route_to_reward_pool(&mut self, rewards: u64) -> Result<(), TipRouterError> {
        if rewards == 0 {
            return Ok(());
        }

        self.total_rewards = PodU64::from(
            self.total_rewards()
                .checked_add(rewards)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );

        self.reward_pool = PodU64::from(
            self.reward_pool()
                .checked_add(rewards)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );
        Ok(())
    }

    pub fn route_from_reward_pool(&mut self, rewards: u64) -> Result<(), TipRouterError> {
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

    // ------------------ REWARDS PROCESSED ---------------------
    pub fn rewards_processed(&self) -> u64 {
        self.rewards_processed.into()
    }

    pub fn increment_rewards_processed(&mut self, rewards: u64) -> Result<(), TipRouterError> {
        if rewards == 0 {
            return Ok(());
        }

        self.rewards_processed = PodU64::from(
            self.rewards_processed()
                .checked_add(rewards)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );
        Ok(())
    }

    pub fn decrement_rewards_processed(&mut self, rewards: u64) -> Result<(), TipRouterError> {
        if rewards == 0 {
            return Ok(());
        }

        self.rewards_processed = PodU64::from(
            self.rewards_processed()
                .checked_sub(rewards)
                .ok_or(TipRouterError::ArithmeticUnderflowError)?,
        );
        Ok(())
    }

    // ------------------ BASE FEE GROUP REWARDS ---------------------

    pub fn base_fee_group_reward(&self, group: BaseFeeGroup) -> Result<u64, TipRouterError> {
        let group_index = group.group_index()?;
        Ok(self.base_fee_group_rewards[group_index].rewards())
    }

    pub fn route_to_base_fee_group_rewards(
        &mut self,
        group: BaseFeeGroup,
        rewards: u64,
    ) -> Result<(), TipRouterError> {
        if rewards == 0 {
            return Ok(());
        }

        let group_index = group.group_index()?;
        self.base_fee_group_rewards[group_index].rewards = PodU64::from(
            self.base_fee_group_reward(group)?
                .checked_add(rewards)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );

        self.increment_rewards_processed(rewards)?;

        Ok(())
    }

    pub fn distribute_base_fee_group_rewards(
        &mut self,
        group: BaseFeeGroup,
    ) -> Result<u64, TipRouterError> {
        let group_index = group.group_index()?;

        let rewards = self.base_fee_group_reward(group)?;
        self.base_fee_group_rewards[group_index].rewards = PodU64::from(
            rewards
                .checked_sub(rewards)
                .ok_or(TipRouterError::ArithmeticUnderflowError)?,
        );

        self.decrement_rewards_processed(rewards)?;

        Ok(rewards)
    }

    // ------------------ NCN FEE GROUP REWARDS ---------------------

    pub fn ncn_fee_group_rewards(&self, group: NcnFeeGroup) -> Result<u64, TipRouterError> {
        let group_index = group.group_index()?;
        Ok(self.ncn_fee_group_rewards[group_index].rewards())
    }

    pub fn route_to_ncn_fee_group_rewards(
        &mut self,
        group: NcnFeeGroup,
        rewards: u64,
    ) -> Result<(), TipRouterError> {
        if rewards == 0 {
            return Ok(());
        }

        let group_index = group.group_index()?;
        self.ncn_fee_group_rewards[group_index].rewards = PodU64::from(
            self.ncn_fee_group_rewards(group)?
                .checked_add(rewards)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );

        self.increment_rewards_processed(rewards)?;

        Ok(())
    }

    pub fn route_from_ncn_fee_group_rewards(
        &mut self,
        group: NcnFeeGroup,
        rewards: u64,
    ) -> Result<(), TipRouterError> {
        if rewards == 0 {
            return Ok(());
        }

        let group_index = group.group_index()?;
        self.ncn_fee_group_rewards[group_index].rewards = PodU64::from(
            self.ncn_fee_group_rewards(group)?
                .checked_sub(rewards)
                .ok_or(TipRouterError::ArithmeticUnderflowError)?,
        );

        Ok(())
    }

    // ------------------ NCN REWARD ROUTES ---------------------

    pub fn has_operator_reward_route(&self, operator: &Pubkey) -> bool {
        for ncn_route_reward in self.ncn_fee_group_reward_routes.iter() {
            if ncn_route_reward.operator.eq(operator) {
                return true;
            }
        }

        false
    }

    pub fn ncn_fee_group_reward_route(
        &self,
        operator: &Pubkey,
    ) -> Result<&NcnRewardRoute, TipRouterError> {
        for ncn_route_reward in self.ncn_fee_group_reward_routes.iter() {
            if ncn_route_reward.operator.eq(operator) {
                return Ok(ncn_route_reward);
            }
        }

        Err(TipRouterError::NcnRewardRouteNotFound)
    }

    pub const fn ncn_fee_group_reward_routes(&self) -> &[NcnRewardRoute; MAX_OPERATORS] {
        &self.ncn_fee_group_reward_routes
    }

    pub fn route_to_ncn_fee_group_reward_route(
        &mut self,
        ncn_fee_group: NcnFeeGroup,
        operator: &Pubkey,
        rewards: u64,
    ) -> Result<(), TipRouterError> {
        if rewards == 0 {
            return Ok(());
        }

        for ncn_route_reward in self.ncn_fee_group_reward_routes.iter_mut() {
            if ncn_route_reward.operator.eq(operator) {
                ncn_route_reward.increment_rewards(ncn_fee_group, rewards)?;
                return Ok(());
            }
        }

        for ncn_route_reward in self.ncn_fee_group_reward_routes.iter_mut() {
            if ncn_route_reward.operator.eq(&Pubkey::default()) {
                *ncn_route_reward = NcnRewardRoute::new(operator, ncn_fee_group, rewards)?;
                return Ok(());
            }
        }

        Err(TipRouterError::OperatorRewardListFull)
    }

    pub fn distribute_ncn_fee_group_reward_route(
        &mut self,
        ncn_fee_group: NcnFeeGroup,
        operator: &Pubkey,
    ) -> Result<u64, TipRouterError> {
        for route in self.ncn_fee_group_reward_routes.iter_mut() {
            if route.operator.eq(operator) {
                let rewards = route.rewards(ncn_fee_group)?;
                route.decrement_rewards(ncn_fee_group, rewards)?;
                self.decrement_rewards_processed(rewards)?;

                return Ok(rewards);
            }
        }

        Err(TipRouterError::OperatorRewardNotFound)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct NcnRewardRoute {
    operator: Pubkey,
    ncn_fee_group_rewards: [BaseRewardRouterRewards; 8],
}

impl Default for NcnRewardRoute {
    fn default() -> Self {
        Self {
            operator: Pubkey::default(),
            ncn_fee_group_rewards: [BaseRewardRouterRewards::default();
                NcnFeeGroup::FEE_GROUP_COUNT],
        }
    }
}

impl NcnRewardRoute {
    pub fn new(
        operator: &Pubkey,
        ncn_fee_group: NcnFeeGroup,
        rewards: u64,
    ) -> Result<Self, TipRouterError> {
        let mut route = Self {
            operator: *operator,
            ncn_fee_group_rewards: [BaseRewardRouterRewards::default();
                NcnFeeGroup::FEE_GROUP_COUNT],
        };

        route.set_rewards(ncn_fee_group, rewards)?;

        Ok(route)
    }

    pub const fn operator(&self) -> &Pubkey {
        &self.operator
    }

    pub fn rewards(&self, ncn_fee_group: NcnFeeGroup) -> Result<u64, TipRouterError> {
        let group_index = ncn_fee_group.group_index()?;
        Ok(self.ncn_fee_group_rewards[group_index].rewards())
    }

    pub fn is_empty(&self) -> bool {
        self.operator.eq(&Pubkey::default())
    }

    pub fn has_rewards(&self) -> Result<bool, TipRouterError> {
        for ncn_fee_group in NcnFeeGroup::all_groups().iter() {
            if self.rewards(*ncn_fee_group)? > 0 {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn set_rewards(
        &mut self,
        ncn_fee_group: NcnFeeGroup,
        rewards: u64,
    ) -> Result<(), TipRouterError> {
        let group_index = ncn_fee_group.group_index()?;
        self.ncn_fee_group_rewards[group_index].rewards = PodU64::from(rewards);

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

/// Uninitiatilized, no-data account used to hold SOL for routing rewards to BaseRewardRouter
/// Must be empty and uninitialized to be used as a payer or `transfer` instructions fail
pub struct BaseRewardReceiver {}

impl BaseRewardReceiver {
    pub fn seeds(ncn: &Pubkey, ncn_epoch: u64) -> Vec<Vec<u8>> {
        vec![
            b"base_reward_receiver".to_vec(),
            ncn.to_bytes().to_vec(),
            ncn_epoch.to_le_bytes().to_vec(),
        ]
    }

    pub fn find_program_address(
        program_id: &Pubkey,
        ncn: &Pubkey,
        ncn_epoch: u64,
    ) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(ncn, ncn_epoch);
        let (address, bump) = Pubkey::find_program_address(
            &seeds.iter().map(|s| s.as_slice()).collect::<Vec<_>>(),
            program_id,
        );
        (address, bump, seeds)
    }

    pub fn load(
        program_id: &Pubkey,
        account: &AccountInfo,
        ncn: &Pubkey,
        ncn_epoch: u64,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        if account.owner.ne(&solana_program::system_program::ID) {
            msg!("BaseRewardRouterReceiver account has an invalid owner");
            return Err(ProgramError::InvalidAccountOwner);
        }

        if expect_writable && !account.is_writable {
            msg!("BaseRewardRouterReceiver account is not writable");
            return Err(ProgramError::InvalidAccountData);
        }

        if account
            .key
            .ne(&Self::find_program_address(program_id, ncn, ncn_epoch).0)
        {
            msg!("BaseRewardRouterReceiver account is not at the correct PDA");
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct BaseRewardRouterRewards {
    rewards: PodU64,
}

impl BaseRewardRouterRewards {
    pub fn rewards(self) -> u64 {
        self.rewards.into()
    }
}

#[cfg(test)]
mod tests {
    use solana_program::pubkey::Pubkey;

    use super::*;
    use crate::{ballot_box::Ballot, stake_weight::StakeWeights, utils::assert_tip_router_error};

    const TEST_EPOCH: u64 = 1;
    const TEST_CURRENT_SLOT: u64 = 100;
    const TEST_VALID_SLOTS_AFTER_CONSENSUS: u64 = 1000;

    pub fn get_test_ballot_box() -> BallotBox {
        let ncn = Pubkey::new_unique();
        let epoch = TEST_EPOCH;
        let current_slot = TEST_CURRENT_SLOT;
        let bump = 1;

        BallotBox::new(&ncn, epoch, bump, current_slot)
    }

    pub fn cast_test_vote(
        ballot_box: &mut BallotBox,
        group: NcnFeeGroup,
        stake_weight: u128,
        reward_multiplier: u64,
        merkle_root_prefix: u8,
    ) {
        let operator = Pubkey::new_unique();
        let merkle_root = [merkle_root_prefix; 32];
        let ballot = Ballot::new(&merkle_root);
        let stake_weights = StakeWeights::snapshot(group, stake_weight, reward_multiplier).unwrap();

        ballot_box
            .cast_vote(
                &operator,
                &ballot,
                &stake_weights,
                TEST_CURRENT_SLOT,
                TEST_VALID_SLOTS_AFTER_CONSENSUS,
            )
            .unwrap();
    }

    pub fn get_test_operators(ballot_box: &BallotBox) -> Vec<Pubkey> {
        ballot_box
            .operator_votes()
            .iter()
            .filter(|vote| !vote.is_empty())
            .map(|votes| *votes.operator())
            .collect()
    }

    pub fn get_test_total_stake_weights(ballot_box: &BallotBox) -> StakeWeights {
        let mut total_stake_weights = StakeWeights::default();
        for vote in ballot_box.operator_votes() {
            total_stake_weights.increment(vote.stake_weights()).unwrap();
        }

        total_stake_weights
    }

    #[test]
    fn test_len() {
        use std::mem::size_of;

        let expected_total = size_of::<Pubkey>() // ncn
            + size_of::<PodU64>() // ncn_epoch
            + 1 // bump
            + size_of::<PodU64>() // slot_created
            + size_of::<PodU64>() // total_rewards
            + size_of::<PodU64>() // reward_pool
            + size_of::<PodU64>() // rewards_processed
            + 128 // reserved
            + 1 // last_ncn_group_index
            + size_of::<PodU16>() // last_vote_index
            + size_of::<PodU64>() // last_rewards_to_process
            + size_of::<BaseRewardRouterRewards>() * NcnFeeGroup::FEE_GROUP_COUNT // base_fee_group_rewards
            + size_of::<BaseRewardRouterRewards>() * NcnFeeGroup::FEE_GROUP_COUNT // ncn_fee_group_rewards
            + size_of::<NcnRewardRoute>() * MAX_OPERATORS; // ncn_fee_group_reward_routes

        assert_eq!(size_of::<BaseRewardRouter>(), expected_total);
    }

    #[test]
    fn test_route_incoming_rewards() {
        let mut router = BaseRewardRouter::new(
            &Pubkey::new_unique(), // ncn
            1,                     // ncn_epoch
            1,                     // bump
            100,                   // slot_created
        );

        // Initial state checks
        assert_eq!(router.total_rewards(), 0);
        assert_eq!(router.reward_pool(), 0);
        assert_eq!(router.rewards_processed(), 0);

        // Test routing 1000 lamports
        let account_balance = 1000;
        router.route_incoming_rewards(0, account_balance).unwrap();

        // Verify rewards were routed correctly
        assert_eq!(router.total_rewards(), 1000);
        assert_eq!(router.reward_pool(), 1000);
        assert_eq!(router.rewards_processed(), 0);

        // Test routing additional 500 lamports
        let account_balance = 1500;
        router.route_incoming_rewards(0, account_balance).unwrap();

        // Verify total rewards increased by difference
        assert_eq!(router.total_rewards(), 1500);
        assert_eq!(router.reward_pool(), 1500);
        assert_eq!(router.rewards_processed(), 0);

        // Test attempting to route with lower balance (should fail)
        let result = router.route_incoming_rewards(0, 1000);
        assert!(result.is_err());

        // Verify state didn't change after failed routing
        assert_eq!(router.total_rewards(), 1500);
        assert_eq!(router.reward_pool(), 1500);
        assert_eq!(router.rewards_processed(), 0);
    }

    #[test]
    fn test_route_reward_pool() {
        const INCOMING_REWARDS: u64 = 1000;

        let mut router = BaseRewardRouter::new(
            &Pubkey::new_unique(), // ncn
            1,                     // ncn_epoch
            1,                     // bump
            100,                   // slot_created
        );

        // Groups
        let base_group = BaseFeeGroup::default();
        let ncn_group = NcnFeeGroup::default();

        // Fees
        let fees = Fees::new(900, 100, 1).unwrap();

        // Route incoming rewards
        router.route_incoming_rewards(0, INCOMING_REWARDS).unwrap();

        assert_eq!(router.total_rewards(), INCOMING_REWARDS);
        assert_eq!(router.reward_pool(), INCOMING_REWARDS);

        router.route_reward_pool(&fees).unwrap();

        assert_eq!(router.total_rewards(), INCOMING_REWARDS);
        assert_eq!(router.reward_pool(), 0);
        assert_eq!(router.base_fee_group_reward(base_group).unwrap(), 900);
        assert_eq!(router.ncn_fee_group_rewards(ncn_group).unwrap(), 100);
    }

    #[test]
    fn test_route_reward_pool_multiple_groups() {
        const INCOMING_REWARDS: u64 = 1600;

        let mut router = BaseRewardRouter::new(
            &Pubkey::new_unique(), // ncn
            1,                     // ncn_epoch
            1,                     // bump
            100,                   // slot_created
        );

        // Fees
        let mut fees = Fees::new(100, 100, 1).unwrap();

        for group in BaseFeeGroup::all_groups().iter() {
            fees.set_base_fee_bps(*group, 100).unwrap();
        }

        for group in NcnFeeGroup::all_groups().iter() {
            fees.set_ncn_fee_bps(*group, 100).unwrap();
        }

        // Route incoming rewards
        router.route_incoming_rewards(0, INCOMING_REWARDS).unwrap();

        assert_eq!(router.total_rewards(), INCOMING_REWARDS);
        assert_eq!(router.reward_pool(), INCOMING_REWARDS);

        router.route_reward_pool(&fees).unwrap();

        assert_eq!(router.total_rewards(), INCOMING_REWARDS);
        assert_eq!(router.reward_pool(), 0);

        for group in BaseFeeGroup::all_groups().iter() {
            assert_eq!(router.base_fee_group_reward(*group).unwrap(), 100);
        }

        for group in NcnFeeGroup::all_groups().iter() {
            assert_eq!(router.ncn_fee_group_rewards(*group).unwrap(), 100);
        }
    }

    #[test]
    fn test_rounding() {
        const INCOMING_REWARDS: u64 = 101;

        let mut router = BaseRewardRouter::new(
            &Pubkey::new_unique(), // ncn
            1,                     // ncn_epoch
            1,                     // bump
            100,                   // slot_created
        );

        // Fees - all base groups and ncn groups
        let fees = Fees::new(99, 1, 1).unwrap();

        // Route incoming rewards
        router.route_incoming_rewards(0, INCOMING_REWARDS).unwrap();

        assert_eq!(router.total_rewards(), INCOMING_REWARDS);
        assert_eq!(router.reward_pool(), INCOMING_REWARDS);

        router.route_reward_pool(&fees).unwrap();

        assert_eq!(router.total_rewards(), INCOMING_REWARDS);
        assert_eq!(router.reward_pool(), 0);

        assert_eq!(
            router
                .base_fee_group_reward(BaseFeeGroup::default())
                .unwrap(),
            100
        );
        assert_eq!(
            router
                .ncn_fee_group_rewards(NcnFeeGroup::default())
                .unwrap(),
            1
        );
    }

    #[test]
    fn test_route_to_operators_consensus_not_reached() {
        const INCOMING_REWARDS: u64 = 1600;

        let mut router = BaseRewardRouter::new(
            &Pubkey::new_unique(), // ncn
            1,                     // ncn_epoch
            1,                     // bump
            100,                   // slot_created
        );

        // Fees
        let mut fees = Fees::new(100, 100, 1).unwrap();

        for group in BaseFeeGroup::all_groups().iter() {
            fees.set_base_fee_bps(*group, 100).unwrap();
        }

        for group in NcnFeeGroup::all_groups().iter() {
            fees.set_ncn_fee_bps(*group, 100).unwrap();
        }

        // Route incoming rewards
        router.route_incoming_rewards(0, INCOMING_REWARDS).unwrap();

        assert_eq!(router.total_rewards(), INCOMING_REWARDS);
        assert_eq!(router.reward_pool(), INCOMING_REWARDS);

        router.route_reward_pool(&fees).unwrap();

        assert_eq!(router.total_rewards(), INCOMING_REWARDS);
        assert_eq!(router.reward_pool(), 0);

        for group in BaseFeeGroup::all_groups().iter() {
            assert_eq!(router.base_fee_group_reward(*group).unwrap(), 100);
        }

        for group in NcnFeeGroup::all_groups().iter() {
            assert_eq!(router.ncn_fee_group_rewards(*group).unwrap(), 100);
        }

        let (ballot_box, _) = {
            let mut ballot_box = get_test_ballot_box();

            for group in NcnFeeGroup::all_groups().iter() {
                cast_test_vote(&mut ballot_box, *group, 200, 1, 1);
            }

            (ballot_box, get_test_operators(&ballot_box))
        };

        let result = router.route_ncn_fee_group_rewards(&ballot_box, 100);

        assert_tip_router_error(result, TipRouterError::ConsensusNotReached);
    }

    #[test]
    fn test_route_to_operators() {
        const INCOMING_REWARDS: u64 = 1600;

        let mut router = BaseRewardRouter::new(
            &Pubkey::new_unique(), // ncn
            1,                     // ncn_epoch
            1,                     // bump
            100,                   // slot_created
        );

        // Fees
        let mut fees = Fees::new(100, 100, 1).unwrap();

        for group in BaseFeeGroup::all_groups().iter() {
            fees.set_base_fee_bps(*group, 100).unwrap();
        }

        for group in NcnFeeGroup::all_groups().iter() {
            fees.set_ncn_fee_bps(*group, 100).unwrap();
        }

        // Route incoming rewards
        router.route_incoming_rewards(0, INCOMING_REWARDS).unwrap();

        assert_eq!(router.total_rewards(), INCOMING_REWARDS);
        assert_eq!(router.reward_pool(), INCOMING_REWARDS);

        router.route_reward_pool(&fees).unwrap();

        assert_eq!(router.total_rewards(), INCOMING_REWARDS);
        assert_eq!(router.reward_pool(), 0);

        for group in BaseFeeGroup::all_groups().iter() {
            assert_eq!(router.base_fee_group_reward(*group).unwrap(), 100);
        }

        for group in NcnFeeGroup::all_groups().iter() {
            assert_eq!(router.ncn_fee_group_rewards(*group).unwrap(), 100);
        }

        let (ballot_box, operators) = {
            let mut ballot_box = get_test_ballot_box();

            for group in NcnFeeGroup::all_groups().iter() {
                cast_test_vote(&mut ballot_box, *group, 200, 1, 1);
            }

            let total_stake_weights = get_test_total_stake_weights(&ballot_box);

            ballot_box
                .tally_votes(total_stake_weights.stake_weight(), TEST_CURRENT_SLOT)
                .unwrap();

            (ballot_box, get_test_operators(&ballot_box))
        };

        router
            .route_ncn_fee_group_rewards(&ballot_box, 100)
            .unwrap();

        for operator in operators.iter() {
            let route = router.ncn_fee_group_reward_route(operator).unwrap();

            let mut rewards = 0;
            for group in NcnFeeGroup::all_groups().iter() {
                rewards += route.rewards(*group).unwrap();
            }

            assert_eq!(rewards, 100);
        }
    }

    #[test]
    fn test_route_to_operators_with_wrong_vote() {
        const INCOMING_REWARDS: u64 = 1600;

        let mut router = BaseRewardRouter::new(
            &Pubkey::new_unique(), // ncn
            1,                     // ncn_epoch
            1,                     // bump
            100,                   // slot_created
        );

        // Fees
        let mut fees = Fees::new(100, 100, 1).unwrap();

        for group in BaseFeeGroup::all_groups().iter() {
            fees.set_base_fee_bps(*group, 100).unwrap();
        }

        for group in NcnFeeGroup::all_groups().iter() {
            fees.set_ncn_fee_bps(*group, 100).unwrap();
        }

        // Route incoming rewards
        router.route_incoming_rewards(0, INCOMING_REWARDS).unwrap();

        assert_eq!(router.total_rewards(), INCOMING_REWARDS);
        assert_eq!(router.reward_pool(), INCOMING_REWARDS);

        router.route_reward_pool(&fees).unwrap();

        assert_eq!(router.total_rewards(), INCOMING_REWARDS);
        assert_eq!(router.reward_pool(), 0);

        for group in BaseFeeGroup::all_groups().iter() {
            assert_eq!(router.base_fee_group_reward(*group).unwrap(), 100);
        }

        for group in NcnFeeGroup::all_groups().iter() {
            assert_eq!(router.ncn_fee_group_rewards(*group).unwrap(), 100);
        }

        let (ballot_box, _) = {
            let mut ballot_box = get_test_ballot_box();

            for group in NcnFeeGroup::all_groups().iter() {
                if group == &NcnFeeGroup::default() {
                    cast_test_vote(&mut ballot_box, *group, 200, 1, 1);
                } else {
                    cast_test_vote(&mut ballot_box, *group, 200, 1, 2);
                }
            }

            let total_stake_weights = get_test_total_stake_weights(&ballot_box);

            ballot_box
                .tally_votes(total_stake_weights.stake_weight(), TEST_CURRENT_SLOT)
                .unwrap();

            (ballot_box, get_test_operators(&ballot_box))
        };

        router
            .route_ncn_fee_group_rewards(&ballot_box, 100)
            .unwrap();

        // Operator 1, did not vote with consensus, so it should not have a route
        let route_count = router
            .ncn_fee_group_reward_routes()
            .iter()
            .filter(|route| !route.is_empty())
            .count();

        assert_eq!(route_count, 7);
    }

    #[test]
    fn test_route_to_max_operators() {
        const INCOMING_REWARDS: u64 = 256_000;

        let mut router = BaseRewardRouter::new(
            &Pubkey::new_unique(), // ncn
            1,                     // ncn_epoch
            1,                     // bump
            100,                   // slot_created
        );

        // Fees
        let fees = Fees::new(0, 100, 1).unwrap();

        // Route incoming rewards
        router.route_incoming_rewards(0, INCOMING_REWARDS).unwrap();

        assert_eq!(router.total_rewards(), INCOMING_REWARDS);
        assert_eq!(router.reward_pool(), INCOMING_REWARDS);

        router.route_reward_pool(&fees).unwrap();

        assert_eq!(router.total_rewards(), INCOMING_REWARDS);
        assert_eq!(router.reward_pool(), 0);

        for group in BaseFeeGroup::all_groups().iter() {
            assert_eq!(router.base_fee_group_reward(*group).unwrap(), 0);
        }

        assert_eq!(
            router
                .ncn_fee_group_rewards(NcnFeeGroup::default())
                .unwrap(),
            INCOMING_REWARDS
        );

        let (ballot_box, operators) = {
            let mut ballot_box = get_test_ballot_box();

            for _ in 0..256 {
                cast_test_vote(&mut ballot_box, NcnFeeGroup::default(), 200, 1, 1);
            }

            let total_stake_weights = get_test_total_stake_weights(&ballot_box);

            ballot_box
                .tally_votes(total_stake_weights.stake_weight(), TEST_CURRENT_SLOT)
                .unwrap();

            (ballot_box, get_test_operators(&ballot_box))
        };

        router
            .route_ncn_fee_group_rewards(&ballot_box, 1000)
            .unwrap();

        assert!(!router.still_routing());

        for operator in operators.iter() {
            let route = router.ncn_fee_group_reward_route(operator).unwrap();

            let mut rewards = 0;
            for group in NcnFeeGroup::all_groups().iter() {
                rewards += route.rewards(*group).unwrap();
            }

            assert_eq!(rewards, 1000);
        }
    }

    #[test]
    fn test_route_with_interruption() {
        const INCOMING_REWARDS: u64 = 256_000;

        let mut router = BaseRewardRouter::new(
            &Pubkey::new_unique(), // ncn
            1,                     // ncn_epoch
            1,                     // bump
            100,                   // slot_created
        );

        // Fees
        let fees = Fees::new(0, 100, 1).unwrap();

        // Route incoming rewards
        router.route_incoming_rewards(0, INCOMING_REWARDS).unwrap();

        assert_eq!(router.total_rewards(), INCOMING_REWARDS);
        assert_eq!(router.reward_pool(), INCOMING_REWARDS);

        router.route_reward_pool(&fees).unwrap();

        assert_eq!(router.total_rewards(), INCOMING_REWARDS);
        assert_eq!(router.reward_pool(), 0);

        for group in BaseFeeGroup::all_groups().iter() {
            assert_eq!(router.base_fee_group_reward(*group).unwrap(), 0);
        }

        assert_eq!(
            router
                .ncn_fee_group_rewards(NcnFeeGroup::default())
                .unwrap(),
            INCOMING_REWARDS
        );

        let (ballot_box, operators) = {
            let mut ballot_box = get_test_ballot_box();

            for _ in 0..256 {
                cast_test_vote(&mut ballot_box, NcnFeeGroup::default(), 200, 1, 1);
            }

            let total_stake_weights = get_test_total_stake_weights(&ballot_box);

            ballot_box
                .tally_votes(total_stake_weights.stake_weight(), TEST_CURRENT_SLOT)
                .unwrap();

            (ballot_box, get_test_operators(&ballot_box))
        };

        router.route_ncn_fee_group_rewards(&ballot_box, 5).unwrap();

        assert!(router.still_routing());

        router
            .route_ncn_fee_group_rewards(&ballot_box, 1000)
            .unwrap();

        assert!(!router.still_routing());

        for operator in operators.iter() {
            let route = router.ncn_fee_group_reward_route(operator).unwrap();

            let mut rewards = 0;
            for group in NcnFeeGroup::all_groups().iter() {
                rewards += route.rewards(*group).unwrap();
            }

            assert_eq!(rewards, 1000);
        }
    }
}
