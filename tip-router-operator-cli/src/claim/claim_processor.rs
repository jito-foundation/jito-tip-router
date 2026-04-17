use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::read_keypair_file};
use tokio::sync::Mutex as AsyncMutex;

use crate::{
    claim::{handle_claim_mev_tips, ClaimMevError},
    Cli,
};

/// Encapsulates the state and logic for processing MEV tip claims.
///
/// `ClaimProcessor` is cheaply cloneable — all fields are either `Copy` or
/// wrapped in `Arc`.  This lets the outer loop hand each epoch task its own
/// clone without a `HashMap<u64, ClaimProcessor>`.  The `skipped_claimants`
/// map is shared across clones via `Arc<Mutex<...>>`, so per-epoch state
/// survives across the 30-minute outer-loop sleep.
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
#[derive(Clone)]
pub struct ClaimProcessor {
    /// Shared CLI config. Stored behind Arc so cloning ClaimProcessor is a
    /// cheap refcount bump rather than a deep copy of all String/PathBuf fields.
    cli: Arc<Cli>,

    /// Tip Distribution Proram ID
    tip_distribution_program_id: Pubkey,

    /// Priority Fee Distribution Program ID
    priority_fee_distribution_program_id: Pubkey,

    /// Tip Router Program ID
    tip_router_program_id: Pubkey,

    /// NCN
    ncn: Pubkey,

    /// RPC client for reading on-chain state (uses confirmed commitment)
    rpc_client: Arc<RpcClient>,

    /// Claimant pubkeys per epoch that are known to not exist on-chain.
    /// Shared across clones so state survives outer-loop sleep cycles.
    skipped_claimants: Arc<Mutex<HashMap<u64, HashSet<Pubkey>>>>,
}

impl ClaimProcessor {
    pub fn new(
        cli: Cli,
        rpc_client: Arc<RpcClient>,
        tip_distribution_program_id: Pubkey,
        priority_fee_distribution_program_id: Pubkey,
        tip_router_program_id: Pubkey,
        ncn: Pubkey,
    ) -> Self {
        Self {
            cli: Arc::new(cli),
            tip_distribution_program_id,
            priority_fee_distribution_program_id,
            tip_router_program_id,
            ncn,
            rpc_client,
            skipped_claimants: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn rpc_client(&self) -> &Arc<RpcClient> {
        &self.rpc_client
    }

    /// Returns a snapshot of the skipped pubkeys for `epoch`.
    pub fn get_epoch_skipped(&self, epoch: u64) -> HashSet<Pubkey> {
        self.skipped_claimants
            .lock()
            .unwrap()
            .get(&epoch)
            .cloned()
            .unwrap_or_default()
    }

    /// Adds `none_claimants` to the skipped set for `epoch`.
    pub fn extend_epoch_skipped(
        &self,
        epoch: u64,
        none_claimants: impl IntoIterator<Item = Pubkey>,
    ) {
        self.skipped_claimants
            .lock()
            .unwrap()
            .entry(epoch)
            .or_default()
            .extend(none_claimants);
    }

    pub fn remove_epoch(&self, target_epoch: u64, current_epoch: u64) -> Result<(), ClaimMevError> {
        let current_claim_epoch =
            current_epoch
                .checked_sub(1)
                .ok_or(ClaimMevError::CompletedEpochsError(
                    "Epoch underflow".to_string(),
                ))?;

        if current_claim_epoch == target_epoch {
            return Ok(());
        }

        self.skipped_claimants.lock().unwrap().remove(&target_epoch);

        Ok(())
    }

    pub async fn claim_mev_tips_with_emit(
        &self,
        epoch: u64,
        max_loop_duration: Duration,
        file_path: &PathBuf,
        file_mutex: &Arc<AsyncMutex<()>>,
    ) -> Result<(), anyhow::Error> {
        let keypair = read_keypair_file(self.cli.keypair_path.clone())
            .map_err(|e| anyhow::anyhow!("Failed to read keypair file: {:?}", e))?;
        let keypair = Arc::new(keypair);
        let rpc_url = self.cli.rpc_url.clone();
        // Arc::clone is a cheap refcount bump — no deep copy of Cli fields.
        let cli = Arc::clone(&self.cli);
        let tip_distribution_program_id = self.tip_distribution_program_id;
        let priority_fee_distribution_program_id = self.priority_fee_distribution_program_id;
        let tip_router_program_id = self.tip_router_program_id;
        let ncn = self.ncn;

        handle_claim_mev_tips(
            &cli,
            epoch,
            tip_distribution_program_id,
            priority_fee_distribution_program_id,
            tip_router_program_id,
            ncn,
            max_loop_duration,
            file_path,
            file_mutex,
            &keypair,
            rpc_url,
            self,
        )
        .await?;

        Ok(())
    }
}
