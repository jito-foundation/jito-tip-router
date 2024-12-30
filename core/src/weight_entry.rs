use bytemuck::{Pod, Zeroable};
use jito_bytemuck::types::{PodU128, PodU64};
use shank::ShankType;
use solana_program::pubkey::Pubkey;
use spl_math::precise_number::PreciseNumber;

use crate::{error::TipRouterError, vault_registry::StMintEntry};

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct WeightEntry {
    /// Info about the ST mint
    st_mint_entry: StMintEntry,
    /// The weight of the ST mint
    weight: PodU128,
    /// The slot the weight was set
    slot_set: PodU64,
    /// The slot the weight was last updated
    slot_updated: PodU64,
    /// Reserved space
    reserved: [u8; 128],
}

impl Default for WeightEntry {
    fn default() -> Self {
        Self {
            st_mint_entry: StMintEntry::default(),
            weight: PodU128::default(),
            slot_set: PodU64::default(),
            slot_updated: PodU64::default(),
            reserved: [0; 128],
        }
    }
}

impl WeightEntry {
    pub fn new(mint_entry: &StMintEntry) -> Self {
        Self {
            st_mint_entry: *mint_entry,
            weight: PodU128::from(0),
            slot_set: PodU64::from(0),
            slot_updated: PodU64::from(0),
            reserved: [0; 128],
        }
    }

    // Empty entry, no mint
    pub fn is_empty(&self) -> bool {
        self.st_mint_entry.is_empty()
    }

    pub fn is_set(&self) -> bool {
        self.slot_set() > 0
    }

    pub fn slot_set(&self) -> u64 {
        self.slot_set.into()
    }

    pub fn slot_updated(&self) -> u64 {
        self.slot_updated.into()
    }

    pub const fn st_mint_entry(&self) -> &StMintEntry {
        &self.st_mint_entry
    }

    pub const fn st_mint(&self) -> &Pubkey {
        self.st_mint_entry.st_mint()
    }

    pub fn weight(&self) -> u128 {
        self.weight.into()
    }

    pub fn precise_weight(&self) -> Result<PreciseNumber, TipRouterError> {
        PreciseNumber::new(self.weight.into()).ok_or(TipRouterError::NewPreciseNumberError)
    }

    pub fn set_weight(&mut self, weight: u128, current_slot: u64) {
        self.weight = PodU128::from(weight);

        if self.slot_set() == 0 {
            self.slot_set = PodU64::from(current_slot);
            self.slot_updated = PodU64::from(current_slot);
        }

        self.slot_updated = PodU64::from(current_slot);
    }
}

#[cfg(test)]
mod tests {
    use solana_program::pubkey::Pubkey;

    use super::*;
    use crate::ncn_fee_group::NcnFeeGroup;

    #[test]
    fn test_weight_entry_new() {
        let mint = Pubkey::new_unique();
        let mint_entry =
            StMintEntry::new(&mint, NcnFeeGroup::default(), 0, &Pubkey::new_unique(), 0);
        let weight_entry = WeightEntry::new(&mint_entry);

        assert_eq!(*weight_entry.st_mint(), mint);
        assert_eq!(weight_entry.weight(), 0);
        assert_eq!(weight_entry.slot_set(), 0);
        assert_eq!(weight_entry.slot_updated(), 0);
    }
}
