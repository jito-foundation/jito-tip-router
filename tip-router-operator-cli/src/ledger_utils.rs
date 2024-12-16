use solana_accounts_db::hardened_unpack::{open_genesis_config, MAX_GENESIS_ARCHIVE_UNPACKED_SIZE};
use solana_ledger::{
    bank_forks_utils::{self},
    blockstore::{Blockstore, BlockstoreError},
    blockstore_options::{AccessType, BlockstoreOptions},
    blockstore_processor::{self, ProcessOptions},
};
use solana_runtime::{
    accounts_background_service::AbsRequestSender,
    bank::Bank,
    snapshot_bank_utils,
    snapshot_config::{SnapshotConfig, SnapshotUsage},
};
use solana_sdk::{clock::Slot, genesis_config::GenesisConfig};

use std::{
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc},
};

// TODO: Use Result and propagate errors more gracefully
// TODO: Handle CLI flag to write snapshot to disk at desired slot
/// Create the Bank for a desired slot for given file paths.
pub fn get_bank_from_ledger(
    ledger_path: &Path,
    account_paths: Vec<PathBuf>,
    full_snapshots_path: PathBuf,
    desired_slot: &Slot,
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

    let mut process_options = ProcessOptions::default();
    process_options.halt_at_slot = Some(desired_slot.to_owned());
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
            exit.clone(),
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

    let bank_forks_read = bank_forks.read().unwrap();
    let working_bank: Arc<Bank> = bank_forks_read.working_bank();

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
        let full_snapshots_path = PathBuf::from(ledger_path.clone());
        let desired_slot = 144;
        let res = get_bank_from_ledger(
            &ledger_path,
            account_paths,
            full_snapshots_path,
            &desired_slot,
        );
        assert_eq!(res.slot(), desired_slot);
    }
}
