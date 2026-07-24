//! Lightweight, dependency-minimal output types produced by stake-meta generation.
//!
//! These are plain serde structs deliberately kept free of the heavy
//! merkle-tree / vault / restaking dependency graph so they can be consumed
//! from other repositories (e.g. the jito-solana snapshot service) via git
//! without dragging in `meta-merkle-tree` and its transitive deps.
//!
//! `meta-merkle-tree` re-exports everything here as the single source of truth.

use std::{
    fs::File,
    io::{BufReader, Write},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};
use solana_clock::{Epoch, Slot};
use solana_pubkey::Pubkey;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StakeMetaCollection {
    /// List of [StakeMeta].
    pub stake_metas: Vec<StakeMeta>,

    /// base58 encoded tip-distribution program id.
    #[serde(with = "pubkey_string_conversion")]
    pub tip_distribution_program_id: Pubkey,

    /// base58 encoded priority-fee-distribution program id.
    #[serde(with = "pubkey_string_conversion")]
    pub priority_fee_distribution_program_id: Pubkey,

    /// Base58 encoded bank hash this object was generated at.
    pub bank_hash: String,

    /// Epoch for which this object was generated for.
    pub epoch: Epoch,

    /// Slot at which this object was generated.
    pub slot: Slot,
}

impl StakeMetaCollection {
    /// Load a serialized [StakeMetaCollection] from a JSON file.
    pub fn new_from_file(path: &PathBuf) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let tree: Self = serde_json::from_reader(reader).map_err(std::io::Error::other)?;

        Ok(tree)
    }

    /// Write a [StakeMetaCollection] to a JSON file.
    pub fn write_to_file(&self, path: &PathBuf) {
        let serialized = serde_json::to_string_pretty(&self).unwrap();
        let mut file = File::create(path).unwrap();
        file.write_all(serialized.as_bytes()).unwrap();
    }
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct StakeMeta {
    #[serde(with = "pubkey_string_conversion")]
    pub validator_vote_account: Pubkey,

    #[serde(with = "pubkey_string_conversion")]
    pub validator_node_pubkey: Pubkey,

    /// The validator's tip-distribution meta if it exists.
    pub maybe_tip_distribution_meta: Option<TipDistributionMeta>,

    /// The validator's priority-fee-distribution meta if it exists.
    pub maybe_priority_fee_distribution_meta: Option<PriorityFeeDistributionMeta>,

    /// Delegations to this validator.
    pub delegations: Vec<Delegation>,

    /// The total amount of delegations to the validator.
    pub total_delegated: u64,

    /// The validator's delegation commission rate as a percentage between 0-100.
    pub commission: u8,
}

impl Ord for StakeMeta {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.validator_vote_account
            .cmp(&other.validator_vote_account)
    }
}

impl PartialOrd<Self> for StakeMeta {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct TipDistributionMeta {
    #[serde(with = "pubkey_string_conversion")]
    pub merkle_root_upload_authority: Pubkey,

    #[serde(with = "pubkey_string_conversion")]
    pub tip_distribution_pubkey: Pubkey,

    /// The validator's total tips in the [TipDistributionAccount].
    pub total_tips: u64,

    /// The validator's cut of tips from [TipDistributionAccount], calculated from the on-chain
    /// commission fee bps.
    pub validator_fee_bps: u16,
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct PriorityFeeDistributionMeta {
    #[serde(with = "pubkey_string_conversion")]
    pub merkle_root_upload_authority: Pubkey,

    #[serde(with = "pubkey_string_conversion")]
    pub priority_fee_distribution_pubkey: Pubkey,

    /// The validator's total tips in the [TipDistributionAccount].
    pub total_tips: u64,

    /// The validator's cut of tips from [TipDistributionAccount], calculated from the on-chain
    /// commission fee bps.
    pub validator_fee_bps: u16,
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct Delegation {
    #[serde(with = "pubkey_string_conversion")]
    pub stake_account_pubkey: Pubkey,

    #[serde(with = "pubkey_string_conversion")]
    pub staker_pubkey: Pubkey,

    #[serde(with = "pubkey_string_conversion")]
    pub withdrawer_pubkey: Pubkey,

    /// Lamports delegated by the stake account
    pub lamports_delegated: u64,
}

impl Ord for Delegation {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (
            self.stake_account_pubkey,
            self.withdrawer_pubkey,
            self.staker_pubkey,
            self.lamports_delegated,
        )
            .cmp(&(
                other.stake_account_pubkey,
                other.withdrawer_pubkey,
                other.staker_pubkey,
                other.lamports_delegated,
            ))
    }
}

impl PartialOrd<Self> for Delegation {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub mod pubkey_string_conversion {
    use std::str::FromStr;

    use serde::{self, Deserialize, Deserializer, Serializer};
    use solana_pubkey::Pubkey;

    pub fn serialize<S>(pubkey: &Pubkey, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&pubkey.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Pubkey, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Pubkey::from_str(&s).map_err(serde::de::Error::custom)
    }
}
