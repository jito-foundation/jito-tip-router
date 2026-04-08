use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use log::info;
use meta_merkle_tree::generated_merkle_tree::GeneratedMerkleTreeCollection;
use rand::{seq::SliceRandom, thread_rng};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_metrics::datapoint_info;
use solana_pubkey::Pubkey;
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};
use tokio::sync::Mutex;

use crate::{
    claim::{
        add_completed_epoch, get_claim_transactions_for_valid_unclaimed, is_epoch_completed,
        is_sufficient_balance, ClaimMevError,
    },
    get_epoch_percentage,
    rpc_utils::send_until_blockhash_expires,
};

/// Encapsulates the state and logic for processing MEV tip claims.
///
/// `ClaimProcessor` holds the RPC clients, program IDs, and merkle tree data
/// needed to fetch unclaimed tips, send claim transactions, verify on-chain
/// confirmation, and mark epochs as complete.
///
/// # Claim Flow
///
/// The processor follows a fetch → send → verify → complete cycle:
///
/// 1. **Fetch**: Query on-chain state to build claim transactions for unclaimed tips
/// 2. **Send**: Submit claim transactions in batches of 2,000
/// 3. **Verify**: Re-fetch on-chain state to confirm which claims landed
/// 4. **Complete**: If all claims are confirmed, mark the epoch as done
/// 5. **Retry**: If claims remain, loop back to step 1
pub struct ClaimProcessor<'a> {
    /// RPC client for reading on-chain state (uses confirmed commitment)
    rpc_client: Arc<RpcClient>,

    /// RPC client for sending transactions (may differ from read client)
    rpc_sender_client: Arc<RpcClient>,

    /// The merkle tree collection containing all claimable nodes for the epoch
    merkle_trees: &'a GeneratedMerkleTreeCollection,

    /// Program ID of the tip distribution program
    tip_distribution_program_id: Pubkey,

    /// Program ID of the priority fee distribution program
    priority_fee_distribution_program_id: Pubkey,

    /// Program ID of the tip router program
    tip_router_program_id: Pubkey,

    /// NCN address
    ncn: Pubkey,

    /// Priority fee in micro-lamports for claim transactions
    micro_lamports: u64,

    /// Public key of the transaction payer
    payer_pubkey: Pubkey,

    /// Operator address string (used for metrics tagging)
    operator_address: String,

    /// Cluster name (e.g. "mainnet", "testnet") for metrics tagging
    cluster: String,
}

impl<'a> ClaimProcessor<'a> {
    /// Create a new `ClaimProcessor`.
    ///
    /// Initializes RPC clients with a 30-minute timeout and confirmed commitment.
    /// The same RPC client is used for both reading and sending by default.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        rpc_url: String,
        merkle_trees: &'a GeneratedMerkleTreeCollection,
        tip_distribution_program_id: Pubkey,
        priority_fee_distribution_program_id: Pubkey,
        tip_router_program_id: Pubkey,
        ncn: Pubkey,
        micro_lamports: u64,
        payer_pubkey: Pubkey,
        operator_address: String,
        cluster: String,
    ) -> Self {
        let rpc_client = Arc::new(RpcClient::new_with_timeout_and_commitment(
            rpc_url,
            Duration::from_secs(1800),
            CommitmentConfig::confirmed(),
        ));

        Self {
            rpc_client: rpc_client.clone(),
            rpc_sender_client: rpc_client,
            merkle_trees,
            tip_distribution_program_id,
            priority_fee_distribution_program_id,
            tip_router_program_id,
            ncn,
            micro_lamports,
            payer_pubkey,
            operator_address,
            cluster,
        }
    }

    /// Fetch all unclaimed tip transactions from on-chain state.
    ///
    /// Returns a tuple of:
    /// - `Vec<Transaction>`: Claim transactions ready to be signed and sent
    /// - `bool`: Whether all validator claims have been processed (validators_processed).
    ///   When false, only validator nodes are returned; when true, all nodes are included.
    pub async fn fetch_unclaimed(&self) -> Result<(Vec<Transaction>, bool), ClaimMevError> {
        get_claim_transactions_for_valid_unclaimed(
            &self.rpc_client,
            self.merkle_trees,
            self.tip_distribution_program_id,
            self.priority_fee_distribution_program_id,
            self.tip_router_program_id,
            self.ncn,
            self.micro_lamports,
            self.payer_pubkey,
            &self.operator_address.to_string(),
            &self.cluster,
        )
        .await
    }

    /// Send claim transactions to the network in batches.
    ///
    /// Transactions are sent in chunks of 2,000. Before each batch, the payer's
    /// balance is checked to ensure sufficient funds for transaction fees.
    /// Sending continues even if individual batches fail — errors are logged
    /// and the next batch is attempted.
    async fn send_claims(
        &self,
        claims: &[Transaction],
        keypair: Arc<Keypair>,
    ) -> Result<(), ClaimMevError> {
        for transactions in claims.chunks(2_000) {
            let transactions: Vec<_> = transactions.to_vec();
            if let Some((start_balance, desired_balance, sol_to_deposit)) = is_sufficient_balance(
                &keypair.pubkey(),
                &self.rpc_client,
                transactions.len() as u64,
            )
            .await
            {
                return Err(ClaimMevError::InsufficientBalance {
                    desired_balance,
                    payer: keypair.pubkey(),
                    start_balance,
                    sol_to_deposit,
                });
            }

            let blockhash = self.rpc_client.get_latest_blockhash().await?;
            if let Err(e) = send_until_blockhash_expires(
                &self.rpc_client,
                &self.rpc_sender_client,
                transactions,
                blockhash,
                &keypair,
            )
            .await
            {
                info!("send_until_blockhash_expires failed: {:?}", e);
            }
        }
        Ok(())
    }

    /// Check if all claims are done and mark the epoch as complete if so.
    ///
    /// An epoch is marked complete only when both conditions are met:
    /// - `validators_processed` is true (all validator claims have been handled)
    /// - `transactions_empty` is true (no unclaimed transactions remain)
    ///
    /// Returns `true` if the epoch was marked complete, `false` otherwise.
    pub async fn try_mark_complete(
        &self,
        validators_processed: bool,
        transactions_empty: bool,
        epoch: u64,
        file_path: &PathBuf,
        file_mutex: &Arc<Mutex<()>>,
    ) -> Result<bool, ClaimMevError> {
        if validators_processed && transactions_empty {
            add_completed_epoch(epoch, file_path, file_mutex).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get the current epoch progress as a percentage (0.0 to 1.0).
    ///
    /// Returns `None` if the RPC call fails.
    pub async fn get_epoch_pct(&self) -> Option<f64> {
        get_epoch_percentage(&self.rpc_client).await.ok()
    }

    /// Execute the full claim workflow for the epoch.
    ///
    /// Runs the fetch → send → verify → complete cycle in a loop until either:
    /// - All claims are confirmed on-chain and the epoch is marked complete
    /// - `max_loop_duration` expires
    ///
    /// # Flow
    ///
    /// ```text
    /// 1. Fetch unclaimed transactions
    /// 2. If empty and validators done → mark complete, return Ok
    /// 3. Send claim transactions in batches
    /// 4. Verify on-chain: re-fetch to check which claims landed
    /// 5. If all confirmed → mark complete, return Ok
    /// 6. Otherwise → loop back to step 1 with remaining claims
    /// ```
    ///
    /// After the loop expires, a final check is performed. If claims still
    /// remain, returns `ClaimMevError::NotFinished` with the remaining count.
    pub async fn claim_mev_tips(
        &self,
        keypair: Arc<Keypair>,
        max_loop_duration: Duration,
        file_path: &PathBuf,
        file_mutex: &Arc<Mutex<()>>,
    ) -> Result<(), ClaimMevError> {
        let epoch = self.merkle_trees.epoch;
        if is_epoch_completed(epoch, file_path, file_mutex).await? {
            return Ok(());
        }

        let start = Instant::now();
        while start.elapsed() <= max_loop_duration {
            // Step 1: Fetch unclaimed transactions
            let (mut claims_to_process, validators_processed) = self.fetch_unclaimed().await?;

            if validators_processed {
                if let Some(epoch_percentage) = self.get_epoch_pct().await {
                    datapoint_info!(
                        "tip_router_cli.claim_mev_tips-send_summary",
                        ("claim_transactions_left", claims_to_process.len(), i64),
                        ("epoch", epoch, i64),
                        ("operator", self.operator_address.to_string(), String),
                        ("epoch_percentage", epoch_percentage, f64),
                        "cluster" => self.cluster,
                    );
                }
            }

            // Nothing to send — mark complete if validators are also done
            if self
                .try_mark_complete(
                    validators_processed,
                    claims_to_process.is_empty(),
                    epoch,
                    file_path,
                    file_mutex,
                )
                .await?
            {
                return Ok(());
            }

            // Step 2: Send claim transactions
            claims_to_process.shuffle(&mut thread_rng());
            self.send_claims(&claims_to_process, keypair.clone())
                .await?;

            // Step 3: Verify which claims landed on-chain
            let (remaining, validators_processed) = self.fetch_unclaimed().await?;

            // Step 4: All claims confirmed — mark epoch complete
            if self
                .try_mark_complete(
                    validators_processed,
                    remaining.is_empty(),
                    epoch,
                    file_path,
                    file_mutex,
                )
                .await?
            {
                info!(
                    "All claims confirmed on-chain for epoch {}, marking complete",
                    epoch
                );
                return Ok(());
            }

            info!(
                "Epoch {}: {} claims still pending after verification, retrying",
                epoch,
                remaining.len()
            );
        }

        // Loop expired — final status check
        let (transactions, validators_processed) = self.fetch_unclaimed().await?;

        if self
            .try_mark_complete(
                validators_processed,
                transactions.is_empty(),
                epoch,
                file_path,
                file_mutex,
            )
            .await?
        {
            return Ok(());
        }

        info!(
            "Not finished claiming for epoch {}, transactions left {}",
            epoch,
            transactions.len()
        );
        Err(ClaimMevError::NotFinished {
            transactions_left: transactions.len(),
        })
    }
}
