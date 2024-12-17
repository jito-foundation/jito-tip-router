use std::{collections::HashSet, mem::size_of};

use bytemuck::{Pod, Zeroable};
use jito_bytemuck::{types::PodU64, AccountDeserialize, Discriminator};
use shank::{ShankAccount, ShankType};
use solana_program::{account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey};
use spl_math::precise_number::PreciseNumber;

use crate::{
    constants::MAX_VAULT_OPERATOR_DELEGATIONS, discriminators::Discriminators,
    error::TipRouterError, weight_entry::WeightEntry,
};

// PDA'd ["WEIGHT_TABLE", NCN, NCN_EPOCH_SLOT]
#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, AccountDeserialize, ShankAccount)]
#[repr(C)]
pub struct WeightTable {
    /// The NCN on-chain program is the signer to create and update this account,
    /// this pushes the responsibility of managing the account to the NCN program.
    ncn: Pubkey,

    /// The NCN epoch for which the weight table is valid
    ncn_epoch: PodU64,

    /// Slot weight table was created
    slot_created: PodU64,

    /// Bump seed for the PDA
    bump: u8,

    /// Reserved space
    reserved: [u8; 128],

    /// The weight table
    table: [WeightEntry; 64],
}

impl Discriminator for WeightTable {
    const DISCRIMINATOR: u8 = Discriminators::WeightTable as u8;
}

impl WeightTable {
    pub const SIZE: usize = 8 + size_of::<Self>();

    pub fn new(ncn: Pubkey, ncn_epoch: u64, slot_created: u64, bump: u8) -> Self {
        Self {
            ncn,
            ncn_epoch: PodU64::from(ncn_epoch),
            slot_created: PodU64::from(slot_created),
            bump,
            reserved: [0; 128],
            table: [WeightEntry::default(); MAX_VAULT_OPERATOR_DELEGATIONS],
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

    pub fn initialize(
        &mut self,
        ncn: Pubkey,
        ncn_epoch: u64,
        slot_created: u64,
        bump: u8,
        config_supported_mints: &[Pubkey],
    ) -> Result<(), TipRouterError> {
        // Initializes field by field to avoid overflowing stack
        self.ncn = ncn;
        self.ncn_epoch = PodU64::from(ncn_epoch);
        self.slot_created = PodU64::from(slot_created);
        self.bump = bump;
        self.reserved = [0; 128];
        self.table = [WeightEntry::default(); MAX_VAULT_OPERATOR_DELEGATIONS];
        self.set_supported_mints(config_supported_mints)?;
        Ok(())
    }

    fn set_supported_mints(
        &mut self,
        config_supported_mints: &[Pubkey],
    ) -> Result<(), TipRouterError> {
        if self.initialized() {
            return Err(TipRouterError::WeightTableAlreadyInitialized);
        }

        // Check for empty vector
        if config_supported_mints.is_empty() {
            return Err(TipRouterError::NoMintsInTable);
        }

        // Check if vector exceeds maximum allowed entries
        if config_supported_mints.len() > MAX_VAULT_OPERATOR_DELEGATIONS {
            return Err(TipRouterError::TooManyMintsForTable);
        }

        // Check for duplicates using nested iterators
        let unique_mints: HashSet<_> = config_supported_mints.iter().collect();

        if unique_mints.len() != config_supported_mints.len() {
            return Err(TipRouterError::DuplicateMintsInTable);
        }

        // Set table using iterator
        self.table
            .iter_mut()
            .zip(config_supported_mints.iter())
            .for_each(|(entry, &mint)| {
                *entry = WeightEntry::new(mint);
            });

        self.check_initialized()?;

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
            .find(|entry| entry.mint() == *mint)
            .map_or(Err(TipRouterError::InvalidMintForWeightTable), |entry| {
                entry.set_weight(weight, current_slot);
                Ok(())
            })
    }

    pub fn get_weight(&self, mint: &Pubkey) -> Result<u128, TipRouterError> {
        self.table
            .iter()
            .find(|entry| entry.mint() == *mint)
            .map_or(Err(TipRouterError::InvalidMintForWeightTable), |entry| {
                Ok(entry.weight())
            })
    }

    pub fn get_precise_weight(&self, mint: &Pubkey) -> Result<PreciseNumber, TipRouterError> {
        let weight = self.get_weight(mint)?;
        PreciseNumber::new(weight).ok_or(TipRouterError::NewPreciseNumberError)
    }

    pub fn get_mints(&self) -> Vec<Pubkey> {
        self.table
            .iter()
            .filter(|entry| !entry.is_empty())
            .map(|entry| entry.mint())
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
        self.ncn_epoch.into()
    }

    pub fn slot_created(&self) -> u64 {
        self.slot_created.into()
    }

    pub fn initialized(&self) -> bool {
        self.mint_count() > 0
    }

    pub fn finalized(&self) -> bool {
        self.initialized() && self.mint_count() == self.weight_count()
    }

    pub fn check_initialized(&self) -> Result<(), TipRouterError> {
        if !self.initialized() {
            return Err(TipRouterError::NoMintsInTable);
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
            msg!("Weight table account has an incorrect discriminator");
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

    fn get_test_pubkeys(count: usize) -> Vec<Pubkey> {
        (0..count).map(|_| Pubkey::new_unique()).collect()
    }

    #[test]
    fn test_len() {
        let expected_total = size_of::<Pubkey>() // ncn
            + size_of::<PodU64>() // ncn_epoch
            + size_of::<PodU64>() // slot_created
            + 1 // bump
            + 128 // reserved
            + size_of::<[WeightEntry; MAX_VAULT_OPERATOR_DELEGATIONS]>(); // weight table

        assert_eq!(size_of::<WeightTable>(), expected_total);
    }

    #[test]
    fn test_initialize_table_success() {
        let ncn = Pubkey::new_unique();
        let mut table = WeightTable::new(ncn, 0, 0, 0);
        assert_eq!(table.mint_count(), 0);

        let mints = get_test_pubkeys(2);
        table.set_supported_mints(&mints).unwrap();
        assert_eq!(table.mint_count(), 2);
    }

    #[test]
    fn test_initialize_table_too_many() {
        let ncn = Pubkey::new_unique();
        let mut table = WeightTable::new(ncn, 0, 0, 0);
        let many_mints = get_test_pubkeys(MAX_VAULT_OPERATOR_DELEGATIONS + 1);
        assert_eq!(
            table.set_supported_mints(&many_mints),
            Err(TipRouterError::TooManyMintsForTable)
        );
    }

    #[test]
    fn test_initialize_table_max() {
        let ncn = Pubkey::new_unique();
        let mut table = WeightTable::new(ncn, 0, 0, 0);
        let max_mints = get_test_pubkeys(MAX_VAULT_OPERATOR_DELEGATIONS);
        table.set_supported_mints(&max_mints).unwrap();
        assert_eq!(table.mint_count(), MAX_VAULT_OPERATOR_DELEGATIONS);
    }

    #[test]
    fn test_initialize_table_reinitialize() {
        let ncn = Pubkey::new_unique();
        let mut table = WeightTable::new(ncn, 0, 0, 0);
        let first_mints = get_test_pubkeys(2);
        table.set_supported_mints(&first_mints).unwrap();
        let second_mints = get_test_pubkeys(3);

        assert_eq!(
            table.set_supported_mints(&second_mints),
            Err(TipRouterError::WeightTableAlreadyInitialized)
        );
    }

    #[test]
    fn test_set_weight_success() {
        let ncn = Pubkey::new_unique();
        let mut table = WeightTable::new(ncn, 0, 0, 0);
        let mints = get_test_pubkeys(2);
        let mint = mints[0];

        table.set_supported_mints(&mints).unwrap();

        table.set_weight(&mint, 100, 1).unwrap();
        assert_eq!(table.get_weight(&mint).unwrap(), 100);
    }

    #[test]
    fn test_set_weight_invalid_mint() {
        let ncn = Pubkey::new_unique();
        let mut table = WeightTable::new(ncn, 0, 0, 0);
        let mints = get_test_pubkeys(2);

        table.set_supported_mints(&mints).unwrap();

        let invalid_mint = Pubkey::new_unique();
        assert_eq!(
            table.set_weight(&invalid_mint, 100, 1),
            Err(TipRouterError::InvalidMintForWeightTable)
        );
    }

    #[test]
    fn test_set_weight_update_existing() {
        let ncn = Pubkey::new_unique();
        let mut table = WeightTable::new(ncn, 0, 0, 0);
        let mints = get_test_pubkeys(2);
        let mint = mints[0];

        table.set_supported_mints(&mints).unwrap();

        table.set_weight(&mint, 100, 1).unwrap();
        assert_eq!(table.get_weight(&mint).unwrap(), 100);

        table.set_weight(&mint, 200, 2).unwrap();
        assert_eq!(table.get_weight(&mint).unwrap(), 200);
    }

    #[test]
    fn test_set_weight_multiple_mints() {
        let ncn = Pubkey::new_unique();
        let mut table = WeightTable::new(ncn, 0, 0, 0);
        let mints = get_test_pubkeys(2);
        let mint1 = mints[0];
        let mint2 = mints[1];

        table.set_supported_mints(&mints).unwrap();

        table.set_weight(&mint1, 100, 1).unwrap();
        table.set_weight(&mint2, 200, 1).unwrap();

        assert_eq!(table.get_weight(&mint1).unwrap(), 100);
        assert_eq!(table.get_weight(&mint2).unwrap(), 200);
    }

    #[test]
    fn test_set_weight_different_slots() {
        let ncn = Pubkey::new_unique();
        let mut table = WeightTable::new(ncn, 0, 0, 0);
        let mints = get_test_pubkeys(2);
        let mint = mints[0];

        table.set_supported_mints(&mints).unwrap();

        table.set_weight(&mint, 100, 1).unwrap();
        assert_eq!(table.get_weight(&mint).unwrap(), 100);

        table.set_weight(&mint, 200, 5).unwrap();
        assert_eq!(table.get_weight(&mint).unwrap(), 200);
    }
}
