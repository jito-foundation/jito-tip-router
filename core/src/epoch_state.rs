use std::mem::size_of;

use bytemuck::{Pod, Zeroable};
use jito_bytemuck::{
    types::{PodBool, PodU64},
    AccountDeserialize, Discriminator,
};
use shank::{ShankAccount, ShankType};
use solana_program::{
    account_info::AccountInfo, epoch_schedule::EpochSchedule, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    constants::{DEFAULT_CONSENSUS_REACHED_SLOT, MAX_OPERATORS},
    discriminators::Discriminators,
    error::TipRouterError,
    loaders::check_load,
    ncn_fee_group::NcnFeeGroup,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AccountStatus {
    DNE = 0,
    Created = 1,
    CreatedWithReceiver = 2,
    Closed = 3,
}

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct EpochAccountStatus {
    epoch_state: u8,
    weight_table: u8,
    epoch_snapshot: u8,
    operator_snapshot: [u8; 256],
    ballot_box: u8,
    base_reward_router: u8,
    ncn_reward_router: [u8; 2048],
}

impl Default for EpochAccountStatus {
    fn default() -> Self {
        Self {
            epoch_state: 0,
            weight_table: 0,
            epoch_snapshot: 0,
            operator_snapshot: [0; MAX_OPERATORS],
            ballot_box: 0,
            base_reward_router: 0,
            ncn_reward_router: [0; MAX_OPERATORS * NcnFeeGroup::FEE_GROUP_COUNT],
        }
    }
}

impl EpochAccountStatus {
    pub const SIZE: usize = size_of::<Self>();

    pub const fn get_account_status(u: u8) -> Result<AccountStatus, TipRouterError> {
        match u {
            0 => Ok(AccountStatus::DNE),
            1 => Ok(AccountStatus::Created),
            2 => Ok(AccountStatus::CreatedWithReceiver),
            3 => Ok(AccountStatus::Closed),
            _ => Err(TipRouterError::InvalidAccountStatus),
        }
    }

    pub const fn epoch_state(&self) -> Result<AccountStatus, TipRouterError> {
        Self::get_account_status(self.epoch_state)
    }

    pub const fn weight_table(&self) -> Result<AccountStatus, TipRouterError> {
        Self::get_account_status(self.weight_table)
    }

    pub const fn epoch_snapshot(&self) -> Result<AccountStatus, TipRouterError> {
        Self::get_account_status(self.epoch_snapshot)
    }

    pub const fn operator_snapshot(&self, index: usize) -> Result<AccountStatus, TipRouterError> {
        Self::get_account_status(self.operator_snapshot[index])
    }

    pub const fn ballot_box(&self) -> Result<AccountStatus, TipRouterError> {
        Self::get_account_status(self.ballot_box)
    }

    pub const fn base_reward_router(&self) -> Result<AccountStatus, TipRouterError> {
        Self::get_account_status(self.base_reward_router)
    }

    pub fn ncn_reward_router(
        &self,
        index: usize,
        group: NcnFeeGroup,
    ) -> Result<AccountStatus, TipRouterError> {
        Self::get_account_status(
            self.ncn_reward_router[EpochState::get_ncn_reward_router_index(index, group)?],
        )
    }

    pub fn set_epoch_state(&mut self, status: AccountStatus) {
        self.epoch_state = status as u8;
    }

    pub fn set_weight_table(&mut self, status: AccountStatus) {
        self.weight_table = status as u8;
    }

    pub fn set_epoch_snapshot(&mut self, status: AccountStatus) {
        self.epoch_snapshot = status as u8;
    }

    pub fn set_operator_snapshot(&mut self, index: usize, status: AccountStatus) {
        self.operator_snapshot[index] = status as u8;
    }

    pub fn set_ballot_box(&mut self, status: AccountStatus) {
        self.ballot_box = status as u8;
    }

    pub fn set_base_reward_router(&mut self, status: AccountStatus) {
        self.base_reward_router = status as u8;
    }

    pub fn set_ncn_reward_router(
        &mut self,
        index: usize,
        group: NcnFeeGroup,
        status: AccountStatus,
    ) -> Result<(), TipRouterError> {
        self.ncn_reward_router[EpochState::get_ncn_reward_router_index(index, group)?] =
            status as u8;
        Ok(())
    }

    pub fn are_all_closed(&self) -> bool {
        // We don't need to check epoch state since it's the account we are closing

        if self.weight_table != AccountStatus::Closed as u8 {
            return false;
        }
        if self.epoch_snapshot != AccountStatus::Closed as u8 {
            return false;
        }
        if self.ballot_box != AccountStatus::Closed as u8 {
            return false;
        }
        if self.base_reward_router != AccountStatus::Closed as u8 {
            return false;
        }

        for operator_snapshot in self.operator_snapshot.iter() {
            if *operator_snapshot == AccountStatus::Created as u8 {
                return false;
            }
        }

        for ncn_reward_router in self.ncn_reward_router.iter() {
            if *ncn_reward_router == AccountStatus::CreatedWithReceiver as u8 {
                return false;
            }
        }

        true
    }
}

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct Progress {
    /// tally
    tally: PodU64,
    /// total
    total: PodU64,
    /// Slot Completed
    reserved: [u8; 8],
}

impl Default for Progress {
    fn default() -> Self {
        Self {
            tally: PodU64::from(Self::INVALID),
            total: PodU64::from(Self::INVALID),
            reserved: [0; 8],
        }
    }
}

impl Progress {
    pub const INVALID: u64 = u64::MAX;
    pub const SIZE: usize = size_of::<Self>();

    pub fn new(total: u64) -> Self {
        Self {
            tally: PodU64::from(0),
            total: PodU64::from(total),
            reserved: [0; 8],
        }
    }

    pub fn tally(&self) -> u64 {
        self.tally.into()
    }

    pub fn total(&self) -> u64 {
        self.total.into()
    }

    pub fn increment_one(&mut self) -> Result<(), TipRouterError> {
        self.increment(1)
    }

    pub fn increment(&mut self, amount: u64) -> Result<(), TipRouterError> {
        self.tally = PodU64::from(
            self.tally()
                .checked_add(amount)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );

        Ok(())
    }

    pub fn set_tally(&mut self, tally: u64) {
        self.tally = PodU64::from(tally);
    }

    pub fn set_total(&mut self, total: u64) {
        self.total = PodU64::from(total);
    }

    pub fn is_invalid(&self) -> bool {
        self.tally.eq(&PodU64::from(Self::INVALID)) || self.total.eq(&PodU64::from(Self::INVALID))
    }

    pub fn is_complete(&self) -> bool {
        if self.is_invalid() {
            false
        } else {
            self.tally.eq(&self.total)
        }
    }
}

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, AccountDeserialize, ShankAccount)]
#[repr(C)]
pub struct EpochState {
    /// The NCN this snapshot is for
    ncn: Pubkey,
    /// The epoch this snapshot is for
    epoch: PodU64,
    /// The bump seed for the PDA
    pub bump: u8,

    /// The time this snapshot was created
    slot_created: PodU64,

    /// Was tie breaker set
    was_tie_breaker_set: PodBool,

    /// The time consensus was reached
    slot_consensus_reached: PodU64,

    /// The number of operators
    operator_count: PodU64,

    /// The number of vaults
    vault_count: PodU64,

    /// All of the epoch accounts status
    account_status: EpochAccountStatus,

    /// Progress on weight set
    set_weight_progress: Progress,

    /// Progress on Snapshotting Epoch
    epoch_snapshot_progress: Progress,

    /// Progress on Snapshotting Operators
    operator_snapshot_progress: [Progress; 256],

    /// Progress on voting
    voting_progress: Progress,

    /// Progress on validation
    validation_progress: Progress,

    /// Upload progress
    upload_progress: Progress,

    /// Distribution progress
    total_distribution_progress: Progress,

    /// base distribution progress
    base_distribution_progress: Progress,

    /// ncn distribution progress
    ncn_distribution_progress: [Progress; 2048],

    /// Reserved space
    reserved: [u8; 1024],
}

impl Discriminator for EpochState {
    const DISCRIMINATOR: u8 = Discriminators::EpochState as u8;
}

impl EpochState {
    pub const SIZE: usize = 8 + size_of::<Self>();

    pub fn new(ncn: &Pubkey, epoch: u64, bump: u8, slot_created: u64) -> Self {
        Self {
            ncn: *ncn,
            epoch: PodU64::from(epoch),
            bump,
            slot_created: PodU64::from(slot_created),
            slot_consensus_reached: PodU64::from(DEFAULT_CONSENSUS_REACHED_SLOT),
            operator_count: PodU64::from(0),
            vault_count: PodU64::from(0),
            account_status: EpochAccountStatus::default(),
            was_tie_breaker_set: PodBool::from(false),
            set_weight_progress: Progress::default(),
            epoch_snapshot_progress: Progress::default(),
            operator_snapshot_progress: [Progress::default(); MAX_OPERATORS],
            voting_progress: Progress::default(),
            validation_progress: Progress::default(),
            upload_progress: Progress::default(),
            total_distribution_progress: Progress::default(),
            base_distribution_progress: Progress::default(),
            ncn_distribution_progress: [Progress::default();
                MAX_OPERATORS * NcnFeeGroup::FEE_GROUP_COUNT],
            reserved: [0; 1024],
        }
    }

    pub fn check_can_close(&self) -> Result<(), TipRouterError> {
        // Check all other accounts are closed
        if !self.account_status.are_all_closed() {
            msg!("Cannot close Epoch State until all other accounts are closed");
            return Err(TipRouterError::CannotCloseEpochStateAccount);
        }

        Ok(())
    }

    pub fn initialize(&mut self, ncn: &Pubkey, epoch: u64, bump: u8, slot_created: u64) {
        // Initializes field by field to avoid overflowing stack
        self.ncn = *ncn;
        self.bump = bump;
        self.epoch = PodU64::from(epoch);
        self.slot_created = PodU64::from(slot_created);
        self.slot_consensus_reached = PodU64::from(DEFAULT_CONSENSUS_REACHED_SLOT);
        self.reserved = [0; 1024];
    }

    pub fn seeds(ncn: &Pubkey, epoch: u64) -> Vec<Vec<u8>> {
        Vec::from_iter(
            [
                b"epoch_state".to_vec(),
                ncn.to_bytes().to_vec(),
                epoch.to_le_bytes().to_vec(),
            ]
            .iter()
            .cloned(),
        )
    }

    pub fn find_program_address(
        program_id: &Pubkey,
        ncn: &Pubkey,
        epoch: u64,
    ) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(ncn, epoch);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (address, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (address, bump, seeds)
    }

    pub fn load(
        program_id: &Pubkey,
        ncn: &Pubkey,
        epoch: u64,
        account: &AccountInfo,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        let expected_pda = Self::find_program_address(program_id, ncn, epoch).0;
        check_load(
            program_id,
            account,
            &expected_pda,
            Some(Self::DISCRIMINATOR),
            expect_writable,
        )
    }

    // ------------ HELPER FUNCTIONS ------------
    pub fn get_ncn_reward_router_index(
        ncn_operator_index: usize,
        group: NcnFeeGroup,
    ) -> Result<usize, TipRouterError> {
        let mut index = ncn_operator_index
            .checked_mul(NcnFeeGroup::FEE_GROUP_COUNT)
            .ok_or(TipRouterError::ArithmeticOverflow)?;
        index = index
            .checked_add(group.group.into())
            .ok_or(TipRouterError::ArithmeticOverflow)?;
        Ok(index)
    }

    pub fn _set_upload_progress(&mut self) {
        self.upload_progress = Progress::new(1);
    }

    // ------------ GETTERS ------------

    pub const fn ncn(&self) -> &Pubkey {
        &self.ncn
    }

    pub fn epoch(&self) -> u64 {
        self.epoch.into()
    }

    pub fn slot_created(&self) -> u64 {
        self.slot_created.into()
    }

    pub fn was_tie_breaker_set(&self) -> bool {
        self.was_tie_breaker_set.into()
    }

    pub fn is_consensus_reached(&self) -> bool {
        self.slot_consensus_reached() != DEFAULT_CONSENSUS_REACHED_SLOT
    }

    pub fn slot_consensus_reached(&self) -> u64 {
        self.slot_consensus_reached.into()
    }

    pub fn get_slot_consensus_reached(&self) -> Result<u64, TipRouterError> {
        if self.slot_consensus_reached() == DEFAULT_CONSENSUS_REACHED_SLOT {
            Err(TipRouterError::ConsensusNotReached)
        } else {
            Ok(self.slot_consensus_reached.into())
        }
    }

    pub fn get_epoch_consensus_reached(
        &self,
        epoch_schedule: &EpochSchedule,
    ) -> Result<u64, ProgramError> {
        let slot_consensus_reached = self.get_slot_consensus_reached()?;
        let epoch_consensus_reached = epoch_schedule.get_epoch(slot_consensus_reached);

        Ok(epoch_consensus_reached)
    }

    pub fn operator_count(&self) -> u64 {
        self.operator_count.into()
    }

    pub fn vault_count(&self) -> u64 {
        self.vault_count.into()
    }

    pub const fn account_status(&self) -> &EpochAccountStatus {
        &self.account_status
    }

    pub const fn set_weight_progress(&self) -> Progress {
        self.set_weight_progress
    }

    pub const fn epoch_snapshot_progress(&self) -> Progress {
        self.epoch_snapshot_progress
    }

    pub const fn operator_snapshot_progress(&self, ncn_operator_index: usize) -> Progress {
        self.operator_snapshot_progress[ncn_operator_index]
    }

    pub const fn voting_progress(&self) -> Progress {
        self.voting_progress
    }

    pub const fn validation_progress(&self) -> Progress {
        self.validation_progress
    }

    pub const fn upload_progress(&self) -> Progress {
        self.upload_progress
    }

    pub const fn total_distribution_progress(&self) -> Progress {
        self.total_distribution_progress
    }

    pub const fn base_distribution_progress(&self) -> Progress {
        self.base_distribution_progress
    }

    pub fn ncn_distribution_progress(
        &self,
        ncn_ncn_operator_index: usize,
        group: NcnFeeGroup,
    ) -> Result<Progress, TipRouterError> {
        let index = Self::get_ncn_reward_router_index(ncn_ncn_operator_index, group)?;
        Ok(self.ncn_distribution_progress[index])
    }

    // ------------ UPDATERS ------------
    pub fn update_realloc_epoch_state(&mut self) {
        self.account_status.set_epoch_state(AccountStatus::Created);
    }

    pub fn update_realloc_weight_table(&mut self, vault_count: u64) {
        self.account_status.set_weight_table(AccountStatus::Created);

        self.vault_count = PodU64::from(vault_count);
        self.set_weight_progress = Progress::new(vault_count);
    }

    pub fn update_set_weight(&mut self, weights_set: u64) {
        self.set_weight_progress.set_tally(weights_set)
    }

    pub fn update_initialize_epoch_snapshot(&mut self, operator_count: u64) {
        self.account_status
            .set_epoch_snapshot(AccountStatus::Created);

        self.operator_count = PodU64::from(operator_count);
        self.epoch_snapshot_progress = Progress::new(operator_count);
    }

    pub fn update_realloc_operator_snapshot(
        &mut self,
        ncn_operator_index: usize,
        is_active: bool,
    ) -> Result<(), TipRouterError> {
        self.account_status
            .set_operator_snapshot(ncn_operator_index, AccountStatus::Created);

        if is_active {
            self.operator_snapshot_progress[ncn_operator_index] =
                Progress::new(self.vault_count.into());
        } else {
            self.operator_snapshot_progress[ncn_operator_index] = Progress::new(1);
            self.operator_snapshot_progress[ncn_operator_index].increment_one()?;
            self.epoch_snapshot_progress.increment_one()?;
        }

        Ok(())
    }

    pub fn update_snapshot_vault_operator_delegation(
        &mut self,
        ncn_operator_index: usize,
        finalized: bool,
    ) -> Result<(), TipRouterError> {
        self.operator_snapshot_progress[ncn_operator_index].increment_one()?;
        if finalized {
            self.epoch_snapshot_progress.increment_one()?;
        }

        Ok(())
    }

    pub fn update_realloc_ballot_box(&mut self) {
        self.account_status.set_ballot_box(AccountStatus::Created);
        self.voting_progress = Progress::new(self.operator_count());
        self.validation_progress = Progress::new(1);
        self.upload_progress = Progress::new(1);
    }

    pub fn update_cast_vote(
        &mut self,
        operators_voted: u64,
        is_consensus_reached: bool,
        current_slot: u64,
    ) -> Result<(), TipRouterError> {
        if is_consensus_reached && !self.is_consensus_reached() {
            self.slot_consensus_reached = PodU64::from(current_slot);
        }

        self.voting_progress.set_tally(operators_voted);

        Ok(())
    }

    pub fn update_set_tie_breaker(
        &mut self,
        is_consensus_reached: bool,
        current_slot: u64,
    ) -> Result<(), TipRouterError> {
        if is_consensus_reached && !self.is_consensus_reached() {
            self.slot_consensus_reached = PodU64::from(current_slot);
            self.was_tie_breaker_set = PodBool::from(true);
        }

        Ok(())
    }

    // Just tracks the amount of times set_merkle_root is called
    pub fn update_set_merkle_root(&mut self) -> Result<(), TipRouterError> {
        self.upload_progress.increment_one()?;
        self.upload_progress.set_total(self.upload_progress.tally());
        Ok(())
    }

    pub fn update_realloc_base_reward_router(&mut self) {
        self.account_status
            .set_base_reward_router(AccountStatus::CreatedWithReceiver);
        self.base_distribution_progress = Progress::new(0);
    }

    pub fn update_realloc_ncn_reward_router(
        &mut self,
        ncn_operator_index: usize,
        group: NcnFeeGroup,
    ) -> Result<(), TipRouterError> {
        self.account_status.set_ncn_reward_router(
            ncn_operator_index,
            group,
            AccountStatus::CreatedWithReceiver,
        )?;
        self.ncn_distribution_progress
            [Self::get_ncn_reward_router_index(ncn_operator_index, group)?] = Progress::new(0);

        Ok(())
    }

    pub fn update_route_base_rewards(&mut self, total_rewards: u64) {
        self.total_distribution_progress.set_total(total_rewards);
        self.base_distribution_progress.set_total(total_rewards);
    }

    pub fn update_route_ncn_rewards(
        &mut self,
        ncn_operator_index: usize,
        group: NcnFeeGroup,
        total_rewards: u64,
    ) -> Result<(), TipRouterError> {
        self.ncn_distribution_progress
            [Self::get_ncn_reward_router_index(ncn_operator_index, group)?]
        .set_total(total_rewards);
        Ok(())
    }

    pub fn update_distribute_base_rewards(&mut self, rewards: u64) -> Result<(), TipRouterError> {
        self.total_distribution_progress.increment(rewards)?;
        self.base_distribution_progress.increment(rewards)?;
        Ok(())
    }

    pub fn update_distribute_base_ncn_rewards(
        &mut self,
        rewards: u64,
    ) -> Result<(), TipRouterError> {
        self.base_distribution_progress.increment(rewards)?;
        Ok(())
    }

    pub fn update_distribute_ncn_rewards(
        &mut self,
        ncn_operator_index: usize,
        group: NcnFeeGroup,
        rewards: u64,
    ) -> Result<(), TipRouterError> {
        self.total_distribution_progress.increment(rewards)?;
        self.ncn_distribution_progress
            [Self::get_ncn_reward_router_index(ncn_operator_index, group)?]
        .increment(rewards)?;
        Ok(())
    }

    // ---------- CLOSERS ----------
    pub fn can_close_epoch_accounts(
        &self,
        epoch_schedule: &EpochSchedule,
        epochs_after_consensus_before_close: u64,
        current_epoch: u64,
    ) -> Result<bool, ProgramError> {
        let epoch_consensus_reached = self.get_epoch_consensus_reached(epoch_schedule)?;
        let epoch_delta = current_epoch.saturating_sub(epoch_consensus_reached);
        let can_close_epoch_accounts = epoch_delta >= epochs_after_consensus_before_close;
        Ok(can_close_epoch_accounts)
    }

    pub fn close_epoch_state(&mut self) {
        self.account_status.set_epoch_state(AccountStatus::Closed);
    }

    pub fn close_weight_table(&mut self) {
        self.account_status.set_weight_table(AccountStatus::Closed);
    }

    pub fn close_epoch_snapshot(&mut self) {
        self.account_status
            .set_epoch_snapshot(AccountStatus::Closed);
    }

    pub fn close_operator_snapshot(&mut self, ncn_operator_index: usize) {
        self.account_status
            .set_operator_snapshot(ncn_operator_index, AccountStatus::Closed);
    }

    pub fn close_ballot_box(&mut self) {
        self.account_status.set_ballot_box(AccountStatus::Closed);
    }

    pub fn close_base_reward_router(&mut self) {
        self.account_status
            .set_base_reward_router(AccountStatus::Closed);
    }

    pub fn close_ncn_reward_router(
        &mut self,
        ncn_operator_index: usize,
        group: NcnFeeGroup,
    ) -> Result<(), TipRouterError> {
        self.account_status
            .set_ncn_reward_router(ncn_operator_index, group, AccountStatus::Closed)
    }

    // ------------ STATE ------------
    pub fn current_state(
        &self,
        epoch_schedule: &EpochSchedule,
        epochs_after_consensus_before_close: u64,
        current_epoch: u64,
    ) -> Result<State, ProgramError> {
        if self.account_status.weight_table()? == AccountStatus::DNE
            || !self.set_weight_progress.is_complete()
        {
            return Ok(State::SetWeight);
        }

        if self.account_status.epoch_snapshot()? == AccountStatus::DNE
            || !self.epoch_snapshot_progress.is_complete()
        {
            return Ok(State::Snapshot);
        }

        if self.account_status.ballot_box()? == AccountStatus::DNE || !self.is_consensus_reached() {
            return Ok(State::Vote);
        }

        if self.account_status.base_reward_router()? == AccountStatus::DNE {
            return Ok(State::SetupRouter);
        }

        // The upload state is not required to progress to the next state

        let can_close_epoch_accounts = self.can_close_epoch_accounts(
            epoch_schedule,
            epochs_after_consensus_before_close,
            current_epoch,
        )?;
        if can_close_epoch_accounts {
            return Ok(State::Close);
        }

        Ok(State::Distribute)
    }
}

#[derive(Debug)]
pub enum State {
    SetWeight,
    Snapshot,
    Vote,
    SetupRouter,
    Upload,
    Distribute,
    Close,
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     pub fn size() {

//     }
// }
