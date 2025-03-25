use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};

use clap_old::ArgMatches;
use log::{info, warn};
use solana_accounts_db::hardened_unpack::{
    open_genesis_config, OpenGenesisConfigError, MAX_GENESIS_ARCHIVE_UNPACKED_SIZE,
};
use solana_ledger::{
    blockstore::{Blockstore, BlockstoreError},
    blockstore_options::{AccessType, BlockstoreOptions},
    blockstore_processor::ProcessOptions,
};
use solana_metrics::{datapoint_error, datapoint_info};
use solana_runtime::{
    bank::Bank,
    snapshot_archive_info::SnapshotArchiveInfoGetter,
    snapshot_bank_utils,
    snapshot_config::SnapshotConfig,
    snapshot_utils::{self, get_full_snapshot_archives, SnapshotError, SnapshotVersion},
};
use solana_sdk::clock::Slot;
use thiserror::Error;

use crate::{arg_matches, load_and_process_ledger, Version};

#[derive(Error, Debug)]
pub enum LedgerUtilsError {
    #[error("BankFromSnapshot error: {0}")]
    BankFromSnapshotError(#[from] SnapshotError),
    #[error("Missing snapshot at slot {0}")]
    MissingSnapshotAtSlot(u64),
    #[error("BankFromSnapshot error: {0}")]
    OpenGenesisConfigError(#[from] OpenGenesisConfigError),
}

// TODO: Use Result and propagate errors more gracefully
/// Create the Bank for a desired slot for given file paths.
#[allow(clippy::cognitive_complexity, clippy::too_many_arguments)]
pub fn get_bank_from_ledger(
    operator_address: String,
    ledger_path: &Path,
    account_paths: Vec<PathBuf>,
    full_snapshots_path: PathBuf,
    incremental_snapshots_path: PathBuf,
    desired_slot: &Slot,
    save_snapshot: bool,
    snapshot_save_path: PathBuf,
) -> Arc<Bank> {
    let start_time = Instant::now();

    // Start validation
    datapoint_info!(
        "tip_router_cli.get_bank",
        ("operator", operator_address, String),
        ("state", "validate_path_start", String),
        ("step", 0, i64),
        ("version", Version::default().to_string(), String),
    );

    // STEP 1: Load genesis config //

    datapoint_info!(
        "tip_router_cli.get_bank",
        ("operator", operator_address, String),
        ("state", "load_genesis_start", String),
        ("step", 1, i64),
        ("duration_ms", start_time.elapsed().as_millis() as i64, i64),
    );

    let genesis_config = match open_genesis_config(ledger_path, MAX_GENESIS_ARCHIVE_UNPACKED_SIZE) {
        Ok(genesis_config) => genesis_config,
        Err(e) => {
            datapoint_error!(
                "tip_router_cli.get_bank",
                ("operator", operator_address, String),
                ("status", "error", String),
                ("state", "load_genesis", String),
                ("step", 1, i64),
                ("error", format!("{:?}", e), String),
            );
            panic!("Failed to load genesis config: {}", e); // TODO should panic here?
        }
    };

    // STEP 2: Load blockstore //

    datapoint_info!(
        "tip_router_cli.get_bank",
        ("operator", operator_address, String),
        ("state", "load_blockstore_start", String),
        ("step", 2, i64),
        ("duration_ms", start_time.elapsed().as_millis() as i64, i64),
    );

    let access_type = AccessType::Secondary;
    // Error handling is a modified copy pasta from ledger utils
    let blockstore = match Blockstore::open_with_options(
        ledger_path,
        BlockstoreOptions {
            access_type: access_type.clone(),
            ..BlockstoreOptions::default()
        },
    ) {
        Ok(blockstore) => blockstore,
        Err(BlockstoreError::RocksDb(err)) => {
            // Missing essential file, indicative of blockstore not existing
            let missing_blockstore = err
                .to_string()
                .starts_with("IO error: No such file or directory:");
            // Missing column in blockstore that is expected by software
            let missing_column = err
                .to_string()
                .starts_with("Invalid argument: Column family not found:");
            // The blockstore settings with Primary access can resolve the
            // above issues automatically, so only emit the help messages
            // if access type is Secondary
            let is_secondary = access_type == AccessType::Secondary;

            let error_str = if missing_blockstore && is_secondary {
                format!(
                    "Failed to open blockstore at {ledger_path:?}, it is missing at least one \
                     critical file: {err:?}"
                )
            } else if missing_column && is_secondary {
                format!(
                    "Failed to open blockstore at {ledger_path:?}, it does not have all necessary \
                     columns: {err:?}"
                )
            } else {
                format!("Failed to open blockstore at {ledger_path:?}: {err:?}")
            };
            datapoint_error!(
                "tip_router_cli.get_bank",
                ("operator", operator_address, String),
                ("status", "error", String),
                ("state", "load_blockstore", String),
                ("step", 2, i64),
                ("error", error_str, String),
                ("duration_ms", start_time.elapsed().as_millis() as i64, i64),
            );
            panic!("{}", error_str);
        }
        Err(err) => {
            let error_str = format!("Failed to open blockstore at {ledger_path:?}: {err:?}");
            datapoint_error!(
                "tip_router_cli.get_bank",
                ("operator", operator_address, String),
                ("status", "error", String),
                ("state", "load_blockstore", String),
                ("step", 2, i64),
                ("error", error_str, String),
                ("duration_ms", start_time.elapsed().as_millis() as i64, i64),
            );
            panic!("{}", error_str);
        }
    };

    let desired_slot_in_blockstore = match blockstore.meta(*desired_slot) {
        Ok(meta) => meta.is_some(),
        Err(err) => {
            warn!("Failed to get meta for slot {}: {:?}", desired_slot, err);
            false
        }
    };
    info!(
        "Desired slot {} in blockstore: {}",
        desired_slot, desired_slot_in_blockstore
    );

    // STEP 3: Load bank forks //

    datapoint_info!(
        "tip_router_cli.get_bank",
        ("operator", operator_address, String),
        ("state", "load_snapshot_config_start", String),
        ("step", 3, i64),
        ("duration_ms", start_time.elapsed().as_millis() as i64, i64),
    );

    let snapshot_config = SnapshotConfig {
        full_snapshot_archives_dir: full_snapshots_path.clone(),
        incremental_snapshot_archives_dir: incremental_snapshots_path.clone(),
        bank_snapshots_dir: full_snapshots_path.clone(),
        ..SnapshotConfig::new_load_only()
    };

    let process_options = ProcessOptions {
        halt_at_slot: Some(desired_slot.to_owned()),
        ..Default::default()
    };

    let mut starting_slot = 0; // default start check with genesis
    if let Some(full_snapshot_slot) = snapshot_utils::get_highest_full_snapshot_archive_slot(
        &full_snapshots_path,
        process_options.halt_at_slot,
    ) {
        let incremental_snapshot_slot =
            snapshot_utils::get_highest_incremental_snapshot_archive_slot(
                &incremental_snapshots_path,
                full_snapshot_slot,
                process_options.halt_at_slot,
            )
            .unwrap_or_default();
        starting_slot = std::cmp::max(full_snapshot_slot, incremental_snapshot_slot);
    }
    info!("Starting slot {}", starting_slot);

    match process_options.halt_at_slot {
        // Skip the following checks for sentinel values of Some(0) and None.
        // For Some(0), no slots will be be replayed after starting_slot.
        // For None, all available children of starting_slot will be replayed.
        None | Some(0) => {}
        Some(halt_slot) => {
            if halt_slot < starting_slot {
                let error_str = String::from("halt_slot < starting_slot");
                datapoint_error!(
                    "tip_router_cli.get_bank",
                    ("operator", operator_address, String),
                    ("status", "error", String),
                    ("state", "load_blockstore", String),
                    ("step", 2, i64),
                    ("error", error_str, String),
                    ("duration_ms", start_time.elapsed().as_millis() as i64, i64),
                );
                panic!("{}", error_str);
            }
            // Check if we have the slot data necessary to replay from starting_slot to >= halt_slot.
            if !blockstore.slot_range_connected(starting_slot, halt_slot) {
                let error_str =
                    format!("Blockstore missing data to replay to slot {}", desired_slot);
                datapoint_error!(
                    "tip_router_cli.get_bank",
                    ("operator", operator_address, String),
                    ("status", "error", String),
                    ("state", "load_blockstore", String),
                    ("step", 2, i64),
                    ("error", error_str, String),
                    ("duration_ms", start_time.elapsed().as_millis() as i64, i64),
                );
                panic!("{}", error_str);
            }
        }
    }
    let exit = Arc::new(AtomicBool::new(false));

    let mut arg_matches = ArgMatches::new();
    arg_matches::set_ledger_tool_arg_matches(
        &mut arg_matches,
        snapshot_config.full_snapshot_archives_dir.clone(),
        snapshot_config.incremental_snapshot_archives_dir.clone(),
        account_paths,
    );

    // Call ledger_utils::load_and_process_ledger here
    let (bank_forks, _starting_snapshot_hashes) =
        match load_and_process_ledger::load_and_process_ledger(
            &arg_matches,
            &genesis_config,
            Arc::new(blockstore),
            process_options,
            Some(full_snapshots_path),
            Some(incremental_snapshots_path),
            operator_address.clone(),
        ) {
            Ok(res) => res,
            Err(e) => {
                datapoint_error!(
                    "tip_router_cli.get_bank",
                    ("operator", operator_address, String),
                    ("state", "load_bank_forks", String),
                    ("status", "error", String),
                    ("step", 4, i64),
                    ("error", format!("{:?}", e), String),
                    ("duration_ms", start_time.elapsed().as_millis() as i64, i64),
                );
                panic!("Failed to load bank forks: {}", e);
            }
        };

    // let (bank_forks, leader_schedule_cache, _starting_snapshot_hashes, ..) =
    //     match bank_forks_utils::load_bank_forks(
    //         &genesis_config,
    //         &blockstore,
    //         account_paths,
    //         None,
    //         Some(&snapshot_config),
    //         &process_options,
    //         None,
    //         None, // Maybe support this later, though
    //         None,
    //         exit.clone(),
    //         false,
    //     ) {
    //         Ok(res) => res,
    //         Err(e) => {
    //             datapoint_error!(
    //                 "tip_router_cli.get_bank",
    //                 ("operator", operator_address.to_string(), String),
    //                 ("state", "load_bank_forks", String),
    //                 ("status", "error", String),
    //                 ("step", 4, i64),
    //                 ("error", format!("{:?}", e), String),
    //                 ("duration_ms", start_time.elapsed().as_millis() as i64, i64),
    //             );
    //             panic!("Failed to load bank forks: {}", e);
    //         }
    //     };

    // STEP 4: Process blockstore from root //

    // datapoint_info!(
    //     "tip_router_cli.get_bank",
    //     ("operator", operator_address.to_string(), String),
    //     ("state", "process_blockstore_from_root_start", String),
    //     ("step", 4, i64),
    //     ("duration_ms", start_time.elapsed().as_millis() as i64, i64),
    // );

    // match blockstore_processor::process_blockstore_from_root(
    //     &blockstore,
    //     &bank_forks,
    //     &leader_schedule_cache,
    //     &process_options,
    //     None,
    //     None,
    //     None,
    //     &AbsRequestSender::default(),
    // ) {
    //     Ok(()) => (),
    //     Err(e) => {
    //         datapoint_error!(
    //             "tip_router_cli.get_bank",
    //             ("operator", operator_address.to_string(), String),
    //             ("status", "error", String),
    //             ("state", "process_blockstore_from_root", String),
    //             ("step", 5, i64),
    //             ("error", format!("{:?}", e), String),
    //             ("duration_ms", start_time.elapsed().as_millis() as i64, i64),
    //         );
    //         panic!("Failed to process blockstore from root: {}", e);
    //     }
    // };

    // STEP 5: Save snapshot //

    let working_bank = bank_forks.read().unwrap().working_bank();

    datapoint_info!(
        "tip_router_cli.get_bank",
        ("operator", operator_address, String),
        ("state", "bank_to_full_snapshot_archive_start", String),
        ("bank_hash", working_bank.hash().to_string(), String),
        ("step", 5, i64),
        ("duration_ms", start_time.elapsed().as_millis() as i64, i64),
    );

    exit.store(true, Ordering::Relaxed);

    if save_snapshot {
        let full_snapshot_archive_info = match snapshot_bank_utils::bank_to_full_snapshot_archive(
            ledger_path,
            &working_bank,
            Some(SnapshotVersion::default()),
            // Use the snapshot_save_path path so the snapshot is stored in a directory different
            // than the node's primary snapshot directory
            snapshot_save_path,
            snapshot_config.incremental_snapshot_archives_dir,
            snapshot_config.archive_format,
        ) {
            Ok(res) => res,
            Err(e) => {
                datapoint_error!(
                    "tip_router_cli.get_bank",
                    ("operator", operator_address, String),
                    ("status", "error", String),
                    ("state", "bank_to_full_snapshot_archive", String),
                    ("step", 6, i64),
                    ("error", format!("{:?}", e), String),
                    ("duration_ms", start_time.elapsed().as_millis() as i64, i64),
                );
                panic!("Failed to create snapshot: {}", e);
            }
        };

        info!(
            "Successfully created snapshot for slot {}, hash {}: {}",
            working_bank.slot(),
            working_bank.hash(),
            full_snapshot_archive_info.path().display(),
        );
    }
    // STEP 6: Complete //

    assert_eq!(
        working_bank.slot(),
        *desired_slot,
        "expected working bank slot {}, found {}",
        desired_slot,
        working_bank.slot()
    );

    datapoint_info!(
        "tip_router_cli.get_bank",
        ("operator", operator_address, String),
        ("state", "get_bank_from_ledger_success", String),
        ("step", 6, i64),
        ("duration_ms", start_time.elapsed().as_millis() as i64, i64),
    );
    working_bank
}

/// Loads the bank from the snapshot at the exact slot. If the snapshot doesn't exist, result is
/// an error.
pub fn get_bank_from_snapshot_at_slot(
    snapshot_slot: u64,
    full_snapshots_path: &PathBuf,
    bank_snapshots_dir: &PathBuf,
    account_paths: Vec<PathBuf>,
    ledger_path: &Path,
) -> Result<Bank, LedgerUtilsError> {
    let mut full_snapshot_archives = get_full_snapshot_archives(full_snapshots_path);
    full_snapshot_archives.retain(|archive| archive.snapshot_archive_info().slot == snapshot_slot);

    if full_snapshot_archives.len() != 1 {
        return Err(LedgerUtilsError::MissingSnapshotAtSlot(snapshot_slot));
    }
    let full_snapshot_archive_info = full_snapshot_archives.first().expect("unreachable");
    let process_options = ProcessOptions {
        halt_at_slot: Some(snapshot_slot.to_owned()),
        ..Default::default()
    };
    let genesis_config = match open_genesis_config(ledger_path, MAX_GENESIS_ARCHIVE_UNPACKED_SIZE) {
        Ok(genesis_config) => genesis_config,
        Err(e) => return Err(e.into()),
    };
    let exit = Arc::new(AtomicBool::new(false));

    let (bank, _) = snapshot_bank_utils::bank_from_snapshot_archives(
        &account_paths,
        bank_snapshots_dir,
        full_snapshot_archive_info,
        None,
        &genesis_config,
        &process_options.runtime_config,
        process_options.debug_keys.clone(),
        None,
        process_options.limit_load_slot_count_from_snapshot,
        process_options.accounts_db_test_hash_calculation,
        process_options.accounts_db_skip_shrink,
        process_options.accounts_db_force_initial_clean,
        process_options.verify_index,
        process_options.accounts_db_config.clone(),
        None,
        exit.clone(),
    )?;
    exit.store(true, Ordering::Relaxed);
    Ok(bank)
}

#[cfg(test)]
mod tests {
    use crate::load_and_process_ledger::LEDGER_TOOL_DIRECTORY;

    use solana_sdk::pubkey::Pubkey;

    use super::*;

    #[test]
    fn test_get_bank_from_snapshot_at_slot() {
        let ledger_path = PathBuf::from("./tests/fixtures/test-ledger");
        let account_paths = vec![ledger_path.join("accounts/run")];
        let full_snapshots_path = ledger_path.clone();
        let snapshot_slot = 100;
        let bank = get_bank_from_snapshot_at_slot(
            snapshot_slot,
            &full_snapshots_path,
            &full_snapshots_path,
            account_paths,
            &ledger_path.as_path(),
        )
        .unwrap();
        assert_eq!(bank.slot(), snapshot_slot);
    }

    #[test]
    fn test_get_bank_from_snapshot_at_slot_snapshot_missing_error() {
        let ledger_path = PathBuf::from("./tests/fixtures/test-ledger");
        let account_paths = vec![ledger_path.join("accounts/run")];
        let full_snapshots_path = ledger_path.clone();
        let snapshot_slot = 105;
        let res = get_bank_from_snapshot_at_slot(
            snapshot_slot,
            &full_snapshots_path,
            &full_snapshots_path,
            account_paths,
            &ledger_path.as_path(),
        );
        assert!(res.is_err());
        let expected_err_str = format!("Missing snapshot at slot {}", snapshot_slot);
        assert_eq!(res.err().unwrap().to_string(), expected_err_str);
    }

    #[test]
    fn test_get_bank_from_ledger_success() {
        let operator_address = Pubkey::new_unique();
        let ledger_path = PathBuf::from("./tests/fixtures/test-ledger");
        let account_paths = vec![ledger_path.join("accounts/run")];
        let full_snapshots_path = ledger_path.clone();
        let desired_slot = 144;
        let res = get_bank_from_ledger(
            operator_address.to_string(),
            &ledger_path,
            account_paths,
            full_snapshots_path.clone(),
            full_snapshots_path.clone(),
            &desired_slot,
            true,
            full_snapshots_path.clone(),
        );
        assert_eq!(res.slot(), desired_slot);
        // Assert that the snapshot was created
        let snapshot_path_str = format!(
            "{}/snapshot-{}-{}.tar.zst",
            full_snapshots_path.to_str().unwrap(),
            desired_slot,
            res.get_accounts_hash().unwrap().0
        );
        let snapshot_path = Path::new(&snapshot_path_str);
        assert!(snapshot_path.exists());
        // Delete the snapshot
        std::fs::remove_file(snapshot_path).unwrap();
        std::fs::remove_dir_all(ledger_path.as_path().join(LEDGER_TOOL_DIRECTORY)).unwrap();
    }
}
