//! This code was AUTOGENERATED using the kinobi library.
//! Please DO NOT EDIT THIS FILE, instead use visitors
//! to add features, then rerun kinobi to update it.
//!
//! <https://github.com/kinobi-so/kinobi>

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

use crate::generated::types::RewardRoutes;

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OperatorEpochRewardRouter {
    pub discriminator: u64,
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
    pub ncn_epoch: u64,
    pub bump: u8,
    pub slot_created: u64,
    pub reward_pool: u64,
    pub operator_rewards: u64,
    #[cfg_attr(feature = "serde", serde(with = "serde_with::As::<serde_with::Bytes>"))]
    pub reserved: [u8; 128],
    pub vault_rewards: [RewardRoutes; 32],
}

impl OperatorEpochRewardRouter {
    #[inline(always)]
    pub fn from_bytes(data: &[u8]) -> Result<Self, std::io::Error> {
        let mut data = data;
        Self::deserialize(&mut data)
    }
}

impl<'a> TryFrom<&solana_program::account_info::AccountInfo<'a>> for OperatorEpochRewardRouter {
    type Error = std::io::Error;

    fn try_from(
        account_info: &solana_program::account_info::AccountInfo<'a>,
    ) -> Result<Self, Self::Error> {
        let mut data: &[u8] = &(*account_info.data).borrow();
        Self::deserialize(&mut data)
    }
}

#[cfg(feature = "anchor")]
impl anchor_lang::AccountDeserialize for OperatorEpochRewardRouter {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        Ok(Self::deserialize(buf)?)
    }
}

#[cfg(feature = "anchor")]
impl anchor_lang::AccountSerialize for OperatorEpochRewardRouter {}

#[cfg(feature = "anchor")]
impl anchor_lang::Owner for OperatorEpochRewardRouter {
    fn owner() -> Pubkey {
        crate::JITO_TIP_ROUTER_ID
    }
}

#[cfg(feature = "anchor-idl-build")]
impl anchor_lang::IdlBuild for OperatorEpochRewardRouter {}

#[cfg(feature = "anchor-idl-build")]
impl anchor_lang::Discriminator for OperatorEpochRewardRouter {
    const DISCRIMINATOR: [u8; 8] = [0; 8];
}
