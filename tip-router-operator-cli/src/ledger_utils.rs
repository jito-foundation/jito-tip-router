use std::{
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc},
};

use log::info;
use solana_accounts_db::hardened_unpack::{open_genesis_config, MAX_GENESIS_ARCHIVE_UNPACKED_SIZE};
use solana_ledger::{
    bank_forks_utils::{self},
    blockstore::{Blockstore, BlockstoreError},
    blockstore_options::{AccessType, BlockstoreOptions},
    blockstore_processor::{self, ProcessOptions},
};
use solana_runtime::{
    accounts_background_service::AbsRequestSender, bank::Bank,
    snapshot_archive_info::SnapshotArchiveInfoGetter, snapshot_bank_utils,
    snapshot_config::SnapshotConfig, snapshot_utils::SnapshotVersion,
};
use solana_sdk::clock::Slot;

// TODO: Use Result and propagate errors more gracefully
/// Create the Bank for a desired slot for given file paths.
pub fn get_bank_from_ledger(
    ledger_path: &Path,
    account_paths: Vec<PathBuf>,
    full_snapshots_path: PathBuf,
    desired_slot: &Slot,
    take_snapshot: bool,
) -> Arc<Bank> {
    let genesis_config =
        open_genesis_config(ledger_path, MAX_GENESIS_ARCHIVE_UNPACKED_SIZE).unwrap();
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

            if missing_blockstore && is_secondary {
                panic!(
                    "Failed to open blockstore at {ledger_path:?}, it is missing at least one \
                     critical file: {err:?}"
                );
            } else if missing_column && is_secondary {
                panic!(
                    "Failed to open blockstore at {ledger_path:?}, it does not have all necessary \
                     columns: {err:?}"
                );
            } else {
                panic!("Failed to open blockstore at {ledger_path:?}: {err:?}");
            }
        }
        Err(err) => {
            panic!("Failed to open blockstore at {ledger_path:?}: {err:?}");
        }
    };

    let snapshot_config = SnapshotConfig {
        full_snapshot_archives_dir: full_snapshots_path.clone(),
        incremental_snapshot_archives_dir: full_snapshots_path.clone(),
        bank_snapshots_dir: full_snapshots_path,
        ..SnapshotConfig::new_load_only()
    };

    let process_options = ProcessOptions {
        halt_at_slot: Some(desired_slot.to_owned()),
        ..Default::default()
    };
    let exit = Arc::new(AtomicBool::new(false));
    let (bank_forks, leader_schedule_cache, _starting_snapshot_hashes, ..) =
        bank_forks_utils::load_bank_forks(
            &genesis_config,
            &blockstore,
            account_paths,
            None,
            Some(&snapshot_config),
            &process_options,
            None,
            None, // Maybe support this later, though
            None,
            exit,
            false,
        )
        .unwrap();
    blockstore_processor::process_blockstore_from_root(
        &blockstore,
        &bank_forks,
        &leader_schedule_cache,
        &process_options,
        None,
        None,
        None,
        &AbsRequestSender::default(),
    )
    .unwrap();

    let working_bank = bank_forks.read().unwrap().working_bank();

    if take_snapshot {
        let full_snapshot_archive_info = snapshot_bank_utils::bank_to_full_snapshot_archive(
            ledger_path,
            &working_bank,
            Some(SnapshotVersion::default()),
            snapshot_config.full_snapshot_archives_dir,
            snapshot_config.incremental_snapshot_archives_dir,
            snapshot_config.archive_format,
            snapshot_config.maximum_full_snapshot_archives_to_retain,
            snapshot_config.maximum_incremental_snapshot_archives_to_retain,
        )
        .unwrap();

        info!(
            "Successfully created snapshot for slot {}, hash {}: {}",
            working_bank.slot(),
            working_bank.hash(),
            full_snapshot_archive_info.path().display(),
        );
    }

    assert_eq!(
        working_bank.slot(),
        *desired_slot,
        "expected working bank slot {}, found {}",
        desired_slot,
        working_bank.slot()
    );
    working_bank
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_bank_from_ledger_success() {
        let ledger_path = PathBuf::from("./tests/fixtures/test-ledger");
        let account_paths = vec![ledger_path.join("accounts/run")];
        let full_snapshots_path = ledger_path.clone();
        let desired_slot = 144;
        let res = get_bank_from_ledger(
            &ledger_path,
            account_paths,
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
