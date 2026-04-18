use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
};
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

    /// Keypair
    keypair: Arc<Keypair>,

    /// Tip Distribution Proram ID
    tip_distribution_program_id: Pubkey,

    /// Priority Fee Distribution Program ID
    priority_fee_distribution_program_id: Pubkey,

    /// Tip Distribution Program ID
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
    ) -> Result<Self, anyhow::Error> {
        let keypair = read_keypair_file(cli.keypair_path.clone())
            .map_err(|e| anyhow::anyhow!("Failed to read keypair file: {:?}", e))?;
        let keypair = Arc::new(keypair);

        Ok(Self {
            cli: Arc::new(cli),
            keypair,
            tip_distribution_program_id,
            priority_fee_distribution_program_id,
            tip_router_program_id,
            ncn,
            rpc_client,
            skipped_claimants: Arc::new(Mutex::new(HashMap::new())),
        })
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

    /// Claim MEV tips for `epoch`, running until all claims land or `max_loop_duration` elapses.
    pub async fn claim_mev_tips_with_emit(
        &self,
        epoch: u64,
        max_loop_duration: Duration,
        file_path: &PathBuf,
        file_mutex: &Arc<AsyncMutex<()>>,
    ) -> Result<(), anyhow::Error> {
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
            &self.keypair,
            rpc_url,
            self,
        )
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Commands;
    use solana_client::nonblocking::rpc_client::RpcClient;
    use solana_sdk::pubkey::Pubkey;

    /// Build a minimal ClaimProcessor without touching disk or network.
    /// The test module lives in the same file so it can access private fields.
    fn test_processor() -> ClaimProcessor {
        #[allow(deprecated)]
        let cli = Cli {
            keypair_path: String::new(),
            operator_address: String::new(),
            rpc_url: String::from("http://localhost:0"),
            ledger_path: PathBuf::new(),
            full_snapshots_path: None,
            backup_snapshots_dir: PathBuf::new(),
            snapshot_output_dir: PathBuf::new(),
            submit_as_memo: false,
            claim_microlamports: 1,
            min_claim_amount: 0,
            vote_microlamports: 1,
            save_path: None,
            claim_tips_epoch_filepath: PathBuf::new(),
            meta_merkle_tree_dir: None,
            cluster: String::from("test"),
            region: String::from("test"),
            localhost_port: 8899,
            heartbeat_interval_seconds: 900,
            command: Commands::SnapshotSlot { slot: 0 },
        };
        ClaimProcessor {
            cli: Arc::new(cli),
            keypair: Arc::new(Keypair::new()),
            tip_distribution_program_id: Pubkey::default(),
            priority_fee_distribution_program_id: Pubkey::default(),
            tip_router_program_id: Pubkey::default(),
            ncn: Pubkey::default(),
            rpc_client: Arc::new(RpcClient::new("http://localhost:0".to_string())),
            skipped_claimants: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    #[test]
    fn test_get_epoch_skipped_empty() {
        let p = test_processor();
        assert!(p.get_epoch_skipped(100).is_empty());
    }

    #[test]
    fn test_extend_and_get_epoch_skipped() {
        let p = test_processor();
        let pk1 = Pubkey::new_unique();
        let pk2 = Pubkey::new_unique();
        p.extend_epoch_skipped(100, [pk1, pk2]);
        let skipped = p.get_epoch_skipped(100);
        assert_eq!(skipped.len(), 2);
        assert!(skipped.contains(&pk1));
        assert!(skipped.contains(&pk2));
    }

    #[test]
    fn test_extend_epoch_skipped_is_additive() {
        let p = test_processor();
        let pk1 = Pubkey::new_unique();
        let pk2 = Pubkey::new_unique();
        p.extend_epoch_skipped(100, [pk1]);
        p.extend_epoch_skipped(100, [pk2]);
        assert_eq!(p.get_epoch_skipped(100).len(), 2);
    }

    #[test]
    fn test_extend_epoch_skipped_different_epochs_are_independent() {
        let p = test_processor();
        let pk = Pubkey::new_unique();
        p.extend_epoch_skipped(100, [pk]);
        assert!(p.get_epoch_skipped(101).is_empty());
    }

    #[test]
    fn test_clone_shares_skipped_claimants() {
        let p = test_processor();
        let clone = p.clone();
        let pk = Pubkey::new_unique();
        p.extend_epoch_skipped(100, [pk]);
        // The clone sees the same underlying map
        assert!(clone.get_epoch_skipped(100).contains(&pk));
    }

    #[test]
    fn test_remove_epoch_clears_skipped() {
        let p = test_processor();
        let pk = Pubkey::new_unique();
        p.extend_epoch_skipped(100, [pk]);
        // current_epoch = 102 → current_claim_epoch = 101 ≠ 100 → removes
        p.remove_epoch(100, 102).unwrap();
        assert!(p.get_epoch_skipped(100).is_empty());
    }

    #[test]
    fn test_remove_epoch_noop_for_current_claim_epoch() {
        let p = test_processor();
        let pk = Pubkey::new_unique();
        p.extend_epoch_skipped(100, [pk]);
        // current_epoch = 101 → current_claim_epoch = 100 == target → no-op
        p.remove_epoch(100, 101).unwrap();
        assert!(p.get_epoch_skipped(100).contains(&pk));
    }

    #[test]
    fn test_remove_epoch_underflow_returns_error() {
        let p = test_processor();
        assert!(p.remove_epoch(0, 0).is_err());
    }
}
