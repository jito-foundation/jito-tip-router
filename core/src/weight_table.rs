use std::mem::size_of;

use bytemuck::{Pod, Zeroable};
use jito_bytemuck::{types::PodU64, AccountDeserialize, Discriminator};
use shank::{ShankAccount, ShankType};
use solana_program::{account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey};
use spl_math::precise_number::PreciseNumber;

use crate::{
    constants::{MAX_ST_MINTS, MAX_VAULTS},
    discriminators::Discriminators,
    error::TipRouterError,
    vault_registry::{StMintEntry, VaultEntry},
    weight_entry::WeightEntry,
};

// PDA'd ["WEIGHT_TABLE", NCN, NCN_EPOCH_SLOT]
#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, AccountDeserialize, ShankAccount)]
#[repr(C)]
pub struct WeightTable {
    /// The NCN on-chain program is the signer to create and update this account,
    /// this pushes the responsibility of managing the account to the NCN program.
    ncn: Pubkey,

    /// The epoch for which the weight table is valid
    epoch: PodU64,

    /// Slot weight table was created
    slot_created: PodU64,

    /// Number of vaults in tracked mints at the time of creation
    vault_count: PodU64,

    /// Bump seed for the PDA
    bump: u8,

    /// Reserved space
    reserved: [u8; 128],

    /// The Vault Registry
    vault_registry: [VaultEntry; 64],

    /// The weight table
    table: [WeightEntry; 64],
}

impl Discriminator for WeightTable {
    const DISCRIMINATOR: u8 = Discriminators::WeightTable as u8;
}

impl WeightTable {
    pub const SIZE: usize = 8 + size_of::<Self>();

    pub fn new(ncn: Pubkey, epoch: u64, slot_created: u64, vault_count: u64, bump: u8) -> Self {
        Self {
            ncn,
            epoch: PodU64::from(epoch),
            slot_created: PodU64::from(slot_created),
            vault_count: PodU64::from(vault_count),
            bump,
            reserved: [0; 128],
            vault_registry: [VaultEntry::default(); MAX_VAULTS],
            table: [WeightEntry::default(); MAX_ST_MINTS],
        }
    }

    pub fn seeds(ncn: &Pubkey, ncn_epoch: u64) -> Vec<Vec<u8>> {
        Vec::from_iter(
            [
                b"WEIGHT_TABLE".to_vec(),
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

    #[allow(clippy::too_many_arguments)]
    pub fn initialize(
        &mut self,
        ncn: Pubkey,
        ncn_epoch: u64,
        slot_created: u64,
        vault_count: u64,
        bump: u8,
        vault_entries: &[VaultEntry; MAX_VAULTS],
        mint_entries: &[StMintEntry; MAX_ST_MINTS],
    ) -> Result<(), TipRouterError> {
        // Initializes field by field to avoid overflowing stack
        self.ncn = ncn;
        self.epoch = PodU64::from(ncn_epoch);
        self.slot_created = PodU64::from(slot_created);
        self.vault_count = PodU64::from(vault_count);
        self.bump = bump;
        self.reserved = [0; 128];
        self.vault_registry = [VaultEntry::default(); MAX_VAULTS];
        self.table = [WeightEntry::default(); MAX_ST_MINTS];
        self.set_vault_entries(vault_entries)?;
        self.set_mint_entries(mint_entries)?;
        Ok(())
    }

    fn set_vault_entries(
        &mut self,
        vault_entries: &[VaultEntry; MAX_VAULTS],
    ) -> Result<(), TipRouterError> {
        if self.vault_registry_initialized() {
            return Err(TipRouterError::WeightTableAlreadyInitialized);
        }

        // Copy the entire slice into vault_registry
        for (i, entry) in vault_entries.iter().enumerate() {
            self.vault_registry[i] = *entry;
        }

        self.check_registry_initialized()?;

        Ok(())
    }

    fn set_mint_entries(
        &mut self,
        mint_entries: &[StMintEntry; MAX_ST_MINTS],
    ) -> Result<(), TipRouterError> {
        if self.table_initialized() {
            return Err(TipRouterError::WeightTableAlreadyInitialized);
        }

        // Set table using iterator
        for (i, entry) in mint_entries.iter().enumerate() {
            self.table[i] = WeightEntry::new(entry)
        }

        // self.table.iter_mut().zip(mint_entries.iter()).for_each(
        //     |(weight_table_entry, &mint_entry)| {
        //         *weight_table_entry = WeightEntry::new(mint_entry);
        //     },
        // );

        self.check_table_initialized()?;

        Ok(())
    }

    pub fn set_weight(
        &mut self,
        mint: &Pubkey,
        weight: u128,
        current_slot: u64,
    ) -> Result<(), TipRouterError> {
        self.table
            .iter_mut()
            .find(|entry| entry.st_mint().eq(mint))
            .map_or(Err(TipRouterError::InvalidMintForWeightTable), |entry| {
                entry.set_weight(weight, current_slot);
                Ok(())
            })
    }

    pub fn get_weight(&self, mint: &Pubkey) -> Result<u128, TipRouterError> {
        self.table
            .iter()
            .find(|entry| entry.st_mint().eq(mint))
            .map_or(Err(TipRouterError::InvalidMintForWeightTable), |entry| {
                Ok(entry.weight())
            })
    }

    pub fn get_weight_entry(&self, mint: &Pubkey) -> Result<&WeightEntry, TipRouterError> {
        self.table
            .iter()
            .find(|entry| entry.st_mint().eq(mint))
            .ok_or(TipRouterError::InvalidMintForWeightTable)
    }

    pub fn get_precise_weight(&self, mint: &Pubkey) -> Result<PreciseNumber, TipRouterError> {
        let weight = self.get_weight(mint)?;
        PreciseNumber::new(weight).ok_or(TipRouterError::NewPreciseNumberError)
    }

    pub fn get_mints(&self) -> Vec<Pubkey> {
        self.table
            .iter()
            .filter(|entry| !entry.is_empty())
            .map(|entry| entry.st_mint())
            .collect()
    }

    pub fn mint_count(&self) -> usize {
        self.table.iter().filter(|entry| !entry.is_empty()).count()
    }

    pub fn weight_count(&self) -> usize {
        self.table.iter().filter(|entry| entry.is_set()).count()
    }

    pub const fn ncn(&self) -> Pubkey {
        self.ncn
    }

    pub fn ncn_epoch(&self) -> u64 {
        self.epoch.into()
    }

    pub fn slot_created(&self) -> u64 {
        self.slot_created.into()
    }

    pub fn vault_count(&self) -> u64 {
        self.vault_count.into()
    }

    pub fn vault_entry_count(&self) -> usize {
        self.vault_registry
            .iter()
            .filter(|entry| !entry.is_empty())
            .count()
    }

    pub fn vault_registry_initialized(&self) -> bool {
        self.vault_count() == self.vault_entry_count() as u64
    }

    pub fn table_initialized(&self) -> bool {
        self.mint_count() > 0
    }

    pub fn finalized(&self) -> bool {
        self.vault_registry_initialized()
            && self.table_initialized()
            && self.mint_count() == self.weight_count()
    }

    pub fn check_table_initialized(&self) -> Result<(), TipRouterError> {
        if !self.table_initialized() {
            msg!("Weight table not initialized");
            return Err(TipRouterError::TableNotInitialized);
        }
        Ok(())
    }

    pub fn check_registry_initialized(&self) -> Result<(), TipRouterError> {
        if !self.vault_registry_initialized() {
            msg!(
                "Vault registry not initialized {}/{}",
                self.vault_count(),
                self.vault_entry_count()
            );
            return Err(TipRouterError::RegistryNotInitialized);
        }
        Ok(())
    }

    pub fn check_registry_for_vault(&self, vault_index: u64) -> Result<(), TipRouterError> {
        if !self
            .vault_registry
            .iter()
            .any(|entry| entry.vault_index().eq(&vault_index))
        {
            return Err(TipRouterError::VaultNotInRegistry);
        }
        Ok(())
    }

    pub fn load(
        program_id: &Pubkey,
        weight_table: &AccountInfo,
        ncn: &AccountInfo,
        ncn_epoch: u64,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        if weight_table.owner.ne(program_id) {
            msg!("Weight table account is not owned by the program");
            return Err(ProgramError::InvalidAccountOwner);
        }
        if weight_table.data_is_empty() {
            msg!("Weight table account is empty");
            return Err(ProgramError::InvalidAccountData);
        }
        if expect_writable && !weight_table.is_writable {
            msg!("Weight table account is not writable");
            return Err(ProgramError::InvalidAccountData);
        }
        if weight_table.data.borrow()[0].ne(&Self::DISCRIMINATOR) {
            msg!("Weight table account has an incorrect discriminator",);
            return Err(ProgramError::InvalidAccountData);
        }
        let expected_pubkey = Self::find_program_address(program_id, ncn.key, ncn_epoch).0;
        if weight_table.key.ne(&expected_pubkey) {
            msg!("Weight table incorrect PDA");
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::mem::size_of;

    use solana_program::pubkey::Pubkey;

    use super::*;
    use crate::ncn_fee_group::NcnFeeGroup;

    fn get_test_mint_entries(count: usize) -> [StMintEntry; 64] {
        let mut mints = [StMintEntry::default(); MAX_ST_MINTS];

        for i in 0..count {
            mints[i] = StMintEntry::new(
                Pubkey::new_unique(),
                NcnFeeGroup::default(),
                0,
                Pubkey::new_unique(),
                0,
            );
        }

        mints
    }

    #[test]
    fn test_len() {
        let expected_total = size_of::<Pubkey>() // ncn
            + size_of::<PodU64>() // ncn_epoch
            + size_of::<PodU64>() // slot_created
            + size_of::<PodU64>() // vault_count
            + 1 // bump
            + 128 // reserved
            + size_of::<[VaultEntry; MAX_VAULTS]>() // vault registry
            + size_of::<[WeightEntry; MAX_ST_MINTS]>(); // weight table

        assert_eq!(size_of::<WeightTable>(), expected_total);
    }

    #[test]
    fn test_initialize_table_success() {
        let ncn = Pubkey::new_unique();
        let mut table = WeightTable::new(ncn, 0, 0, 0, 0);
        assert_eq!(table.mint_count(), 0);

        let mints = get_test_mint_entries(2);

        table.set_mint_entries(&mints).unwrap();
        assert_eq!(table.mint_count(), 2);
    }

    #[test]
    fn test_initialize_table_max() {
        let ncn = Pubkey::new_unique();
        let mut table = WeightTable::new(ncn, 0, 0, 0, 0);
        let max_mints = get_test_mint_entries(MAX_ST_MINTS);
        table.set_mint_entries(&max_mints).unwrap();
        assert_eq!(table.mint_count(), MAX_ST_MINTS);
    }

    #[test]
    fn test_initialize_table_reinitialize() {
        let ncn = Pubkey::new_unique();
        let mut table = WeightTable::new(ncn, 0, 0, 0, 0);
        let first_mints = get_test_mint_entries(2);
        table.set_mint_entries(&first_mints).unwrap();
        let second_mints = get_test_mint_entries(3);

        assert_eq!(
            table.set_mint_entries(&second_mints),
            Err(TipRouterError::WeightTableAlreadyInitialized)
        );
    }

    #[test]
    fn test_set_weight_success() {
        let ncn = Pubkey::new_unique();
        let mut table = WeightTable::new(ncn, 0, 0, 0, 0);
        let mints = get_test_mint_entries(2);
        let mint_entry = mints[0];

        table.set_mint_entries(&mints).unwrap();

        table.set_weight(&mint_entry.st_mint(), 100, 1).unwrap();
        assert_eq!(table.get_weight(&mint_entry.st_mint()).unwrap(), 100);
    }

    #[test]
    fn test_set_weight_invalid_mint() {
        let ncn = Pubkey::new_unique();
        let mut table = WeightTable::new(ncn, 0, 0, 0, 0);
        let mints = get_test_mint_entries(2);

        table.set_mint_entries(&mints).unwrap();

        let invalid_mint = Pubkey::new_unique();
        assert_eq!(
            table.set_weight(&invalid_mint, 100, 1),
            Err(TipRouterError::InvalidMintForWeightTable)
        );
    }

    #[test]
    fn test_set_weight_update_existing() {
        let ncn = Pubkey::new_unique();
        let mut table = WeightTable::new(ncn, 0, 0, 0, 0);
        let mints = get_test_mint_entries(2);
        let mint = mints[0];

        table.set_mint_entries(&mints).unwrap();

        table.set_weight(&mint.st_mint(), 100, 1).unwrap();
        assert_eq!(table.get_weight(&mint.st_mint()).unwrap(), 100);

        table.set_weight(&mint.st_mint(), 200, 2).unwrap();
        assert_eq!(table.get_weight(&mint.st_mint()).unwrap(), 200);
    }

    #[test]
    fn test_set_weight_multiple_mints() {
        let ncn = Pubkey::new_unique();
        let mut table = WeightTable::new(ncn, 0, 0, 0, 0);
        let mints = get_test_mint_entries(2);
        let mint1 = mints[0];
        let mint2 = mints[1];

        table.set_mint_entries(&mints).unwrap();

        table.set_weight(&mint1.st_mint(), 100, 1).unwrap();
        table.set_weight(&mint2.st_mint(), 200, 1).unwrap();

        assert_eq!(table.get_weight(&mint1.st_mint()).unwrap(), 100);
        assert_eq!(table.get_weight(&mint2.st_mint()).unwrap(), 200);
    }

    #[test]
    fn test_set_weight_different_slots() {
        let ncn = Pubkey::new_unique();
        let mut table = WeightTable::new(ncn, 0, 0, 0, 0);
        let mints = get_test_mint_entries(2);
        let mint = mints[0];

        table.set_mint_entries(&mints).unwrap();

        table.set_weight(&mint.st_mint(), 100, 1).unwrap();
        assert_eq!(table.get_weight(&mint.st_mint()).unwrap(), 100);

        table.set_weight(&mint.st_mint(), 200, 5).unwrap();
        assert_eq!(table.get_weight(&mint.st_mint()).unwrap(), 200);
    }
}
