use std::path::{Path, PathBuf};

use notify::{
    event::{ModifyKind, RenameMode},
    Event, EventKind, RecursiveMode, Watcher,
};
use thiserror::Error;

use crate::stake_meta_file_candidates;
use meta_merkle_tree::generated_merkle_tree::StakeMetaCollection;

#[derive(Debug, Error)]
pub enum DirectoryWatcherError {
    #[error("directory watcher failed: {0}")]
    Watch(#[from] notify::Error),
    #[error("failed to load stake-meta file {stake_meta_path}: {source}")]
    StakeMetaLoad {
        stake_meta_path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(
        "stake-meta file {stake_meta_path} is for epoch {actual_epoch}, expected epoch {expected_epoch}"
    )]
    UnexpectedEpoch {
        stake_meta_path: PathBuf,
        expected_epoch: u64,
        actual_epoch: u64,
    },
    #[error("directory watcher event channel closed")]
    EventChannelClosed,
}

/// Returns the published stake-meta path from a create or rename-to event.
///
/// Stake-meta producers publish a temporary file before renaming it to its final name, so files
/// containing `tmp` are deliberately ignored.
fn handle_file_event(event: Event, expected_file_names: &[String; 2]) -> Option<PathBuf> {
    let is_file_created = matches!(
        event.kind,
        EventKind::Create(_)
            | EventKind::Modify(ModifyKind::Name(RenameMode::To | RenameMode::Both))
    );
    if !is_file_created {
        return None;
    }

    event.paths.into_iter().find(|path| {
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            return false;
        };

        !file_name.contains("tmp") && expected_file_names.iter().any(|name| name == file_name)
    })
}

fn find_stake_meta_file(directory: &Path, expected_file_names: &[String; 2]) -> Option<PathBuf> {
    expected_file_names
        .iter()
        .map(|file_name| directory.join(file_name))
        .find(|path| path.is_file())
}

fn load_stake_meta(
    stake_meta_path: PathBuf,
    expected_epoch: u64,
) -> Result<StakeMetaCollection, DirectoryWatcherError> {
    let stake_meta_collection =
        StakeMetaCollection::new_from_file(&stake_meta_path).map_err(|source| {
            DirectoryWatcherError::StakeMetaLoad {
                stake_meta_path: stake_meta_path.clone(),
                source,
            }
        })?;

    if stake_meta_collection.epoch != expected_epoch {
        return Err(DirectoryWatcherError::UnexpectedEpoch {
            stake_meta_path,
            expected_epoch,
            actual_epoch: stake_meta_collection.epoch,
        });
    }

    Ok(stake_meta_collection)
}

pub fn wait_for_stake_meta(
    directory: PathBuf,
    expected_epoch: u64,
) -> Result<StakeMetaCollection, DirectoryWatcherError> {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = notify::recommended_watcher(tx)?;
    watcher.watch(directory.as_path(), RecursiveMode::NonRecursive)?;
    let expected_file_names = stake_meta_file_candidates(expected_epoch);

    if let Some(stake_meta_path) = find_stake_meta_file(&directory, &expected_file_names) {
        return load_stake_meta(stake_meta_path, expected_epoch);
    }

    while let Ok(event) = rx.recv() {
        if let Some(stake_meta_path) = handle_file_event(event?, &expected_file_names) {
            return load_stake_meta(stake_meta_path, expected_epoch);
        }
    }
    Err(DirectoryWatcherError::EventChannelClosed)
}
