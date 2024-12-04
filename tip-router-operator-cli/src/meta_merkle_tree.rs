use {
    solana_sdk::{hash::Hash, pubkey::Pubkey},
    thiserror::Error,
};

#[derive(Debug)]
pub struct MetaMerkleNode {
    pub validator: Pubkey,
    pub merkle_root: Hash,
    pub total_claim: u64,
    pub num_nodes: u64,
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
        validator_entries: Vec<MetaMerkleNode>,
        program_id: Pubkey,
    ) -> Result<Self, MetaMerkleError> {
        // TODO: Implement merkle tree construction
        Ok(Self {
            epoch,
            root: [0; 32], // Placeholder
            validator_entries,
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