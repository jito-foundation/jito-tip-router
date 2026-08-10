use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
};

use jito_stake_meta_types::StakeMetaCollection;
use notify::{
    event::{ModifyKind, RenameMode},
    Event, EventKind, RecursiveMode, Watcher,
};
use thiserror::Error;
use tokio::sync::mpsc::{self, UnboundedReceiver};

use crate::stake_meta_file_name;

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
/// Stake-meta producers publish a temporary file before renaming it to the exact canonical name,
/// so only the published artifact name is accepted.
fn handle_file_event(event: Event, expected_file_name: &str) -> Option<PathBuf> {
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

        file_name == expected_file_name
    })
}

fn find_stake_meta_files(directory: &Path, expected_file_name: &str) -> VecDeque<PathBuf> {
    let stake_meta_path = directory.join(expected_file_name);
    stake_meta_path
        .is_file()
        .then_some(stake_meta_path)
        .into_iter()
        .collect()
}

pub fn load_stake_meta(
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

impl DirectoryWatcherError {
    pub const fn is_invalid_artifact(&self) -> bool {
        matches!(
            self,
            Self::StakeMetaLoad { .. } | Self::UnexpectedEpoch { .. }
        )
    }
}

/// Watches for one exact previous-epoch stake-meta artifact.
///
/// The watcher is registered before the initial scan. This ensures a producer
/// publication between construction and the scan is either found by the scan
/// or already queued as an event. Dropping this wrapper drops the underlying
/// watcher and cancels an outstanding `next_path` wait.
pub struct StakeMetaWatcher {
    expected_file_name: String,
    existing_paths: VecDeque<PathBuf>,
    events: UnboundedReceiver<notify::Result<Event>>,
    _watcher: notify::RecommendedWatcher,
}

impl StakeMetaWatcher {
    pub(crate) fn new(
        directory: PathBuf,
        expected_epoch: u64,
    ) -> Result<Self, DirectoryWatcherError> {
        let expected_file_name = stake_meta_file_name(expected_epoch);
        let (event_sender, events) = mpsc::unbounded_channel();
        let mut watcher = notify::recommended_watcher(move |event| {
            // It is normal for the receiver to disappear when an epoch
            // transition supersedes this watcher.
            let _ = event_sender.send(event);
        })?;
        watcher.watch(directory.as_path(), RecursiveMode::NonRecursive)?;

        let existing_paths = find_stake_meta_files(&directory, &expected_file_name);

        Ok(Self {
            expected_file_name,
            existing_paths,
            events,
            _watcher: watcher,
        })
    }

    pub(crate) async fn next_path(&mut self) -> Result<PathBuf, DirectoryWatcherError> {
        if let Some(path) = self.existing_paths.pop_front() {
            return Ok(path);
        }

        loop {
            let event = self
                .events
                .recv()
                .await
                .ok_or(DirectoryWatcherError::EventChannelClosed)??;

            if let Some(stake_meta_path) = handle_file_event(event, &self.expected_file_name) {
                return Ok(stake_meta_path);
            }
        }
    }
}
