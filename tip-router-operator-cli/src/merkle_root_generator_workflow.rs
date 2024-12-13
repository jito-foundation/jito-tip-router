use {
    crate::{read_json_from_file, GeneratedMerkleTreeCollection, StakeMetaCollection},
    log::*,
    solana_client::rpc_client::RpcClient,
    std::{
        fmt::Debug,
        fs::File,
        io::{BufWriter, Write},
        path::PathBuf,
    },
    thiserror::Error,
};

#[derive(Error, Debug)]
pub enum MerkleRootGeneratorError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    RpcError(#[from] Box<solana_client::client_error::ClientError>),

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
}

pub fn generate_merkle_root(
    stake_meta_coll: StakeMetaCollection,
) -> Result<GeneratedMerkleTreeCollection, MerkleRootGeneratorError> {
    let merkle_tree_coll = GeneratedMerkleTreeCollection::new_from_stake_meta_collection(
        stake_meta_coll,
        None,
    )?;

    Ok(merkle_tree_coll)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::{
        pubkey::Pubkey,
        signature::{Keypair, Signer},
    };
    use solana_program::hash::{Hash, hashv};
    use crate::{StakeMeta, TipDistributionMeta, Delegation};
    use meta_merkle_tree::merkle_tree::MerkleTree;

    #[test]
    fn test_generate_merkle_root() -> Result<(), MerkleRootGeneratorError> {
        // Create test stake meta with two delegations
        let stake_meta = StakeMeta {
            validator_vote_account: Pubkey::new_unique(),
            validator_node_pubkey: Pubkey::new_unique(),
            maybe_tip_distribution_meta: Some(TipDistributionMeta {
                total_tips: 1_000_000,
                merkle_root_upload_authority: Keypair::new().pubkey(),
                tip_distribution_pubkey: Pubkey::new_unique(),
                validator_fee_bps: 1000,
            }),
            delegations: vec![
                Delegation {
                    stake_account_pubkey: Pubkey::new_unique(),
                    staker_pubkey: Pubkey::new_unique(),
                    withdrawer_pubkey: Pubkey::new_unique(),
                    lamports_delegated: 1_000_000,
                },
                Delegation {
                    stake_account_pubkey: Pubkey::new_unique(),
                    staker_pubkey: Pubkey::new_unique(),
                    withdrawer_pubkey: Pubkey::new_unique(),
                    lamports_delegated: 2_000_000,
                },
            ],
            total_delegated: 3_000_000,
            commission: 10,
        };

        let stake_meta_collection = StakeMetaCollection {
            epoch: 0,
            stake_metas: vec![stake_meta],
            bank_hash: "test_bank_hash".to_string(),
            slot: 0,
            tip_distribution_program_id: Pubkey::new_unique(),
        };

        // Generate merkle tree
        let merkle_tree_coll = generate_merkle_root(stake_meta_collection.clone())?;

        // Basic validations
        assert_eq!(merkle_tree_coll.epoch, stake_meta_collection.epoch);
        assert_eq!(merkle_tree_coll.generated_merkle_trees.len(), stake_meta_collection.stake_metas.len());
        
        // Validate generated merkle tree
        let generated_tree = &merkle_tree_coll.generated_merkle_trees[0];
        
        // Assert merkle root is not default
        assert_ne!(generated_tree.merkle_root, Hash::default(), "Merkle root should not be zero");
        
        // Assert the expected merkle root hash
        // Note: This hash value needs to be updated if the merkle tree generation logic changes
        assert_eq!(
            generated_tree.merkle_root.to_string(),
            "6AS26Yncvk8AqWyZt5nc4LCJt4d5JpNEFzd78DuTi55C",
            "Merkle root hash changed - update test if this was intentional"
        );
        
        // Verify the structure is valid
        for node in &generated_tree.tree_nodes {
            // Verify node has required data
            assert_ne!(node.claimant, Pubkey::default(), "Node claimant should not be default");
            assert_ne!(node.claim_status_pubkey, Pubkey::default(), "Node claim status should not be default");
            assert!(node.amount > 0, "Node amount should be greater than 0");
            
            // Verify node has a proof (except possibly the root node)
            let total_delegated: u64 = stake_meta_collection.stake_metas[0]
                .delegations
                .iter()
                .map(|d| d.lamports_delegated)
                .sum();
                
            if node.amount != total_delegated {
                assert!(node.proof.is_some(), "Non-root node should have a proof");
            }
        }

        Ok(())
    }
}