use core::mem::size_of;

use bytemuck::{Pod, Zeroable};
use jito_bytemuck::{
    types::{PodU16, PodU64},
    AccountDeserialize, Discriminator,
};
use jito_vault_core::MAX_BPS;
use shank::{ShankAccount, ShankType};
use solana_program::{
    account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey, rent::Rent,
};
use spl_math::precise_number::PreciseNumber;

use crate::{
    constants::MAX_VAULTS, discriminators::Discriminators, epoch_snapshot::OperatorSnapshot,
    error::TipRouterError, ncn_fee_group::NcnFeeGroup,
};

// PDA'd ["epoch_reward_router", NCN, NCN_EPOCH_SLOT]
#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, AccountDeserialize, ShankAccount)]
#[repr(C)]
pub struct NcnRewardRouter {
    /// The NcnFeeGroup this router is associated with
    ncn_fee_group: NcnFeeGroup,
    /// The operator the router is associated with
    operator: Pubkey,
    /// The NCN the router is associated with
    ncn: Pubkey,
    /// The epoch the router is associated with
    epoch: PodU64,
    /// The bump seed for the PDA
    bump: u8,
    /// The slot the router was created
    slot_created: PodU64,
    /// The total rewards that have been routed ( in lamports )
    total_rewards: PodU64,
    /// The rewards in the reward pool ( in lamports )
    reward_pool: PodU64,
    /// The rewards that have been processed ( in lamports )
    rewards_processed: PodU64,
    /// Rewards to go to the operator ( in lamports )
    operator_rewards: PodU64,
    /// Reserved space
    reserved: [u8; 128],
    // Routing state - so we can recover from a partial routing
    /// The last rewards to process
    last_rewards_to_process: PodU64,
    /// The last vault operator delegation index
    last_vault_operator_delegation_index: PodU16,
    /// Routes to vaults
    vault_reward_routes: [VaultRewardRoute; 64],
}

impl Discriminator for NcnRewardRouter {
    const DISCRIMINATOR: u8 = Discriminators::NcnRewardRouter as u8;
}

impl NcnRewardRouter {
    pub const SIZE: usize = 8 + size_of::<Self>();

    pub const NO_LAST_REWARDS_TO_PROCESS: u64 = u64::MAX;
    pub const NO_LAST_VAULT_OPERATION_DELEGATION_INDEX: u16 = u16::MAX;
    pub const MAX_ROUTE_NCN_ITERATIONS: u16 = 30;

    pub fn new(
        ncn_fee_group: NcnFeeGroup,
        operator: &Pubkey,
        ncn: &Pubkey,
        ncn_epoch: u64,
        bump: u8,
        slot_created: u64,
    ) -> Self {
        Self {
            ncn_fee_group,
            operator: *operator,
            ncn: *ncn,
            epoch: PodU64::from(ncn_epoch),
            bump,
            slot_created: PodU64::from(slot_created),
            total_rewards: PodU64::from(0),
            reward_pool: PodU64::from(0),
            rewards_processed: PodU64::from(0),
            operator_rewards: PodU64::from(0),
            reserved: [0; 128],
            last_rewards_to_process: PodU64::from(Self::NO_LAST_REWARDS_TO_PROCESS),
            last_vault_operator_delegation_index: PodU16::from(
                Self::NO_LAST_VAULT_OPERATION_DELEGATION_INDEX,
            ),
            vault_reward_routes: [VaultRewardRoute::default(); MAX_VAULTS],
        }
    }

    pub fn seeds(
        ncn_fee_group: NcnFeeGroup,
        operator: &Pubkey,
        ncn: &Pubkey,
        ncn_epoch: u64,
    ) -> Vec<Vec<u8>> {
        Vec::from_iter(
            [
                b"ncn_reward_router".to_vec(),
                vec![ncn_fee_group.group],
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
        ncn_fee_group: NcnFeeGroup,
        operator: &Pubkey,
        ncn: &Pubkey,
        ncn_epoch: u64,
    ) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(ncn_fee_group, operator, ncn, ncn_epoch);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
    }

    pub fn load(
        program_id: &Pubkey,
        ncn_fee_group: NcnFeeGroup,
        operator: &Pubkey,
        ncn: &Pubkey,
        ncn_epoch: u64,
        account: &AccountInfo,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        if account.owner.ne(program_id) {
            msg!("NCN Reward Router account has an invalid owner");
            return Err(ProgramError::InvalidAccountOwner);
        }
        if account.data_is_empty() {
            msg!("NCN Reward Router account data is empty");
            return Err(ProgramError::InvalidAccountData);
        }
        if expect_writable && !account.is_writable {
            msg!("NCN Reward Router account is not writable");
            return Err(ProgramError::InvalidAccountData);
        }
        if account.data.borrow()[0].ne(&Self::DISCRIMINATOR) {
            msg!("NCN Reward Router account discriminator is invalid");
            return Err(ProgramError::InvalidAccountData);
        }
        if account.key.ne(&Self::find_program_address(
            program_id,
            ncn_fee_group,
            operator,
            ncn,
            ncn_epoch,
        )
        .0)
        {
            msg!("NCN Reward Router account is not at the correct PDA");
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }

    pub const fn ncn_fee_group(&self) -> NcnFeeGroup {
        self.ncn_fee_group
    }

    pub const fn operator(&self) -> &Pubkey {
        &self.operator
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

    pub const fn vault_reward_routes(&self) -> &[VaultRewardRoute] {
        &self.vault_reward_routes
    }

    // ----------------- ROUTE STATE TRACKING --------------
    pub fn last_rewards_to_process(&self) -> u64 {
        self.last_rewards_to_process.into()
    }

    pub fn last_vault_operator_delegation_index(&self) -> u16 {
        self.last_vault_operator_delegation_index.into()
    }

    pub fn resume_routing_state(&mut self, rewards_to_process: u64) -> (u64, usize) {
        if !self.still_routing() {
            return (rewards_to_process, 0);
        }

        (
            self.last_rewards_to_process(),
            self.last_vault_operator_delegation_index() as usize,
        )
    }

    pub fn save_routing_state(
        &mut self,
        rewards_to_process: u64,
        vault_operator_delegation_index: usize,
    ) {
        self.last_rewards_to_process = PodU64::from(rewards_to_process);
        self.last_vault_operator_delegation_index =
            PodU16::from(vault_operator_delegation_index as u16);
    }

    pub fn finish_routing_state(&mut self) {
        self.last_rewards_to_process = PodU64::from(Self::NO_LAST_REWARDS_TO_PROCESS);
        self.last_vault_operator_delegation_index =
            PodU16::from(Self::NO_LAST_VAULT_OPERATION_DELEGATION_INDEX);
    }

    pub fn still_routing(&self) -> bool {
        self.last_rewards_to_process() != Self::NO_LAST_REWARDS_TO_PROCESS
            || self.last_vault_operator_delegation_index()
                != Self::NO_LAST_VAULT_OPERATION_DELEGATION_INDEX
    }

    // ------------------------ ROUTING ------------------------
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

    pub fn route_operator_rewards(
        &mut self,
        operator_snapshot: &OperatorSnapshot,
    ) -> Result<(), TipRouterError> {
        let rewards_to_process: u64 = self.reward_pool();

        // Operator Fee Rewards
        {
            let operator_fee_bps = operator_snapshot.operator_fee_bps();
            let operator_rewards =
                Self::calculate_operator_reward(operator_fee_bps as u64, rewards_to_process)?;

            self.route_from_reward_pool(operator_rewards)?;
            self.route_to_operator_rewards(operator_rewards)?;
        }

        Ok(())
    }

    pub fn route_reward_pool(
        &mut self,
        operator_snapshot: &OperatorSnapshot,
        max_iterations: u16,
    ) -> Result<(), TipRouterError> {
        {
            let operator_stake_weight = operator_snapshot.stake_weights();
            let vault_ncn_fee_group = self.ncn_fee_group();
            let rewards_to_process: u64 = self.reward_pool();

            let (rewards_to_process, starting_vault_operator_delegation_index) =
                self.resume_routing_state(rewards_to_process);

            let mut iterations: u16 = 0;

            for vault_operator_delegation_index in starting_vault_operator_delegation_index
                ..operator_snapshot.vault_operator_stake_weight().len()
            {
                let vault_operator_delegation = operator_snapshot.vault_operator_stake_weight()
                    [vault_operator_delegation_index];

                // Update iteration state
                {
                    iterations = iterations
                        .checked_add(1)
                        .ok_or(TipRouterError::ArithmeticOverflow)?;

                    if iterations >= max_iterations {
                        msg!(
                            "Reached max iterations, saving state and exiting {}/{}",
                            rewards_to_process,
                            vault_operator_delegation_index
                        );
                        self.save_routing_state(
                            rewards_to_process,
                            vault_operator_delegation_index,
                        );
                        return Ok(());
                    }
                }

                let vault = vault_operator_delegation.vault();

                let vault_reward_stake_weight = vault_operator_delegation
                    .stake_weights()
                    .ncn_fee_group_stake_weight(vault_ncn_fee_group)?;

                let operator_reward_stake_weight =
                    operator_stake_weight.ncn_fee_group_stake_weight(vault_ncn_fee_group)?;

                let vault_reward = Self::calculate_vault_reward(
                    vault_reward_stake_weight,
                    operator_reward_stake_weight,
                    rewards_to_process,
                )?;

                self.route_from_reward_pool(vault_reward)?;
                self.route_to_vault_reward_route(vault, vault_reward)?;
            }

            self.finish_routing_state();
        }

        // Operator gets any remainder
        {
            let leftover_rewards = self.reward_pool();

            self.route_from_reward_pool(leftover_rewards)?;
            self.route_to_operator_rewards(leftover_rewards)?;
        }

        Ok(())
    }

    // ------------------------ CALCULATIONS ------------------------
    fn calculate_operator_reward(
        fee_bps: u64,
        rewards_to_process: u64,
    ) -> Result<u64, TipRouterError> {
        if fee_bps == 0 || rewards_to_process == 0 {
            return Ok(0);
        }

        let precise_operator_fee_bps =
            PreciseNumber::new(fee_bps as u128).ok_or(TipRouterError::NewPreciseNumberError)?;

        let precise_max_bps =
            PreciseNumber::new(MAX_BPS as u128).ok_or(TipRouterError::NewPreciseNumberError)?;

        let precise_rewards_to_process = PreciseNumber::new(rewards_to_process as u128)
            .ok_or(TipRouterError::NewPreciseNumberError)?;

        let precise_operator_rewards = precise_rewards_to_process
            .checked_mul(&precise_operator_fee_bps)
            .and_then(|x| x.checked_div(&precise_max_bps))
            .ok_or(TipRouterError::ArithmeticOverflow)?;

        let floored_precise_operator_rewards = precise_operator_rewards
            .floor()
            .ok_or(TipRouterError::ArithmeticFloorError)?;

        let operator_rewards_u128: u128 = floored_precise_operator_rewards
            .to_imprecise()
            .ok_or(TipRouterError::CastToImpreciseNumberError)?;
        let operator_rewards: u64 = operator_rewards_u128
            .try_into()
            .map_err(|_| TipRouterError::CastToU64Error)?;

        Ok(operator_rewards)
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

        let precise_vault_reward = precise_rewards_to_process
            .checked_mul(&precise_vault_reward_stake_weight)
            .and_then(|x| x.checked_div(&precise_operator_reward_stake_weight))
            .ok_or(TipRouterError::ArithmeticOverflow)?;

        let floored_precise_vault_reward = precise_vault_reward
            .floor()
            .ok_or(TipRouterError::ArithmeticFloorError)?;

        let vault_reward_u128: u128 = floored_precise_vault_reward
            .to_imprecise()
            .ok_or(TipRouterError::CastToImpreciseNumberError)?;

        let vault_reward: u64 = vault_reward_u128
            .try_into()
            .map_err(|_| TipRouterError::CastToU64Error)?;

        Ok(vault_reward)
    }

    // ------------------------ REWARD POOL ------------------------
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

    // ------------------------ REWARDS PROCESSED ------------------------
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

    // ------------------------ OPERATOR REWARDS ------------------------

    pub fn operator_rewards(&self) -> u64 {
        self.operator_rewards.into()
    }

    pub fn route_to_operator_rewards(&mut self, rewards: u64) -> Result<(), TipRouterError> {
        if rewards == 0 {
            return Ok(());
        }

        self.increment_rewards_processed(rewards)?;

        self.operator_rewards = PodU64::from(
            self.operator_rewards()
                .checked_add(rewards)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );
        Ok(())
    }

    pub fn distribute_operator_rewards(&mut self) -> Result<u64, TipRouterError> {
        let rewards = self.operator_rewards();

        self.operator_rewards = PodU64::from(
            self.operator_rewards()
                .checked_sub(rewards)
                .ok_or(TipRouterError::ArithmeticUnderflowError)?,
        );

        self.decrement_rewards_processed(rewards)?;
        Ok(rewards)
    }

    // ------------------------ VAULT REWARD ROUTES ------------------------

    pub fn vault_reward_route(&self, vault: &Pubkey) -> Result<&VaultRewardRoute, TipRouterError> {
        for vault_reward in self.vault_reward_routes.iter() {
            if vault_reward.vault().eq(vault) {
                return Ok(vault_reward);
            }
        }
        Err(TipRouterError::VaultRewardNotFound)
    }

    pub fn route_to_vault_reward_route(
        &mut self,
        vault: &Pubkey,
        rewards: u64,
    ) -> Result<(), TipRouterError> {
        if rewards == 0 {
            return Ok(());
        }

        self.increment_rewards_processed(rewards)?;

        for vault_reward in self.vault_reward_routes.iter_mut() {
            if vault_reward.vault().eq(vault) {
                vault_reward.increment_rewards(rewards)?;
                return Ok(());
            }
        }

        for vault_reward in self.vault_reward_routes.iter_mut() {
            if vault_reward.vault().eq(&Pubkey::default()) {
                *vault_reward = VaultRewardRoute::new(vault, rewards)?;
                return Ok(());
            }
        }

        Err(TipRouterError::OperatorRewardListFull)
    }

    pub fn distribute_vault_reward_route(&mut self, vault: &Pubkey) -> Result<u64, TipRouterError> {
        for route in self.vault_reward_routes.iter_mut() {
            if route.vault().eq(vault) {
                let rewards = route.rewards();

                route.decrement_rewards(rewards)?;
                self.decrement_rewards_processed(rewards)?;
                return Ok(rewards);
            }
        }
        Err(TipRouterError::OperatorRewardNotFound)
    }
}

/// Uninitiatilized, no-data account used to hold SOL for routing rewards to NcnRewardRouter
/// Must be empty and uninitialized to be used as a payer or `transfer` instructions fail
pub struct NcnRewardReceiver {}

impl NcnRewardReceiver {
    pub fn seeds(
        ncn_fee_group: NcnFeeGroup,
        operator: &Pubkey,
        ncn: &Pubkey,
        ncn_epoch: u64,
    ) -> Vec<Vec<u8>> {
        vec![
            b"ncn_reward_receiver".to_vec(),
            vec![ncn_fee_group.group],
            operator.to_bytes().to_vec(),
            ncn.to_bytes().to_vec(),
            ncn_epoch.to_le_bytes().to_vec(),
        ]
    }

    pub fn find_program_address(
        program_id: &Pubkey,
        ncn_fee_group: NcnFeeGroup,
        operator: &Pubkey,
        ncn: &Pubkey,
        ncn_epoch: u64,
    ) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(ncn_fee_group, operator, ncn, ncn_epoch);
        let (address, bump) = Pubkey::find_program_address(
            &seeds.iter().map(|s| s.as_slice()).collect::<Vec<_>>(),
            program_id,
        );
        (address, bump, seeds)
    }

    pub fn load(
        program_id: &Pubkey,
        account: &AccountInfo,
        ncn_fee_group: NcnFeeGroup,
        operator: &Pubkey,
        ncn: &Pubkey,
        ncn_epoch: u64,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        if account.owner.ne(&solana_program::system_program::ID) {
            msg!("NcnRewardRouterReceiver account has an invalid owner");
            return Err(ProgramError::InvalidAccountOwner);
        }

        if expect_writable && !account.is_writable {
            msg!("NcnRewardRouterReceiver account is not writable");
            return Err(ProgramError::InvalidAccountData);
        }

        if account.key.ne(&Self::find_program_address(
            program_id,
            ncn_fee_group,
            operator,
            ncn,
            ncn_epoch,
        )
        .0)
        {
            msg!("NcnRewardRouterReceiver account is not at the correct PDA");
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct VaultRewardRoute {
    /// The vault the rewards are routed to
    vault: Pubkey,
    /// The amount of rewards ( in lamports )
    rewards: PodU64,
}

impl VaultRewardRoute {
    pub fn new(vault: &Pubkey, rewards: u64) -> Result<Self, TipRouterError> {
        Ok(Self {
            vault: *vault,
            rewards: PodU64::from(rewards),
        })
    }

    pub const fn vault(&self) -> Pubkey {
        self.vault
    }

    pub fn rewards(&self) -> u64 {
        self.rewards.into()
    }

    pub fn is_empty(&self) -> bool {
        self.vault.eq(&Pubkey::default())
    }

    pub fn has_rewards(&self) -> bool {
        self.rewards() > 0
    }

    fn set_rewards(&mut self, rewards: u64) -> Result<(), TipRouterError> {
        self.rewards = PodU64::from(rewards);
        Ok(())
    }

    pub fn increment_rewards(&mut self, rewards: u64) -> Result<(), TipRouterError> {
        let current_rewards = self.rewards();

        let new_rewards = current_rewards
            .checked_add(rewards)
            .ok_or(TipRouterError::ArithmeticOverflow)?;

        self.set_rewards(new_rewards)
    }

    pub fn decrement_rewards(&mut self, rewards: u64) -> Result<(), TipRouterError> {
        let current_rewards = self.rewards();

        let new_rewards = current_rewards
            .checked_sub(rewards)
            .ok_or(TipRouterError::ArithmeticUnderflowError)?;

        self.set_rewards(new_rewards)
    }
}

#[cfg(test)]
mod tests {
    use solana_program::pubkey::Pubkey;

    use super::*;

    #[test]
    fn test_len() {
        use std::mem::size_of;

        let expected_total = size_of::<NcnFeeGroup>() // ncn_fee_group
            + size_of::<Pubkey>() // operator
            + size_of::<Pubkey>() // ncn
            + size_of::<PodU64>() // ncn_epoch
            + 1 // bump
            + size_of::<PodU64>() // slot_created
            + size_of::<PodU64>() // total_rewards
            + size_of::<PodU64>() // reward_pool
            + size_of::<PodU64>() // rewards_processed
            + size_of::<PodU64>() // operator_rewards
            + 128 // reserved
            + size_of::<PodU64>() // last_rewards_to_process
            + size_of::<PodU16>() // last_vault_operator_delegation_index
            + size_of::<VaultRewardRoute>() * MAX_VAULTS; // vault_reward_routes

        assert_eq!(size_of::<NcnRewardRouter>(), expected_total);
    }

    #[test]
    fn test_route_incoming_rewards() {
        let mut router = NcnRewardRouter::new(
            NcnFeeGroup::default(),
            &Pubkey::new_unique(), // ncn
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
}
