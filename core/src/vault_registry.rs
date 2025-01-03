use std::mem::size_of;

use bytemuck::{Pod, Zeroable};
use jito_bytemuck::{
    types::{PodU128, PodU64},
    AccountDeserialize, Discriminator,
};
use shank::{ShankAccount, ShankType};
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

use crate::{
    constants::{MAX_ST_MINTS, MAX_VAULTS},
    discriminators::Discriminators,
    error::TipRouterError,
    loaders::check_load,
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
    pub const EMPTY_VAULT_INDEX: u64 = u64::MAX;
    pub const EMPTY_SLOT_REGISTERED: u64 = u64::MAX;

    pub fn new(vault: &Pubkey, st_mint: &Pubkey, vault_index: u64, slot_registered: u64) -> Self {
        Self {
            vault: *vault,
            st_mint: *st_mint,
            vault_index: PodU64::from(vault_index),
            slot_registered: PodU64::from(slot_registered),
            reserved: [0; 128],
        }
    }

    pub fn vault_index(&self) -> u64 {
        self.vault_index.into()
    }

    pub fn slot_registered(&self) -> u64 {
        self.slot_registered.into()
    }

    pub fn is_empty(&self) -> bool {
        self.slot_registered() == u64::MAX
    }
}

impl Default for VaultEntry {
    fn default() -> Self {
        Self::new(
            &Pubkey::default(),
            &Pubkey::default(),
            Self::EMPTY_VAULT_INDEX,
            Self::EMPTY_SLOT_REGISTERED,
        )
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
        let expected_pda = Self::find_program_address(program_id, ncn).0;
        check_load(
            program_id,
            account,
            &expected_pda,
            Some(Self::DISCRIMINATOR),
            expect_writable,
        )
    }

    pub fn has_st_mint(&self, mint: &Pubkey) -> bool {
        self.st_mint_list.iter().any(|m| m.st_mint.eq(mint))
    }

    pub fn check_st_mint_entry(entry: &StMintEntry) -> Result<(), ProgramError> {
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

        let new_mint_entry = StMintEntry::new(
            st_mint,
            ncn_fee_group,
            reward_multiplier_bps,
            switchboard_feed,
            no_feed_weight,
        );

        Self::check_st_mint_entry(&new_mint_entry)?;

        *mint_entry = new_mint_entry;

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

        let mut updated_mint_entry = *mint_entry;

        if let Some(ncn_fee_group) = ncn_fee_group {
            updated_mint_entry.ncn_fee_group = NcnFeeGroup::try_from(ncn_fee_group)?;
        }

        if let Some(reward_multiplier_bps) = reward_multiplier_bps {
            updated_mint_entry.reward_multiplier_bps = PodU64::from(reward_multiplier_bps);
        }

        if let Some(switchboard_feed) = switchboard_feed {
            updated_mint_entry.switchboard_feed = switchboard_feed;
        }

        if let Some(no_feed_weight) = no_feed_weight {
            updated_mint_entry.no_feed_weight = PodU128::from(no_feed_weight);
        }

        Self::check_st_mint_entry(&updated_mint_entry)?;

        *mint_entry = updated_mint_entry;

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

    #[test]
    fn test_add_mint() {
        let mut vault_registry = VaultRegistry::new(&Pubkey::default(), 0);
        let mint = Pubkey::new_unique();
        let switchboard_feed = Pubkey::new_unique();

        // Test 1: Initial registration should succeed
        assert_eq!(vault_registry.get_valid_mint_entries().len(), 0);
        vault_registry
            .register_st_mint(&mint, NcnFeeGroup::jto(), 1000, &switchboard_feed, 0)
            .unwrap();
        assert_eq!(vault_registry.get_valid_mint_entries().len(), 1);

        // Test 2: Trying to add the same mint should fail
        let result =
            vault_registry.register_st_mint(&mint, NcnFeeGroup::jto(), 1000, &switchboard_feed, 0);
        assert!(result.is_err());
        assert_eq!(vault_registry.get_valid_mint_entries().len(), 1);

        // Test 3: Adding a different mint should succeed
        let mint2 = Pubkey::new_unique();
        vault_registry
            .register_st_mint(&mint2, NcnFeeGroup::jto(), 1000, &switchboard_feed, 0)
            .unwrap();
        assert_eq!(vault_registry.get_valid_mint_entries().len(), 2);

        // Test 4: Verify mint entry data is stored correctly
        let entry = vault_registry.get_mint_entry(&mint).unwrap();
        assert_eq!(entry.st_mint(), &mint);
        assert_eq!(entry.ncn_fee_group(), NcnFeeGroup::jto());
        assert_eq!(entry.reward_multiplier_bps(), 1000);
        assert_eq!(entry.switchboard_feed(), &switchboard_feed);
        assert_eq!(entry.no_feed_weight(), 0);

        // Test 5: Adding a mint without either switchboard feed or no_feed_weight should fail
        let mint3 = Pubkey::new_unique();
        let result = vault_registry.register_st_mint(
            &mint3,
            NcnFeeGroup::jto(),
            1000,
            &Pubkey::default(),
            0,
        );
        assert!(result.is_err());
        assert_eq!(vault_registry.get_valid_mint_entries().len(), 2);

        // Test 6: Fill up the mint list
        for _ in 2..MAX_ST_MINTS {
            let new_mint = Pubkey::new_unique();
            vault_registry
                .register_st_mint(&new_mint, NcnFeeGroup::jto(), 1000, &switchboard_feed, 0)
                .unwrap();
        }

        // Test 7: Attempting to add to a full list should fail
        let overflow_mint = Pubkey::new_unique();
        let result = vault_registry.register_st_mint(
            &overflow_mint,
            NcnFeeGroup::jto(),
            1000,
            &switchboard_feed,
            0,
        );
        assert!(result.is_err());
        assert_eq!(vault_registry.get_valid_mint_entries().len(), MAX_ST_MINTS);

        // Test 8: has_st_mint should work correctly
        assert!(vault_registry.has_st_mint(&mint));
        assert!(vault_registry.has_st_mint(&mint2));
        assert!(!vault_registry.has_st_mint(&overflow_mint));

        // Test 9: Test mint with no_feed_weight instead of switchboard feed
        let mut fresh_registry = VaultRegistry::new(&Pubkey::default(), 0);
        let mint_with_weight = Pubkey::new_unique();
        fresh_registry
            .register_st_mint(
                &mint_with_weight,
                NcnFeeGroup::jto(),
                1000,
                &Pubkey::default(),
                100,
            )
            .unwrap();

        let entry = fresh_registry.get_mint_entry(&mint_with_weight).unwrap();
        assert_eq!(entry.no_feed_weight(), 100);
        assert_eq!(entry.switchboard_feed(), &Pubkey::default());
    }

    #[test]
    fn test_set_st_mint() {
        let mut vault_registry = VaultRegistry::new(&Pubkey::default(), 0);
        let mint = Pubkey::new_unique();
        let switchboard_feed = Pubkey::new_unique();

        // First register a mint to update
        vault_registry
            .register_st_mint(&mint, NcnFeeGroup::lst(), 1000, &switchboard_feed, 0)
            .unwrap();

        // Test 1: Verify initial state
        let entry = vault_registry.get_mint_entry(&mint).unwrap();
        assert_eq!(entry.st_mint(), &mint);
        assert_eq!(entry.ncn_fee_group(), NcnFeeGroup::lst());
        assert_eq!(entry.reward_multiplier_bps(), 1000);
        assert_eq!(entry.switchboard_feed(), &switchboard_feed);
        assert_eq!(entry.no_feed_weight(), 0);

        // Test 2: Update ncn_fee_group only
        vault_registry
            .set_st_mint(&mint, Some(NcnFeeGroup::lst().group), None, None, None)
            .unwrap();
        let entry = vault_registry.get_mint_entry(&mint).unwrap();
        assert_eq!(entry.ncn_fee_group(), NcnFeeGroup::lst());
        assert_eq!(entry.reward_multiplier_bps(), 1000); // unchanged
        assert_eq!(entry.switchboard_feed(), &switchboard_feed); // unchanged
        assert_eq!(entry.no_feed_weight(), 0); // unchanged

        // Test 3: Update reward_multiplier_bps only
        vault_registry
            .set_st_mint(&mint, None, Some(2000), None, None)
            .unwrap();
        let entry = vault_registry.get_mint_entry(&mint).unwrap();
        assert_eq!(entry.ncn_fee_group(), NcnFeeGroup::lst()); // unchanged
        assert_eq!(entry.reward_multiplier_bps(), 2000);
        assert_eq!(entry.switchboard_feed(), &switchboard_feed); // unchanged
        assert_eq!(entry.no_feed_weight(), 0); // unchanged

        // Test 4: Update switchboard_feed only
        let new_switchboard_feed = Pubkey::new_unique();
        vault_registry
            .set_st_mint(&mint, None, None, Some(new_switchboard_feed), None)
            .unwrap();
        let entry = vault_registry.get_mint_entry(&mint).unwrap();
        assert_eq!(entry.ncn_fee_group(), NcnFeeGroup::lst()); // unchanged
        assert_eq!(entry.reward_multiplier_bps(), 2000); // unchanged
        assert_eq!(entry.switchboard_feed(), &new_switchboard_feed);
        assert_eq!(entry.no_feed_weight(), 0); // unchanged

        // Test 5: Update no_feed_weight only
        vault_registry
            .set_st_mint(&mint, None, None, None, Some(100))
            .unwrap();
        let entry = vault_registry.get_mint_entry(&mint).unwrap();
        assert_eq!(entry.ncn_fee_group(), NcnFeeGroup::lst()); // unchanged
        assert_eq!(entry.reward_multiplier_bps(), 2000); // unchanged
        assert_eq!(entry.switchboard_feed(), &new_switchboard_feed); // unchanged
        assert_eq!(entry.no_feed_weight(), 100);

        // Test 6: Update multiple fields at once
        vault_registry
            .set_st_mint(
                &mint,
                Some(NcnFeeGroup::jto().group),
                Some(3000),
                Some(switchboard_feed),
                Some(200),
            )
            .unwrap();
        let entry = vault_registry.get_mint_entry(&mint).unwrap();
        assert_eq!(entry.ncn_fee_group(), NcnFeeGroup::jto());
        assert_eq!(entry.reward_multiplier_bps(), 3000);
        assert_eq!(entry.switchboard_feed(), &switchboard_feed);
        assert_eq!(entry.no_feed_weight(), 200);

        // Test 7: Attempt to update non-existent mint
        let nonexistent_mint = Pubkey::new_unique();
        let result = vault_registry.set_st_mint(
            &nonexistent_mint,
            Some(NcnFeeGroup::jto().group),
            None,
            None,
            None,
        );
        assert_eq!(
            result.unwrap_err(),
            ProgramError::from(TipRouterError::MintEntryNotFound)
        );

        // Test 8: Setting both switchboard_feed and no_feed_weight to invalid values should fail
        let result =
            vault_registry.set_st_mint(&mint, None, None, Some(Pubkey::default()), Some(0));
        assert!(result.is_err());

        // Test 9: Verify original values remain after failed update
        let entry = vault_registry.get_mint_entry(&mint).unwrap();
        assert_eq!(entry.ncn_fee_group(), NcnFeeGroup::jto());
        assert_eq!(entry.reward_multiplier_bps(), 3000);
        assert_eq!(entry.switchboard_feed(), &switchboard_feed);
        assert_eq!(entry.no_feed_weight(), 200);

        // Test 10: Invalid ncn_fee_group value should fail
        let result = vault_registry.set_st_mint(
            &mint,
            Some(255), // Invalid group
            None,
            None,
            None,
        );
        assert!(result.is_err());
    }

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
