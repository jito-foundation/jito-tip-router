use bytemuck::{Pod, Zeroable};
use jito_bytemuck::{
    types::{PodBool, PodU128, PodU16, PodU64},
    AccountDeserialize, Discriminator,
};
use jito_vault_core::vault_operator_delegation::VaultOperatorDelegation;
use shank::{ShankAccount, ShankType};
use solana_program::{account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey};
use spl_math::precise_number::PreciseNumber;

use crate::{
    discriminators::Discriminators, error::TipRouterError, fees::Fees, weight_table::WeightTable,
};

// PDA'd ["EPOCH_SNAPSHOT", NCN, NCN_EPOCH_SLOT]
#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, AccountDeserialize, ShankAccount)]
#[repr(C)]
pub struct EpochSnapshot {
    /// The NCN on-chain program is the signer to create and update this account,
    /// this pushes the responsibility of managing the account to the NCN program.
    ncn: Pubkey,

    /// The NCN epoch for which the Epoch snapshot is valid
    ncn_epoch: PodU64,

    /// Bump seed for the PDA
    bump: u8,

    /// Slot Epoch snapshot was created
    slot_created: PodU64,
    slot_finalized: PodU64,

    ncn_fees: Fees,

    operator_count: PodU64,
    operators_registered: PodU64,
    valid_operator_vault_delegations: PodU64,

    /// Counted as each delegate gets added
    ///TODO What happens if `finalized() && total_votes() == 0`?
    total_votes: PodU128,

    /// Reserved space
    reserved: [u8; 128],
}

impl Discriminator for EpochSnapshot {
    const DISCRIMINATOR: u8 = Discriminators::EpochSnapshot as u8;
}

impl EpochSnapshot {
    pub fn new(
        ncn: Pubkey,
        ncn_epoch: u64,
        bump: u8,
        current_slot: u64,
        ncn_fees: Fees,
        num_operators: u64,
    ) -> Self {
        Self {
            ncn,
            ncn_epoch: PodU64::from(ncn_epoch),
            slot_created: PodU64::from(current_slot),
            slot_finalized: PodU64::from(0),
            bump,
            ncn_fees,
            operator_count: PodU64::from(num_operators),
            operators_registered: PodU64::from(0),
            valid_operator_vault_delegations: PodU64::from(0),
            total_votes: PodU128::from(0),
            reserved: [0; 128],
        }
    }

    pub fn seeds(ncn: &Pubkey, ncn_epoch: u64) -> Vec<Vec<u8>> {
        Vec::from_iter(
            [
                b"EPOCH_SNAPSHOT".to_vec(),
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
        epoch_snapshot: &AccountInfo,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        if epoch_snapshot.owner.ne(program_id) {
            msg!("Epoch Snapshot account has an invalid owner");
            return Err(ProgramError::InvalidAccountOwner);
        }
        if epoch_snapshot.data_is_empty() {
            msg!("Epoch Snapshot account data is empty");
            return Err(ProgramError::InvalidAccountData);
        }
        if expect_writable && !epoch_snapshot.is_writable {
            msg!("Epoch Snapshot account is not writable");
            return Err(ProgramError::InvalidAccountData);
        }
        if epoch_snapshot.data.borrow()[0].ne(&Self::DISCRIMINATOR) {
            msg!("Epoch Snapshot account discriminator is invalid");
            return Err(ProgramError::InvalidAccountData);
        }
        if epoch_snapshot
            .key
            .ne(&Self::find_program_address(program_id, ncn, ncn_epoch).0)
        {
            msg!("Epoch Snapshot account is not at the correct PDA");
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }

    pub fn operator_count(&self) -> u64 {
        self.operator_count.into()
    }

    pub fn operators_registered(&self) -> u64 {
        self.operators_registered.into()
    }

    pub fn valid_operator_vault_delegations(&self) -> u64 {
        self.valid_operator_vault_delegations.into()
    }

    pub fn total_votes(&self) -> u128 {
        self.total_votes.into()
    }

    pub fn finalized(&self) -> bool {
        self.operators_registered() == self.operator_count()
    }

    pub fn increment_operator_registration(
        &mut self,
        current_slot: u64,
        vault_operator_delegations: u64,
        votes: u128,
    ) -> Result<(), TipRouterError> {
        if self.finalized() {
            return Err(TipRouterError::OperatorFinalized);
        }

        self.operators_registered = PodU64::from(
            self.operators_registered()
                .checked_add(1)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );

        self.valid_operator_vault_delegations = PodU64::from(
            self.valid_operator_vault_delegations()
                .checked_add(vault_operator_delegations)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );

        self.total_votes = PodU128::from(
            self.total_votes()
                .checked_add(votes)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );

        if self.finalized() {
            self.slot_finalized = PodU64::from(current_slot);
        }

        Ok(())
    }
}

// PDA'd ["OPERATOR_SNAPSHOT", OPERATOR, NCN, NCN_EPOCH_SLOT]
#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, AccountDeserialize, ShankAccount)]
#[repr(C)]
pub struct OperatorSnapshot {
    operator: Pubkey,
    ncn: Pubkey,
    ncn_epoch: PodU64,
    bump: u8,

    slot_created: PodU64,
    slot_finalized: PodU64,

    is_active: PodBool,

    operator_index: PodU64,
    operator_fee_bps: PodU16,

    vault_operator_delegation_count: PodU64,
    vault_operator_delegations_registered: PodU64,
    valid_operator_vault_delegations: PodU64,

    total_votes: PodU128,
    reserved: [u8; 256],

    // needs to be last item in struct such that it can grow later
    vault_operator_votes: [VaultOperatorVotes; 64],
}

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, ShankType)]
#[repr(C)]
pub struct VaultOperatorVotes {
    vault: Pubkey,
    votes: PodU128,
    vault_index: PodU64,
    reserved: [u8; 32],
}

impl Default for VaultOperatorVotes {
    fn default() -> Self {
        Self {
            vault: Pubkey::default(),
            vault_index: PodU64::from(u64::MAX),
            votes: PodU128::from(0),
            reserved: [0; 32],
        }
    }
}

impl VaultOperatorVotes {
    pub fn new(vault: Pubkey, votes: u128, vault_index: u64) -> Self {
        Self {
            vault,
            vault_index: PodU64::from(vault_index),
            votes: PodU128::from(votes),
            reserved: [0; 32],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.vault_index() == u64::MAX
    }

    pub fn vault_index(&self) -> u64 {
        self.vault_index.into()
    }

    pub fn votes(&self) -> u128 {
        self.votes.into()
    }
}

impl Discriminator for OperatorSnapshot {
    const DISCRIMINATOR: u8 = Discriminators::OperatorSnapshot as u8;
}

impl OperatorSnapshot {
    pub const MAX_VAULT_OPERATOR_VOTES: usize = 64;

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        operator: Pubkey,
        ncn: Pubkey,
        ncn_epoch: u64,
        bump: u8,
        current_slot: u64,
        is_active: bool,
        operator_index: u64,
        operator_fee_bps: u16,
        vault_operator_delegation_count: u64,
    ) -> Result<Self, TipRouterError> {
        if vault_operator_delegation_count > Self::MAX_VAULT_OPERATOR_VOTES as u64 {
            return Err(TipRouterError::TooManyVaultOperatorDelegations);
        }

        Ok(Self {
            operator,
            ncn,
            ncn_epoch: PodU64::from(ncn_epoch),
            bump,
            slot_created: PodU64::from(current_slot),
            slot_finalized: PodU64::from(0),
            is_active: PodBool::from(is_active),
            operator_index: PodU64::from(operator_index),
            operator_fee_bps: PodU16::from(operator_fee_bps),
            vault_operator_delegation_count: PodU64::from(vault_operator_delegation_count),
            vault_operator_delegations_registered: PodU64::from(0),
            valid_operator_vault_delegations: PodU64::from(0),
            total_votes: PodU128::from(0),
            reserved: [0; 256],
            vault_operator_votes: [VaultOperatorVotes::default(); 64],
        })
    }

    pub fn new_active(
        operator: Pubkey,
        ncn: Pubkey,
        ncn_epoch: u64,
        bump: u8,
        current_slot: u64,
        operator_index: u64,
        operator_fee_bps: u16,
        vault_count: u64,
    ) -> Result<Self, TipRouterError> {
        Self::new(
            operator,
            ncn,
            ncn_epoch,
            bump,
            current_slot,
            true,
            operator_index,
            operator_fee_bps,
            vault_count,
        )
    }

    pub fn new_inactive(
        operator: Pubkey,
        ncn: Pubkey,
        ncn_epoch: u64,
        bump: u8,
        current_slot: u64,
        operator_index: u64,
    ) -> Result<Self, TipRouterError> {
        let mut snapshot = Self::new(
            operator,
            ncn,
            ncn_epoch,
            bump,
            current_slot,
            false,
            operator_index,
            0,
            0,
        )?;

        snapshot.slot_finalized = PodU64::from(current_slot);
        Ok(snapshot)
    }

    pub fn seeds(operator: &Pubkey, ncn: &Pubkey, ncn_epoch: u64) -> Vec<Vec<u8>> {
        Vec::from_iter(
            [
                b"OPERATOR_SNAPSHOT".to_vec(),
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
        operator_snapshot: &AccountInfo,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        if operator_snapshot.owner.ne(program_id) {
            msg!("Operator Snapshot account has an invalid owner");
            return Err(ProgramError::InvalidAccountOwner);
        }
        if operator_snapshot.data_is_empty() {
            msg!("Operator Snapshot account data is empty");
            return Err(ProgramError::InvalidAccountData);
        }
        if expect_writable && !operator_snapshot.is_writable {
            msg!("Operator Snapshot account is not writable");
            return Err(ProgramError::InvalidAccountData);
        }
        if operator_snapshot.data.borrow()[0].ne(&Self::DISCRIMINATOR) {
            msg!("Operator Snapshot account discriminator is invalid");
            return Err(ProgramError::InvalidAccountData);
        }
        if operator_snapshot
            .key
            .ne(&Self::find_program_address(program_id, operator, ncn, ncn_epoch).0)
        {
            msg!("Operator Snapshot account is not at the correct PDA");
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }

    pub fn vault_operator_delegation_count(&self) -> u64 {
        self.vault_operator_delegation_count.into()
    }

    pub fn vault_operator_delegations_registered(&self) -> u64 {
        self.vault_operator_delegations_registered.into()
    }

    pub fn valid_operator_vault_delegations(&self) -> u64 {
        self.valid_operator_vault_delegations.into()
    }

    pub fn total_votes(&self) -> u128 {
        self.total_votes.into()
    }

    pub fn finalized(&self) -> bool {
        self.vault_operator_delegations_registered() == self.vault_operator_delegation_count()
    }

    pub fn insert_vault_operator_votes(
        &mut self,
        vault: Pubkey,
        vault_index: u64,
        votes: u128,
    ) -> Result<(), TipRouterError> {
        // Check for duplicate vaults
        for vault_operator_vote in self.vault_operator_votes.iter_mut() {
            if vault_operator_vote.vault_index() == vault_index {
                return Err(TipRouterError::DuplicateVaultOperatorDelegation);
            }
        }

        if self.vault_operator_delegations_registered() > Self::MAX_VAULT_OPERATOR_VOTES as u64 {
            return Err(TipRouterError::TooManyVaultOperatorDelegations);
        }

        self.vault_operator_votes[self.vault_operator_delegations_registered() as usize] =
            VaultOperatorVotes::new(vault, votes, vault_index);

        Ok(())
    }

    pub fn increment_vault_operator_delegation_registration(
        &mut self,
        current_slot: u64,
        vault: Pubkey,
        vault_index: u64,
        votes: u128,
    ) -> Result<(), TipRouterError> {
        if self.finalized() {
            return Err(TipRouterError::VaultOperatorDelegationFinalized);
        }

        self.insert_vault_operator_votes(vault, vault_index, votes)?;

        self.vault_operator_delegations_registered = PodU64::from(
            self.vault_operator_delegations_registered()
                .checked_add(1)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );

        if votes > 0 {
            self.valid_operator_vault_delegations = PodU64::from(
                self.valid_operator_vault_delegations()
                    .checked_add(1)
                    .ok_or(TipRouterError::ArithmeticOverflow)?,
            );
        }

        self.total_votes = PodU128::from(
            self.total_votes()
                .checked_add(votes)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );

        if self.finalized() {
            self.slot_finalized = PodU64::from(current_slot);
        }

        Ok(())
    }
}

// PDA'd ["OPERATOR_SNAPSHOT", VAULT, OPERATOR, NCN, NCN_EPOCH_SLOT]
#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, AccountDeserialize, ShankAccount)]
#[repr(C)]
pub struct VaultOperatorDelegationSnapshot {
    vault: Pubkey,
    operator: Pubkey,
    ncn: Pubkey,
    ncn_epoch: PodU64,
    bump: u8,

    slot_created: PodU64,

    is_active: PodBool,

    vault_index: PodU64,

    st_mint: Pubkey,
    total_security: PodU64,
    total_votes: PodU128,

    reserved: [u8; 128],
}

impl Discriminator for VaultOperatorDelegationSnapshot {
    const DISCRIMINATOR: u8 = Discriminators::VaultOperatorDelegationSnapshot as u8;
}

impl VaultOperatorDelegationSnapshot {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        vault: Pubkey,
        operator: Pubkey,
        ncn: Pubkey,
        ncn_epoch: u64,
        bump: u8,
        current_slot: u64,
        is_active: bool,
        vault_index: u64,
        st_mint: Pubkey,
        total_security: u64,
        total_votes: u128,
    ) -> Self {
        Self {
            vault,
            operator,
            ncn,
            ncn_epoch: PodU64::from(ncn_epoch),
            bump,
            slot_created: PodU64::from(current_slot),
            is_active: PodBool::from(is_active),
            vault_index: PodU64::from(vault_index),
            st_mint,
            total_security: PodU64::from(total_security),
            total_votes: PodU128::from(total_votes),
            reserved: [0; 128],
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn new_active(
        vault: Pubkey,
        operator: Pubkey,
        ncn: Pubkey,
        ncn_epoch: u64,
        bump: u8,
        current_slot: u64,
        st_mint: Pubkey,
        vault_index: u64,
        total_security: u64,
        total_votes: u128,
    ) -> Self {
        Self::new(
            vault,
            operator,
            ncn,
            ncn_epoch,
            bump,
            current_slot,
            true,
            vault_index,
            st_mint,
            total_security,
            total_votes,
        )
    }

    pub fn new_inactive(
        vault: Pubkey,
        operator: Pubkey,
        ncn: Pubkey,
        ncn_epoch: u64,
        bump: u8,
        current_slot: u64,
        vault_index: u64,
        st_mint: Pubkey,
    ) -> Self {
        Self::new(
            vault,
            operator,
            ncn,
            ncn_epoch,
            bump,
            current_slot,
            false,
            vault_index,
            st_mint,
            0,
            0,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_snapshot(
        vault: Pubkey,
        operator: Pubkey,
        ncn: Pubkey,
        ncn_epoch: u64,
        bump: u8,
        current_slot: u64,
        vault_index: u64,
        st_mint: Pubkey,
        vault_operator_delegation: &VaultOperatorDelegation,
        weight_table: &WeightTable,
    ) -> Result<Self, ProgramError> {
        let total_security = vault_operator_delegation
            .delegation_state
            .total_security()?;
        let precise_total_security = PreciseNumber::new(total_security as u128)
            .ok_or(TipRouterError::NewPreciseNumberError)?;

        let precise_weight = weight_table.get_precise_weight(&st_mint)?;

        let precise_total_votes = precise_total_security
            .checked_mul(&precise_weight)
            .ok_or(TipRouterError::ArithmeticOverflow)?;

        let total_votes = precise_total_votes
            .to_imprecise()
            .ok_or(TipRouterError::CastToImpreciseNumberError)?;

        Ok(Self::new_active(
            vault,
            operator,
            ncn,
            ncn_epoch,
            bump,
            current_slot,
            st_mint,
            vault_index,
            total_security,
            total_votes,
        ))
    }

    pub fn seeds(vault: &Pubkey, operator: &Pubkey, ncn: &Pubkey, ncn_epoch: u64) -> Vec<Vec<u8>> {
        Vec::from_iter(
            [
                b"VAULT_OPERATOR_DELEGATION_SNAPSHOT".to_vec(),
                vault.to_bytes().to_vec(),
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
        vault: &Pubkey,
        operator: &Pubkey,
        ncn: &Pubkey,
        ncn_epoch: u64,
    ) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(vault, operator, ncn, ncn_epoch);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
    }

    pub fn load(
        program_id: &Pubkey,
        vault: &Pubkey,
        operator: &Pubkey,
        ncn: &Pubkey,
        ncn_epoch: u64,
        vault_operator_delegation_snapshot: &AccountInfo,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        if vault_operator_delegation_snapshot.owner.ne(program_id) {
            msg!("Operator Snapshot account has an invalid owner");
            return Err(ProgramError::InvalidAccountOwner);
        }
        if vault_operator_delegation_snapshot.data_is_empty() {
            msg!("Operator Snapshot account data is empty");
            return Err(ProgramError::InvalidAccountData);
        }
        if expect_writable && !vault_operator_delegation_snapshot.is_writable {
            msg!("Operator Snapshot account is not writable");
            return Err(ProgramError::InvalidAccountData);
        }
        if vault_operator_delegation_snapshot.data.borrow()[0].ne(&Self::DISCRIMINATOR) {
            msg!("Operator Snapshot account discriminator is invalid");
            return Err(ProgramError::InvalidAccountData);
        }
        if vault_operator_delegation_snapshot
            .key
            .ne(&Self::find_program_address(program_id, vault, operator, ncn, ncn_epoch).0)
        {
            msg!("Operator Snapshot account is not at the correct PDA");
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }

    pub fn total_security(&self) -> u64 {
        self.total_security.into()
    }

    pub fn total_votes(&self) -> u128 {
        self.total_votes.into()
    }

    pub fn vault_index(&self) -> u64 {
        self.vault_index.into()
    }

    pub fn vault(&self) -> Pubkey {
        self.vault
    }
}
