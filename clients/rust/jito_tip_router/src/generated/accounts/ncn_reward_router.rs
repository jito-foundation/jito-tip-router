//! This code was AUTOGENERATED using the kinobi library.
//! Please DO NOT EDIT THIS FILE, instead use visitors
//! to add features, then rerun kinobi to update it.
//!
//! <https://github.com/kinobi-so/kinobi>

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

use crate::generated::types::{NcnFeeGroup, VaultRewardRoute};

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NcnRewardRouter {
    pub discriminator: u64,
    pub ncn_fee_group: NcnFeeGroup,
    #[cfg_attr(
        feature = "serde",
        serde(with = "serde_with::As::<serde_with::DisplayFromStr>")
    )]
    pub operator: Pubkey,
    #[cfg_attr(
        feature = "serde",
        serde(with = "serde_with::As::<serde_with::DisplayFromStr>")
    )]
    pub ncn: Pubkey,
    pub epoch: u64,
    pub bump: u8,
    pub slot_created: u64,
    pub total_rewards: u64,
    pub reward_pool: u64,
    pub rewards_processed: u64,
    pub operator_rewards: u64,
    #[cfg_attr(feature = "serde", serde(with = "serde_big_array::BigArray"))]
    pub reserved: [u8; 128],
    pub last_rewards_to_process: u64,
    pub last_vault_operator_delegation_index: u16,
    #[cfg_attr(feature = "serde", serde(with = "serde_big_array::BigArray"))]
    pub vault_reward_routes: [VaultRewardRoute; 64],
}

impl NcnRewardRouter {
    #[inline(always)]
    pub fn from_bytes(data: &[u8]) -> Result<Self, std::io::Error> {
        let mut data = data;
        Self::deserialize(&mut data)
    }
}

impl<'a> TryFrom<&solana_program::account_info::AccountInfo<'a>> for NcnRewardRouter {
    type Error = std::io::Error;

    fn try_from(
        account_info: &solana_program::account_info::AccountInfo<'a>,
    ) -> Result<Self, Self::Error> {
        let mut data: &[u8] = &(*account_info.data).borrow();
        Self::deserialize(&mut data)
    }
}

#[cfg(feature = "anchor")]
impl anchor_lang::AccountDeserialize for NcnRewardRouter {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        Ok(Self::deserialize(buf)?)
    }
}

#[cfg(feature = "anchor")]
impl anchor_lang::AccountSerialize for NcnRewardRouter {}

#[cfg(feature = "anchor")]
impl anchor_lang::Owner for NcnRewardRouter {
    fn owner() -> Pubkey {
        crate::JITO_TIP_ROUTER_ID
    }
}

#[cfg(feature = "anchor-idl-build")]
impl anchor_lang::IdlBuild for NcnRewardRouter {}

#[cfg(feature = "anchor-idl-build")]
impl anchor_lang::Discriminator for NcnRewardRouter {
    const DISCRIMINATOR: [u8; 8] = [0; 8];
}
