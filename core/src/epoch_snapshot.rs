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
    discriminators::Discriminators,
    error::TipRouterError,
    fees::{FeeConfig, NcnFeeGroup},
    weight_table::WeightTable,
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

    fees: FeeConfig,

    operator_count: PodU64,
    vault_count: PodU64,
    operators_registered: PodU64,
    valid_operator_vault_delegations: PodU64,

    /// Counted as each delegate gets added
    ///TODO What happens if `finalized() && total_votes() == 0`?
    stake_weight: PodU128,
    reward_stake_weight: PodU128,

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
        ncn_fees: FeeConfig,
        operator_count: u64,
        vault_count: u64,
    ) -> Self {
        Self {
            ncn,
            ncn_epoch: PodU64::from(ncn_epoch),
            slot_created: PodU64::from(current_slot),
            slot_finalized: PodU64::from(0),
            bump,
            fees: ncn_fees,
            operator_count: PodU64::from(operator_count),
            vault_count: PodU64::from(vault_count),
            operators_registered: PodU64::from(0),
            valid_operator_vault_delegations: PodU64::from(0),
            stake_weight: PodU128::from(0),
            reward_stake_weight: PodU128::from(0),
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

    pub fn vault_count(&self) -> u64 {
        self.vault_count.into()
    }

    pub fn operators_registered(&self) -> u64 {
        self.operators_registered.into()
    }

    pub fn valid_operator_vault_delegations(&self) -> u64 {
        self.valid_operator_vault_delegations.into()
    }

    pub fn stake_weight(&self) -> u128 {
        self.stake_weight.into()
    }

    pub fn reward_stake_weight(&self) -> u128 {
        self.reward_stake_weight.into()
    }

    pub const fn fees(&self) -> &FeeConfig {
        &self.fees
    }

    pub fn finalized(&self) -> bool {
        self.operators_registered() == self.operator_count()
    }

    pub fn increment_operator_registration(
        &mut self,
        current_slot: u64,
        vault_operator_delegations: u64,
        stake_weight: u128,
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

        self.stake_weight = PodU128::from(
            self.stake_weight()
                .checked_add(stake_weight)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );

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

    stake_weight: PodU128,
    reward_stake_weight: PodU128,
    reserved: [u8; 256],

    //TODO change to 64
    vault_operator_stake_weight: [VaultOperatorStakeWeight; 32],
}

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, ShankType)]
#[repr(C)]
pub struct VaultOperatorStakeWeight {
    vault: Pubkey,
    stake_weight: PodU128,
    reward_stake_weight: PodU128,
    vault_index: PodU64,
    reserved: [u8; 32],
}

impl Default for VaultOperatorStakeWeight {
    fn default() -> Self {
        Self {
            vault: Pubkey::default(),
            vault_index: PodU64::from(u64::MAX),
            stake_weight: PodU128::from(0),
            reward_stake_weight: PodU128::from(0),
            reserved: [0; 32],
        }
    }
}

impl VaultOperatorStakeWeight {
    pub fn new(
        vault: Pubkey,
        stake_weight: u128,
        reward_stake_weight: u128,
        vault_index: u64,
    ) -> Self {
        Self {
            vault,
            vault_index: PodU64::from(vault_index),
            stake_weight: PodU128::from(stake_weight),
            reward_stake_weight: PodU128::from(reward_stake_weight),
            reserved: [0; 32],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.vault_index() == u64::MAX
    }

    pub fn vault_index(&self) -> u64 {
        self.vault_index.into()
    }

    pub fn stake_weight(&self) -> u128 {
        self.stake_weight.into()
    }
}

impl Discriminator for OperatorSnapshot {
    const DISCRIMINATOR: u8 = Discriminators::OperatorSnapshot as u8;
}

impl OperatorSnapshot {
    pub const MAX_VAULT_OPERATOR_STAKE_WEIGHT: usize = 64;

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
        if vault_operator_delegation_count > Self::MAX_VAULT_OPERATOR_STAKE_WEIGHT as u64 {
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
            stake_weight: PodU128::from(0),
            reward_stake_weight: PodU128::from(0),
            reserved: [0; 256],
            vault_operator_stake_weight: [VaultOperatorStakeWeight::default(); 32],
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

    pub fn vault_operator_delegation_count(&self) -> u64 {
        self.vault_operator_delegation_count.into()
    }

    pub fn vault_operator_delegations_registered(&self) -> u64 {
        self.vault_operator_delegations_registered.into()
    }

    pub fn valid_operator_vault_delegations(&self) -> u64 {
        self.valid_operator_vault_delegations.into()
    }

    pub fn stake_weight(&self) -> u128 {
        self.stake_weight.into()
    }

    pub fn reward_stake_weight(&self) -> u128 {
        self.reward_stake_weight.into()
    }

    pub fn finalized(&self) -> bool {
        self.vault_operator_delegations_registered() == self.vault_operator_delegation_count()
    }

    pub fn contains_vault_index(&self, vault_index: u64) -> bool {
        self.vault_operator_stake_weight
            .iter()
            .any(|v| v.vault_index() == vault_index)
    }

    pub fn insert_vault_operator_stake_weight(
        &mut self,
        vault: Pubkey,
        vault_index: u64,
        stake_weight: u128,
        reward_stake_weight: u128,
    ) -> Result<(), TipRouterError> {
        if self.vault_operator_delegations_registered()
            > Self::MAX_VAULT_OPERATOR_STAKE_WEIGHT as u64
        {
            return Err(TipRouterError::TooManyVaultOperatorDelegations);
        }

        if self.contains_vault_index(vault_index) {
            return Err(TipRouterError::DuplicateVaultOperatorDelegation);
        }

        self.vault_operator_stake_weight[self.vault_operator_delegations_registered() as usize] =
            VaultOperatorStakeWeight::new(vault, stake_weight, reward_stake_weight, vault_index);

        Ok(())
    }

    pub fn increment_vault_operator_delegation_registration(
        &mut self,
        current_slot: u64,
        vault: Pubkey,
        vault_index: u64,
        stake_weight: u128,
        reward_stake_weight: u128,
    ) -> Result<(), TipRouterError> {
        if self.finalized() {
            return Err(TipRouterError::VaultOperatorDelegationFinalized);
        }

        self.insert_vault_operator_stake_weight(
            vault,
            vault_index,
            stake_weight,
            reward_stake_weight,
        )?;

        self.vault_operator_delegations_registered = PodU64::from(
            self.vault_operator_delegations_registered()
                .checked_add(1)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );

        if stake_weight > 0 {
            self.valid_operator_vault_delegations = PodU64::from(
                self.valid_operator_vault_delegations()
                    .checked_add(1)
                    .ok_or(TipRouterError::ArithmeticOverflow)?,
            );
        }

        self.stake_weight = PodU128::from(
            self.stake_weight()
                .checked_add(stake_weight)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );

        self.reward_stake_weight = PodU128::from(
            self.reward_stake_weight()
                .checked_add(reward_stake_weight)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );

        if self.finalized() {
            self.slot_finalized = PodU64::from(current_slot);
        }

        Ok(())
    }

    pub fn calculate_stake_weight(
        vault_operator_delegation: &VaultOperatorDelegation,
        weight_table: &WeightTable,
        st_mint: &Pubkey,
    ) -> Result<u128, ProgramError> {
        let total_security = vault_operator_delegation
            .delegation_state
            .total_security()?;

        let precise_total_security = PreciseNumber::new(total_security as u128)
            .ok_or(TipRouterError::NewPreciseNumberError)?;

        let precise_weight: PreciseNumber = weight_table.get_precise_weight(st_mint)?;

        let precise_stake_weight = precise_total_security
            .checked_mul(&precise_weight)
            .ok_or(TipRouterError::ArithmeticOverflow)?;

        let stake_weight = precise_stake_weight
            .to_imprecise()
            .ok_or(TipRouterError::CastToImpreciseNumberError)?;

        Ok(stake_weight)
    }

    pub fn calculate_reward_stake_weight(
        stake_weight: u128,
        ncn_fee_group: NcnFeeGroup,
        fee_config: &FeeConfig,
        current_epoch: u64,
    ) -> Result<u128, ProgramError> {
        let precise_stake_weight =
            PreciseNumber::new(stake_weight).ok_or(TipRouterError::NewPreciseNumberError)?;

        let precise_ncn_fee =
            fee_config.adjusted_precise_ncn_fee_bps(ncn_fee_group, current_epoch)?;

        let precise_reward_stake_weight = precise_stake_weight
            .checked_mul(&precise_ncn_fee)
            .ok_or(TipRouterError::ArithmeticOverflow)?;

        let reward_stake_weight: u128 = precise_reward_stake_weight
            .to_imprecise()
            .ok_or(TipRouterError::CastToImpreciseNumberError)?;

        Ok(reward_stake_weight)
    }
}
