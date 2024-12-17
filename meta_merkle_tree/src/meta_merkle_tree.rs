use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufReader, Write},
    path::PathBuf,
    result,
};

use serde::{Deserialize, Serialize};
use solana_program::{hash::hashv, pubkey::Pubkey};

use crate::{
    error::MerkleTreeError::{self, MerkleValidationError},
    generated_merkle_tree::GeneratedMerkleTreeCollection,
    merkle_tree::MerkleTree,
    tree_node::TreeNode,
    utils::get_proof,
    verify::verify,
};

// We need to discern between leaf and intermediate nodes to prevent trivial second
// pre-image attacks.
// https://flawed.net.nz/2018/02/21/attacking-merkle-trees-with-a-second-preimage-attack
pub const LEAF_PREFIX: &[u8] = &[0];

/// Merkle Tree which will be used to set the merkle root for each tip distribution account.
/// Contains all the information necessary to verify claims against the Merkle Tree.
/// Wrapper around solana MerkleTree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaMerkleTree {
    /// The merkle root, which is uploaded on-chain
    pub merkle_root: [u8; 32],
    pub num_nodes: u64,
    pub tree_nodes: Vec<TreeNode>,
}

pub type Result<T> = result::Result<T, MerkleTreeError>;

impl MetaMerkleTree {
    pub fn new(mut tree_nodes: Vec<TreeNode>) -> Result<Self> {
        // Sort by hash to ensure consistent trees
        tree_nodes.sort_by_key(|node| node.hash());

        let hashed_nodes = tree_nodes
            .iter()
            .map(|claim_info| claim_info.hash().to_bytes())
            .collect::<Vec<_>>();

        let tree = MerkleTree::new(&hashed_nodes[..], true);

        for (i, tree_node) in tree_nodes.iter_mut().enumerate() {
            tree_node.proof = Some(get_proof(&tree, i));
        }

        let tree = Self {
            merkle_root: tree
                .get_root()
                .ok_or(MerkleTreeError::MerkleRootError)?
                .to_bytes(),
            num_nodes: tree_nodes.len() as u64,
            tree_nodes,
        };

        println!(
            "created merkle tree with {} nodes and max total claim of {}",
            tree.num_nodes, tree.num_nodes
        );
        tree.validate()?;
        Ok(tree)
    }

    // TODO replace this with the GeneratedMerkleTreeCollection from the Operator module once that's created
    pub fn new_from_generated_merkle_tree_collection(
        generated_merkle_tree_collection: GeneratedMerkleTreeCollection,
    ) -> Result<Self> {
        let tree_nodes = generated_merkle_tree_collection
            .generated_merkle_trees
            .into_iter()
            .map(TreeNode::from)
            .collect();
        Self::new(tree_nodes)
    }

    // TODO if we need to load this from a file (for operator?)
    /// Load a merkle tree from a csv path
    // pub fn new_from_csv(path: &PathBuf) -> Result<Self> {
    //     let csv_entries = CsvEntry::new_from_file(path)?;
    //     let tree_nodes: Vec<TreeNode> = csv_entries.into_iter().map(TreeNode::from).collect();
    //     let tree = Self::new(tree_nodes)?;
    //     Ok(tree)
    // }

    /// Load a serialized merkle tree from file path
    pub fn new_from_file(path: &PathBuf) -> Result<Self> {
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

    pub fn get_node(&self, tip_distribution_account: &Pubkey) -> TreeNode {
        for i in self.tree_nodes.iter() {
            if i.tip_distribution_account == *tip_distribution_account {
                return i.clone();
            }
        }

        panic!("Claimant not found in tree");
    }

    fn validate(&self) -> Result<()> {
        // The Merkle tree can be at most height 32, implying a max node count of 2^32 - 1
        let max_nodes = 2u64
            .checked_pow(32)
            .and_then(|x| x.checked_sub(1))
            .ok_or(MerkleTreeError::ArithmeticOverflow)?;
        if self.num_nodes > max_nodes {
            return Err(MerkleValidationError(format!(
                "Max num nodes {} is greater than 2^32 - 1",
                self.num_nodes
            )));
        }

        // validate that the length is equal to the max_num_nodes
        if self.tree_nodes.len() != self.num_nodes as usize {
            return Err(MerkleValidationError(format!(
                "Tree nodes length {} does not match max_num_nodes {}",
                self.tree_nodes.len(),
                self.num_nodes
            )));
        }

        // validate that there are no duplicate vote_accounts
        let unique_nodes: HashSet<_> = self
            .tree_nodes
            .iter()
            .map(|n| n.tip_distribution_account)
            .collect();

        if unique_nodes.len() != self.tree_nodes.len() {
            return Err(MerkleValidationError(
                "Duplicate vote_accounts found".to_string(),
            ));
        }

        if self.verify_proof().is_err() {
            return Err(MerkleValidationError(
                "Merkle root is invalid given nodes".to_string(),
            ));
        }

        Ok(())
    }

    /// verify that the leaves of the merkle tree match the nodes
    pub fn verify_proof(&self) -> Result<()> {
        let root = self.merkle_root;

        // Recreate root given nodes
        let hashed_nodes: Vec<[u8; 32]> = self
            .tree_nodes
            .iter()
            .map(|n| n.hash().to_bytes())
            .collect();
        let mk = MerkleTree::new(&hashed_nodes[..], true);

        assert_eq!(
            mk.get_root()
                .ok_or(MerkleValidationError("invalid merkle proof".to_string()))?
                .to_bytes(),
            root
        );

        // Verify each node against the root
        for (i, _node) in hashed_nodes.iter().enumerate() {
            let node = hashv(&[LEAF_PREFIX, &hashed_nodes[i]]);
            let proof = get_proof(&mk, i);

            if !verify(proof, root, node.to_bytes()) {
                return Err(MerkleValidationError("invalid merkle proof".to_string()));
            }
        }

        Ok(())
    }

    // Converts Merkle Tree to a map for faster key access
    pub fn convert_to_hashmap(&self) -> HashMap<Pubkey, TreeNode> {
        self.tree_nodes
            .iter()
            .map(|n| (n.tip_distribution_account, n.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use solana_program::pubkey::Pubkey;
    use solana_sdk::{
        signature::{EncodableKey, Keypair},
        signer::Signer,
    };

    use super::*;

    pub fn new_test_key() -> Pubkey {
        let kp = Keypair::new();
        let out_path = format!("./test_keys/{}.json", kp.pubkey());

        kp.write_to_file(out_path)
            .expect("Failed to write to signer");

        kp.pubkey()
    }

    #[test]
    fn test_verify_new_merkle_tree() {
        let tree_nodes = vec![TreeNode::new(Pubkey::default(), [0; 32], 100, 10)];
        let merkle_tree = MetaMerkleTree::new(tree_nodes).unwrap();
        assert!(merkle_tree.verify_proof().is_ok(), "verify failed");
    }

    #[ignore]
    #[test]
    fn test_write_merkle_distributor_to_file() {
        let tree_nodes = vec![
            TreeNode::new(
                new_test_key(),
                [0; 32],
                100 * u64::pow(10, 9),
                100 * u64::pow(10, 9),
            ),
            TreeNode::new(
                new_test_key(),
                [0; 32],
                100 * u64::pow(10, 9),
                100 * u64::pow(10, 9),
            ),
            TreeNode::new(
                new_test_key(),
                [0; 32],
                100 * u64::pow(10, 9),
                100 * u64::pow(10, 9),
            ),
        ];

        let merkle_distributor_info = MetaMerkleTree::new(tree_nodes).unwrap();
        let path = PathBuf::from("merkle_tree.json");

        // serialize merkle distributor to file
        merkle_distributor_info.write_to_file(&path);
        // now test we can successfully read from file
        let merkle_distributor_read: MetaMerkleTree = MetaMerkleTree::new_from_file(&path).unwrap();

        assert_eq!(merkle_distributor_read.tree_nodes.len(), 3);
    }

    // Test creating a merkle tree from Tree Nodes
    #[test]
    fn test_new_merkle_tree() {
        let pubkey1 = Pubkey::new_unique();
        let pubkey2 = Pubkey::new_unique();
        let pubkey3 = Pubkey::new_unique();

        let mut tree_nodes = vec![
            TreeNode::new(pubkey1, [0; 32], 10, 20),
            TreeNode::new(pubkey2, [0; 32], 1, 2),
            TreeNode::new(pubkey3, [0; 32], 3, 4),
        ];

        // Sort by hash
        tree_nodes.sort_by_key(|node| node.hash());

        let tree = MetaMerkleTree::new(tree_nodes).unwrap();

        assert_eq!(tree.tree_nodes.len(), 3);
        assert_eq!(tree.tree_nodes[0].max_total_claim, 10);
        assert_eq!(tree.tree_nodes[0].max_num_nodes, 20);
        assert_eq!(tree.tree_nodes[0].validator_merkle_root, [0; 32]);
        assert_eq!(tree.tree_nodes[0].tip_distribution_account, pubkey1);
        assert!(tree.tree_nodes[0].proof.is_some());
    }
}
