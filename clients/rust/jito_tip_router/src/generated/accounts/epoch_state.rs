//! This code was AUTOGENERATED using the kinobi library.
//! Please DO NOT EDIT THIS FILE, instead use visitors
//! to add features, then rerun kinobi to update it.
//!
//! <https://github.com/kinobi-so/kinobi>
//!

use crate::generated::types::EpochAccountStatus;
use crate::generated::types::Progress;
use borsh::BorshDeserialize;
use borsh::BorshSerialize;
use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EpochState {
    pub discriminator: u64,
    #[cfg_attr(
        feature = "serde",
        serde(with = "serde_with::As::<serde_with::DisplayFromStr>")
    )]
    pub ncn: Pubkey,
    pub epoch: u64,
    pub bump: u8,
    pub slot_created: u64,
    pub slot_consensus_reached: u64,
    pub operator_count: u64,
    pub vault_count: u64,
    pub account_status: EpochAccountStatus,
    pub set_weight_progress: Progress,
    pub epoch_snapshot_progress: Progress,
    #[cfg_attr(feature = "serde", serde(with = "serde_big_array::BigArray"))]
    pub operator_snapshot_progress: [Progress; 256],
    pub voting_progress: Progress,
    pub validation_progress: Progress,
    pub upload_progress: Progress,
    pub total_distribution_progress: Progress,
    pub base_distribution_progress: Progress,
    #[cfg_attr(feature = "serde", serde(with = "serde_big_array::BigArray"))]
    pub ncn_distribution_progress: [Progress; 2048],
    #[cfg_attr(feature = "serde", serde(with = "serde_big_array::BigArray"))]
    pub reserved: [u8; 1024],
}

impl EpochState {
    #[inline(always)]
    pub fn from_bytes(data: &[u8]) -> Result<Self, std::io::Error> {
        let mut data = data;
        Self::deserialize(&mut data)
    }
}

impl<'a> TryFrom<&solana_program::account_info::AccountInfo<'a>> for EpochState {
    type Error = std::io::Error;

    fn try_from(
        account_info: &solana_program::account_info::AccountInfo<'a>,
    ) -> Result<Self, Self::Error> {
        let mut data: &[u8] = &(*account_info.data).borrow();
        Self::deserialize(&mut data)
    }
}

#[cfg(feature = "anchor")]
impl anchor_lang::AccountDeserialize for EpochState {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        Ok(Self::deserialize(buf)?)
    }
}

#[cfg(feature = "anchor")]
impl anchor_lang::AccountSerialize for EpochState {}

#[cfg(feature = "anchor")]
impl anchor_lang::Owner for EpochState {
    fn owner() -> Pubkey {
        crate::JITO_TIP_ROUTER_ID
    }
}

#[cfg(feature = "anchor-idl-build")]
impl anchor_lang::IdlBuild for EpochState {}

#[cfg(feature = "anchor-idl-build")]
impl anchor_lang::Discriminator for EpochState {
    const DISCRIMINATOR: [u8; 8] = [0; 8];
}
