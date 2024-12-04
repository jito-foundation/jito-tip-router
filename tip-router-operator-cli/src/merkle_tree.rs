use {
    crate::{
        GeneratedMerkleTree,
        GeneratedMerkleTreeCollection,
        StakeMetaCollection,
        read_json_from_file,
        sign_and_send_transactions_with_retries,
    },
    jito_tip_distribution::{
        program::JitoTipDistribution,
        sdk::instruction::{upload_merkle_root_ix, UploadMerkleRootAccounts, UploadMerkleRootArgs},
    },
    log::info,
    solana_client::nonblocking::rpc_client::RpcClient,
    solana_metrics::datapoint_info,
    solana_sdk::{
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        transaction::Transaction,
    },
    std::{path::PathBuf, time::{Duration, Instant}},
    thiserror::Error,
    anyhow::Result,
};

pub struct MerkleTreeGenerator {
    rpc_url: String,
    keypair: Keypair,
    ncn_address: String,
    output_path: PathBuf,
    rpc_client: RpcClient,
    tip_distribution_program_id: Pubkey,
    tip_distribution_config: Pubkey,
    merkle_root_upload_authority: Keypair,
    ncn_program_id: Pubkey,
}

#[derive(Error, Debug)]
pub enum MerkleTreeError {
    #[error(transparent)] IoError(#[from] std::io::Error),

    #[error(transparent)] JsonError(#[from] serde_json::Error),

    #[error(transparent)] RpcError(#[from] Box<solana_client::client_error::ClientError>),

    #[error("Epoch calculation error")]
    EpochError,

    #[error("Tree modification error: {0}")] TreeModificationError(String),

    #[error("NCN upload error: {0}")] NcnError(String),
}
impl MerkleTreeGenerator {
    pub fn new(
        rpc_url: &str,
        keypair: Keypair,
        ncn_address: String,
        output_path: PathBuf,
        tip_distribution_program_id: Pubkey,
        tip_distribution_config: Pubkey,
        merkle_root_upload_authority: Keypair,
        ncn_program_id: Pubkey,
    ) -> Result<Self> {
        let rpc_client = RpcClient::new(rpc_url.to_string());
        
        Ok(Self {
            rpc_url: rpc_url.to_string(),
            keypair,
            ncn_address,
            output_path,
            rpc_client,
            tip_distribution_program_id,
            tip_distribution_config,
            merkle_root_upload_authority,
            ncn_program_id,
        })
    }

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

    pub fn modify_tree_for_tip_supply(
        &mut self,
        tree: &mut GeneratedMerkleTree
    ) -> Result<(), MerkleTreeError> {
        const TIP_PERCENTAGE: f64 = 0.03; // 3%

        // Calculate tip amount
        let total_tips = tree.max_total_claim;
        let tip_amount = ((total_tips as f64) * TIP_PERCENTAGE) as u64;

        // Create new node for tip claim
        let epoch_pda = self.derive_epoch_tip_pda(tree.epoch)?;
        tree.add_claimant(epoch_pda, tip_amount)?;

        // Adjust other amounts
        for node in &mut tree.tree_nodes {
            node.amount = node.amount.saturating_sub(
                ((node.amount as f64) * TIP_PERCENTAGE) as u64
            );
        }

        Ok(())
    }

    pub async fn wait_for_epoch_boundary(&self) -> Result<u64, MerkleTreeError> {
        let epoch_schedule = self.rpc_client.get_epoch_schedule().await?;
        let slot = self.rpc_client.get_slot().await?;

        // Logic from autosnapshot_inner.sh to wait for epoch boundary
        let current_epoch = epoch_schedule.get_epoch(slot);
        while
            self.rpc_client.get_slot().await? < epoch_schedule.get_last_slot_in_epoch(current_epoch)
        {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        Ok(current_epoch)
    }

    pub async fn generate_meta_merkle_tree(
        &self,
        merkle_trees: &GeneratedMerkleTreeCollection
    ) -> Result<MetaMerkleTree, MerkleTreeError> {
        let start = Instant::now();
        info!("Generating meta merkle tree for epoch {}", merkle_trees.epoch);

        // Extract all validator merkle roots and their associated data
        let validator_entries: Vec<MetaMerkleNode> = merkle_trees.generated_merkle_trees
            .iter()
            .map(|tree| MetaMerkleNode {
                validator: tree.tip_distribution_account,
                merkle_root: tree.merkle_root,
                total_claim: tree.max_total_claim,
                num_nodes: tree.max_num_nodes,
            })
            .collect();

        let meta_tree = MetaMerkleTree::new(
            merkle_trees.epoch,
            validator_entries,
            self.tip_distribution_program_id
        )?;

        let elapsed = start.elapsed();
        datapoint_info!(
            "tip_router_meta_merkle_tree",
            ("epoch", merkle_trees.epoch, i64),
            ("validator_count", validator_entries.len(), i64),
            ("elapsed_ms", elapsed.as_millis(), i64)
        );

        Ok(meta_tree)
    }

    pub async fn upload_to_ncn(
        &self,
        meta_merkle_tree: &MetaMerkleTree
    ) -> Result<(), MerkleTreeError> {
        let start = Instant::now();
        info!("Uploading meta merkle root to NCN for epoch {}", meta_merkle_tree.epoch);

        // Create the cast vote instruction
        let cast_vote_ix = create_cast_vote_ix(
            self.ncn_program_id,
            CastVoteArgs {
                epoch: meta_merkle_tree.epoch,
                merkle_root: meta_merkle_tree.root,
                validator_count: meta_merkle_tree.validator_count(),
            },
            CastVoteAccounts {
                voter: self.merkle_root_upload_authority.pubkey(),
                tip_distribution_config: self.tip_distribution_config,
            }
        );

        // Until NCN is ready, use memo instruction
        let memo_ix = create_memo_ix(
            format!(
                "Meta Merkle Root: {:?}, Epoch: {}, TDA: {:?}",
                meta_merkle_tree.root,
                meta_merkle_tree.epoch,
                self.tip_distribution_config
            )
        );

        let transaction = Transaction::new_with_payer(
            &[memo_ix],
            Some(&self.merkle_root_upload_authority.pubkey())
        );

        // Send and confirm transaction
        let signature = self.rpc_client
            .send_and_confirm_transaction(&transaction).await
            .map_err(|e| MerkleTreeError::NcnError(format!("Failed to send transaction: {}", e)))?;

        let elapsed = start.elapsed();
        datapoint_info!(
            "tip_router_ncn_upload",
            ("epoch", meta_merkle_tree.epoch, i64),
            ("signature", signature.to_string(), String),
            ("elapsed_ms", elapsed.as_millis(), i64)
        );

        Ok(())
    }
}
