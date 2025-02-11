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
use solana_accounts_db::hardened_unpack::{open_genesis_config, MAX_GENESIS_ARCHIVE_UNPACKED_SIZE};
use solana_ledger::{
    blockstore::{Blockstore, BlockstoreError},
    blockstore_options::{AccessType, BlockstoreOptions},
    blockstore_processor::ProcessOptions,
};
use solana_metrics::{datapoint_error, datapoint_info};
use solana_runtime::{
    bank::Bank, snapshot_archive_info::SnapshotArchiveInfoGetter, snapshot_bank_utils,
    snapshot_config::SnapshotConfig, snapshot_utils::SnapshotVersion,
};
use solana_sdk::{clock::Slot, pubkey::Pubkey};

use crate::{arg_matches, load_and_process_ledger};

// TODO: Use Result and propagate errors more gracefully
/// Create the Bank for a desired slot for given file paths.
pub fn get_bank_from_ledger(
    operator_address: &Pubkey,
    ledger_path: &Path,
    account_paths: Vec<PathBuf>,
    full_snapshots_path: PathBuf,
    incremental_snapshots_path: PathBuf,
    desired_slot: &Slot,
    take_snapshot: bool,
) -> Arc<Bank> {
    let start_time = Instant::now();

    // Start validation
    datapoint_info!(
        "tip_router_cli.get_bank",
        ("operator", operator_address.to_string(), String),
        ("state", "validate_path_start", String),
        ("step", 0, i64),
    );

    // STEP 1: Load genesis config //

    datapoint_info!(
        "tip_router_cli.get_bank",
        ("operator", operator_address.to_string(), String),
        ("state", "load_genesis_start", String),
        ("step", 1, i64),
        ("duration_ms", start_time.elapsed().as_millis() as i64, i64),
    );

    let genesis_config = match open_genesis_config(ledger_path, MAX_GENESIS_ARCHIVE_UNPACKED_SIZE) {
        Ok(genesis_config) => genesis_config,
        Err(e) => {
            datapoint_error!(
                "tip_router_cli.get_bank",
                ("operator", operator_address.to_string(), String),
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
        ("operator", operator_address.to_string(), String),
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
                ("operator", operator_address.to_string(), String),
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
                ("operator", operator_address.to_string(), String),
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
        ("operator", operator_address.to_string(), String),
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
            operator_address,
        ) {
            Ok(res) => res,
            Err(e) => {
                datapoint_error!(
                    "tip_router_cli.get_bank",
                    ("operator", operator_address.to_string(), String),
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
        ("operator", operator_address.to_string(), String),
        ("state", "bank_to_full_snapshot_archive_start", String),
        ("bank_hash", working_bank.hash().to_string(), String),
        ("step", 5, i64),
        ("duration_ms", start_time.elapsed().as_millis() as i64, i64),
    );

    exit.store(true, Ordering::Relaxed);

    if take_snapshot {
        let full_snapshot_archive_info = match snapshot_bank_utils::bank_to_full_snapshot_archive(
            ledger_path,
            &working_bank,
            Some(SnapshotVersion::default()),
            snapshot_config.full_snapshot_archives_dir,
            snapshot_config.incremental_snapshot_archives_dir,
            snapshot_config.archive_format,
            snapshot_config.maximum_full_snapshot_archives_to_retain,
            snapshot_config.maximum_incremental_snapshot_archives_to_retain,
        ) {
            Ok(res) => res,
            Err(e) => {
                datapoint_error!(
                    "tip_router_cli.get_bank",
                    ("operator", operator_address.to_string(), String),
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
        ("operator", operator_address.to_string(), String),
        ("state", "get_bank_from_ledger_success", String),
        ("step", 6, i64),
        ("duration_ms", start_time.elapsed().as_millis() as i64, i64),
    );
    working_bank
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_bank_from_ledger_success() {
        let operator_address = Pubkey::new_unique();
        let ledger_path = PathBuf::from("./tests/fixtures/test-ledger");
        let account_paths = vec![ledger_path.join("accounts/run")];
        let full_snapshots_path = ledger_path.clone();
        let desired_slot = 144;
        let res = get_bank_from_ledger(
            &operator_address,
            &ledger_path,
            account_paths,
            full_snapshots_path.clone(),
            full_snapshots_path.clone(),
            &desired_slot,
            true,
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
        std::fs::remove_dir_all(
            ledger_path
                .as_path()
                .join(format!("accounts/snapshot/{}", desired_slot)),
        )
        .unwrap();
    }
}
