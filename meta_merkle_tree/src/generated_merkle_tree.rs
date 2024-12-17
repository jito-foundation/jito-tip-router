// Mostly copied from modules in jito-solana/tip-distributor/src
// To be replaced by tip distributor code in this repo
use std::{fs::File, io::BufReader, path::PathBuf};

use jito_tip_distribution_sdk::{jito_tip_distribution, CLAIM_STATUS_SEED};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use solana_program::{
    clock::{Epoch, Slot},
    hash::{Hash, Hasher},
    pubkey::Pubkey,
};
use thiserror::Error;

use crate::{merkle_tree::MerkleTree, utils::get_proof};

#[derive(Error, Debug)]
pub enum MerkleRootGeneratorError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
}

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct GeneratedMerkleTreeCollection {
    pub generated_merkle_trees: Vec<GeneratedMerkleTree>,
    pub bank_hash: String,
    pub epoch: Epoch,
    pub slot: Slot,
}

#[derive(Clone, Eq, Debug, Hash, PartialEq, Deserialize, Serialize)]
pub struct GeneratedMerkleTree {
    #[serde(with = "pubkey_string_conversion")]
    pub tip_distribution_account: Pubkey,
    #[serde(with = "pubkey_string_conversion")]
    pub merkle_root_upload_authority: Pubkey,
    pub merkle_root: Hash,
    pub tree_nodes: Vec<TreeNode>,
    pub max_total_claim: u64,
    pub max_num_nodes: u64,
}

impl GeneratedMerkleTreeCollection {
    pub fn new_from_stake_meta_collection(
        stake_meta_coll: StakeMetaCollection,
    ) -> Result<Self, MerkleRootGeneratorError> {
        let generated_merkle_trees = stake_meta_coll
            .stake_metas
            .into_iter()
            .filter(|stake_meta| stake_meta.maybe_tip_distribution_meta.is_some())
            .filter_map(|stake_meta| {
                let mut tree_nodes = match TreeNode::vec_from_stake_meta(&stake_meta) {
                    Err(e) => return Some(Err(e)),
                    Ok(maybe_tree_nodes) => maybe_tree_nodes,
                }?;

                // if let Some(rpc_client) = &maybe_rpc_client {
                //     if let Some(tda) = stake_meta.maybe_tip_distribution_meta.as_ref() {
                // emit_inconsistent_tree_node_amount_dp(
                //     &tree_nodes[..],
                //     &tda.tip_distribution_pubkey,
                //     rpc_client,
                // );
                //     }
                // }

                let hashed_nodes: Vec<[u8; 32]> =
                    tree_nodes.iter().map(|n| n.hash().to_bytes()).collect();

                let tip_distribution_meta = stake_meta.maybe_tip_distribution_meta.unwrap();

                let merkle_tree = MerkleTree::new(&hashed_nodes[..], true);
                let max_num_nodes = tree_nodes.len() as u64;

                for (i, tree_node) in tree_nodes.iter_mut().enumerate() {
                    tree_node.proof = Some(get_proof(&merkle_tree, i));
                }

                Some(Ok(GeneratedMerkleTree {
                    max_num_nodes,
                    tip_distribution_account: tip_distribution_meta.tip_distribution_pubkey,
                    merkle_root_upload_authority: tip_distribution_meta
                        .merkle_root_upload_authority,
                    merkle_root: *merkle_tree.get_root().unwrap(),
                    tree_nodes,
                    max_total_claim: tip_distribution_meta.total_tips,
                }))
            })
            .collect::<Result<Vec<GeneratedMerkleTree>, MerkleRootGeneratorError>>()?;

        Ok(Self {
            generated_merkle_trees,
            bank_hash: stake_meta_coll.bank_hash,
            epoch: stake_meta_coll.epoch,
            slot: stake_meta_coll.slot,
        })
    }
}

#[derive(Clone, Eq, Debug, Hash, PartialEq, Deserialize, Serialize)]
pub struct TreeNode {
    /// The stake account entitled to redeem.
    #[serde(with = "pubkey_string_conversion")]
    pub claimant: Pubkey,

    /// Pubkey of the ClaimStatus PDA account, this account should be closed to reclaim rent.
    #[serde(with = "pubkey_string_conversion")]
    pub claim_status_pubkey: Pubkey,

    /// Bump of the ClaimStatus PDA account
    pub claim_status_bump: u8,

    #[serde(with = "pubkey_string_conversion")]
    pub staker_pubkey: Pubkey,

    #[serde(with = "pubkey_string_conversion")]
    pub withdrawer_pubkey: Pubkey,

    /// The amount this account is entitled to.
    pub amount: u64,

    /// The proof associated with this TreeNode
    pub proof: Option<Vec<[u8; 32]>>,
}
impl TreeNode {
    fn vec_from_stake_meta(
        stake_meta: &StakeMeta,
    ) -> Result<Option<Vec<Self>>, MerkleRootGeneratorError> {
        if let Some(tip_distribution_meta) = stake_meta.maybe_tip_distribution_meta.as_ref() {
            let validator_amount = (tip_distribution_meta.total_tips as u128)
                .checked_mul(tip_distribution_meta.validator_fee_bps as u128)
                .unwrap()
                .checked_div(10_000)
                .unwrap() as u64;
            let (claim_status_pubkey, claim_status_bump) = Pubkey::find_program_address(
                &[
                    CLAIM_STATUS_SEED,
                    &stake_meta.validator_vote_account.to_bytes(),
                    &tip_distribution_meta.tip_distribution_pubkey.to_bytes(),
                ],
                &jito_tip_distribution::ID,
            );
            let mut tree_nodes = vec![Self {
                claimant: stake_meta.validator_vote_account,
                claim_status_pubkey,
                claim_status_bump,
                staker_pubkey: Pubkey::default(),
                withdrawer_pubkey: Pubkey::default(),
                amount: validator_amount,
                proof: None,
            }];

            let remaining_total_rewards = tip_distribution_meta
                .total_tips
                .checked_sub(validator_amount)
                .unwrap() as u128;

            let total_delegated = stake_meta.total_delegated as u128;
            tree_nodes.extend(
                stake_meta
                    .delegations
                    .iter()
                    .map(|delegation| {
                        let amount_delegated = delegation.lamports_delegated as u128;
                        let reward_amount = (amount_delegated.checked_mul(remaining_total_rewards))
                            .unwrap()
                            .checked_div(total_delegated)
                            .unwrap();
                        let (claim_status_pubkey, claim_status_bump) = Pubkey::find_program_address(
                            &[
                                CLAIM_STATUS_SEED,
                                &delegation.stake_account_pubkey.to_bytes(),
                                &tip_distribution_meta.tip_distribution_pubkey.to_bytes(),
                            ],
                            &jito_tip_distribution::ID,
                        );
                        Ok(Self {
                            claimant: delegation.stake_account_pubkey,
                            claim_status_pubkey,
                            claim_status_bump,
                            staker_pubkey: delegation.staker_pubkey,
                            withdrawer_pubkey: delegation.withdrawer_pubkey,
                            amount: reward_amount as u64,
                            proof: None,
                        })
                    })
                    .collect::<Result<Vec<Self>, MerkleRootGeneratorError>>()?,
            );

            Ok(Some(tree_nodes))
        } else {
            Ok(None)
        }
    }

    fn hash(&self) -> Hash {
        let mut hasher = Hasher::default();
        hasher.hash(self.claimant.as_ref());
        hasher.hash(self.amount.to_le_bytes().as_ref());
        hasher.result()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StakeMetaCollection {
    /// List of [StakeMeta].
    pub stake_metas: Vec<StakeMeta>,

    /// base58 encoded tip-distribution program id.
    #[serde(with = "pubkey_string_conversion")]
    pub tip_distribution_program_id: Pubkey,

    /// Base58 encoded bank hash this object was generated at.
    pub bank_hash: String,

    /// Epoch for which this object was generated for.
    pub epoch: Epoch,

    /// Slot at which this object was generated.
    pub slot: Slot,
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct StakeMeta {
    #[serde(with = "pubkey_string_conversion")]
    pub validator_vote_account: Pubkey,

    #[serde(with = "pubkey_string_conversion")]
    pub validator_node_pubkey: Pubkey,

    /// The validator's tip-distribution meta if it exists.
    pub maybe_tip_distribution_meta: Option<TipDistributionMeta>,

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

mod pubkey_string_conversion {
    use std::str::FromStr;

    use serde::{self, Deserialize, Deserializer, Serializer};
    use solana_program::pubkey::Pubkey;

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

pub fn read_json_from_file<T>(path: &PathBuf) -> serde_json::Result<T>
where
    T: DeserializeOwned,
{
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    serde_json::from_reader(reader)
}
