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
    pub theoretical_last_slot: u64,
}

impl CompletedEpochBoundary {
    fn from_finalized_epoch_info(
        current_epoch: u64,
        absolute_slot: u64,
        slot_index: u64,
    ) -> Result<Self> {
        let theoretical_last_slot = absolute_slot
            .checked_sub(slot_index)
            .and_then(|epoch_start_slot| epoch_start_slot.checked_sub(1))
            .ok_or_else(|| anyhow!("cannot calculate the previous epoch boundary"))?;

        Ok(Self {
            epoch: current_epoch
                .checked_sub(1)
                .ok_or_else(|| anyhow!("epoch 0 has no previous epoch boundary"))?,
            theoretical_last_slot,
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

    pub async fn find_latest_finalized_block_slot(
        &self,
        boundary: &CompletedEpochBoundary,
    ) -> Result<Option<u64>> {
        for offset in 0..BOUNDARY_SEARCH_SLOTS {
            let Some(slot) = boundary.theoretical_last_slot.checked_sub(offset) else {
                break;
            };

            if self.finalized_block_exists(slot).await? {
                return Ok(Some(slot));
            }
        }

        Ok(None)
    }

    /// Returns false only for a null block or an RPC response that specifically
    /// identifies the slot as missing or skipped. All other RPC failures are
    /// retried indefinitely.
    pub async fn finalized_block_exists(&self, slot: u64) -> Result<bool> {
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
