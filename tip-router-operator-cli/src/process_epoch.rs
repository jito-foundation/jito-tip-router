use std::{
    path::PathBuf,
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};

use anchor_lang::{AccountDeserialize, Discriminator};
use anyhow::Result;
use jito_tip_distribution_sdk::{
    jito_tip_distribution::accounts::ClaimStatus, TipDistributionAccount,
};
use log::{error, info};
use meta_merkle_tree::generated_merkle_tree::{GeneratedMerkleTreeCollection, StakeMetaCollection};
use solana_account_decoder::UiAccountEncoding;
use solana_metrics::{datapoint_error, datapoint_info};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_runtime::bank::Bank;
use solana_sdk::{
    account::Account,
    commitment_config::CommitmentConfig,
    epoch_info::EpochInfo,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
};
use tokio::time;
use solana_client::{rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig}, rpc_filter::{Memcmp, RpcFilterType}};
use jito_priority_fee_distribution_sdk::PriorityFeeDistributionAccount;
use crate::{
    backup_snapshots::SnapshotInfo, cli::SnapshotPaths, create_merkle_tree_collection,
    create_meta_merkle_tree, create_stake_meta, ledger_utils::get_bank_from_snapshot_at_slot,
    load_bank_from_snapshot, meta_merkle_tree_path, read_merkle_tree_collection,
    read_stake_meta_collection, submit::submit_to_ncn, tip_router::get_ncn_config, Cli,
    OperatorState, Version,
};

const MAX_WAIT_FOR_INCREMENTAL_SNAPSHOT_TICKS: u64 = 1200; // Experimentally determined
const OPTIMAL_INCREMENTAL_SNAPSHOT_SLOT_RANGE: u64 = 800; // Experimentally determined

pub async fn fetch_reclaimable_accounts(rpc_client: &RpcClient, tip_distribution_program_id: Pubkey, priority_fee_distribution_program_id: Pubkey) -> Result<(Vec<(Pubkey, Account)>, Vec<(Pubkey, Account)>)> {
    let tda_filter = vec![
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            0,
            TipDistributionAccount::DISCRIMINATOR.to_vec(),
        )),
    ];
    let tda_accounts = rpc_client
        .get_program_accounts_with_config(
            &tip_distribution_program_id,
            RpcProgramAccountsConfig {
                filters: Some(tda_filter),
                account_config: RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    ..RpcAccountInfoConfig::default()
                },
                ..RpcProgramAccountsConfig::default()
            },
        );

    let pfda_filter = vec![
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            0,
            PriorityFeeDistributionAccount::DISCRIMINATOR.to_vec(),
        )),
    ];
    let pfda_accounts = rpc_client.get_program_accounts_with_config(
        &priority_fee_distribution_program_id,
        RpcProgramAccountsConfig {
            filters: Some(pfda_filter),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                ..RpcAccountInfoConfig::default()
            },
            ..RpcProgramAccountsConfig::default()
        },
    );

    let (tda_accounts, pfda_accounts) = tokio::join!(tda_accounts, pfda_accounts);

    Ok((tda_accounts?, pfda_accounts?))
}

async fn fetch_reclaimable_claims(rpc_client: &RpcClient, tip_distribution_program_id: Pubkey, priority_fee_distribution_program_id: Pubkey, num_monitored_epochs: u64) -> Result<(Vec<(Pubkey, Account)>, Vec<(Pubkey, Account)>)> {
    let epochs_to_fetch = (815..816);
    let tip_distribution_claim_filters = epochs_to_fetch.clone().flat_map(|epoch: u64| {
        vec![
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
                0 ,
                jito_tip_distribution_sdk::jito_tip_distribution::accounts::ClaimStatus::DISCRIMINATOR.to_vec(),
            )),
            RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
                105,
                (epoch).to_le_bytes().to_vec(),
            )),
        ]
    }).collect();

    let tip_distribution_claim_accounts = rpc_client.get_program_accounts_with_config(
        &tip_distribution_program_id,
        RpcProgramAccountsConfig {
            filters: Some(tip_distribution_claim_filters),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                ..RpcAccountInfoConfig::default()
            },
            ..RpcProgramAccountsConfig::default()
        },
    );

    let priority_fee_distribution_claim_filters = epochs_to_fetch.clone().flat_map(|epoch: u64| {
        vec![
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            0,
            jito_priority_fee_distribution_sdk::jito_priority_fee_distribution::accounts::ClaimStatus::DISCRIMINATOR.to_vec(),
        )),
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            40,
            (epoch+10).to_le_bytes().to_vec(),
        )),
        ]
    }).collect();
    let priority_fee_distribution_claim_accounts = rpc_client.get_program_accounts_with_config(
        &priority_fee_distribution_program_id,
        RpcProgramAccountsConfig {
            filters: Some(priority_fee_distribution_claim_filters),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                ..RpcAccountInfoConfig::default()
            },
            ..RpcProgramAccountsConfig::default()
        },
    );

    let (tip_distribution_claim_accounts, priority_fee_distribution_claim_accounts) = tokio::join!(tip_distribution_claim_accounts, priority_fee_distribution_claim_accounts);

    Ok((tip_distribution_claim_accounts?, priority_fee_distribution_claim_accounts?))
}

#[cfg(feature = "close-expired-accounts")]
pub async fn close_expired_accounts(
    rpc_url: &str,
    tip_distribution_program_id: Pubkey,
    priority_fee_distribution_program_id: Pubkey,
    signer: Arc<Keypair>,
    max_loop_duration: Duration,
    // Optionally reclaim TipDistributionAccount rents on behalf of validators.
    should_reclaim_tdas: bool,
    micro_lamports: u64,
    num_monitored_epochs: u64,
) -> Result<()> {
    let rpc_client = RpcClient::new_with_timeout_and_commitment(
        rpc_url.to_string(),
        Duration::from_secs(1800),
        CommitmentConfig::processed(),
    );
    /*info!("Fetching TipDistributionAccounts and PriorityFeeDistributionAccounts");
    let start = Instant::now();
    let (tda_accounts, pfda_accounts) = fetch_reclaimable_accounts(&rpc_client, tip_distribution_program_id, priority_fee_distribution_program_id).await?;
    let duration = start.elapsed();
    info!("Found {} TipDistributionAccounts and {} PriorityFeeDistributionAccounts in {:?}", tda_accounts.len(), pfda_accounts.len(), duration);

    let current_epoch = rpc_client.get_epoch_info().await?.epoch;
    let expired_tda_accounts = find_expired_tda_accounts(&tda_accounts, current_epoch);
    info!("Found {} expired TipDistributionAccounts", expired_tda_accounts.len());

    let expired_pfda_accounts = find_expired_pfda_accounts(&pfda_accounts, current_epoch);
    info!("Found {} expired PriorityFeeDistributionAccounts", expired_pfda_accounts.len());*/
    
    info!("Fetching TipDistribution and PriorityFeeDistribution Claim Statuses");
    let start = Instant::now();
    let (tip_distribution_claim_accounts, priority_fee_distribution_claim_accounts) = fetch_reclaimable_claims(&rpc_client, tip_distribution_program_id, priority_fee_distribution_program_id, num_monitored_epochs).await?;
    let duration = start.elapsed();
    info!("Found {} TipDistribution Claim Statuses and {} PriorityFeeDistribution Claim Statuses in {:?}", tip_distribution_claim_accounts.len(), priority_fee_distribution_claim_accounts.len(), duration);

    Ok(())
}

fn find_expired_tda_accounts(accounts: &[(Pubkey, Account)], epoch: u64) -> Vec<Pubkey> {
    info!("Filtering expired TipDistributionAccounts");
    let mut expired_accounts = vec![];
    for (pubkey,  account) in accounts {
        let tip_distribution_account = TipDistributionAccount::try_deserialize(&mut account.data.as_slice());
        if let Ok(tip_distribution_account) = tip_distribution_account {
            if tip_distribution_account.expires_at < epoch {
                expired_accounts.push(*pubkey);
            }
        }
    }
    expired_accounts
}

fn find_expired_pfda_accounts(accounts: &[(Pubkey, Account)], epoch: u64) -> Vec<Pubkey> {
    info!("Filtering expired PriorityFeeDistributionAccounts");
    let mut expired_accounts = vec![];
    for (pubkey, account) in accounts {
        let priority_fee_distribution_account = PriorityFeeDistributionAccount::try_deserialize(&mut account.data.as_slice());
        if let Ok(priority_fee_distribution_account) = priority_fee_distribution_account {
            if priority_fee_distribution_account.expires_at < epoch {
                expired_accounts.push(*pubkey);
            }
        }
    }
    expired_accounts
}

pub async fn wait_for_next_epoch(rpc_client: &RpcClient, current_epoch: u64) -> EpochInfo {
    loop {
        tokio::time::sleep(Duration::from_secs(10)).await; // Check every 10 seconds
        let new_epoch_info = match rpc_client.get_epoch_info().await {
            Ok(info) => info,
            Err(e) => {
                error!("Error getting epoch info: {:?}", e);
                continue;
            }
        };

        if new_epoch_info.epoch > current_epoch {
            info!(
                "New epoch detected: {} -> {}",
                current_epoch, new_epoch_info.epoch
            );
            return new_epoch_info;
        }
    }
}

pub async fn get_previous_epoch_last_slot(rpc_client: &RpcClient) -> Result<(u64, u64)> {
    let epoch_info = rpc_client.get_epoch_info().await?;
    calc_prev_epoch_and_final_slot(&epoch_info)
}

pub fn calc_prev_epoch_and_final_slot(epoch_info: &EpochInfo) -> Result<(u64, u64)> {
    let current_slot = epoch_info.absolute_slot;
    let slot_index = epoch_info.slot_index;

    // Handle case where we're in the first epoch
    if current_slot < slot_index {
        return Ok((0, 0));
    }

    let epoch_start_slot = current_slot
        .checked_sub(slot_index)
        .ok_or_else(|| anyhow::anyhow!("epoch_start_slot subtraction overflow"))?;
    let previous_epoch_final_slot = epoch_start_slot.saturating_sub(1);
    let previous_epoch = epoch_info.epoch.saturating_sub(1);

    Ok((previous_epoch, previous_epoch_final_slot))
}

/// Wait for the optimal incremental snapshot to be available to speed up full snapshot generation
/// Automatically returns after MAX_WAIT_FOR_INCREMENTAL_SNAPSHOT_TICKS seconds
pub async fn wait_for_optimal_incremental_snapshot(
    incremental_snapshots_dir: PathBuf,
    target_slot: u64,
) -> Result<()> {
    let mut interval = time::interval(Duration::from_secs(1));
    let mut ticks = 0;

    while ticks < MAX_WAIT_FOR_INCREMENTAL_SNAPSHOT_TICKS {
        let dir_entries = std::fs::read_dir(&incremental_snapshots_dir)?;

        for entry in dir_entries {
            if let Some(snapshot_info) = SnapshotInfo::from_path(entry?.path()) {
                if target_slot - OPTIMAL_INCREMENTAL_SNAPSHOT_SLOT_RANGE < snapshot_info.end_slot
                    && snapshot_info.end_slot <= target_slot
                {
                    return Ok(());
                }
            }
        }

        interval.tick().await;
        ticks += 1;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn loop_stages(
    rpc_client: Arc<RpcClient>,
    cli: Cli,
    starting_stage: OperatorState,
    override_target_slot: Option<u64>,
    tip_router_program_id: &Pubkey,
    tip_distribution_program_id: &Pubkey,
    priority_fee_distribution_program_id: &Pubkey,
    tip_payment_program_id: &Pubkey,
    ncn_address: &Pubkey,
    enable_snapshots: bool,
    save_stages: bool,
) -> Result<()> {
    let keypair = read_keypair_file(&cli.keypair_path).expect("Failed to read keypair file");
    let mut current_epoch_info = rpc_client.get_epoch_info().await?;
    let epoch_schedule = rpc_client.get_epoch_schedule().await?;

    // Track runs that are starting right at the beginning of a new epoch
    let operator_address = cli.operator_address.clone();
    let mut stage = starting_stage;
    let mut bank: Option<Arc<Bank>> = None;
    let mut stake_meta_collection: Option<StakeMetaCollection> = None;
    let mut merkle_tree_collection: Option<GeneratedMerkleTreeCollection> = None;
    let mut epoch_to_process = current_epoch_info.epoch.saturating_sub(1);
    let mut slot_to_process = if let Some(slot) = override_target_slot {
        slot
    } else {
        let (_, prev_slot) = calc_prev_epoch_and_final_slot(&current_epoch_info)?;
        prev_slot
    };
    loop {
        match stage {
            OperatorState::LoadBankFromSnapshot => {
                info!("Ensuring localhost RPC is caught up with remote validator...");

                let try_catchup =
                    crate::solana_cli::catchup(cli.rpc_url.to_owned(), cli.localhost_port);
                if let Err(ref e) = try_catchup {
                    datapoint_error!(
                        "tip_router_cli.load_bank_from_snapshot",
                        ("operator_address", operator_address, String),
                        ("epoch", epoch_to_process, i64),
                        ("status", "error", String),
                        ("error", e.to_string(), String),
                        ("state", "load_bank_from_snapshot", String),
                        "cluster" => &cli.cluster,
                    );
                    error!("Failed to catch up: {}", e);
                }

                if let Ok(command_output) = try_catchup {
                    info!("{}", command_output);
                }
                let incremental_snapshots_path = cli.backup_snapshots_dir.clone();
                wait_for_optimal_incremental_snapshot(incremental_snapshots_path, slot_to_process)
                    .await?;

                if current_epoch_info.epoch
                    < legacy_tip_router_operator_cli::PRIORITY_FEE_MERKLE_TREE_START_EPOCH
                {
                    bank = Some(legacy_tip_router_operator_cli::load_bank_from_snapshot(
                        cli.as_legacy(),
                        slot_to_process,
                        enable_snapshots,
                    ));
                } else {
                    bank = Some(load_bank_from_snapshot(
                        cli.clone(),
                        slot_to_process,
                        enable_snapshots,
                    ));
                }
                // Transition to the next stage
                stage = OperatorState::CreateStakeMeta;
            }
            OperatorState::CreateStakeMeta => {
                let start = Instant::now();
                if bank.is_none() {
                    let SnapshotPaths {
                        ledger_path,
                        account_paths,
                        full_snapshots_path: _,
                        incremental_snapshots_path: _,
                        backup_snapshots_dir,
                    } = cli.get_snapshot_paths();
                    // We can safely expect to use the backup_snapshots_dir as the full snapshot path because
                    //  _get_bank_from_snapshot_at_slot_ expects the snapshot at the exact `slot` to have
                    //  already been taken.
                    let maybe_bank = get_bank_from_snapshot_at_slot(
                        slot_to_process,
                        &backup_snapshots_dir,
                        &backup_snapshots_dir,
                        account_paths,
                        ledger_path.as_path(),
                    );
                    match maybe_bank {
                        Ok(some_bank) => bank = Some(Arc::new(some_bank)),
                        Err(e) => {
                            datapoint_error!(
                                "tip_router_cli.create_stake_meta",
                                ("operator_address", operator_address, String),
                                ("epoch", epoch_to_process, i64),
                                ("status", "error", String),
                                ("error", e.to_string(), String),
                                ("state", "create_stake_meta", String),
                                ("duration_ms", start.elapsed().as_millis() as i64, i64),
                                "cluster" => &cli.cluster,
                            );
                            panic!("{}", e.to_string());
                        }
                    }
                }
                if current_epoch_info.epoch
                    < legacy_tip_router_operator_cli::PRIORITY_FEE_MERKLE_TREE_START_EPOCH
                {
                    stake_meta_collection = Some(StakeMetaCollection::from_legacy(
                        legacy_tip_router_operator_cli::create_stake_meta(
                            operator_address.clone(),
                            epoch_to_process,
                            bank.as_ref().expect("Bank was not set"),
                            tip_distribution_program_id,
                            tip_payment_program_id,
                            &cli.get_save_path(),
                            save_stages,
                            &cli.cluster,
                        ),
                    ));
                } else {
                    stake_meta_collection = Some(create_stake_meta(
                        operator_address.clone(),
                        epoch_to_process,
                        bank.as_ref().expect("Bank was not set"),
                        tip_distribution_program_id,
                        priority_fee_distribution_program_id,
                        tip_payment_program_id,
                        &cli.get_save_path(),
                        save_stages,
                        &cli.cluster,
                    ));
                }
                // we should be able to safely drop the bank in this loop
                bank = None;
                // Transition to the next stage
                stage = OperatorState::CreateMerkleTreeCollection;
            }
            OperatorState::CreateMerkleTreeCollection => {
                let config =
                    get_ncn_config(&rpc_client, tip_router_program_id, ncn_address).await?;
                // Tip Router looks backwards in time (typically current_epoch - 1) to calculated
                //  distributions. Meanwhile the NCN's Ballot is for the current_epoch. So we
                //  use epoch + 1 here
                let ballot_epoch = epoch_to_process.checked_add(1).unwrap();
                let fees = config.fee_config.current_fees(ballot_epoch);
                let protocol_fee_bps = config.fee_config.adjusted_total_fees_bps(ballot_epoch)?;

                // Generate the merkle tree collection
                if current_epoch_info.epoch
                    < legacy_tip_router_operator_cli::PRIORITY_FEE_MERKLE_TREE_START_EPOCH
                {
                    let some_stake_meta_collection = stake_meta_collection.to_owned().map_or_else(
                        || {
                            legacy_tip_router_operator_cli::read_stake_meta_collection(
                                epoch_to_process,
                                &cli.get_save_path(),
                            )
                        },
                        |collection| collection.to_legacy(),
                    );
                    merkle_tree_collection = Some(GeneratedMerkleTreeCollection::from_legacy(
                        legacy_tip_router_operator_cli::create_merkle_tree_collection(
                            cli.operator_address.clone(),
                            tip_router_program_id,
                            some_stake_meta_collection,
                            epoch_to_process,
                            ncn_address,
                            protocol_fee_bps,
                            &cli.get_save_path(),
                            save_stages,
                            &cli.cluster,
                        ),
                    ));
                } else {
                    let some_stake_meta_collection = stake_meta_collection.to_owned().map_or_else(
                        || read_stake_meta_collection(epoch_to_process, &cli.get_save_path()),
                        |collection| collection,
                    );
                    merkle_tree_collection = Some(create_merkle_tree_collection(
                        cli.operator_address.clone(),
                        tip_router_program_id,
                        some_stake_meta_collection,
                        epoch_to_process,
                        ncn_address,
                        protocol_fee_bps,
                        fees.priority_fee_distribution_fee_bps(),
                        &cli.get_save_path(),
                        save_stages,
                        &cli.cluster,
                    ));
                }

                stake_meta_collection = None;
                // Transition to the next stage
                stage = OperatorState::CreateMetaMerkleTree;
            }
            OperatorState::CreateMetaMerkleTree => {
                let merkle_root = if current_epoch_info.epoch
                    < legacy_tip_router_operator_cli::PRIORITY_FEE_MERKLE_TREE_START_EPOCH
                {
                    let some_merkle_tree_collection =
                        merkle_tree_collection.to_owned().map_or_else(
                            || {
                                legacy_tip_router_operator_cli::read_merkle_tree_collection(
                                    epoch_to_process,
                                    &cli.get_save_path(),
                                )
                            },
                            |collection| collection.to_legacy(),
                        );
                    let merkle_tree = legacy_tip_router_operator_cli::create_meta_merkle_tree(
                        cli.operator_address.clone(),
                        some_merkle_tree_collection,
                        epoch_to_process,
                        &cli.get_save_path(),
                        // This is defaulted to true because the output file is required by the
                        //  task that sets TipDistributionAccounts' merkle roots
                        true,
                        &cli.cluster,
                    );
                    merkle_tree.merkle_root
                } else {
                    let some_merkle_tree_collection =
                        merkle_tree_collection.to_owned().map_or_else(
                            || read_merkle_tree_collection(epoch_to_process, &cli.get_save_path()),
                            |collection| collection,
                        );
                    let merkle_tree = create_meta_merkle_tree(
                        cli.operator_address.clone(),
                        some_merkle_tree_collection,
                        epoch_to_process,
                        &cli.get_save_path(),
                        // This is defaulted to true because the output file is required by the
                        //  task that sets TipDistributionAccounts' merkle roots
                        true,
                        &cli.cluster,
                    );
                    merkle_tree.merkle_root
                };

                datapoint_info!(
                    "tip_router_cli.process_epoch",
                    ("operator_address", operator_address, String),
                    ("epoch", epoch_to_process, i64),
                    ("status", "success", String),
                    ("state", "epoch_processing_completed", String),
                    (
                        "meta_merkle_root",
                        format!("{:?}", merkle_root),
                        String
                    ),
                    ("version", Version::default().to_string(), String),
                    "cluster" => &cli.cluster,
                );
                stage = OperatorState::CastVote;
            }
            OperatorState::CastVote => {
                let meta_merkle_tree_path =
                    meta_merkle_tree_path(epoch_to_process, &cli.get_save_path());

                let operator_address = Pubkey::from_str(&cli.operator_address)?;
                submit_to_ncn(
                    &rpc_client,
                    &keypair,
                    &operator_address,
                    &meta_merkle_tree_path,
                    epoch_to_process,
                    ncn_address,
                    tip_router_program_id,
                    tip_distribution_program_id,
                    priority_fee_distribution_program_id,
                    cli.submit_as_memo,
                    // We let the submit task handle setting merkle roots
                    false,
                    cli.vote_microlamports,
                    &cli.cluster,
                )
                .await?;
                stage = OperatorState::WaitForNextEpoch;
            }
            OperatorState::WaitForNextEpoch => {
                current_epoch_info =
                    wait_for_next_epoch(&rpc_client, current_epoch_info.epoch).await;

                epoch_to_process = current_epoch_info.epoch - 1;
                slot_to_process = epoch_schedule.get_last_slot_in_epoch(epoch_to_process);

                stage = OperatorState::LoadBankFromSnapshot;
            }
        }
    }
}
