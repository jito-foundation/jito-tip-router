//! This code was AUTOGENERATED using the codama library.
//! Please DO NOT EDIT THIS FILE, instead use visitors
//! to add features, then rerun codama to update it.
//!
//! <https://github.com/codama-idl/codama>
//!

use crate::generated::types::StakeWeights;
use crate::generated::types::VaultOperatorStakeWeight;
use borsh::BorshDeserialize;
use borsh::BorshSerialize;
use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OperatorSnapshot {
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
    pub slot_finalized: u64,
    pub is_active: bool,
    pub ncn_operator_index: u64,
    pub operator_index: u64,
    pub operator_fee_bps: u16,
    pub vault_operator_delegation_count: u64,
    pub vault_operator_delegations_registered: u64,
    pub valid_operator_vault_delegations: u64,
    pub stake_weights: StakeWeights,
    #[cfg_attr(feature = "serde", serde(with = "serde_big_array::BigArray"))]
    pub reserved: [u8; 256],
    #[cfg_attr(feature = "serde", serde(with = "serde_big_array::BigArray"))]
    pub vault_operator_stake_weight: [VaultOperatorStakeWeight; 64],
}

impl OperatorSnapshot {
    #[inline(always)]
    pub fn from_bytes(data: &[u8]) -> Result<Self, std::io::Error> {
        let mut data = data;
        Self::deserialize(&mut data)
    }
}

impl<'a> TryFrom<&solana_program::account_info::AccountInfo<'a>> for OperatorSnapshot {
    type Error = std::io::Error;

    fn try_from(
        account_info: &solana_program::account_info::AccountInfo<'a>,
    ) -> Result<Self, Self::Error> {
        let mut data: &[u8] = &(*account_info.data).borrow();
        Self::deserialize(&mut data)
    }
}

#[cfg(feature = "anchor")]
impl anchor_lang::AccountDeserialize for OperatorSnapshot {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        Ok(Self::deserialize(buf)?)
    }
}

#[cfg(feature = "anchor")]
impl anchor_lang::AccountSerialize for OperatorSnapshot {}

#[cfg(feature = "anchor")]
impl anchor_lang::Owner for OperatorSnapshot {
    fn owner() -> Pubkey {
        crate::JITO_TIP_ROUTER_ID
    }
}

#[cfg(feature = "anchor-idl-build")]
impl anchor_lang::IdlBuild for OperatorSnapshot {}

#[cfg(feature = "anchor-idl-build")]
impl anchor_lang::Discriminator for OperatorSnapshot {
    const DISCRIMINATOR: [u8; 8] = [0; 8];
}
