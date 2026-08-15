use std::{cmp::min, time::Duration};

use anyhow::{anyhow, Result};
use solana_commitment_config::CommitmentConfig;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;

const POLL_INTERVAL: Duration = Duration::from_secs(2);
const RPC_TIMEOUT: Duration = Duration::from_secs(10);
const INITIAL_RPC_RETRY_DELAY: Duration = Duration::from_secs(1);
const MAX_RPC_RETRY_DELAY: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompletedEpochBoundary {
    pub epoch: u64,
    /// A finalized block in the epoch immediately following `epoch`.
    ///
    /// It is safe to snapshot at this slot without querying block history.
    pub snapshot_slot: u64,
}

impl CompletedEpochBoundary {
    fn from_finalized_epoch_info(current_epoch: u64, snapshot_slot: u64) -> Result<Self> {
        Ok(Self {
            epoch: current_epoch
                .checked_sub(1)
                .ok_or_else(|| anyhow!("epoch 0 has no previous epoch boundary"))?,
            snapshot_slot,
        })
    }
}

pub struct SolanaRpcClient {
    rpc: RpcClient,
}

impl SolanaRpcClient {
    pub fn new(rpc_url: String) -> Self {
        Self {
            rpc: RpcClient::new_with_timeout_and_commitment(
                rpc_url,
                RPC_TIMEOUT,
                CommitmentConfig::finalized(),
            ),
        }
    }

    /// Waits for the finalized epoch to advance and returns the boundary of the
    /// most recently completed epoch. Reading the initial epoch here ensures
    /// that startup and time spent creating a snapshot never trigger catch-up
    /// snapshots.
    pub async fn wait_for_epoch_boundary_final(&self) -> Result<CompletedEpochBoundary> {
        let initial_epoch_position = self.fetch_finalized_epoch_position().await;
        let initial_epoch = initial_epoch_position.epoch;
        log::info!("Waiting for finalized epoch {initial_epoch} to end");

        loop {
            tokio::time::sleep(POLL_INTERVAL).await;

            let current_epoch_position = self.fetch_finalized_epoch_position().await;
            if current_epoch_position.epoch > initial_epoch {
                return CompletedEpochBoundary::from_finalized_epoch_info(
                    current_epoch_position.epoch,
                    current_epoch_position.absolute_slot,
                );
            }
        }
    }

    async fn fetch_finalized_epoch_position(&self) -> FinalizedEpochPosition {
        let mut retry_delay = INITIAL_RPC_RETRY_DELAY;

        loop {
            match self.rpc.get_epoch_info().await {
                Ok(epoch_info) => {
                    return FinalizedEpochPosition {
                        epoch: epoch_info.epoch,
                        absolute_slot: epoch_info.absolute_slot,
                    };
                }
                Err(error) => {
                    log::warn!(
                        "RPC getEpochInfo failed: {error}; retrying in {}s",
                        retry_delay.as_secs()
                    );
                    tokio::time::sleep(retry_delay).await;
                    retry_delay = min(retry_delay.saturating_mul(2), MAX_RPC_RETRY_DELAY);
                }
            }
        }
    }
}

struct FinalizedEpochPosition {
    epoch: u64,
    absolute_slot: u64,
}

#[cfg(test)]
mod tests {
    use super::CompletedEpochBoundary;

    #[test]
    fn uses_the_first_observed_finalized_slot_after_the_epoch_rollover() {
        let boundary =
            CompletedEpochBoundary::from_finalized_epoch_info(1_234, 439_344_001).unwrap();

        assert_eq!(boundary.epoch, 1_233);
        assert_eq!(boundary.snapshot_slot, 439_344_001);
    }

    #[test]
    fn rejects_an_epoch_zero_boundary() {
        assert!(CompletedEpochBoundary::from_finalized_epoch_info(0, 0).is_err());
    }
}
