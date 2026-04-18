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

/// Encapsulates the config and per-epoch state needed to claim MEV tips.
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

    pub fn cli(&self) -> &Cli {
        &self.cli
    }

    pub fn rpc_client(&self) -> Arc<RpcClient> {
        self.rpc_client.clone()
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

    /// Claim MEV tips
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
        let cli = self.cli.clone();

        handle_claim_mev_tips(
            &cli,
            epoch,
            self.tip_distribution_program_id,
            self.priority_fee_distribution_program_id,
            self.tip_router_program_id,
            self.ncn,
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
