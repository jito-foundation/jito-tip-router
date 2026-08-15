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
    /// A snapshot target in the epoch immediately following `epoch`.
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

    /// Waits until the finalized slot has advanced by at least `slots_ahead`
    /// slot numbers, then returns the first observed finalized slot at or after
    /// that target.
    pub async fn wait_for_finalized_slots_ahead(&self, slots_ahead: u64) -> Result<u64> {
        let initial_epoch_position = self.fetch_finalized_epoch_position().await;
        let initial_slot = initial_epoch_position.absolute_slot;
        let target_slot = finalized_slot_target(initial_slot, slots_ahead)?;
        log::info!(
            "Waiting for finalized slot {target_slot} ({slots_ahead} slots after startup finalized slot {initial_slot})"
        );

        if slots_ahead == 0 {
            return Ok(initial_slot);
        }

        loop {
            tokio::time::sleep(POLL_INTERVAL).await;

            let current_epoch_position = self.fetch_finalized_epoch_position().await;
            if current_epoch_position.absolute_slot >= target_slot {
                return Ok(current_epoch_position.absolute_slot);
            }
        }
    }

    /// Returns every completed epoch boundary strictly after `start_slot` that
    /// has been reached by the current finalized slot. Each target is the
    /// exact first slot of its following epoch and may be skipped.
    pub async fn completed_epoch_boundaries_since(
        &self,
        start_slot: u64,
    ) -> Result<Vec<CompletedEpochBoundary>> {
        let current_epoch_position = self.fetch_finalized_epoch_position().await;
        let mut retry_delay = INITIAL_RPC_RETRY_DELAY;
        let epoch_schedule = loop {
            match self.rpc.get_epoch_schedule().await {
                Ok(epoch_schedule) => break epoch_schedule,
                Err(error) => {
                    log::warn!(
                        "RPC getEpochSchedule failed: {error}; retrying in {}s",
                        retry_delay.as_secs()
                    );
                    tokio::time::sleep(retry_delay).await;
                    retry_delay = min(retry_delay.saturating_mul(2), MAX_RPC_RETRY_DELAY);
                }
            }
        };
        let start_epoch = epoch_schedule.get_epoch(start_slot);

        let mut boundaries = Vec::new();
        for epoch in start_epoch.saturating_add(1)..=current_epoch_position.epoch {
            let snapshot_slot = epoch_schedule.get_first_slot_in_epoch(epoch);
            if snapshot_slot > start_slot && snapshot_slot <= current_epoch_position.absolute_slot {
                boundaries.push(CompletedEpochBoundary::from_finalized_epoch_info(
                    epoch,
                    snapshot_slot,
                )?);
            }
        }

        Ok(boundaries)
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

fn finalized_slot_target(initial_slot: u64, slots_ahead: u64) -> Result<u64> {
    initial_slot.checked_add(slots_ahead).ok_or_else(|| {
        anyhow!("cannot add test slot offset {slots_ahead} to finalized slot {initial_slot}")
    })
}

#[cfg(test)]
mod tests {
    use super::{finalized_slot_target, CompletedEpochBoundary};

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

    #[test]
    fn calculates_the_finalized_slot_test_target() {
        assert_eq!(
            finalized_slot_target(439_343_998, 100).unwrap(),
            439_344_098
        );
    }

    #[test]
    fn rejects_a_finalized_slot_test_target_that_overflows() {
        assert!(finalized_slot_target(u64::MAX, 1).is_err());
    }
}
