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
    // rpc_url: &str,
) -> Result<GeneratedMerkleTreeCollection, MerkleRootGeneratorError> {
    // let rpc_client = RpcClient::new(rpc_url);
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
    use solana_program::hash::Hash;
    use crate::{StakeMeta, TipDistributionMeta, Delegation};

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
        let merkle_tree_coll = generate_merkle_root(stake_meta_collection)?;

        // Basic validations
        assert_eq!(merkle_tree_coll.epoch, 0);
        assert_eq!(merkle_tree_coll.generated_merkle_trees.len(), 1);
        
        // Validate generated merkle tree
        let generated_tree = &merkle_tree_coll.generated_merkle_trees[0];
        assert!(generated_tree.merkle_root != Hash::default(), "Merkle root should not be zero");
        
        // The tree has 3 nodes because:
        // - 2 leaf nodes (one for each delegation)
        // - 1 intermediate node (parent of the two leaf nodes)
        assert_eq!(generated_tree.tree_nodes.len(), 3, "Should have three tree nodes (2 leaves + 1 intermediate)");

        Ok(())
    }
}