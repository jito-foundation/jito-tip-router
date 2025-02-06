#![allow(clippy::arithmetic_side_effects, clippy::integer_division)]
use anyhow::{Context, Result};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::clock::DEFAULT_SLOTS_PER_EPOCH;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time;

use crate::process_epoch::get_previous_epoch_last_slot;

const MAXIMUM_BACKUP_INCREMENTAL_SNAPSHOTS_PER_EPOCH: usize = 3;

/// Represents a parsed incremental snapshot filename
#[derive(Debug)]
pub struct SnapshotInfo {
    path: PathBuf,
    _start_slot: u64,
    pub end_slot: u64,
}

impl SnapshotInfo {
    /// Try to parse a snapshot filename into slot information
    pub fn from_path(path: PathBuf) -> Option<Self> {
        let file_name = path.file_name()?.to_str()?;

        // Only try to parse if it's an incremental snapshot
        if !file_name.starts_with("incremental-snapshot-") {
            return None;
        }

        // Split on hyphens and take the slot numbers
        // Format: incremental-snapshot-<start>-<end>-<hash>.tar.zst
        let parts: Vec<&str> = file_name.split('-').collect();
        if parts.len() < 5 {
            return None;
        }

        // Parse start and end slots
        let start_slot = parts[2].parse().ok()?;
        let end_slot = parts[3].parse().ok()?;

        Some(Self {
            path,
            _start_slot: start_slot,
            end_slot,
        })
    }
}

pub struct BackupSnapshotMonitor {
    rpc_client: RpcClient,
    snapshots_dir: PathBuf,
    backup_dir: PathBuf,
    override_target_slot: Option<u64>,
}

impl BackupSnapshotMonitor {
    pub fn new(
        rpc_url: &str,
        snapshots_dir: PathBuf,
        backup_dir: PathBuf,
        override_target_slot: Option<u64>,
    ) -> Self {
        Self {
            rpc_client: RpcClient::new(rpc_url.to_string()),
            snapshots_dir,
            backup_dir,
            override_target_slot,
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
                before_target_slot && in_same_epoch
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

    fn evict_same_epoch_incremental(&self, target_slot: u64) -> Result<()> {
        let slots_per_epoch = DEFAULT_SLOTS_PER_EPOCH;
        let target_epoch = target_slot / slots_per_epoch;

        let dir_entries = std::fs::read_dir(&self.backup_dir)?;

        // Find all snapshots for the given epoch
        let mut same_epoch_snapshots: Vec<SnapshotInfo> = dir_entries
            .filter_map(Result::ok)
            .filter_map(|entry| SnapshotInfo::from_path(entry.path()))
            .filter(|snap| snap.end_slot / slots_per_epoch == target_epoch)
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
                if let Err(e) = self.evict_all_epoch_snapshots(current_epoch - 2) {
                    log::error!("Failed to evict old snapshots: {}", e);
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
        assert_eq!(info._start_slot, 100);
        assert_eq!(info.end_slot, 150);
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
        );

        let missing_path = source_dir.path().join("nonexistent.tar.zst");

        // Should error when source doesn't exist
        assert!(monitor
            .backup_incremental_snapshot(&missing_path)
            .await
            .is_err());
    }
}
