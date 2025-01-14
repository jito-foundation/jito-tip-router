//! This code was AUTOGENERATED using the kinobi library.
//! Please DO NOT EDIT THIS FILE, instead use visitors
//! to add features, then rerun kinobi to update it.
//!
//! <https://github.com/kinobi-so/kinobi>
//!

use borsh::BorshDeserialize;
use borsh::BorshSerialize;

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EpochAccountStatus {
    pub epoch_state: u8,
    pub weight_table: u8,
    pub epoch_snapshot: u8,
    #[cfg_attr(feature = "serde", serde(with = "serde_big_array::BigArray"))]
    pub operator_snapshot: [u8; 256],
    pub ballot_box: u8,
    pub base_reward_router: u8,
    #[cfg_attr(feature = "serde", serde(with = "serde_big_array::BigArray"))]
    pub ncn_reward_router: [u8; 2048],
}
