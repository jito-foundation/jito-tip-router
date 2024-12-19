use std::mem::size_of;

use bytemuck::{Pod, Zeroable};
use jito_bytemuck::{
    types::{PodBool, PodU16, PodU64},
    AccountDeserialize, Discriminator,
};
use jito_vault_core::vault_operator_delegation::VaultOperatorDelegation;
use shank::{ShankAccount, ShankType};
use solana_program::{account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey};
use spl_math::precise_number::PreciseNumber;

use crate::{
    constants::MAX_VAULTS, discriminators::Discriminators, error::TipRouterError, fees::Fees,
    ncn_fee_group::NcnFeeGroup, stake_weight::StakeWeights, weight_table::WeightTable,
};

// PDA'd ["epoch_snapshot", NCN, NCN_EPOCH_SLOT]
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

    fees: Fees,

    operator_count: PodU64,
    vault_count: PodU64,
    operators_registered: PodU64,
    valid_operator_vault_delegations: PodU64,

    stake_weights: StakeWeights,

    /// Reserved space
    reserved: [u8; 128],
}

impl Discriminator for EpochSnapshot {
    const DISCRIMINATOR: u8 = Discriminators::EpochSnapshot as u8;
}

impl EpochSnapshot {
    pub const SIZE: usize = 8 + size_of::<Self>();

    pub fn new(
        ncn: Pubkey,
        ncn_epoch: u64,
        bump: u8,
        current_slot: u64,
        fees: Fees,
        operator_count: u64,
        vault_count: u64,
    ) -> Self {
        Self {
            ncn,
            ncn_epoch: PodU64::from(ncn_epoch),
            slot_created: PodU64::from(current_slot),
            slot_finalized: PodU64::from(0),
            bump,
            fees,
            operator_count: PodU64::from(operator_count),
            vault_count: PodU64::from(vault_count),
            operators_registered: PodU64::from(0),
            valid_operator_vault_delegations: PodU64::from(0),
            stake_weights: StakeWeights::default(),
            reserved: [0; 128],
        }
    }

    pub fn seeds(ncn: &Pubkey, ncn_epoch: u64) -> Vec<Vec<u8>> {
        Vec::from_iter(
            [
                b"epoch_snapshot".to_vec(),
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
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }

    pub fn operator_count(&self) -> u64 {
        self.operator_count.into()
    }

    pub fn vault_count(&self) -> u64 {
        self.vault_count.into()
    }

    pub fn operators_registered(&self) -> u64 {
        self.operators_registered.into()
    }

    pub fn valid_operator_vault_delegations(&self) -> u64 {
        self.valid_operator_vault_delegations.into()
    }

    pub const fn stake_weights(&self) -> &StakeWeights {
        &self.stake_weights
    }

    pub const fn fees(&self) -> &Fees {
        &self.fees
    }

    pub fn finalized(&self) -> bool {
        self.operators_registered() == self.operator_count()
    }

    pub fn increment_operator_registration(
        &mut self,
        current_slot: u64,
        vault_operator_delegations: u64,
        stake_weight: &StakeWeights,
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

        self.stake_weights.increment(stake_weight)?;

        if self.finalized() {
            self.slot_finalized = PodU64::from(current_slot);
        }

        Ok(())
    }
}

// PDA'd ["operator_snapshot", OPERATOR, NCN, NCN_EPOCH_SLOT]
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

    ncn_operator_index: PodU64,
    operator_index: PodU64,
    operator_fee_bps: PodU16,

    vault_operator_delegation_count: PodU64,
    vault_operator_delegations_registered: PodU64,
    valid_operator_vault_delegations: PodU64,

    stake_weights: StakeWeights,
    reserved: [u8; 256],

    vault_operator_stake_weight: [VaultOperatorStakeWeight; 64],
}

impl Discriminator for OperatorSnapshot {
    const DISCRIMINATOR: u8 = Discriminators::OperatorSnapshot as u8;
}

impl OperatorSnapshot {
    pub const SIZE: usize = 8 + size_of::<Self>();

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        operator: Pubkey,
        ncn: Pubkey,
        ncn_epoch: u64,
        bump: u8,
        current_slot: u64,
        is_active: bool,
        ncn_operator_index: u64,
        operator_index: u64,
        operator_fee_bps: u16,
        vault_operator_delegation_count: u64,
    ) -> Result<Self, TipRouterError> {
        if vault_operator_delegation_count > MAX_VAULTS as u64 {
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
            ncn_operator_index: PodU64::from(ncn_operator_index),
            operator_index: PodU64::from(operator_index),
            operator_fee_bps: PodU16::from(operator_fee_bps),
            vault_operator_delegation_count: PodU64::from(vault_operator_delegation_count),
            vault_operator_delegations_registered: PodU64::from(0),
            valid_operator_vault_delegations: PodU64::from(0),
            stake_weights: StakeWeights::default(),
            reserved: [0; 256],
            vault_operator_stake_weight: [VaultOperatorStakeWeight::default(); MAX_VAULTS],
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_active(
        operator: Pubkey,
        ncn: Pubkey,
        ncn_epoch: u64,
        bump: u8,
        current_slot: u64,
        ncn_operator_index: u64,
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
            ncn_operator_index,
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
        ncn_operator_index: u64,
        operator_index: u64,
    ) -> Result<Self, TipRouterError> {
        let mut snapshot = Self::new(
            operator,
            ncn,
            ncn_epoch,
            bump,
            current_slot,
            false,
            ncn_operator_index,
            operator_index,
            0,
            0,
        )?;

        snapshot.slot_finalized = PodU64::from(current_slot);
        Ok(snapshot)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn initialize(
        &mut self,
        operator: Pubkey,
        ncn: Pubkey,
        ncn_epoch: u64,
        bump: u8,
        current_slot: u64,
        is_active: bool,
        ncn_operator_index: u64,
        operator_index: u64,
        operator_fee_bps: u16,
        vault_operator_delegation_count: u64,
    ) -> Result<(), TipRouterError> {
        if vault_operator_delegation_count > MAX_VAULTS as u64 {
            return Err(TipRouterError::TooManyVaultOperatorDelegations);
        }
        let slot_finalized = if !is_active { current_slot } else { 0 };
        let operator_fee_bps_val = if is_active { operator_fee_bps } else { 0 };
        let vault_operator_delegation_count_val = if is_active {
            vault_operator_delegation_count
        } else {
            0
        };

        // Initializes field by field to avoid overflowing stack
        self.operator = operator;
        self.ncn = ncn;
        self.ncn_epoch = PodU64::from(ncn_epoch);
        self.bump = bump;
        self.slot_created = PodU64::from(current_slot);
        self.slot_finalized = PodU64::from(slot_finalized);
        self.is_active = PodBool::from(is_active);
        self.ncn_operator_index = PodU64::from(ncn_operator_index);
        self.operator_index = PodU64::from(operator_index);
        self.operator_fee_bps = PodU16::from(operator_fee_bps_val);
        self.vault_operator_delegation_count = PodU64::from(vault_operator_delegation_count_val);
        self.vault_operator_delegations_registered = PodU64::from(0);
        self.valid_operator_vault_delegations = PodU64::from(0);
        self.stake_weights = StakeWeights::default();
        self.reserved = [0; 256];
        self.vault_operator_stake_weight = [VaultOperatorStakeWeight::default(); MAX_VAULTS];

        Ok(())
    }

    pub fn seeds(operator: &Pubkey, ncn: &Pubkey, ncn_epoch: u64) -> Vec<Vec<u8>> {
        Vec::from_iter(
            [
                b"operator_snapshot".to_vec(),
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

    pub const fn operator(&self) -> Pubkey {
        self.operator
    }

    pub const fn ncn(&self) -> Pubkey {
        self.ncn
    }

    pub fn operator_fee_bps(&self) -> u16 {
        self.operator_fee_bps.into()
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

    pub const fn stake_weights(&self) -> &StakeWeights {
        &self.stake_weights
    }

    pub fn finalized(&self) -> bool {
        self.vault_operator_delegations_registered() == self.vault_operator_delegation_count()
    }

    pub fn contains_vault_index(&self, vault_index: u64) -> bool {
        self.vault_operator_stake_weight
            .iter()
            .any(|v| v.vault_index() == vault_index)
    }

    pub const fn vault_operator_stake_weight(&self) -> &[VaultOperatorStakeWeight] {
        &self.vault_operator_stake_weight
    }

    pub fn insert_vault_operator_stake_weight(
        &mut self,
        vault: Pubkey,
        vault_index: u64,
        ncn_fee_group: NcnFeeGroup,
        stake_weights: &StakeWeights,
    ) -> Result<(), TipRouterError> {
        if self.vault_operator_delegations_registered() > MAX_VAULTS as u64 {
            return Err(TipRouterError::TooManyVaultOperatorDelegations);
        }

        if self.contains_vault_index(vault_index) {
            return Err(TipRouterError::DuplicateVaultOperatorDelegation);
        }

        self.vault_operator_stake_weight[self.vault_operator_delegations_registered() as usize] =
            VaultOperatorStakeWeight::new(vault, vault_index, ncn_fee_group, stake_weights);

        Ok(())
    }

    pub fn increment_vault_operator_delegation_registration(
        &mut self,
        current_slot: u64,
        vault: Pubkey,
        vault_index: u64,
        ncn_fee_group: NcnFeeGroup,
        stake_weights: &StakeWeights,
    ) -> Result<(), TipRouterError> {
        if self.finalized() {
            return Err(TipRouterError::VaultOperatorDelegationFinalized);
        }

        self.insert_vault_operator_stake_weight(vault, vault_index, ncn_fee_group, stake_weights)?;

        self.vault_operator_delegations_registered = PodU64::from(
            self.vault_operator_delegations_registered()
                .checked_add(1)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );

        if stake_weights.stake_weight() > 0 {
            self.valid_operator_vault_delegations = PodU64::from(
                self.valid_operator_vault_delegations()
                    .checked_add(1)
                    .ok_or(TipRouterError::ArithmeticOverflow)?,
            );
        }

        self.stake_weights.increment(stake_weights)?;

        if self.finalized() {
            self.slot_finalized = PodU64::from(current_slot);
        }

        Ok(())
    }

    pub fn calculate_total_stake_weight(
        vault_operator_delegation: &VaultOperatorDelegation,
        weight_table: &WeightTable,
        st_mint: &Pubkey,
    ) -> Result<u128, ProgramError> {
        let total_security = vault_operator_delegation
            .delegation_state
            .total_security()?;

        let precise_total_security = PreciseNumber::new(total_security as u128)
            .ok_or(TipRouterError::NewPreciseNumberError)?;

        let precise_weight = weight_table.get_precise_weight(st_mint)?;

        let precise_total_stake_weight = precise_total_security
            .checked_mul(&precise_weight)
            .ok_or(TipRouterError::ArithmeticOverflow)?;

        let total_stake_weight = precise_total_stake_weight
            .to_imprecise()
            .ok_or(TipRouterError::CastToImpreciseNumberError)?;

        Ok(total_stake_weight)
    }
}

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, ShankType)]
#[repr(C)]
pub struct VaultOperatorStakeWeight {
    vault: Pubkey,
    vault_index: PodU64,
    ncn_fee_group: NcnFeeGroup,
    stake_weight: StakeWeights,
    reserved: [u8; 32],
}

impl Default for VaultOperatorStakeWeight {
    fn default() -> Self {
        Self {
            vault: Pubkey::default(),
            ncn_fee_group: NcnFeeGroup::default(),
            vault_index: PodU64::from(u64::MAX),
            stake_weight: StakeWeights::default(),
            reserved: [0; 32],
        }
    }
}

impl VaultOperatorStakeWeight {
    pub fn new(
        vault: Pubkey,
        vault_index: u64,
        ncn_fee_group: NcnFeeGroup,
        stake_weight: &StakeWeights,
    ) -> Self {
        Self {
            vault,
            vault_index: PodU64::from(vault_index),
            ncn_fee_group,
            stake_weight: *stake_weight,
            reserved: [0; 32],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.vault_index() == u64::MAX
    }

    pub fn vault_index(&self) -> u64 {
        self.vault_index.into()
    }

    pub const fn stake_weights(&self) -> &StakeWeights {
        &self.stake_weight
    }

    pub const fn vault(&self) -> Pubkey {
        self.vault
    }

    pub const fn ncn_fee_group(&self) -> NcnFeeGroup {
        self.ncn_fee_group
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operator_snapshot_size() {
        use std::mem::size_of;

        let expected_total = size_of::<Pubkey>() // operator
            + size_of::<Pubkey>() // ncn
            + size_of::<PodU64>() // ncn_epoch
            + 1 // bump
            + size_of::<PodU64>() // slot_created
            + size_of::<PodU64>() // slot_finalized
            + size_of::<PodBool>() // is_active
            + size_of::<PodU64>() // ncn_operator_index
            + size_of::<PodU64>() // operator_index
            + size_of::<PodU16>() // operator_fee_bps
            + size_of::<PodU64>() // vault_operator_delegation_count
            + size_of::<PodU64>() // vault_operator_delegations_registered
            + size_of::<PodU64>() // valid_operator_vault_delegations
            + size_of::<StakeWeights>() // stake_weight
            + 256 // reserved
            + size_of::<VaultOperatorStakeWeight>() * MAX_VAULTS; // vault_operator_stake_weight

        assert_eq!(size_of::<OperatorSnapshot>(), expected_total);
    }
}
