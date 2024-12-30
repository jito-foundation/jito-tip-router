use std::mem::size_of;

use bytemuck::{Pod, Zeroable};
use jito_bytemuck::{
    types::{PodU128, PodU64},
    AccountDeserialize, Discriminator,
};
use shank::{ShankAccount, ShankType};
use solana_program::{account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey};

use crate::{
    constants::{MAX_ST_MINTS, MAX_VAULTS},
    discriminators::Discriminators,
    error::TipRouterError,
    ncn_fee_group::NcnFeeGroup,
};

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct StMintEntry {
    /// The supported token ( ST ) mint
    st_mint: Pubkey,
    /// The fee group for the mint
    ncn_fee_group: NcnFeeGroup,
    /// The reward multiplier in basis points
    reward_multiplier_bps: PodU64,
    // Either a switchboard feed or a no feed weight must be set
    /// The switchboard feed for the mint
    switchboard_feed: Pubkey,
    /// The weight when no feed is available
    no_feed_weight: PodU128,
    /// Reserved space
    reserved: [u8; 128],
}

impl StMintEntry {
    pub fn new(
        st_mint: &Pubkey,
        ncn_fee_group: NcnFeeGroup,
        reward_multiplier_bps: u64,
        switchboard_feed: &Pubkey,
        no_feed_weight: u128,
    ) -> Self {
        Self {
            st_mint: *st_mint,
            ncn_fee_group,
            reward_multiplier_bps: PodU64::from(reward_multiplier_bps),
            switchboard_feed: *switchboard_feed,
            no_feed_weight: PodU128::from(no_feed_weight),
            reserved: [0; 128],
        }
    }

    pub fn no_feed_weight(&self) -> u128 {
        self.no_feed_weight.into()
    }

    pub const fn st_mint(&self) -> &Pubkey {
        &self.st_mint
    }

    pub const fn ncn_fee_group(&self) -> NcnFeeGroup {
        self.ncn_fee_group
    }

    pub fn reward_multiplier_bps(&self) -> u64 {
        self.reward_multiplier_bps.into()
    }

    pub const fn switchboard_feed(&self) -> &Pubkey {
        &self.switchboard_feed
    }

    pub fn is_empty(&self) -> bool {
        self.st_mint().eq(&Pubkey::default())
    }
}

impl Default for StMintEntry {
    fn default() -> Self {
        Self::new(
            &Pubkey::default(),
            NcnFeeGroup::default(),
            0,
            &Pubkey::default(),
            0,
        )
    }
}

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct VaultEntry {
    /// The vault account
    vault: Pubkey,
    /// The supported token ( ST ) mint of the vault
    st_mint: Pubkey,
    /// The index of the vault in respect to the NCN account
    vault_index: PodU64,
    /// The slot the vault was registered
    slot_registered: PodU64,
    /// Reserved space
    reserved: [u8; 128],
}

impl VaultEntry {
    pub fn new(vault: &Pubkey, st_mint: &Pubkey, vault_index: u64, slot_registered: u64) -> Self {
        Self {
            vault: *vault,
            st_mint: *st_mint,
            vault_index: PodU64::from(vault_index),
            slot_registered: PodU64::from(slot_registered),
            reserved: [0; 128],
        }
    }

    pub const fn vault(&self) -> &Pubkey {
        &self.vault
    }

    pub fn vault_index(&self) -> u64 {
        self.vault_index.into()
    }

    pub fn slot_registered(&self) -> u64 {
        self.slot_registered.into()
    }

    pub fn is_empty(&self) -> bool {
        // self.vault.eq(&Pubkey::default())
        self.slot_registered() == u64::MAX
    }
}

impl Default for VaultEntry {
    fn default() -> Self {
        Self::new(&Pubkey::default(), &Pubkey::default(), u64::MAX, u64::MAX)
    }
}

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, AccountDeserialize, ShankAccount)]
#[repr(C)]
pub struct VaultRegistry {
    /// The NCN the vault registry is associated with
    pub ncn: Pubkey,
    /// The bump seed for the PDA
    pub bump: u8,
    /// Reserved space
    pub reserved: [u8; 127],
    /// The list of supported token ( ST ) mints
    pub st_mint_list: [StMintEntry; 64],
    /// The list of vaults
    pub vault_list: [VaultEntry; 64],
}

impl Discriminator for VaultRegistry {
    const DISCRIMINATOR: u8 = Discriminators::VaultRegistry as u8;
}

impl VaultRegistry {
    pub const SIZE: usize = 8 + size_of::<Self>();

    pub fn new(ncn: &Pubkey, bump: u8) -> Self {
        Self {
            ncn: *ncn,
            bump,
            reserved: [0; 127],
            st_mint_list: [StMintEntry::default(); MAX_ST_MINTS],
            vault_list: [VaultEntry::default(); MAX_VAULTS],
        }
    }

    pub fn initialize(&mut self, ncn: &Pubkey, bump: u8) {
        // Initializes field by field to avoid overflowing stack
        self.ncn = *ncn;
        self.bump = bump;
        self.reserved = [0; 127];
        self.st_mint_list = [StMintEntry::default(); MAX_ST_MINTS];
        self.vault_list = [VaultEntry::default(); MAX_VAULTS];
    }

    pub fn seeds(ncn: &Pubkey) -> Vec<Vec<u8>> {
        Vec::from_iter(
            [b"vault_registry".to_vec(), ncn.to_bytes().to_vec()]
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

    pub fn load(
        program_id: &Pubkey,
        ncn: &Pubkey,
        account: &AccountInfo,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        if account.owner.ne(program_id) {
            msg!("Vault Registry account has an invalid owner");
            return Err(ProgramError::InvalidAccountOwner);
        }

        if account.data_is_empty() {
            msg!("Vault Registry account data is empty");
            return Err(ProgramError::InvalidAccountData);
        }

        if expect_writable && !account.is_writable {
            msg!("Vault Registry account is not writable");
            return Err(ProgramError::InvalidAccountData);
        }

        if account.data.borrow()[0].ne(&Self::DISCRIMINATOR) {
            msg!("Vault Registry account discriminator is invalid");
            return Err(ProgramError::InvalidAccountData);
        }

        if account
            .key
            .ne(&Self::find_program_address(program_id, ncn).0)
        {
            msg!("Vault Registry account is not at the correct PDA");
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(())
    }

    pub fn has_st_mint(&self, mint: &Pubkey) -> bool {
        self.st_mint_list.iter().any(|m| m.st_mint.eq(mint))
    }

    pub fn check_st_mint(&self, st_mint: &Pubkey) -> Result<(), ProgramError> {
        let entry = self.get_mint_entry(st_mint)?;

        if entry.no_feed_weight() == 0 && entry.switchboard_feed().eq(&Pubkey::default()) {
            return Err(TipRouterError::NoFeedWeightOrSwitchboardFeed.into());
        }

        Ok(())
    }

    pub fn register_st_mint(
        &mut self,
        st_mint: &Pubkey,
        ncn_fee_group: NcnFeeGroup,
        reward_multiplier_bps: u64,
        switchboard_feed: &Pubkey,
        no_feed_weight: u128,
    ) -> Result<(), ProgramError> {
        // Check if mint is already in the list
        if self.st_mint_list.iter().any(|m| m.st_mint.eq(st_mint)) {
            return Err(TipRouterError::MintInTable.into());
        }

        // Insert at the first empty slot
        let mint_entry = self
            .st_mint_list
            .iter_mut()
            .find(|m| m.st_mint == StMintEntry::default().st_mint)
            .ok_or(TipRouterError::VaultRegistryListFull)?;

        *mint_entry = StMintEntry::new(
            st_mint,
            ncn_fee_group,
            reward_multiplier_bps,
            switchboard_feed,
            no_feed_weight,
        );

        self.check_st_mint(st_mint)?;

        Ok(())
    }

    pub fn set_st_mint(
        &mut self,
        st_mint: &Pubkey,
        ncn_fee_group: Option<u8>,
        reward_multiplier_bps: Option<u64>,
        switchboard_feed: Option<Pubkey>,
        no_feed_weight: Option<u128>,
    ) -> Result<(), ProgramError> {
        let mint_entry = self
            .st_mint_list
            .iter_mut()
            .find(|m| m.st_mint.eq(st_mint))
            .ok_or(TipRouterError::MintEntryNotFound)?;

        if let Some(ncn_fee_group) = ncn_fee_group {
            mint_entry.ncn_fee_group = NcnFeeGroup::try_from(ncn_fee_group)?;
        }

        if let Some(reward_multiplier_bps) = reward_multiplier_bps {
            mint_entry.reward_multiplier_bps = PodU64::from(reward_multiplier_bps);
        }

        if let Some(switchboard_feed) = switchboard_feed {
            mint_entry.switchboard_feed = switchboard_feed;
        }

        if let Some(no_feed_weight) = no_feed_weight {
            mint_entry.no_feed_weight = PodU128::from(no_feed_weight);
        }

        self.check_st_mint(st_mint)?;

        Ok(())
    }

    pub fn register_vault(
        &mut self,
        vault: &Pubkey,
        st_mint: &Pubkey,
        vault_index: u64,
        current_slot: u64,
    ) -> Result<(), ProgramError> {
        // Check if (mint, vault_index) is already in the list
        if self.vault_list.iter().any(|m| m.vault.eq(vault)) {
            return Ok(());
        }

        // Insert at the first empty slot
        let mint_entry = self
            .vault_list
            .iter_mut()
            .find(|m| m.st_mint == VaultEntry::default().st_mint)
            .ok_or(TipRouterError::VaultRegistryListFull)?;

        *mint_entry = VaultEntry::new(vault, st_mint, vault_index, current_slot);
        Ok(())
    }

    pub fn get_valid_vault_entries(&self) -> Vec<VaultEntry> {
        self.vault_list
            .iter()
            .filter(|m| !m.is_empty())
            .copied()
            .collect()
    }

    pub const fn get_vault_entries(&self) -> &[VaultEntry; MAX_VAULTS] {
        &self.vault_list
    }

    pub fn vault_count(&self) -> u64 {
        self.vault_list.iter().filter(|m| !m.is_empty()).count() as u64
    }

    pub fn get_valid_mint_entries(&self) -> Vec<StMintEntry> {
        self.st_mint_list
            .iter()
            .filter(|m| !m.is_empty())
            .copied()
            .collect()
    }

    pub const fn get_mint_entries(&self) -> &[StMintEntry; MAX_ST_MINTS] {
        &self.st_mint_list
    }

    pub fn get_mint_entry(&self, st_mint: &Pubkey) -> Result<StMintEntry, ProgramError> {
        let mint_entry = self
            .st_mint_list
            .iter()
            .find(|m| m.st_mint().eq(st_mint))
            .ok_or(TipRouterError::MintEntryNotFound)?;

        Ok(*mint_entry)
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
            + size_of::<StMintEntry>() * MAX_ST_MINTS // st_mint_list
            + size_of::<VaultEntry>() * MAX_VAULTS; // vault_list

        assert_eq!(size_of::<VaultRegistry>(), expected_total);

        let vault_registry = VaultRegistry::new(&Pubkey::default(), 0);
        assert_eq!(vault_registry.vault_list.len(), MAX_VAULTS);
    }

    // #[test]
    // fn test_add_mint() {
    //     let mut vault_registry = VaultRegistry::new(Pubkey::default(), 0);
    //     let vault = Pubkey::new_unique();
    //     let mint = Pubkey::new_unique();

    //     assert_eq!(vault_registry.vault_count(), 0);
    //     vault_registry.register_vault(mint, 0, 0).unwrap();
    //     assert_eq!(vault_registry.vault_count(), 1);

    //     // Adding same mint with different vault index should succeed
    //     vault_registry.register_vault(mint, 1, 0).unwrap();
    //     assert_eq!(vault_registry.vault_count(), 2);

    //     // Adding same mint with same vault index should succeed but do nothing
    //     vault_registry.register_vault(mint, 1, 0).unwrap();
    //     assert_eq!(vault_registry.vault_count(), 2);

    //     // Adding different mint with same vault index should fail
    //     let mint2 = Pubkey::new_unique();
    //     assert!(vault_registry.register_vault(mint2, 1, 0).is_err());

    //     // Adding to a full list should fail
    //     for i in (vault_registry.vault_count() as usize)..vault_registry.vault_list.len() {
    //         vault_registry
    //             .register_vault(Pubkey::new_unique(), Pubkey::new_unique(), i as u64, 0)
    //             .unwrap();
    //     }
    //     assert!(vault_registry
    //         .register_vault(Pubkey::new_unique(), Pubkey::new_unique(), 0, 0)
    //         .is_err());
    // }

    #[test]
    fn test_mint_count() {
        let mut vault_registry = VaultRegistry::new(&Pubkey::default(), 0);
        assert_eq!(vault_registry.vault_count(), 0);

        for i in 0..3 {
            vault_registry
                .register_vault(&Pubkey::new_unique(), &Pubkey::new_unique(), i, 0)
                .unwrap();
        }
        assert_eq!(vault_registry.vault_count(), 3);
    }

    #[test]
    fn test_no_duplicate_mints() {
        let mut vault_registry = VaultRegistry::new(&Pubkey::default(), 0);

        let mint1 = Pubkey::new_unique();
        let mint2 = Pubkey::new_unique();
        vault_registry
            .register_st_mint(&mint1, NcnFeeGroup::jto(), 0, &Pubkey::new_unique(), 0)
            .unwrap();
        vault_registry
            .register_st_mint(&mint2, NcnFeeGroup::jto(), 0, &Pubkey::new_unique(), 0)
            .unwrap();

        let result = vault_registry.register_st_mint(
            &mint1,
            NcnFeeGroup::jto(),
            0,
            &Pubkey::new_unique(),
            0,
        );

        assert!(result.is_err());
    }
}
