use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Write},
    path::PathBuf,
};
use regex::Regex;

use ellipsis_client::{EllipsisClient, EllipsisClientError};
use log::info;
use serde::{Deserialize, Serialize};
use solana_client::{
    client_error::{ClientError, ClientErrorKind},
    rpc_config::RpcBlockConfig,
    rpc_request::RpcError,
};
use solana_sdk::reward_type::RewardType;
use solana_transaction_status::{TransactionDetails, UiConfirmedBlock, UiTransactionEncoding};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PriorityFeeUtilsError {
    #[error("EllipsisClient error: {0}")]
    EllipsisClientError(#[from] EllipsisClientError),
    #[error("SoloanaClientError error: {0}")]
    SoloanaClientError(#[from] ClientError),
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
    #[error(transparent)]
    RpcError(#[from] RpcError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error("No leader schedule for epoch found")]
    ErrorGettingLeaderSchedule,
    #[error("Block was skipped")]
    SkippedBlock,
}

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct LeaderEpochPriorityFees {
    epoch: u64,
    pub leader_priority_fee_map: HashMap<String, u64>,
}

impl LeaderEpochPriorityFees {
    /// Load a serialized LeaderEpochPriorityFees from file path
    pub fn new_from_file(path: &PathBuf) -> Result<Self, PriorityFeeUtilsError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let tree: Self = serde_json::from_reader(reader)?;

        Ok(tree)
    }
    /// Write a LeaderEpochPriorityFees to a filepath
    pub fn write_to_file(&self, path: &PathBuf) -> Result<(), PriorityFeeUtilsError> {
        let serialized = serde_json::to_string_pretty(&self)?;
        let mut file = File::create(path)?;
        file.write_all(serialized.as_bytes())?;
        Ok(())
    }
}

pub async fn get_priority_fees_for_epoch(
    client: &EllipsisClient,
    epoch: u64,
) -> Result<LeaderEpochPriorityFees, PriorityFeeUtilsError> {
    // Get the start and ending slot of the epoch
    let epoch_schedule = client.get_epoch_schedule().await?;
    let starting_slot = epoch_schedule.get_first_slot_in_epoch(epoch);
    // Get the leader schedule for the epoch
    let maybe_leader_schedule = client.get_leader_schedule(Some(starting_slot)).await?;
    let leader_schedule = match maybe_leader_schedule {
        Some(schedule) => schedule,
        None => return Err(PriorityFeeUtilsError::ErrorGettingLeaderSchedule),
    };

    let mut res: HashMap<String, u64> = HashMap::with_capacity(leader_schedule.len());

    for (leader, relative_leader_slots) in leader_schedule.into_iter() {
        let mut leader_epoch_block_rewards: i64 = 0;
        for relative_slot in relative_leader_slots.into_iter() {
            // adjust the relative_slot to the canonical slot
            let slot = starting_slot.saturating_add(relative_slot as u64);
            let block_res = get_block(client, slot).await;
            let block = match block_res {
                Ok(block) => block,
                Err(err) => match err {
                    PriorityFeeUtilsError::SkippedBlock => {
                        info!("Slot {} skipped", slot);
                        continue;
                    }
                    _ => return Err(err),
                },
            };
            // get the priority fee rewards for the block.
            let maybe_block_rewards = block
                .rewards
                .unwrap()
                .into_iter()
                .find(|r| r.reward_type == Some(RewardType::Fee));
            match maybe_block_rewards {
                Some(block_reward) => {
                    leader_epoch_block_rewards = leader_epoch_block_rewards.saturating_add(block_reward.lamports);
                },
                None => {}
            }
        }
        res.insert(
            leader,
            u64::try_from(leader_epoch_block_rewards).unwrap_or(0),
        );
    }
    Ok(LeaderEpochPriorityFees {
        epoch,
        leader_priority_fee_map: res,
    })
}

async fn get_block(
    client: &EllipsisClient,
    slot: u64,
) -> Result<UiConfirmedBlock, PriorityFeeUtilsError> {
    let block_res = client
        .get_block_with_config(
            slot,
            RpcBlockConfig {
                encoding: Some(UiTransactionEncoding::Json),
                transaction_details: Some(TransactionDetails::None),
                rewards: Some(true),
                commitment: None,
                max_supported_transaction_version: Some(0),
            },
        )
        .await;
    let block = match block_res {
        Ok(block) => block,
        Err(err) => match err.kind {
            ClientErrorKind::RpcError(client_rpc_err) => match client_rpc_err {
                RpcError::RpcResponseError {
                    code,
                    message,
                    data,
                } => {
                    let slot_skipped_regex = Regex::new(r"^Slot [\d]+ was skipped").unwrap();
                    if slot_skipped_regex.is_match(&message) {
                        return Err(PriorityFeeUtilsError::SkippedBlock);
                    }
                    return Err(PriorityFeeUtilsError::RpcError(
                        RpcError::RpcResponseError {
                            code,
                            message,
                            data,
                        },
                    ));
                }
                _ => return Err(PriorityFeeUtilsError::RpcError(client_rpc_err)),
            },
            _ => return Err(PriorityFeeUtilsError::SoloanaClientError(err)),
        },
    };
    Ok(block)
}
