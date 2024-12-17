use std::{collections::HashSet, mem::size_of};

use bytemuck::{Pod, Zeroable};
use jito_bytemuck::{types::PodU64, AccountDeserialize, Discriminator};
use shank::{ShankAccount, ShankType};
use solana_program::{account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey};

use crate::{
    constants::MAX_VAULT_OPERATOR_DELEGATIONS, discriminators::Discriminators,
    error::TipRouterError, ncn_fee_group::NcnFeeGroup,
};

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct MintEntry {
    st_mint: Pubkey,
    vault_index: PodU64,
    ncn_fee_group: NcnFeeGroup,
    reward_multiplier_bps: PodU64,
    reserved: [u8; 32],
}

impl MintEntry {
    pub fn new(mint: Pubkey, vault_index: u64) -> Self {
        const REWARD_FEE_MULTIPLIER_BPS: u64 = 10_000; // 100% as default

        Self {
            st_mint: mint,
            vault_index: PodU64::from(vault_index),
            ncn_fee_group: NcnFeeGroup::default(),
            reward_multiplier_bps: PodU64::from(REWARD_FEE_MULTIPLIER_BPS),
            reserved: [0; 32],
        }
    }

    pub fn vault_index(&self) -> u64 {
        self.vault_index.into()
    }

    pub const fn ncn_fee_group(&self) -> NcnFeeGroup {
        self.ncn_fee_group
    }

    pub fn reward_multiplier_bps(&self) -> u64 {
        self.reward_multiplier_bps.into()
    }
}

impl Default for MintEntry {
    fn default() -> Self {
        Self::new(Pubkey::default(), u64::MAX)
    }
}

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, AccountDeserialize, ShankAccount)]
#[repr(C)]
pub struct TrackedMints {
    pub ncn: Pubkey,
    pub bump: u8,
    pub reserved: [u8; 127],
    pub st_mint_list: [MintEntry; 64],
}

impl Discriminator for TrackedMints {
    const DISCRIMINATOR: u8 = Discriminators::TrackedMints as u8;
}

impl TrackedMints {
    pub const SIZE: usize = 8 + size_of::<Self>();

    pub fn new(ncn: Pubkey, bump: u8) -> Self {
        Self {
            ncn,
            bump,
            reserved: [0; 127],
            st_mint_list: [MintEntry::default(); MAX_VAULT_OPERATOR_DELEGATIONS],
        }
    }

    pub fn initialize(&mut self, ncn: Pubkey, bump: u8) {
        // Initializes field by field to avoid overflowing stack
        self.ncn = ncn;
        self.bump = bump;
        self.reserved = [0; 127];
        self.st_mint_list = [MintEntry::default(); MAX_VAULT_OPERATOR_DELEGATIONS];
    }

    pub fn seeds(ncn: &Pubkey) -> Vec<Vec<u8>> {
        Vec::from_iter(
            [b"tracked_mints".to_vec(), ncn.to_bytes().to_vec()]
                .iter()
                .cloned(),
        )
    }

    pub fn find_program_address(program_id: &Pubkey, ncn: &Pubkey) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(ncn);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (address, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (address, bump, seeds)
    }

    pub fn add_mint(&mut self, mint: Pubkey, vault_index: u64) -> Result<(), ProgramError> {
        // Check if (mint, vault_index) is already in the list
        if self
            .st_mint_list
            .iter()
            .any(|m| m.st_mint == mint && m.vault_index() == vault_index)
        {
            return Ok(());
        }

        // Check if vault_index is already in use by a different mint
        if self
            .st_mint_list
            .iter()
            .any(|m| m.vault_index() == vault_index)
        {
            return Err(TipRouterError::VaultIndexAlreadyInUse.into());
        }

        // Insert at the first empty slot
        let mint_entry = self
            .st_mint_list
            .iter_mut()
            .find(|m| m.st_mint == MintEntry::default().st_mint)
            .ok_or(TipRouterError::TrackedMintListFull)?;

        *mint_entry = MintEntry::new(mint, vault_index);
        Ok(())
    }

    pub fn mint_count(&self) -> u64 {
        self.st_mint_list
            .iter()
            .filter(|m| m.st_mint != Pubkey::default())
            .count() as u64
    }

    pub fn get_unique_mints(&self) -> Vec<Pubkey> {
        let mut unique_mints: HashSet<Pubkey> = HashSet::new();
        self.st_mint_list
            .iter()
            .filter(|m| m.st_mint != Pubkey::default())
            .for_each(|m| {
                unique_mints.insert(m.st_mint);
            });
        unique_mints.into_iter().collect()
    }

    pub fn load(
        program_id: &Pubkey,
        ncn: &Pubkey,
        account: &AccountInfo,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        if account.owner.ne(program_id) {
            msg!("Tracked Mints account has an invalid owner");
            return Err(ProgramError::InvalidAccountOwner);
        }

        if account.data_is_empty() {
            msg!("Tracked Mints account data is empty");
            return Err(ProgramError::InvalidAccountData);
        }

        if expect_writable && !account.is_writable {
            msg!("Tracked Mints account is not writable");
            return Err(ProgramError::InvalidAccountData);
        }

        if account.data.borrow()[0].ne(&Self::DISCRIMINATOR) {
            msg!("Tracked Mints account discriminator is invalid");
            return Err(ProgramError::InvalidAccountData);
        }

        if account
            .key
            .ne(&Self::find_program_address(program_id, ncn).0)
        {
            msg!("Tracked Mints account is not at the correct PDA");
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(())
    }

    pub fn get_ncn_fee_group(&self, vault_index: u64) -> Result<NcnFeeGroup, ProgramError> {
        let mint_entry = self
            .st_mint_list
            .iter()
            .find(|m| m.vault_index() == vault_index)
            .ok_or(TipRouterError::MintEntryNotFound)?;

        Ok(mint_entry.ncn_fee_group)
    }

    pub fn get_mint_entry(&self, vault_index: u64) -> Result<MintEntry, ProgramError> {
        let mint_entry = self
            .st_mint_list
            .iter()
            .find(|m| m.vault_index() == vault_index)
            .ok_or(TipRouterError::MintEntryNotFound)?;

        Ok(*mint_entry)
    }

    pub fn set_ncn_fee_group(
        &mut self,
        vault_index: u64,
        ncn_fee_group: NcnFeeGroup,
    ) -> Result<(), ProgramError> {
        let mint_entry = self
            .st_mint_list
            .iter_mut()
            .find(|m| m.vault_index() == vault_index)
            .ok_or(TipRouterError::MintEntryNotFound)?;

        mint_entry.ncn_fee_group = ncn_fee_group;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_len() {
        use std::mem::size_of;

        let expected_total = size_of::<Pubkey>() // ncn
            + 1 // bump
            + 127 // reserved
            + size_of::<MintEntry>() * MAX_VAULT_OPERATOR_DELEGATIONS; // st_mint_list

        assert_eq!(size_of::<TrackedMints>(), expected_total);

        let tracked_mints = TrackedMints::new(Pubkey::default(), 0);
        assert_eq!(
            tracked_mints.st_mint_list.len(),
            MAX_VAULT_OPERATOR_DELEGATIONS
        );
    }

    #[test]
    fn test_add_mint() {
        let mut tracked_mints = TrackedMints::new(Pubkey::default(), 0);
        let mint = Pubkey::new_unique();

        assert_eq!(tracked_mints.mint_count(), 0);
        tracked_mints.add_mint(mint, 0).unwrap();
        assert_eq!(tracked_mints.mint_count(), 1);

        // Adding same mint with different vault index should succeed
        tracked_mints.add_mint(mint, 1).unwrap();
        assert_eq!(tracked_mints.mint_count(), 2);

        // Adding same mint with same vault index should succeed but do nothing
        tracked_mints.add_mint(mint, 1).unwrap();
        assert_eq!(tracked_mints.mint_count(), 2);

        // Adding different mint with same vault index should fail
        let mint2 = Pubkey::new_unique();
        assert!(tracked_mints.add_mint(mint2, 1).is_err());

        // Adding to a full list should fail
        for i in (tracked_mints.mint_count() as usize)..tracked_mints.st_mint_list.len() {
            tracked_mints
                .add_mint(Pubkey::new_unique(), i as u64)
                .unwrap();
        }
        assert!(tracked_mints.add_mint(Pubkey::new_unique(), 0).is_err());
    }

    #[test]
    fn test_mint_count() {
        let mut tracked_mints = TrackedMints::new(Pubkey::default(), 0);
        assert_eq!(tracked_mints.mint_count(), 0);

        for i in 0..3 {
            tracked_mints.add_mint(Pubkey::new_unique(), i).unwrap();
        }
        assert_eq!(tracked_mints.mint_count(), 3);
    }

    #[test]
    fn test_get_unique_mints() {
        let mut tracked_mints = TrackedMints::new(Pubkey::default(), 0);

        let mint1 = Pubkey::new_unique();
        let mint2 = Pubkey::new_unique();
        tracked_mints.add_mint(mint1, 0).unwrap();
        tracked_mints.add_mint(mint2, 1).unwrap();
        tracked_mints.add_mint(mint1, 2).unwrap();

        let unique_mints = tracked_mints.get_unique_mints();
        assert_eq!(unique_mints.len(), 2);
        assert!(unique_mints.contains(&mint1));
        assert!(unique_mints.contains(&mint2));

        // Default pubkeys should not be included
        let empty_tracked_mints = TrackedMints::new(Pubkey::default(), 0);
        assert_eq!(empty_tracked_mints.get_unique_mints().len(), 0);
    }
}
