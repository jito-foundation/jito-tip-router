use serde::{Deserialize, Serialize};
use solana_program::{
    hash::{hashv, Hash},
    pubkey::Pubkey,
};

use crate::generated_merkle_tree::GeneratedMerkleTree;

/// Represents the information for activating a tip distribution account.
#[derive(Debug, Clone, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct TreeNode {
    /// Pubkey of the vote account for setting the merkle root
    pub tip_distribution_account: Pubkey,
    /// Claimant's proof of inclusion in the Merkle Tree
    pub proof: Option<Vec<[u8; 32]>>,
    /// Validator merkle root to be set for the tip distribution account
    pub validator_merkle_root: [u8; 32],
    /// Maximum total claimable for the tip distribution account
    pub max_total_claim: u64,
    /// Number of nodes to claim
    pub max_num_nodes: u64,
}

impl TreeNode {
    pub const fn new(
        tip_distribution_account: &Pubkey,
        validator_merkle_root: &[u8; 32],
        max_total_claim: u64,
        max_num_nodes: u64,
    ) -> Self {
        Self {
            tip_distribution_account: *tip_distribution_account,
            proof: None,
            validator_merkle_root: *validator_merkle_root,
            max_total_claim,
            max_num_nodes,
        }
    }

    pub fn hash(&self) -> Hash {
        hashv(&[
            &self.tip_distribution_account.to_bytes(),
            &self.validator_merkle_root,
            &self.max_total_claim.to_le_bytes(),
            &self.max_num_nodes.to_le_bytes(),
        ])
    }
}

// TODO replace this with the GeneratedMerkleTree from the Operator module once that's created
impl From<GeneratedMerkleTree> for TreeNode {
    fn from(generated_merkle_tree: GeneratedMerkleTree) -> Self {
        Self {
            tip_distribution_account: generated_merkle_tree.tip_distribution_account,
            validator_merkle_root: generated_merkle_tree.merkle_root.to_bytes(),
            max_total_claim: generated_merkle_tree.max_total_claim,
            max_num_nodes: generated_merkle_tree.max_num_nodes,
            proof: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_tree_node() {
        let tree_node = TreeNode {
            tip_distribution_account: Pubkey::default(),
            proof: None,
            validator_merkle_root: [0; 32],
            max_total_claim: 0,
            max_num_nodes: 0,
        };
        let serialized = serde_json::to_string(&tree_node).unwrap();
        let deserialized: TreeNode = serde_json::from_str(&serialized).unwrap();
        assert_eq!(tree_node, deserialized);
    }
}
