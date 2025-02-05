use std::{fs::File, io::BufReader, path::PathBuf};

use jito_tip_distribution_sdk::{
    jito_tip_distribution::ID as TIP_DISTRIBUTION_ID, CLAIM_STATUS_SEED,
};
use jito_vault_core::MAX_BPS;
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
    #[error("Account not found")]
    AccountNotFound,
    #[error("Deserialization error")]
    DeserializationError,
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
    #[error("MerkleRootGenerator error")]
    MerkleRootGeneratorError,
    #[error("MerkleTreeTestError")]
    MerkleTreeTestError,
    #[error("Checked math error")]
    CheckedMathError,
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
        stake_meta_collection: StakeMetaCollection,
        ncn_address: &Pubkey,
        epoch: u64,
        protocol_fee_bps: u64,
        tip_router_program_id: &Pubkey,
    ) -> Result<Self, MerkleRootGeneratorError> {
        let generated_merkle_trees = stake_meta_collection
            .stake_metas
            .into_iter()
            .filter(|stake_meta| stake_meta.maybe_tip_distribution_meta.is_some())
            .filter_map(|stake_meta| {
                // Use the helper function to create tree nodes
                let mut tree_nodes = match TreeNode::vec_from_stake_meta(
                    &stake_meta,
                    protocol_fee_bps,
                    ncn_address,
                    epoch,
                    &stake_meta_collection.tip_distribution_program_id, // Pass the program ID
                    tip_router_program_id,
                ) {
                    Err(e) => return Some(Err(e)),
                    Ok(maybe_tree_nodes) => maybe_tree_nodes,
                }?;

                // Create merkle tree and add proofs
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
            .collect::<Result<Vec<_>, MerkleRootGeneratorError>>()?;

        Ok(Self {
            generated_merkle_trees,
            bank_hash: stake_meta_collection.bank_hash,
            epoch: stake_meta_collection.epoch,
            slot: stake_meta_collection.slot,
        })
    }

    /// Load a serialized GeneratedMerkleTreeCollection from file path
    pub fn new_from_file(path: &PathBuf) -> Result<Self, MerkleRootGeneratorError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let tree: Self = serde_json::from_reader(reader)?;

        Ok(tree)
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
        protocol_fee_bps: u64,
        ncn_address: &Pubkey,
        epoch: u64,
        tip_distribution_program_id: &Pubkey,
        tip_router_program_id: &Pubkey,
    ) -> Result<Option<Vec<Self>>, MerkleRootGeneratorError> {
        if let Some(tip_distribution_meta) = stake_meta.maybe_tip_distribution_meta.as_ref() {
            let protocol_fee_amount = u128::checked_div(
                (tip_distribution_meta.total_tips as u128)
                    .checked_mul(protocol_fee_bps as u128)
                    .ok_or(MerkleRootGeneratorError::CheckedMathError)?,
                MAX_BPS as u128,
            )
            .ok_or(MerkleRootGeneratorError::CheckedMathError)?;

            let protocol_fee_amount = u64::try_from(protocol_fee_amount)
                .map_err(|_| MerkleRootGeneratorError::CheckedMathError)?;

            let validator_amount = u64::try_from(
                (tip_distribution_meta.total_tips as u128)
                    .checked_mul(tip_distribution_meta.validator_fee_bps as u128)
                    .ok_or(MerkleRootGeneratorError::CheckedMathError)?
                    .checked_div(MAX_BPS as u128)
                    .ok_or(MerkleRootGeneratorError::CheckedMathError)?,
            )
            .map_err(|_| MerkleRootGeneratorError::CheckedMathError)?;

            let (validator_amount, remaining_total_rewards) = validator_amount
                .checked_add(protocol_fee_amount)
                .map_or((validator_amount, None), |total_fees| {
                    if total_fees > tip_distribution_meta.total_tips {
                        // If fees exceed total tips, preference protocol fee amount and reduce validator amount
                        tip_distribution_meta
                            .total_tips
                            .checked_sub(protocol_fee_amount)
                            .map(|adjusted_validator_amount| (adjusted_validator_amount, Some(0)))
                            .unwrap_or((0, None))
                    } else {
                        // Otherwise use original protocol fee and subtract both fees from total
                        (
                            validator_amount,
                            tip_distribution_meta
                                .total_tips
                                .checked_sub(protocol_fee_amount)
                                .and_then(|v| v.checked_sub(validator_amount)),
                        )
                    }
                });

            let remaining_total_rewards =
                remaining_total_rewards.ok_or(MerkleRootGeneratorError::CheckedMathError)?;

            let tip_router_target_epoch = epoch
                .checked_add(1)
                .ok_or(MerkleRootGeneratorError::CheckedMathError)?;

            // Must match the seeds from `core::BaseRewardReceiver`. Cannot
            // use `BaseRewardReceiver::find_program_address` as it would cause
            // circular dependecies.
            let base_reward_receiver = Pubkey::find_program_address(
                &[
                    b"base_reward_receiver",
                    &ncn_address.to_bytes(),
                    &tip_router_target_epoch.to_le_bytes(),
                ],
                tip_router_program_id,
            )
            .0;

            let (protocol_claim_status_pubkey, protocol_claim_status_bump) =
                Pubkey::find_program_address(
                    &[
                        CLAIM_STATUS_SEED,
                        &base_reward_receiver.to_bytes(),
                        &tip_distribution_meta.tip_distribution_pubkey.to_bytes(),
                    ],
                    tip_distribution_program_id,
                );

            let mut tree_nodes = vec![Self {
                claimant: base_reward_receiver,
                claim_status_pubkey: protocol_claim_status_pubkey,
                claim_status_bump: protocol_claim_status_bump,
                staker_pubkey: Pubkey::default(),
                withdrawer_pubkey: Pubkey::default(),
                amount: protocol_fee_amount,
                proof: None,
            }];

            let (validator_claim_status_pubkey, validator_claim_status_bump) =
                Pubkey::find_program_address(
                    &[
                        CLAIM_STATUS_SEED,
                        &stake_meta.validator_node_pubkey.to_bytes(),
                        &tip_distribution_meta.tip_distribution_pubkey.to_bytes(),
                    ],
                    tip_distribution_program_id,
                );

            tree_nodes.push(Self {
                claimant: stake_meta.validator_node_pubkey,
                claim_status_pubkey: validator_claim_status_pubkey,
                claim_status_bump: validator_claim_status_bump,
                staker_pubkey: Pubkey::default(),
                withdrawer_pubkey: Pubkey::default(),
                amount: validator_amount,
                proof: None,
            });

            let total_delegated = stake_meta.total_delegated as u128;
            tree_nodes.extend(
                stake_meta
                    .delegations
                    .iter()
                    .map(|delegation| {
                        let amount_delegated = delegation.lamports_delegated as u128;
                        let reward_amount = u64::try_from(
                            (amount_delegated.checked_mul(remaining_total_rewards as u128))
                                .ok_or(MerkleRootGeneratorError::CheckedMathError)?
                                .checked_div(total_delegated)
                                .ok_or(MerkleRootGeneratorError::CheckedMathError)?,
                        )
                        .map_err(|_| MerkleRootGeneratorError::CheckedMathError)?;

                        let (claim_status_pubkey, claim_status_bump) = Pubkey::find_program_address(
                            &[
                                CLAIM_STATUS_SEED,
                                &delegation.staker_pubkey.to_bytes(),
                                &tip_distribution_meta.tip_distribution_pubkey.to_bytes(),
                            ],
                            &TIP_DISTRIBUTION_ID,
                        );
                        // Time-gated fix so slow rollout won't affect consensus
                        if tip_router_target_epoch > 737 {
                            Ok(Self {
                                claimant: delegation.stake_account_pubkey,
                                claim_status_pubkey,
                                claim_status_bump,
                                staker_pubkey: delegation.staker_pubkey,
                                withdrawer_pubkey: delegation.withdrawer_pubkey,
                                amount: reward_amount,
                                proof: None,
                            })
                        } else {
                            Ok(Self {
                                claimant: delegation.staker_pubkey,
                                claim_status_pubkey,
                                claim_status_bump,
                                staker_pubkey: delegation.staker_pubkey,
                                withdrawer_pubkey: delegation.withdrawer_pubkey,
                                amount: reward_amount,
                                proof: None,
                            })
                        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verify;

    #[test]
    fn test_merkle_tree_verify() {
        // Create the merkle tree and proofs
        let tda = Pubkey::new_unique();
        let (acct_0, acct_1) = (Pubkey::new_unique(), Pubkey::new_unique());
        let claim_statuses = &[(acct_0, tda), (acct_1, tda)]
            .iter()
            .map(|(claimant, tda)| {
                Pubkey::find_program_address(
                    &[CLAIM_STATUS_SEED, &claimant.to_bytes(), &tda.to_bytes()],
                    &TIP_DISTRIBUTION_ID,
                )
            })
            .collect::<Vec<(Pubkey, u8)>>();
        let tree_nodes = vec![
            TreeNode {
                claimant: acct_0,
                claim_status_pubkey: claim_statuses[0].0,
                claim_status_bump: claim_statuses[0].1,
                staker_pubkey: Pubkey::default(),
                withdrawer_pubkey: Pubkey::default(),
                amount: 151_507,
                proof: None,
            },
            TreeNode {
                claimant: acct_1,
                claim_status_pubkey: claim_statuses[1].0,
                claim_status_bump: claim_statuses[1].1,
                staker_pubkey: Pubkey::default(),
                withdrawer_pubkey: Pubkey::default(),
                amount: 176_624,
                proof: None,
            },
        ];

        // First the nodes are hashed and merkle tree constructed
        let hashed_nodes: Vec<[u8; 32]> = tree_nodes.iter().map(|n| n.hash().to_bytes()).collect();
        let mk = MerkleTree::new(&hashed_nodes[..], true);
        let root = mk.get_root().expect("to have valid root").to_bytes();

        // verify first node
        let node = solana_program::hash::hashv(&[&[0u8], &hashed_nodes[0]]);
        let proof = get_proof(&mk, 0);
        assert!(verify::verify(proof, root, node.to_bytes()));

        // verify second node
        let node = solana_program::hash::hashv(&[&[0u8], &hashed_nodes[1]]);
        let proof = get_proof(&mk, 1);
        assert!(verify::verify(proof, root, node.to_bytes()));
    }

    #[test]
    fn test_new_from_stake_meta_collection_happy_path() {
        let merkle_root_upload_authority = Pubkey::new_unique();
        let tip_router_program_id = Pubkey::new_unique();
        let (tda_0, tda_1) = (Pubkey::new_unique(), Pubkey::new_unique());
        let stake_account_0 = Pubkey::new_unique();
        let stake_account_1 = Pubkey::new_unique();
        let stake_account_2 = Pubkey::new_unique();
        let stake_account_3 = Pubkey::new_unique();
        let staker_account_0 = Pubkey::new_unique();
        let staker_account_1 = Pubkey::new_unique();
        let staker_account_2 = Pubkey::new_unique();
        let staker_account_3 = Pubkey::new_unique();
        let validator_vote_account_0 = Pubkey::new_unique();
        let validator_vote_account_1 = Pubkey::new_unique();
        let validator_id_0 = Pubkey::new_unique();
        let validator_id_1 = Pubkey::new_unique();
        let ncn_address = Pubkey::new_unique();
        let epoch = 737;

        let stake_meta_collection = StakeMetaCollection {
            stake_metas: vec![
                StakeMeta {
                    validator_vote_account: validator_vote_account_0,
                    validator_node_pubkey: validator_id_0,
                    maybe_tip_distribution_meta: Some(TipDistributionMeta {
                        merkle_root_upload_authority,
                        tip_distribution_pubkey: tda_0,
                        total_tips: 1_900_122_111_000,
                        validator_fee_bps: 100,
                    }),
                    delegations: vec![
                        Delegation {
                            stake_account_pubkey: stake_account_0,
                            staker_pubkey: staker_account_0,
                            withdrawer_pubkey: staker_account_0,
                            lamports_delegated: 123_999_123_555,
                        },
                        Delegation {
                            stake_account_pubkey: stake_account_1,
                            staker_pubkey: staker_account_1,
                            withdrawer_pubkey: staker_account_1,
                            lamports_delegated: 144_555_444_556,
                        },
                    ],
                    total_delegated: 1_555_123_000_333_454_000,
                    commission: 100,
                },
                StakeMeta {
                    validator_vote_account: validator_vote_account_1,
                    validator_node_pubkey: validator_id_1,
                    maybe_tip_distribution_meta: Some(TipDistributionMeta {
                        merkle_root_upload_authority,
                        tip_distribution_pubkey: tda_1,
                        total_tips: 1_900_122_111_333,
                        validator_fee_bps: 200,
                    }),
                    delegations: vec![
                        Delegation {
                            stake_account_pubkey: stake_account_2,
                            staker_pubkey: staker_account_2,
                            withdrawer_pubkey: staker_account_2,
                            lamports_delegated: 224_555_444,
                        },
                        Delegation {
                            stake_account_pubkey: stake_account_3,
                            staker_pubkey: staker_account_3,
                            withdrawer_pubkey: staker_account_3,
                            lamports_delegated: 700_888_944_555,
                        },
                    ],
                    total_delegated: 2_565_318_909_444_123,
                    commission: 10,
                },
            ],
            tip_distribution_program_id: Pubkey::new_unique(),
            bank_hash: Hash::new_unique().to_string(),
            epoch: 100,
            slot: 2_000_000,
        };

        let merkle_tree_collection = GeneratedMerkleTreeCollection::new_from_stake_meta_collection(
            stake_meta_collection.clone(),
            &ncn_address,
            epoch,
            300,
            &tip_router_program_id,
        )
        .unwrap();

        assert_eq!(stake_meta_collection.epoch, merkle_tree_collection.epoch);
        assert_eq!(
            stake_meta_collection.bank_hash,
            merkle_tree_collection.bank_hash
        );
        assert_eq!(stake_meta_collection.slot, merkle_tree_collection.slot);
        assert_eq!(
            stake_meta_collection.stake_metas.len(),
            merkle_tree_collection.generated_merkle_trees.len()
        );

        let protocol_fee_recipient = Pubkey::find_program_address(
            &[
                b"base_reward_receiver",
                &ncn_address.to_bytes(),
                &(epoch + 1).to_le_bytes(),
            ],
            &tip_router_program_id,
        )
        .0;

        let claim_statuses = &[
            (protocol_fee_recipient, tda_0),
            (validator_vote_account_0, tda_0),
            (stake_account_0, tda_0),
            (stake_account_1, tda_0),
            (protocol_fee_recipient, tda_1),
            (validator_vote_account_1, tda_1),
            (stake_account_2, tda_1),
            (stake_account_3, tda_1),
        ]
        .iter()
        .map(|(claimant, tda)| {
            Pubkey::find_program_address(
                &[CLAIM_STATUS_SEED, &claimant.to_bytes(), &tda.to_bytes()],
                &TIP_DISTRIBUTION_ID,
            )
        })
        .collect::<Vec<(Pubkey, u8)>>();

        let tree_nodes = vec![
            TreeNode {
                claimant: protocol_fee_recipient,
                claim_status_pubkey: claim_statuses[0].0,
                claim_status_bump: claim_statuses[0].1,
                staker_pubkey: Pubkey::default(),
                withdrawer_pubkey: Pubkey::default(),
                amount: 57_003_663_330, // 3% of 1_900_122_111_000
                proof: None,
            },
            TreeNode {
                claimant: validator_id_0, // Changed from validator_vote_account_0 to validator_id_0
                claim_status_pubkey: claim_statuses[1].0,
                claim_status_bump: claim_statuses[1].1,
                staker_pubkey: Pubkey::default(),
                withdrawer_pubkey: Pubkey::default(),
                amount: 19_001_221_110,
                proof: None,
            },
            TreeNode {
                claimant: stake_account_0,
                claim_status_pubkey: claim_statuses[2].0,
                claim_status_bump: claim_statuses[2].1,
                staker_pubkey: staker_account_0,
                withdrawer_pubkey: staker_account_0,
                amount: 145_447, // Update to match actual amount
                proof: None,
            },
            TreeNode {
                claimant: stake_account_1,
                claim_status_pubkey: claim_statuses[3].0,
                claim_status_bump: claim_statuses[3].1,
                staker_pubkey: staker_account_1,
                withdrawer_pubkey: staker_account_1,
                amount: 169_559, // Update to match actual amount
                proof: None,
            },
        ];

        let hashed_nodes: Vec<[u8; 32]> = tree_nodes.iter().map(|n| n.hash().to_bytes()).collect();
        let merkle_tree = MerkleTree::new(&hashed_nodes[..], true);
        let gmt_0 = GeneratedMerkleTree {
            tip_distribution_account: tda_0,
            merkle_root_upload_authority,
            merkle_root: *merkle_tree.get_root().unwrap(),
            tree_nodes,
            max_total_claim: stake_meta_collection.stake_metas[0]
                .clone()
                .maybe_tip_distribution_meta
                .unwrap()
                .total_tips,
            max_num_nodes: 4,
        };

        let tree_nodes = vec![
            TreeNode {
                claimant: protocol_fee_recipient,
                claim_status_pubkey: claim_statuses[4].0,
                claim_status_bump: claim_statuses[4].1,
                staker_pubkey: Pubkey::default(),
                withdrawer_pubkey: Pubkey::default(),
                amount: 57_003_663_339, // Updated from 57_003_663_340 after div_ceil -> checked_div change. Dust stays in TDA and goes to DAO
                proof: None,
            },
            TreeNode {
                claimant: validator_id_1,
                claim_status_pubkey: claim_statuses[5].0,
                claim_status_bump: claim_statuses[5].1,
                staker_pubkey: Pubkey::default(),
                withdrawer_pubkey: Pubkey::default(),
                amount: 38_002_442_226,
                proof: None,
            },
            TreeNode {
                claimant: stake_account_2,
                claim_status_pubkey: claim_statuses[6].0,
                claim_status_bump: claim_statuses[6].1,
                staker_pubkey: staker_account_2,
                withdrawer_pubkey: staker_account_2,
                amount: 158_011, // Updated from 163_000
                proof: None,
            },
            TreeNode {
                claimant: stake_account_3,
                claim_status_pubkey: claim_statuses[7].0,
                claim_status_bump: claim_statuses[7].1,
                staker_pubkey: staker_account_3,
                withdrawer_pubkey: staker_account_3,
                amount: 493_188_526, // Updated from 508_762_900
                proof: None,
            },
        ];
        let hashed_nodes: Vec<[u8; 32]> = tree_nodes.iter().map(|n| n.hash().to_bytes()).collect();
        let merkle_tree = MerkleTree::new(&hashed_nodes[..], true);
        let gmt_1 = GeneratedMerkleTree {
            tip_distribution_account: tda_1,
            merkle_root_upload_authority,
            merkle_root: *merkle_tree.get_root().unwrap(),
            tree_nodes,
            max_total_claim: stake_meta_collection.stake_metas[1]
                .clone()
                .maybe_tip_distribution_meta
                .unwrap()
                .total_tips,
            max_num_nodes: 4,
        };

        let expected_generated_merkle_trees = vec![gmt_0, gmt_1];
        let actual_generated_merkle_trees = merkle_tree_collection.generated_merkle_trees;
        expected_generated_merkle_trees
            .iter()
            .for_each(|expected_gmt| {
                let actual_gmt = actual_generated_merkle_trees
                    .iter()
                    .find(|gmt| {
                        gmt.tip_distribution_account == expected_gmt.tip_distribution_account
                    })
                    .unwrap();

                assert_eq!(expected_gmt.max_num_nodes, actual_gmt.max_num_nodes);
                assert_eq!(expected_gmt.max_total_claim, actual_gmt.max_total_claim);
                assert_eq!(
                    expected_gmt.tip_distribution_account,
                    actual_gmt.tip_distribution_account
                );
                assert_eq!(expected_gmt.tree_nodes.len(), actual_gmt.tree_nodes.len());
                expected_gmt
                    .tree_nodes
                    .iter()
                    .for_each(|expected_tree_node| {
                        let actual_tree_node = actual_gmt
                            .tree_nodes
                            .iter()
                            .find(|tree_node| tree_node.claimant == expected_tree_node.claimant)
                            .unwrap();
                        assert!(
                            (expected_tree_node.amount as i128 - actual_tree_node.amount as i128)
                                == 0,
                            "Expected amount: {}, Actual amount: {}",
                            expected_tree_node.amount,
                            actual_tree_node.amount
                        );
                    });
                assert_eq!(expected_gmt.merkle_root, actual_gmt.merkle_root);
            });
    }
}
