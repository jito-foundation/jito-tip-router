use jito_priority_fee_distribution_sdk::jito_priority_fee_distribution::ID as PRIORITY_FEE_DISTRIBUTION_ID;
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
use std::{
    fs::File,
    io::{BufReader, Write},
    path::PathBuf,
};
use thiserror::Error;

use crate::{merkle_tree::MerkleTree, utils::get_proof};

pub const VOTE_ACCOUNT_CLAIM_CALCULATION_UPDATE_EPOCH: u64 = 815;

pub fn mul_div(a: u64, b: u64, q: u64) -> Result<u64, MerkleRootGeneratorError> {
    (a as u128)
        .checked_mul(b as u128)
        .and_then(|x| x.checked_div(q as u128))
        .and_then(|x| u64::try_from(x).ok())
        .ok_or(MerkleRootGeneratorError::CheckedMathError)
}

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
    #[error("Distribution program not known")]
    UnknownDistributionProgram,
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
    /// The distribution program this node came from (E.g. Tip Distributor OR Priority Fee
    /// Distributor)
    #[serde(with = "pubkey_string_conversion")]
    pub distribution_program: Pubkey,
    #[serde(with = "pubkey_string_conversion")]
    pub distribution_account: Pubkey,
    #[serde(with = "pubkey_string_conversion")]
    pub merkle_root_upload_authority: Pubkey,
    pub merkle_root: Hash,
    pub tree_nodes: Vec<TreeNode>,
    pub max_total_claim: u64,
    pub max_num_nodes: u64,
}

impl GeneratedMerkleTree {
    fn new_from_stake_meta_for_distribution_program(
        stake_meta: &StakeMeta,
        tip_router_program_id: &Pubkey,
        distribution_program: &Pubkey,
        ncn_address: &Pubkey,
        protocol_fee_bps: u64,
        epoch: u64,
    ) -> Result<Self, MerkleRootGeneratorError> {
        let (mut tree_nodes, tip_distribution_pubkey, merkle_root_upload_authority, total_tips) =
            if distribution_program.eq(&TIP_DISTRIBUTION_ID) {
                let tip_distribution_meta =
                    stake_meta.maybe_tip_distribution_meta.as_ref().unwrap();

                let tree_nodes = match TreeNode::vec_from_stake_meta_for_distribution_meta(
                    stake_meta,
                    tip_router_program_id,
                    distribution_program,
                    &tip_distribution_meta.tip_distribution_pubkey,
                    ncn_address,
                    tip_distribution_meta.total_tips,
                    protocol_fee_bps,
                    tip_distribution_meta.validator_fee_bps,
                    epoch,
                ) {
                    Err(e) => return Err(e),
                    Ok(maybe_tree_nodes) => maybe_tree_nodes.unwrap_or_default(),
                };

                (
                    tree_nodes,
                    tip_distribution_meta.tip_distribution_pubkey,
                    tip_distribution_meta.merkle_root_upload_authority,
                    tip_distribution_meta.total_tips,
                )
            } else if distribution_program.eq(&PRIORITY_FEE_DISTRIBUTION_ID) {
                let priority_fee_distribution_meta = stake_meta
                    .maybe_priority_fee_distribution_meta
                    .as_ref()
                    .unwrap();

                let tree_nodes = match TreeNode::vec_from_stake_meta_for_distribution_meta(
                    stake_meta,
                    tip_router_program_id,
                    distribution_program,
                    &priority_fee_distribution_meta.priority_fee_distribution_pubkey,
                    ncn_address,
                    priority_fee_distribution_meta.total_tips,
                    protocol_fee_bps,
                    // Priority fee distributions always have 0 protocol commissions because they
                    // retain their portion and transfer the rest of the priority fees after each epoch.
                    0,
                    epoch,
                ) {
                    Err(e) => return Err(e),
                    Ok(maybe_tree_nodes) => maybe_tree_nodes.unwrap_or_default(),
                };

                (
                    tree_nodes,
                    priority_fee_distribution_meta.priority_fee_distribution_pubkey,
                    priority_fee_distribution_meta.merkle_root_upload_authority,
                    priority_fee_distribution_meta.total_tips,
                )
            } else {
                return Err(MerkleRootGeneratorError::UnknownDistributionProgram);
            };

        // Create merkle tree and add proofs
        let hashed_nodes: Vec<[u8; 32]> = tree_nodes.iter().map(|n| n.hash().to_bytes()).collect();

        let merkle_tree = MerkleTree::new(&hashed_nodes[..], true);
        let max_num_nodes = tree_nodes.len() as u64;

        for (i, tree_node) in tree_nodes.iter_mut().enumerate() {
            tree_node.proof = Some(get_proof(&merkle_tree, i));
        }

        Ok(Self {
            distribution_program: distribution_program.to_owned(),
            max_num_nodes,
            distribution_account: tip_distribution_pubkey,
            merkle_root_upload_authority,
            merkle_root: *merkle_tree.get_root().unwrap(),
            tree_nodes,
            max_total_claim: total_tips,
        })
    }
}

impl GeneratedMerkleTreeCollection {
    /// Create a collection of Generated Merkle Trees that includes both the MEV Tip Distributions
    /// and the Priority Fee Distributions.
    pub fn new_from_stake_meta_collection(
        stake_meta_collection: StakeMetaCollection,
        ncn_address: &Pubkey,
        epoch: u64,
        protocol_fee_bps: u64,
        pf_distribution_protocol_fee_bps: u64,
        tip_router_program_id: &Pubkey,
    ) -> Result<Self, MerkleRootGeneratorError> {
        let generated_merkle_trees = stake_meta_collection
            .stake_metas
            .into_iter()
            .filter(|stake_meta| {
                stake_meta.maybe_tip_distribution_meta.is_some()
                    || stake_meta.maybe_priority_fee_distribution_meta.is_some()
            })
            .flat_map(|stake_meta| {
                let mut res = Vec::new();
                if stake_meta.maybe_tip_distribution_meta.is_some() {
                    let tip_distribution_tree =
                        GeneratedMerkleTree::new_from_stake_meta_for_distribution_program(
                            &stake_meta,
                            tip_router_program_id,
                            &TIP_DISTRIBUTION_ID,
                            ncn_address,
                            protocol_fee_bps,
                            epoch,
                        );
                    res.push(tip_distribution_tree);
                }

                if stake_meta.maybe_priority_fee_distribution_meta.is_some() {
                    let priority_fee_distribution_tree =
                        GeneratedMerkleTree::new_from_stake_meta_for_distribution_program(
                            &stake_meta,
                            tip_router_program_id,
                            &PRIORITY_FEE_DISTRIBUTION_ID,
                            ncn_address,
                            pf_distribution_protocol_fee_bps,
                            epoch,
                        );
                    res.push(priority_fee_distribution_tree);
                }
                res
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

    /// Write a GeneratedMerkleTreeCollection to a filepath
    pub fn write_to_file(&self, path: &PathBuf) -> Result<(), MerkleRootGeneratorError> {
        let serialized = serde_json::to_string_pretty(&self)?;
        let mut file = File::create(path)?;
        file.write_all(serialized.as_bytes())?;
        Ok(())
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

#[allow(dead_code)]
impl TreeNode {
    /// Given a StakeMeta for a validator, extract the tree nodes for a given
    /// _distribution_program_. _distribution_program_ should match
    /// _tip_distribution_program_id_ or _priority_fee_distribution_program_id_.
    #[allow(clippy::too_many_arguments)]
    fn vec_from_stake_meta_for_distribution_meta(
        stake_meta: &StakeMeta,
        tip_router_program_id: &Pubkey,
        distribution_program_id: &Pubkey,
        distribution_account_pubkey: &Pubkey,
        ncn_address: &Pubkey,
        total_tips: u64,
        protocol_fee_bps: u64,
        validator_fee_bps: u16,
        epoch: u64,
    ) -> Result<Option<Vec<Self>>, MerkleRootGeneratorError> {
        let protocol_fee_amount: u64 = mul_div(total_tips, protocol_fee_bps, MAX_BPS as u64)?;

        // For Priority Fee Distributions, there is no validator amount, 0 is passed in for
        // validator_fee_bps
        let mut validator_amount = mul_div(total_tips, validator_fee_bps as u64, MAX_BPS as u64)?;

        let (updated_validator_amount, remaining_total_rewards) = validator_amount
            .checked_add(protocol_fee_amount)
            .map_or((validator_amount, None), |total_fees| {
                if total_fees > total_tips {
                    // If fees exceed total tips, preference protocol fee amount and reduce validator amount
                    total_tips
                        .checked_sub(protocol_fee_amount)
                        .map(|adjusted_validator_amount| (adjusted_validator_amount, Some(0)))
                        .unwrap_or((0, None))
                } else {
                    // Otherwise use original protocol fee and subtract both fees from total
                    (
                        validator_amount,
                        total_tips
                            .checked_sub(protocol_fee_amount)
                            .and_then(|v| v.checked_sub(validator_amount)),
                    )
                }
            });

        validator_amount = updated_validator_amount;

        let remaining_total_rewards =
            remaining_total_rewards.ok_or(MerkleRootGeneratorError::CheckedMathError)?;

        let tip_router_target_epoch = epoch
            .checked_add(1)
            .ok_or(MerkleRootGeneratorError::CheckedMathError)?;

        let mut tree_nodes = vec![Self::generate_base_reward_node(
            tip_router_program_id,
            ncn_address,
            tip_router_target_epoch,
            distribution_account_pubkey,
            distribution_program_id,
            protocol_fee_amount,
        )];

        tree_nodes.push(Self::generate_validator_node(
            &stake_meta.validator_vote_account,
            distribution_account_pubkey,
            distribution_program_id,
            validator_amount,
        ));

        tree_nodes.extend(
            stake_meta
                .delegations
                .iter()
                .map(|delegation| {
                    Self::generate_delegator_node(
                        delegation,
                        distribution_account_pubkey,
                        distribution_program_id,
                        stake_meta.total_delegated,
                        remaining_total_rewards,
                    )
                })
                .collect::<Result<Vec<Self>, MerkleRootGeneratorError>>()?,
        );

        Ok(Some(tree_nodes))
    }

    /// Generates a TreeNode for the NCN's base reward receiver
    fn generate_base_reward_node(
        tip_router_program_id: &Pubkey,
        ncn_address: &Pubkey,
        epoch: u64,
        distribution_account_pubkey: &Pubkey,
        distribution_program_id: &Pubkey,
        protocol_fee_amount: u64,
    ) -> Self {
        // Must match the seeds from `core::BaseRewardReceiver`. Cannot
        // use `BaseRewardReceiver::find_program_address` as it would cause
        // circular dependecies.
        let base_reward_receiver = Pubkey::find_program_address(
            &[
                b"base_reward_receiver",
                &ncn_address.to_bytes(),
                &epoch.to_le_bytes(),
            ],
            tip_router_program_id,
        )
        .0;

        let (protocol_claim_status_pubkey, protocol_claim_status_bump) =
            Pubkey::find_program_address(
                &[
                    CLAIM_STATUS_SEED,
                    &base_reward_receiver.to_bytes(),
                    &distribution_account_pubkey.to_bytes(),
                ],
                distribution_program_id,
            );

        Self {
            claimant: base_reward_receiver,
            claim_status_pubkey: protocol_claim_status_pubkey,
            claim_status_bump: protocol_claim_status_bump,
            staker_pubkey: Pubkey::default(),
            withdrawer_pubkey: Pubkey::default(),
            amount: protocol_fee_amount,
            proof: None,
        }
    }

    /// Generates a TreeNode for a validator's vote account
    fn generate_validator_node(
        validator_vote_account: &Pubkey,
        distribution_account_pubkey: &Pubkey,
        distribution_program_id: &Pubkey,
        amount: u64,
    ) -> Self {
        let (validator_claim_status_pubkey, validator_claim_status_bump) =
            Pubkey::find_program_address(
                &[
                    CLAIM_STATUS_SEED,
                    &validator_vote_account.to_bytes(),
                    &distribution_account_pubkey.to_bytes(),
                ],
                distribution_program_id,
            );

        Self {
            claimant: *validator_vote_account,
            claim_status_pubkey: validator_claim_status_pubkey,
            claim_status_bump: validator_claim_status_bump,
            staker_pubkey: Pubkey::default(),
            withdrawer_pubkey: Pubkey::default(),
            amount,
            proof: None,
        }
    }

    /// Generates a TreeNode for a given Delegation
    fn generate_delegator_node(
        delegation: &Delegation,
        distribution_account_pubkey: &Pubkey,
        distribution_program_id: &Pubkey,
        total_delegated: u64,
        remaining_total_rewards: u64,
    ) -> Result<Self, MerkleRootGeneratorError> {
        let reward_amount = mul_div(
            delegation.lamports_delegated,
            remaining_total_rewards,
            total_delegated,
        )?;

        let (claim_status_pubkey, claim_status_bump) = Pubkey::find_program_address(
            &[
                CLAIM_STATUS_SEED,
                &delegation.stake_account_pubkey.to_bytes(),
                &distribution_account_pubkey.to_bytes(),
            ],
            distribution_program_id,
        );

        Ok(Self {
            claimant: delegation.stake_account_pubkey,
            claim_status_pubkey,
            claim_status_bump,
            staker_pubkey: delegation.staker_pubkey,
            withdrawer_pubkey: delegation.withdrawer_pubkey,
            amount: reward_amount,
            proof: None,
        })
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
    /// Load a serialized merkle tree from file path
    pub fn new_from_file(path: &PathBuf) -> Result<Self, MerkleRootGeneratorError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let tree: Self = serde_json::from_reader(reader)?;

        Ok(tree)
    }

    /// Write a merkle tree to a filepath
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
    use jito_priority_fee_distribution_sdk::jito_priority_fee_distribution::ID as PRIORITY_FEE_DISTRIBUTION_ID;

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
        let tip_distribution_program_id = TIP_DISTRIBUTION_ID;
        let priority_fee_distribution_program_id = PRIORITY_FEE_DISTRIBUTION_ID;
        let tip_router_program_id = Pubkey::new_unique();
        let (tda_0, tda_1) = (Pubkey::new_unique(), Pubkey::new_unique());
        let (pf_tda_0, pf_tda_1) = (Pubkey::new_unique(), Pubkey::new_unique());
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
                    maybe_priority_fee_distribution_meta: Some(PriorityFeeDistributionMeta {
                        merkle_root_upload_authority,
                        priority_fee_distribution_pubkey: pf_tda_0,
                        total_tips: 2_546_000_000,
                        validator_fee_bps: 5_000,
                    }),
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
                    maybe_priority_fee_distribution_meta: Some(PriorityFeeDistributionMeta {
                        merkle_root_upload_authority,
                        priority_fee_distribution_pubkey: pf_tda_1,
                        total_tips: 3_210_000_000,
                        validator_fee_bps: 1_000,
                    }),
                },
            ],
            tip_distribution_program_id,
            priority_fee_distribution_program_id,
            bank_hash: Hash::new_unique().to_string(),
            epoch: 100,
            slot: 2_000_000,
        };

        let merkle_tree_collection = GeneratedMerkleTreeCollection::new_from_stake_meta_collection(
            stake_meta_collection.clone(),
            &ncn_address,
            epoch,
            300,
            150,
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
            stake_meta_collection.stake_metas.len() * 2,
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

        let pf_claim_statuses = &[
            (protocol_fee_recipient, pf_tda_0),
            (validator_vote_account_0, pf_tda_0),
            (stake_account_0, pf_tda_0),
            (stake_account_1, pf_tda_0),
            (protocol_fee_recipient, pf_tda_1),
            (validator_vote_account_1, pf_tda_1),
            (stake_account_2, pf_tda_1),
            (stake_account_3, pf_tda_1),
        ]
        .iter()
        .map(|(claimant, tda)| {
            Pubkey::find_program_address(
                &[CLAIM_STATUS_SEED, &claimant.to_bytes(), &tda.to_bytes()],
                &PRIORITY_FEE_DISTRIBUTION_ID,
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
                claimant: validator_vote_account_0,
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
            distribution_program: TIP_DISTRIBUTION_ID,
            distribution_account: tda_0,
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

        // Priority Fee Distribution nodes for Validator 0
        let tree_nodes = vec![
            TreeNode {
                claimant: protocol_fee_recipient,
                claim_status_pubkey: pf_claim_statuses[0].0,
                claim_status_bump: pf_claim_statuses[0].1,
                staker_pubkey: Pubkey::default(),
                withdrawer_pubkey: Pubkey::default(),
                amount: 38_190_000, // 1.5% of 2_546_000_000
                proof: None,
            },
            TreeNode {
                claimant: validator_vote_account_0,
                claim_status_pubkey: claim_statuses[1].0,
                claim_status_bump: claim_statuses[1].1,
                staker_pubkey: Pubkey::default(),
                withdrawer_pubkey: Pubkey::default(),
                amount: 0,
                proof: None,
            },
            TreeNode {
                claimant: stake_account_0,
                claim_status_pubkey: pf_claim_statuses[2].0,
                claim_status_bump: pf_claim_statuses[2].1,
                staker_pubkey: staker_account_0,
                withdrawer_pubkey: staker_account_0,
                amount: 199,
                proof: None,
            },
            TreeNode {
                claimant: stake_account_1,
                claim_status_pubkey: pf_claim_statuses[3].0,
                claim_status_bump: pf_claim_statuses[3].1,
                staker_pubkey: staker_account_1,
                withdrawer_pubkey: staker_account_1,
                amount: 233,
                proof: None,
            },
        ];

        // Handle creating expected PF GMT
        let hashed_nodes: Vec<[u8; 32]> = tree_nodes.iter().map(|n| n.hash().to_bytes()).collect();
        let merkle_tree = MerkleTree::new(&hashed_nodes[..], true);
        let gmt_1 = GeneratedMerkleTree {
            distribution_program: PRIORITY_FEE_DISTRIBUTION_ID,
            distribution_account: pf_tda_0,
            merkle_root_upload_authority,
            merkle_root: *merkle_tree.get_root().unwrap(),
            tree_nodes,
            max_total_claim: stake_meta_collection.stake_metas[0]
                .clone()
                .maybe_priority_fee_distribution_meta
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
                claimant: validator_vote_account_1,
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
        let gmt_2 = GeneratedMerkleTree {
            distribution_program: TIP_DISTRIBUTION_ID,
            distribution_account: tda_1,
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

        // Priority Fee Distribution nodes for Validator 1
        let tree_nodes = vec![
            TreeNode {
                claimant: protocol_fee_recipient,
                claim_status_pubkey: pf_claim_statuses[4].0,
                claim_status_bump: pf_claim_statuses[4].1,
                staker_pubkey: Pubkey::default(),
                withdrawer_pubkey: Pubkey::default(),
                amount: 48_150_000, // 1.5% of 3_210_000_000
                proof: None,
            },
            TreeNode {
                claimant: validator_vote_account_1,
                claim_status_pubkey: claim_statuses[5].0,
                claim_status_bump: claim_statuses[5].1,
                staker_pubkey: Pubkey::default(),
                withdrawer_pubkey: Pubkey::default(),
                amount: 0,
                proof: None,
            },
            TreeNode {
                claimant: stake_account_2,
                claim_status_pubkey: pf_claim_statuses[6].0,
                claim_status_bump: pf_claim_statuses[6].1,
                staker_pubkey: staker_account_2,
                withdrawer_pubkey: staker_account_2,
                amount: 276,
                proof: None,
            },
            TreeNode {
                claimant: stake_account_3,
                claim_status_pubkey: pf_claim_statuses[7].0,
                claim_status_bump: pf_claim_statuses[7].1,
                staker_pubkey: staker_account_3,
                withdrawer_pubkey: staker_account_3,
                amount: 863_871,
                proof: None,
            },
        ];
        // Create expected PF GMT
        let hashed_nodes: Vec<[u8; 32]> = tree_nodes.iter().map(|n| n.hash().to_bytes()).collect();
        let merkle_tree = MerkleTree::new(&hashed_nodes[..], true);
        let gmt_3 = GeneratedMerkleTree {
            distribution_program: PRIORITY_FEE_DISTRIBUTION_ID,
            distribution_account: pf_tda_1,
            merkle_root_upload_authority,
            merkle_root: *merkle_tree.get_root().unwrap(),
            tree_nodes,
            max_total_claim: stake_meta_collection.stake_metas[1]
                .clone()
                .maybe_priority_fee_distribution_meta
                .unwrap()
                .total_tips,
            max_num_nodes: 4, // stake acc 1 + stake acc 2 + protocol fee recipient + validator
        };

        let expected_generated_merkle_trees = vec![gmt_0, gmt_1, gmt_2, gmt_3];
        let actual_generated_merkle_trees = merkle_tree_collection.generated_merkle_trees;
        expected_generated_merkle_trees
            .iter()
            .for_each(|expected_gmt| {
                let actual_gmt = actual_generated_merkle_trees
                    .iter()
                    .find(|gmt| {
                        gmt.distribution_account == expected_gmt.distribution_account
                            && gmt.distribution_program == expected_gmt.distribution_program
                    })
                    .unwrap();
                assert_eq!(expected_gmt.max_num_nodes, actual_gmt.max_num_nodes);
                assert_eq!(expected_gmt.max_total_claim, actual_gmt.max_total_claim);
                assert_eq!(
                    expected_gmt.distribution_account,
                    actual_gmt.distribution_account
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

        let epoch = 761;
        let merkle_tree_collection = GeneratedMerkleTreeCollection::new_from_stake_meta_collection(
            stake_meta_collection.clone(),
            &ncn_address,
            epoch,
            300,
            150,
            &tip_router_program_id,
        )
        .unwrap();
        merkle_tree_collection
            .generated_merkle_trees
            .iter()
            .for_each(|gmt| {
                // Ensure that validator vote account exists as a claimant in the new merkle tree collection
                // and does not contain identity account as claimant.
                assert!(gmt
                    .tree_nodes
                    .iter()
                    .any(|node| node.claimant == validator_vote_account_0
                        || node.claimant == validator_vote_account_1));
                assert!(
                    !(gmt
                        .tree_nodes
                        .iter()
                        .any(|node| node.claimant == validator_id_0
                            || node.claimant == validator_id_1))
                );
            });
    }

    #[test]
    fn test_new_from_stake_meta_collection_updated_full_commission_claim() {
        let merkle_root_upload_authority = Pubkey::new_unique();
        let tip_distribution_program_id = TIP_DISTRIBUTION_ID;
        let priority_fee_distribution_program_id = PRIORITY_FEE_DISTRIBUTION_ID;
        let tip_router_program_id = Pubkey::new_unique();
        let (tda_0, tda_1) = (Pubkey::new_unique(), Pubkey::new_unique());
        let pf_tda_0 = Pubkey::new_unique();
        let stake_account_0 = Pubkey::new_unique();
        let stake_account_1 = Pubkey::new_unique();
        let stake_account_2 = Pubkey::new_unique();
        let stake_account_3 = Pubkey::new_unique();
        let staker_account_0 = Pubkey::new_unique();
        let staker_account_1 = Pubkey::new_unique();

        let validator_vote_account_0 = Pubkey::new_unique();
        let validator_vote_account_1 = Pubkey::new_unique();
        let validator_id_0 = Pubkey::new_unique();
        let ncn_address = Pubkey::new_unique();
        let epoch = 815;

        let stake_meta_collection = StakeMetaCollection {
            stake_metas: vec![StakeMeta {
                validator_vote_account: validator_vote_account_0,
                validator_node_pubkey: validator_id_0,
                maybe_tip_distribution_meta: Some(TipDistributionMeta {
                    merkle_root_upload_authority,
                    tip_distribution_pubkey: tda_0,
                    total_tips: 1_900_122_111_000,
                    validator_fee_bps: 10_000,
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
                maybe_priority_fee_distribution_meta: Some(PriorityFeeDistributionMeta {
                    merkle_root_upload_authority,
                    priority_fee_distribution_pubkey: pf_tda_0,
                    total_tips: 2_546_000_000,
                    validator_fee_bps: 10_000,
                }),
            }],
            tip_distribution_program_id,
            priority_fee_distribution_program_id,
            bank_hash: Hash::new_unique().to_string(),
            epoch: 100,
            slot: 2_000_000,
        };

        let merkle_tree_collection = GeneratedMerkleTreeCollection::new_from_stake_meta_collection(
            stake_meta_collection.clone(),
            &ncn_address,
            epoch,
            300,
            150,
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
            stake_meta_collection.stake_metas.len() * 2,
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
                claimant: validator_vote_account_0,
                claim_status_pubkey: claim_statuses[1].0,
                claim_status_bump: claim_statuses[1].1,
                staker_pubkey: Pubkey::default(),
                withdrawer_pubkey: Pubkey::default(),
                amount: (1_900_122_111_000i128 - 57_003_663_330i128) as u64, // 97% of total tips
                proof: None,
            },
            TreeNode {
                claimant: stake_account_0,
                claim_status_pubkey: claim_statuses[2].0,
                claim_status_bump: claim_statuses[2].1,
                staker_pubkey: staker_account_0,
                withdrawer_pubkey: staker_account_0,
                amount: 0,
                proof: None,
            },
            TreeNode {
                claimant: stake_account_1,
                claim_status_pubkey: claim_statuses[3].0,
                claim_status_bump: claim_statuses[3].1,
                staker_pubkey: staker_account_1,
                withdrawer_pubkey: staker_account_1,
                amount: 0,
                proof: None,
            },
        ];

        let hashed_nodes: Vec<[u8; 32]> = tree_nodes.iter().map(|n| n.hash().to_bytes()).collect();
        let merkle_tree = MerkleTree::new(&hashed_nodes[..], true);
        let gmt_0 = GeneratedMerkleTree {
            distribution_program: TIP_DISTRIBUTION_ID,
            distribution_account: tda_0,
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

        let expected_generated_merkle_trees = vec![gmt_0];
        let actual_generated_merkle_trees = merkle_tree_collection.generated_merkle_trees;
        expected_generated_merkle_trees
            .iter()
            .for_each(|expected_gmt| {
                let actual_gmt = actual_generated_merkle_trees
                    .iter()
                    .find(|gmt| {
                        gmt.distribution_account == expected_gmt.distribution_account
                            && gmt.distribution_program == expected_gmt.distribution_program
                    })
                    .unwrap();
                assert_eq!(expected_gmt.max_num_nodes, actual_gmt.max_num_nodes);
                assert_eq!(expected_gmt.max_total_claim, actual_gmt.max_total_claim);
                assert_eq!(
                    expected_gmt.distribution_account,
                    actual_gmt.distribution_account
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
                        if expected_tree_node.claimant == validator_vote_account_0 {
                            // Difference should be the protocol fee bps against the total tips
                            let expected_amount =
                                ((expected_gmt.max_total_claim as i128) * 9700i128 / 10_000i128)
                                    as u64;
                            let actual_amount = actual_tree_node.amount;
                            assert_eq!(
                                expected_amount, actual_amount,
                                "Expected amount: {}, Actual amount: {}",
                                expected_amount, actual_amount
                            );
                        } else {
                            assert_eq!(
                                expected_tree_node.amount, actual_tree_node.amount,
                                "Expected amount: {}, Actual amount: {}",
                                expected_tree_node.amount, actual_tree_node.amount
                            );
                        }
                    });
                assert_eq!(expected_gmt.merkle_root, actual_gmt.merkle_root);
            });
    }
}
