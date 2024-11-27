use ::{
    crate::{
        read_json_from_file,
        GeneratedMerkleTree,
        GeneratedMerkleTreeCollection,
        StakeMetaCollection,
        sign_and_send_transactions_with_retries,
    },
    anchor_lang::AccountDeserialize,
    jito_tip_distribution::{
        sdk::instruction::{ upload_merkle_root_ix, UploadMerkleRootAccounts, UploadMerkleRootArgs },
        state::{ Config, TipDistributionAccount },
    },
    solana_sdk::{ pubkey::Pubkey, signature::Keypair },
    std::{ path::PathBuf, time::Duration },
    thiserror::Error,
};

#[derive(Error, Debug)]
pub enum MerkleTreeError {
    #[error(transparent)] IoError(#[from] std::io::Error),

    #[error(transparent)] JsonError(#[from] serde_json::Error),

    #[error(transparent)] RpcError(#[from] Box<solana_client::client_error::ClientError>),
}

impl MerkleTreeGenerator {
    pub async fn generate_and_upload_merkle_trees(
        &self,
        stake_meta_collection: StakeMetaCollection
    ) -> Result<(), MerkleTreeError> {
        // Generate merkle trees
        let merkle_tree_coll = self.generate_merkle_trees(stake_meta_collection)?;

        // Write to file
        let out_path = self.output_dir.join(
            format!("merkle-trees-{}.json", merkle_tree_coll.epoch)
        );
        self.write_to_json_file(&merkle_tree_coll, &out_path)?;

        // Upload merkle roots
        self.upload_merkle_roots(&merkle_tree_coll).await?;

        Ok(())
    }

    pub async fn generate_meta_merkle_tree(
        &self,
        merkle_trees: &GeneratedMerkleTreeCollection
    ) -> Result<MetaMerkleTree, MerkleTreeError> {
        // New implementation for meta merkle tree
        // This will combine all validator merkle trees into a single meta tree
    }

    pub async fn upload_to_ncn(
        &self,
        meta_merkle_tree: &MetaMerkleTree
    ) -> Result<(), MerkleTreeError> {
        // New implementation for NCN upload
    }
}
