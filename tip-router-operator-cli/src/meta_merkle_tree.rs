use {
    solana_sdk::{hash::Hash, pubkey::Pubkey, hash::Hasher},
    solana_merkle_tree::MerkleTree,
    thiserror::Error,
};

#[derive(Debug, Clone)]
pub struct MetaMerkleNode {
    pub validator: Pubkey,
    pub merkle_root: Hash,
    pub total_claim: u64,
    pub num_nodes: u64,
}

impl MetaMerkleNode {
    fn hash(&self) -> Hash {
        let mut hasher = Hasher::default();
        hasher.hash(self.validator.as_ref());
        hasher.hash(self.merkle_root.as_ref());
        hasher.hash(&self.total_claim.to_le_bytes());
        hasher.hash(&self.num_nodes.to_le_bytes());
        hasher.result()
    }
}

#[derive(Debug)]
pub struct MetaMerkleTree {
    pub epoch: u64,
    pub root: [u8; 32],
    pub validator_entries: Vec<MetaMerkleNode>,
}

impl MetaMerkleTree {
    pub fn new(
        epoch: u64,
        validator_entries: &[MetaMerkleNode],
        _program_id: Pubkey,
    ) -> Result<Self, MetaMerkleError> {
        if validator_entries.is_empty() {
            return Err(MetaMerkleError::Construction(
                "No validator entries provided".to_string()
            ));
        }

        // Hash each validator entry
        let hashed_entries: Vec<[u8; 32]> = validator_entries
            .iter()
            .map(|entry| entry.hash().to_bytes())
            .collect();

        // Create merkle tree
        let merkle_tree = MerkleTree::new(&hashed_entries, true);
        let root = merkle_tree
            .get_root()
            .ok_or_else(|| MetaMerkleError::Construction(
                "Failed to get merkle root".to_string()
            ))?
            .to_bytes();

            Ok(Self {
            epoch,
            validator_entries: validator_entries.to_vec(),
            root,
        })
    }

    pub fn validator_count(&self) -> u64 {
        self.validator_entries.len() as u64
    }
}

#[derive(Error, Debug)]
pub enum MetaMerkleError {
    #[error("Failed to construct merkle tree: {0}")]
    Construction(String),
}