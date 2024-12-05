use {
    solana_sdk::{
        signature::{Keypair, Signer, Signature},  // Add Signature here
        transaction::Transaction,
        pubkey::Pubkey,
    },
    jito_tip_distribution::sdk::instruction::{
        upload_merkle_root_ix,
        UploadMerkleRootArgs,
        UploadMerkleRootAccounts,
    },
    ellipsis_client::EllipsisClient,
    std::{ 
        path::PathBuf, 
        time::{ Duration, Instant }, 
        fs::File, 
        io::BufWriter,
        sync::Arc,
    },
    thiserror::Error,
    anyhow::Result,
    log::info,
    solana_metrics::datapoint_info,
    crate::{
        GeneratedMerkleTree,
        GeneratedMerkleTreeCollection,
        StakeMetaCollection,
        meta_merkle_tree::{ MetaMerkleTree, MetaMerkleNode, MetaMerkleError },
        merkle_root_generator_workflow::MerkleRootGeneratorError,
    },
    spl_memo::build_memo,
};

#[derive(Error, Debug)]
pub enum MerkleTreeError {
    #[error(transparent)] IoError(#[from] std::io::Error),
    #[error(transparent)] JsonError(#[from] serde_json::Error),
    #[error(transparent)] RpcError(#[from] Box<solana_client::client_error::ClientError>),
    #[error("Epoch calculation error")] EpochError,
    #[error("Tree modification error: {0}")] TreeModificationError(String),
    #[error("NCN upload error: {0}")] NcnError(String),
    #[error(transparent)] MetaMerkleError(#[from] MetaMerkleError),
    #[error(transparent)] MerkleRootGeneratorError(#[from] MerkleRootGeneratorError),
    #[error(transparent)] ClientError(#[from] solana_client::client_error::ClientError),
}

pub struct MerkleTreeGenerator {
    rpc_url: String,
    keypair: Keypair,
    ncn_address: String,
    output_dir: PathBuf,
    rpc_client: Arc<EllipsisClient>,  // Change this line
    tip_distribution_program_id: Pubkey,
    merkle_root_upload_authority: Keypair,
    tip_distribution_config: Pubkey,
}

impl MerkleTreeGenerator {
    pub fn new(
        rpc_url: &str,
        keypair: Keypair,
        ncn_address: String,
        output_dir: PathBuf,
        tip_distribution_program_id: Pubkey,
        merkle_root_upload_authority: Keypair,
        tip_distribution_config: Pubkey
    ) -> Result<Self> {
        let rpc_client = Arc::new(EllipsisClient::from_rpc_with_timeout(
            solana_client::rpc_client::RpcClient::new(rpc_url.to_string()),
            &keypair,
            300_000,
        )?);
    
        Ok(Self {
            rpc_url: rpc_url.to_string(),
            keypair,
            ncn_address,
            output_dir,
            rpc_client,
            tip_distribution_program_id,
            merkle_root_upload_authority,
            tip_distribution_config,
        })
    }

    pub async fn generate_and_upload_merkle_trees(
        &self,
        stake_meta_collection: StakeMetaCollection
    ) -> Result<GeneratedMerkleTreeCollection, MerkleTreeError> {
        let start = Instant::now();
        info!("Generating merkle trees for epoch {}", stake_meta_collection.epoch);

        // Generate merkle trees from stake meta collection
        let mut merkle_tree_coll = GeneratedMerkleTreeCollection::new_from_stake_meta_collection(
            stake_meta_collection.clone(),
            Some(self.rpc_client.clone())
        )?;

        // Modify each tree to include tip supply
        for tree in &mut merkle_tree_coll.generated_merkle_trees {
            self.modify_tree_for_tip_supply(tree, merkle_tree_coll.epoch)?;
        }

        // Write to file
        let out_path = self.output_dir.join(
            format!("merkle-trees-{}.json", merkle_tree_coll.epoch)
        );
        self.write_to_json_file(&merkle_tree_coll, &out_path)?;

        // Upload merkle roots
        self.upload_merkle_roots(&merkle_tree_coll).await?;

        let elapsed = start.elapsed();
        datapoint_info!(
            "tip_router_merkle_trees",
            ("epoch", merkle_tree_coll.epoch, i64),
            ("tree_count", merkle_tree_coll.generated_merkle_trees.len(), i64),
            ("elapsed_ms", elapsed.as_millis(), i64)
        );

        Ok(merkle_tree_coll)
    }

    fn write_to_json_file<T: serde::Serialize>(
        &self,
        data: &T,
        path: &PathBuf
    ) -> Result<(), MerkleTreeError> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, data)?;
        Ok(())
    }

    async fn upload_merkle_roots(
        &self,
        merkle_tree_coll: &GeneratedMerkleTreeCollection
    ) -> Result<(), MerkleTreeError> {
        let start = Instant::now();
        info!("Uploading merkle roots for epoch {}", merkle_tree_coll.epoch);
    
        for tree in &merkle_tree_coll.generated_merkle_trees {
            let ix = upload_merkle_root_ix(
                self.tip_distribution_program_id,
                UploadMerkleRootArgs {
                    root: tree.merkle_root.to_bytes(),
                    max_total_claim: tree.max_total_claim,
                    max_num_nodes: tree.max_num_nodes,
                },
                UploadMerkleRootAccounts {
                    config: self.tip_distribution_config,
                    merkle_root_upload_authority: self.merkle_root_upload_authority.pubkey(),
                    tip_distribution_account: tree.tip_distribution_account,
                }
            );
    
            let transaction = Transaction::new_with_payer(
                &[ix],
                Some(&self.merkle_root_upload_authority.pubkey())
            );
    
            self.send_and_confirm_transaction(&transaction)
                .await
                .map_err(|e| 
                    MerkleTreeError::NcnError(format!("Failed to upload merkle root: {}", e))
                )?;
        }
    
        let elapsed = start.elapsed();
        datapoint_info!(
            "tip_router_merkle_root_upload",
            ("epoch", merkle_tree_coll.epoch, i64),
            ("tree_count", merkle_tree_coll.generated_merkle_trees.len(), i64),
            ("elapsed_ms", elapsed.as_millis(), i64)
        );
    
        Ok(())
    }

    fn derive_epoch_tip_pda(&self, epoch: u64) -> Result<Pubkey, MerkleTreeError> {
        let (pda, _) = Pubkey::find_program_address(
            &[b"epoch_tip", &epoch.to_le_bytes()],
            &self.tip_distribution_program_id
        );
        Ok(pda)
    }

    pub fn modify_tree_for_tip_supply(
        &self,
        tree: &mut GeneratedMerkleTree,
        epoch: u64,
    ) -> Result<(), MerkleTreeError> {
        const TIP_PERCENTAGE: f64 = 0.03; // 3%
    
        // Calculate tip amount
        let total_tips = tree.max_total_claim;
        let tip_amount = ((total_tips as f64) * TIP_PERCENTAGE) as u64;
    
        // Create new node for tip claim
        let epoch_pda = self.derive_epoch_tip_pda(epoch)?;
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
        let epoch_schedule = self.rpc_client.get_epoch_schedule()?;
        let slot = self.rpc_client.get_slot()?;
        let current_epoch = epoch_schedule.get_epoch(slot);
        
        while self.rpc_client.get_slot()? < epoch_schedule.get_last_slot_in_epoch(current_epoch) {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        
        Ok(current_epoch)
    }
    
    pub async fn send_and_confirm_transaction(&self, transaction: &Transaction) -> Result<Signature, MerkleTreeError> {
        self.rpc_client
            .send_and_confirm_transaction(transaction)
            .map_err(MerkleTreeError::ClientError)
    }
    pub async fn generate_meta_merkle_tree(
        &self,
        merkle_trees: &GeneratedMerkleTreeCollection
    ) -> Result<MetaMerkleTree, MerkleTreeError> {
        let start = Instant::now();
        info!("Generating meta merkle tree for epoch {}", merkle_trees.epoch);

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
                &validator_entries,  // Add & here
                self.tip_distribution_program_id,
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

        // Create memo instruction (temporary until NCN is ready)
        let memo_ix = build_memo(
            format!(
                "Meta Merkle Root: {:?}, Epoch: {}, TDA: {:?}",
                meta_merkle_tree.root,
                meta_merkle_tree.epoch,
                self.tip_distribution_config
            ).as_bytes(),
            &[&self.merkle_root_upload_authority.pubkey()]
        );

        let transaction = Transaction::new_with_payer(
            &[memo_ix],
            Some(&self.merkle_root_upload_authority.pubkey())
        );

        let signature = self
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
