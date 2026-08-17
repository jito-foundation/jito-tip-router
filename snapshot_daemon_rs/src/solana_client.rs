use std::{cmp::min, time::Duration};

use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use solana_commitment_config::CommitmentConfig;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::{
    client_error::{Error as ClientError, ErrorKind},
    config::{RpcBlockConfig, TransactionDetails},
    custom_error::{
        JSON_RPC_SERVER_ERROR_BLOCK_NOT_AVAILABLE,
        JSON_RPC_SERVER_ERROR_LONG_TERM_STORAGE_SLOT_SKIPPED, JSON_RPC_SERVER_ERROR_SLOT_SKIPPED,
    },
    request::{RpcError, RpcRequest},
};

const BOUNDARY_SEARCH_SLOTS: u64 = 16;
const POLL_INTERVAL: Duration = Duration::from_secs(2);
const RPC_TIMEOUT: Duration = Duration::from_secs(10);
const INITIAL_RPC_RETRY_DELAY: Duration = Duration::from_secs(1);
const MAX_RPC_RETRY_DELAY: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompletedEpochBoundary {
    pub epoch: u64,
    /// The theoretical final slot of `epoch`. The actual snapshot slot is the
    /// latest finalized block at or before this slot.
    pub theoretical_last_slot: u64,
}

impl CompletedEpochBoundary {
    fn from_finalized_epoch_info(
        current_epoch: u64,
        absolute_slot: u64,
        slot_index: u64,
    ) -> Result<Self> {
        let current_epoch_first_slot = absolute_slot
            .checked_sub(slot_index)
            .ok_or_else(|| anyhow!("cannot calculate the current epoch's first slot"))?;
        Self::from_epoch_first_slot(current_epoch, current_epoch_first_slot)
    }

    fn from_epoch_first_slot(current_epoch: u64, current_epoch_first_slot: u64) -> Result<Self> {
        Ok(Self {
            epoch: current_epoch
                .checked_sub(1)
                .ok_or_else(|| anyhow!("epoch 0 has no previous epoch boundary"))?,
            theoretical_last_slot: current_epoch_first_slot
                .checked_sub(1)
                .ok_or_else(|| anyhow!("cannot calculate the previous epoch's last slot"))?,
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
                    current_epoch_position.slot_index,
                );
            }
        }
    }

    /// Finds the latest finalized slot containing a block within the boundary
    /// search window. Missing and skipped slots are valid candidates to skip.
    pub async fn find_latest_finalized_block_slot(
        &self,
        boundary: &CompletedEpochBoundary,
    ) -> Result<Option<u64>> {
        let oldest_candidate = boundary
            .theoretical_last_slot
            .saturating_sub(BOUNDARY_SEARCH_SLOTS - 1);
        log::info!(
            "Searching for epoch {} boundary bank from slot {} through {oldest_candidate}",
            boundary.epoch,
            boundary.theoretical_last_slot,
        );

        for slot in boundary_candidate_slots(boundary.theoretical_last_slot) {
            if self.finalized_block_exists(slot).await? {
                return Ok(Some(slot));
            }
            log::info!("Boundary candidate slot {slot} has no finalized block");
        }

        Ok(None)
    }

    /// Returns false only for a null block or an RPC response that specifically
    /// identifies the slot as missing or skipped. All other RPC failures are
    /// retried indefinitely.
    async fn finalized_block_exists(&self, slot: u64) -> Result<bool> {
        let config = RpcBlockConfig {
            transaction_details: Some(TransactionDetails::None),
            rewards: Some(false),
            commitment: Some(CommitmentConfig::finalized()),
            ..RpcBlockConfig::default()
        };
        let mut retry_delay = INITIAL_RPC_RETRY_DELAY;

        loop {
            let response = self
                .rpc
                .send::<Option<Value>>(RpcRequest::GetBlock, json!([slot, config]))
                .await;

            match response {
                Ok(block) => return Ok(block.is_some()),
                Err(error) if is_missing_or_skipped_slot(&error) => return Ok(false),
                Err(error) => {
                    log::warn!(
                        "RPC getBlock({slot}) failed: {error}; retrying in {}s",
                        retry_delay.as_secs()
                    );
                    tokio::time::sleep(retry_delay).await;
                    retry_delay = min(retry_delay.saturating_mul(2), MAX_RPC_RETRY_DELAY);
                }
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
    /// theoretical final slot of the completed epoch.
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
            let epoch_first_slot = epoch_schedule.get_first_slot_in_epoch(epoch);
            if epoch_first_slot > start_slot
                && epoch_first_slot <= current_epoch_position.absolute_slot
            {
                boundaries.push(CompletedEpochBoundary::from_epoch_first_slot(
                    epoch,
                    epoch_first_slot,
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
                        slot_index: epoch_info.slot_index,
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
    slot_index: u64,
}

fn boundary_candidate_slots(theoretical_last_slot: u64) -> impl Iterator<Item = u64> {
    (0..BOUNDARY_SEARCH_SLOTS).map_while(move |offset| theoretical_last_slot.checked_sub(offset))
}

fn is_missing_or_skipped_slot(error: &ClientError) -> bool {
    matches!(
        error.kind(),
        ErrorKind::RpcError(RpcError::RpcResponseError { code, .. })
            if matches!(
                *code,
                JSON_RPC_SERVER_ERROR_BLOCK_NOT_AVAILABLE
                    | JSON_RPC_SERVER_ERROR_SLOT_SKIPPED
                    | JSON_RPC_SERVER_ERROR_LONG_TERM_STORAGE_SLOT_SKIPPED
            )
    )
}

fn finalized_slot_target(initial_slot: u64, slots_ahead: u64) -> Result<u64> {
    initial_slot.checked_add(slots_ahead).ok_or_else(|| {
        anyhow!("cannot add test slot offset {slots_ahead} to finalized slot {initial_slot}")
    })
}

#[cfg(test)]
mod tests {
    use super::{boundary_candidate_slots, finalized_slot_target, CompletedEpochBoundary};

    #[test]
    fn calculates_previous_epoch_last_slot_after_late_rollover_observation() {
        let boundary =
            CompletedEpochBoundary::from_finalized_epoch_info(1_018, 439_776_004, 4).unwrap();

        assert_eq!(boundary.epoch, 1_017);
        assert_eq!(boundary.theoretical_last_slot, 439_775_999);
    }

    #[test]
    fn rejects_an_epoch_zero_boundary() {
        assert!(CompletedEpochBoundary::from_finalized_epoch_info(0, 0, 0).is_err());
    }

    #[test]
    fn searches_exactly_sixteen_boundary_slots() {
        let slots = boundary_candidate_slots(100).collect::<Vec<_>>();

        assert_eq!(slots, (85..=100).rev().collect::<Vec<_>>());
    }

    #[test]
    fn boundary_search_does_not_underflow() {
        assert_eq!(
            boundary_candidate_slots(2).collect::<Vec<_>>(),
            vec![2, 1, 0]
        );
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
