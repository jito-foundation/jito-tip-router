#![allow(clippy::arithmetic_side_effects, clippy::integer_division)]
use anyhow::{Context, Result};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::clock::DEFAULT_SLOTS_PER_EPOCH;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time;

use crate::process_epoch::get_previous_epoch_last_slot;
use crate::{merkle_tree_collection_file_name, meta_merkle_tree_file_name, stake_meta_file_name};

const MAXIMUM_BACKUP_INCREMENTAL_SNAPSHOTS_PER_EPOCH: usize = 3;

/// Represents a parsed incremental snapshot filename
#[derive(Debug)]
pub struct SnapshotInfo {
    path: PathBuf,
    _start_slot: Option<u64>,
    pub end_slot: u64,
}

impl SnapshotInfo {
    /// Try to parse a snapshot filename into slot information
    pub fn from_path(path: PathBuf) -> Option<Self> {
        let file_name = path.file_name()?.to_str()?;

        // Split on hyphens and take the slot numbers
        let parts: Vec<&str> = file_name.split('-').collect();
        if parts.len() == 5 {
            // incremental snapshot
            // Format: incremental-snapshot-<start>-<end>-<hash>.tar.zst
            // Parse start and end slots
            let start_slot: u64 = parts[2].parse().ok()?;
            let end_slot = parts[3].parse().ok()?;

            Some(Self {
                path,
                _start_slot: Some(start_slot),
                end_slot,
            })
        } else if parts.len() == 3 {
            // Full snapshot
            // Format: snapshot-<end>-<hash>.tar.zst
            let end_slot = parts[1].parse().ok()?;

            Some(Self {
                path,
                _start_slot: None,
                end_slot,
            })
        } else {
            None
        }
    }

    pub const fn is_incremental(&self) -> bool {
        self._start_slot.is_some()
    }
}

/// Represents a parsed incremental snapshot filename
#[derive(Debug)]
pub struct SavedTipRouterFile {
    path: PathBuf,
    epoch: u64,
}

impl SavedTipRouterFile {
    /// Try to parse a TipRouter saved filename with epoch information
    pub fn from_path(path: PathBuf) -> Option<Self> {
        let file_name = path.file_name()?.to_str()?;

        // Split on underscore to get epoch
        let parts: Vec<&str> = file_name.split('_').collect();
        let epoch: u64 = parts[0].parse().ok()?;

        let is_tip_router_file = [
            stake_meta_file_name(epoch),
            merkle_tree_collection_file_name(epoch),
            meta_merkle_tree_file_name(epoch),
        ]
        .iter()
        .any(|x| *x == file_name);

        if is_tip_router_file {
            Some(Self { path, epoch })
        } else {
            None
        }
    }
}

pub struct BackupSnapshotMonitor {
    rpc_client: RpcClient,
    snapshots_dir: PathBuf,
    backup_dir: PathBuf,
    override_target_slot: Option<u64>,
    save_path: PathBuf,
    num_monitored_epochs: u64,
}

impl BackupSnapshotMonitor {
    pub fn new(
        rpc_url: &str,
        snapshots_dir: PathBuf,
        backup_dir: PathBuf,
        override_target_slot: Option<u64>,
        save_path: PathBuf,
        num_monitored_epochs: u64,
    ) -> Self {
        Self {
            rpc_client: RpcClient::new(rpc_url.to_string()),
            snapshots_dir,
            backup_dir,
            override_target_slot,
            save_path,
            num_monitored_epochs,
        }
    }

    /// Gets target slot for current epoch
    async fn get_target_slots(&self) -> Result<(u64, u64)> {
        // Get the last slot of the current epoch
        let (_, last_epoch_target_slot) = get_previous_epoch_last_slot(&self.rpc_client).await?;
        let next_epoch_target_slot = last_epoch_target_slot + DEFAULT_SLOTS_PER_EPOCH;

        if let Some(target_slot) = self.override_target_slot {
            return Ok((last_epoch_target_slot, target_slot));
        }

        Ok((last_epoch_target_slot, next_epoch_target_slot))
    }

    /// Finds the most recent incremental snapshot that's before our target slot
    fn find_closest_incremental(&self, target_slot: u64) -> Option<PathBuf> {
        let dir_entries = std::fs::read_dir(&self.snapshots_dir).ok()?;

        // Find the snapshot that ends closest to but not after target_slot, in the same epoch
        dir_entries
            .filter_map(Result::ok)
            .filter_map(|entry| SnapshotInfo::from_path(entry.path()))
            .filter(|snap| {
                let before_target_slot = snap.end_slot <= target_slot;
                let in_same_epoch = (snap.end_slot / DEFAULT_SLOTS_PER_EPOCH)
                    == (target_slot / DEFAULT_SLOTS_PER_EPOCH);
                snap.is_incremental() && before_target_slot && in_same_epoch
            })
            .max_by_key(|snap| snap.end_slot)
            .map(|snap| snap.path)
    }

    /// Copies incremental snapshot files to backup directory
    async fn backup_incremental_snapshot(&self, snapshot_path: &Path) -> Result<()> {
        let file_name = snapshot_path
            .file_name()
            .context("Failed to get incremental snapshot filename")?;

        let dest_path = self.backup_dir.join(file_name);

        // Check if file already exists in backup
        if dest_path.exists() {
            log::info!(
                "Incremental snapshot already exists in backup dir: {:?}",
                dest_path
            );
            return Ok(());
        }

        log::debug!(
            "Copying incremental snapshot from {:?} to {:?}",
            snapshot_path,
            dest_path
        );

        // Copy the file
        std::fs::copy(snapshot_path, &dest_path).with_context(|| {
            format!(
                "Failed to copy incremental snapshot from {:?} to {:?}",
                snapshot_path, dest_path
            )
        })?;

        // Verify file size matches
        let source_size = std::fs::metadata(snapshot_path)?.len();
        let dest_size = std::fs::metadata(&dest_path)?.len();

        if source_size != dest_size {
            // If sizes don't match, remove the corrupted copy and error
            let _ = std::fs::remove_file(&dest_path);
            anyhow::bail!(
                "Backup size mismatch: source {}, dest {}",
                source_size,
                dest_size
            );
        }

        log::debug!(
            "Successfully backed up incremental snapshot ({} bytes)",
            source_size
        );

        Ok(())
    }

    fn evict_all_epoch_snapshots(&self, epoch: u64) -> Result<()> {
        let dir_entries = std::fs::read_dir(&self.backup_dir)?;

        // Find all snapshots for the given epoch and remove them
        dir_entries
            .filter_map(Result::ok)
            .filter_map(|entry| SnapshotInfo::from_path(entry.path()))
            .filter(|snap| snap.end_slot / DEFAULT_SLOTS_PER_EPOCH == epoch)
            .try_for_each(|snapshot| {
                log::debug!(
                    "Removing old snapshot from epoch {} with slot {}: {:?}",
                    epoch,
                    snapshot.end_slot,
                    snapshot.path
                );
                std::fs::remove_file(snapshot.path.as_path())
            })?;

        Ok(())
    }

    /// Deletes TipRouter saved files that were created <= epoch
    fn evict_saved_files(&self, epoch: u64) -> Result<()> {
        let dir_entries = std::fs::read_dir(&self.save_path)?;
        // Filter the files and evict files that are <= epoch
        dir_entries
            .filter_map(Result::ok)
            .filter_map(|entry| SavedTipRouterFile::from_path(entry.path()))
            .filter(|saved_file| saved_file.epoch <= epoch)
            .try_for_each(|saved_file| {
                log::debug!(
                    "Removing old asved file from epoch {}: {:?}",
                    saved_file.epoch,
                    saved_file.path
                );
                std::fs::remove_file(saved_file.path.as_path())
            })?;
        Ok(())
    }

    fn evict_same_epoch_incremental(&self, target_slot: u64) -> Result<()> {
        let slots_per_epoch = DEFAULT_SLOTS_PER_EPOCH;
        let target_epoch = target_slot / slots_per_epoch;

        let dir_entries = std::fs::read_dir(&self.backup_dir)?;

        // Find all snapshots for the given epoch
        let mut same_epoch_snapshots: Vec<SnapshotInfo> = dir_entries
            .filter_map(Result::ok)
            .filter_map(|entry| SnapshotInfo::from_path(entry.path()))
            .filter(|snap| snap.is_incremental() && snap.end_slot / slots_per_epoch == target_epoch)
            .collect();

        // Sort by end_slot ascending so we can remove oldest
        same_epoch_snapshots.sort_by_key(|snap| snap.end_slot);

        // Remove oldest snapshots if we have more than MAXIMUM_BACKUP_INCREMENTAL_SNAPSHOTS_PER_EPOCH
        while same_epoch_snapshots.len() > MAXIMUM_BACKUP_INCREMENTAL_SNAPSHOTS_PER_EPOCH {
            if let Some(oldest_snapshot) = same_epoch_snapshots.first() {
                log::debug!(
                    "Removing old snapshot from epoch {} with slot {}: {:?}",
                    target_epoch,
                    oldest_snapshot.end_slot,
                    oldest_snapshot.path
                );
                std::fs::remove_file(oldest_snapshot.path.as_path())?;
                same_epoch_snapshots.remove(0);
            }
        }

        Ok(())
    }

    async fn backup_latest_for_target_slot(
        &self,
        mut current_backup_path: Option<PathBuf>,
        target_slot: u64,
    ) -> Option<PathBuf> {
        if let Some(snapshot) = self.find_closest_incremental(target_slot) {
            if current_backup_path.as_ref() != Some(&snapshot) {
                log::debug!(
                    "Found new best snapshot for slot {}: {:?}",
                    target_slot,
                    snapshot
                );

                if let Err(e) = self.backup_incremental_snapshot(&snapshot).await {
                    log::error!("Failed to backup snapshot: {}", e);
                    return current_backup_path;
                }

                current_backup_path = Some(snapshot);

                // After saving best snapshot, evict oldest one from same epoch
                if let Err(e) = self.evict_same_epoch_incremental(target_slot) {
                    log::error!("Failed to evict old snapshots: {}", e);
                }
            }
        }

        current_backup_path
    }

    /// Runs the snapshot backup process to continually back up the latest incremental snapshot for the previous epoch and the current epoch
    /// Keeps at most MAXIMUM_BACKUP_INCREMENTAL_SNAPSHOTS_PER_EPOCH snapshots per epoch in the backup
    /// Purges old incremental snapshots in the backup after 2 epochs
    pub async fn run(&self) -> Result<()> {
        let mut interval = time::interval(Duration::from_secs(10));
        let mut current_target_slot = None;
        let mut last_epoch_backup_path = None;
        let mut this_epoch_backup_path = None;

        loop {
            interval.tick().await;

            let (last_epoch_target_slot, this_epoch_target_slot) = self.get_target_slots().await?;

            // Detect new epoch
            if current_target_slot != Some(this_epoch_target_slot) {
                log::info!("New target slot: {}", this_epoch_target_slot);
                last_epoch_backup_path = this_epoch_backup_path;
                this_epoch_backup_path = None;
                let current_epoch = this_epoch_target_slot / DEFAULT_SLOTS_PER_EPOCH;
                if let Err(e) = self.evict_all_epoch_snapshots(
                    current_epoch - self.num_monitored_epochs.saturating_sub(1),
                ) {
                    log::error!("Failed to evict old snapshots: {}", e);
                }
                // evict all saved files
                if let Err(e) = self.evict_saved_files(current_epoch - self.num_monitored_epochs) {
                    log::error!("Failed to evict old TipRouter saved files: {}", e);
                }
            }

            // Backup latest snapshot for last epoch and this epoch
            last_epoch_backup_path = self
                .backup_latest_for_target_slot(last_epoch_backup_path, last_epoch_target_slot)
                .await;
            this_epoch_backup_path = self
                .backup_latest_for_target_slot(this_epoch_backup_path, this_epoch_target_slot)
                .await;

            current_target_slot = Some(this_epoch_target_slot);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;

    use crate::{
        merkle_tree_collection_file_name, meta_merkle_tree_file_name, stake_meta_file_name,
    };

    use super::*;
    use std::io::Write;
    use tempfile::TempDir;
    use tokio;

    #[tokio::test]
    async fn test_snapshot_monitoring() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = TempDir::new().unwrap();

        let _monitor = BackupSnapshotMonitor::new(
            "http://localhost:8899",
            temp_dir.path().to_path_buf(),
            backup_dir.path().to_path_buf(),
            None,
            backup_dir.path().to_path_buf(),
            3,
        );

        // The test version will use the fixed slot from cfg(test) get_target_slot
        // TODO: Add test cases
        // 1. Create test snapshots
        // 2. Verify correct snapshot selection
        // 3. Test backup functionality
    }

    #[test]
    fn test_snapshot_info_parsing() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir
            .path()
            .join("incremental-snapshot-100-150-hash1.tar.zst");

        let info = SnapshotInfo::from_path(path.clone()).unwrap();
        assert_eq!(info._start_slot.unwrap(), 100);
        assert_eq!(info.end_slot, 150);
        assert_eq!(info.path, path);

        // Full snapshot
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("snapshot-323710005-hash.tar.zst");

        let info = SnapshotInfo::from_path(path.clone()).unwrap();
        assert_eq!(info._start_slot, None);
        assert_eq!(info.end_slot, 323710005);
        assert_eq!(info.path, path);

        // Test invalid cases
        assert!(SnapshotInfo::from_path(temp_dir.path().join("not-a-snapshot.txt")).is_none());
        assert!(
            SnapshotInfo::from_path(temp_dir.path().join("snapshot-100-150-hash.tar.zst"))
                .is_none()
        );
    }

    #[test]
    fn test_find_closest_incremental() {
        let temp_dir = TempDir::new().unwrap();
        let monitor = BackupSnapshotMonitor::new(
            "http://localhost:8899",
            temp_dir.path().to_path_buf(),
            temp_dir.path().to_path_buf(),
            None,
            temp_dir.path().to_path_buf(),
            3,
        );

        // Create test snapshot files
        let snapshots = [
            "incremental-snapshot-100-150-hash1.tar.zst",
            "incremental-snapshot-200-250-hash2.tar.zst",
            "incremental-snapshot-300-350-hash3.tar.zst",
        ];

        for name in snapshots.iter() {
            let path = temp_dir.path().join(name);
            File::create(path).unwrap();
        }

        // Test finding closest snapshot
        let result = monitor
            .find_closest_incremental(200)
            .map(|p| p.file_name().unwrap().to_str().unwrap().to_string());

        assert_eq!(
            result,
            Some("incremental-snapshot-100-150-hash1.tar.zst".to_string()),
            "Should find snapshot ending at 150 for target 200"
        );

        // Test no valid snapshot
        assert_eq!(
            monitor.find_closest_incremental(100),
            None,
            "Should find no snapshot for target 100"
        );
    }

    #[tokio::test]
    async fn test_backup_snapshot() {
        let source_dir = TempDir::new().unwrap();
        let backup_dir = TempDir::new().unwrap();

        let monitor = BackupSnapshotMonitor::new(
            "http://localhost:8899",
            source_dir.path().to_path_buf(),
            backup_dir.path().to_path_buf(),
            None,
            backup_dir.path().to_path_buf(),
            3,
        );

        // Create test snapshot with some content
        let snapshot_name = "incremental-snapshot-100-150-hash1.tar.zst";
        let source_path = source_dir.path().join(snapshot_name);
        let mut file = File::create(&source_path).unwrap();
        file.write_all(b"test snapshot content").unwrap();

        // Test backup
        monitor
            .backup_incremental_snapshot(&source_path)
            .await
            .unwrap();

        // Verify backup exists and has correct content
        let backup_path = backup_dir.path().join(snapshot_name);
        assert!(backup_path.exists());

        let backup_content = std::fs::read_to_string(backup_path).unwrap();
        assert_eq!(backup_content, "test snapshot content");

        // Test idempotency - should succeed without error
        monitor
            .backup_incremental_snapshot(&source_path)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_backup_snapshot_missing_source() {
        let source_dir = TempDir::new().unwrap();
        let backup_dir = TempDir::new().unwrap();

        let monitor = BackupSnapshotMonitor::new(
            "http://localhost:8899",
            source_dir.path().to_path_buf(),
            backup_dir.path().to_path_buf(),
            None,
            backup_dir.path().to_path_buf(),
            3,
        );

        let missing_path = source_dir.path().join("nonexistent.tar.zst");

        // Should error when source doesn't exist
        assert!(monitor
            .backup_incremental_snapshot(&missing_path)
            .await
            .is_err());
    }

    #[test]
    fn test_evict_saved_files() {
        let temp_dir = TempDir::new().unwrap();
        let monitor = BackupSnapshotMonitor::new(
            "http://localhost:8899",
            temp_dir.path().to_path_buf(),
            temp_dir.path().to_path_buf(),
            None,
            temp_dir.path().to_path_buf(),
            3,
        );
        let current_epoch = 749;
        let first_epoch = current_epoch - 5;

        for i in first_epoch..current_epoch {
            File::create(&monitor.save_path.join(stake_meta_file_name(i))).unwrap();
            File::create(&monitor.save_path.join(merkle_tree_collection_file_name(i))).unwrap();
            File::create(&monitor.save_path.join(meta_merkle_tree_file_name(i))).unwrap();
        }
        let dir_entries: Vec<PathBuf> = std::fs::read_dir(&monitor.save_path)
            .unwrap()
            .map(|x| x.unwrap().path())
            .collect();
        assert_eq!(dir_entries.len(), 5 * 3);

        monitor
            .evict_saved_files(current_epoch - monitor.num_monitored_epochs)
            .unwrap();
        let dir_entries: Vec<PathBuf> = std::fs::read_dir(&monitor.save_path)
            .unwrap()
            .map(|x| x.unwrap().path())
            .collect();
        assert_eq!(dir_entries.len(), 6);

        // test not evicting some other similar file in the same directory
        let file_path = monitor
            .save_path
            .join(format!("{first_epoch}_other_similar_file.json"));
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"test").unwrap();
        monitor
            .evict_saved_files(current_epoch - monitor.num_monitored_epochs)
            .unwrap();
        assert!(File::open(file_path).is_ok());
    }

    #[test]
    fn test_evict_same_epoch_incremental() {
        let temp_dir = TempDir::new().unwrap();
        let monitor = BackupSnapshotMonitor::new(
            "http://localhost:8899",
            temp_dir.path().to_path_buf(),
            temp_dir.path().to_path_buf(),
            None,
            temp_dir.path().to_path_buf(),
            3,
        );

        // Create test snapshot files
        let snapshots = [
            "incremental-snapshot-100-324431477-hash1.tar.zst",
            "incremental-snapshot-200-324431877-hash2.tar.zst",
            "incremental-snapshot-300-324431977-hash3.tar.zst",
            "incremental-snapshot-100-324589366-hash1.tar.zst",
            "incremental-snapshot-200-324589866-hash2.tar.zst",
            "incremental-snapshot-300-324590366-hash3.tar.zst",
            "snapshot-324431977-hash.tar.zst",
        ];

        for name in snapshots.iter() {
            let path = temp_dir.path().join(name);
            File::create(path).unwrap();
        }

        // Test that it only keeps 3 incrementals when there's a full snapshot
        monitor.evict_same_epoch_incremental(324431977).unwrap();
        let dir_entries: Vec<PathBuf> = std::fs::read_dir(&monitor.backup_dir)
            .unwrap()
            .map(|x| x.unwrap().path())
            .collect();
        assert_eq!(dir_entries.len(), snapshots.len());
    }
}
