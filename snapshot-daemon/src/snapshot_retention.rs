use std::{
    fs,
    path::{Path, PathBuf},
};

use agave_snapshots::paths::parse_full_snapshot_archive_filename;
use anyhow::{Context, Result};

const MAX_RETAINED_FULL_SNAPSHOT_ARCHIVES: usize = 2;

/// Keep the newest completed full archives by slot in the daemon's output directory.
pub fn enforce_snapshot_retention(output_dir: &Path) -> Result<()> {
    let mut archives_by_slot = find_completed_full_snapshot_archives(output_dir)?;
    archives_by_slot.sort_unstable();
    let expired_archive_count = archives_by_slot
        .len()
        .saturating_sub(MAX_RETAINED_FULL_SNAPSHOT_ARCHIVES);
    remove_expired_snapshot_archives(
        archives_by_slot
            .into_iter()
            .take(expired_archive_count)
            .map(|(_, archive_path)| archive_path),
    )
}

fn find_completed_full_snapshot_archives(output_dir: &Path) -> Result<Vec<(u64, PathBuf)>> {
    let mut archives_by_slot = Vec::new();
    for directory_entry in fs::read_dir(output_dir)
        .with_context(|| format!("reading snapshot directory {}", output_dir.display()))?
    {
        let directory_entry = directory_entry?;
        // Never recurse into directories or follow links into the validator's archives.
        if !directory_entry.file_type()?.is_file() {
            continue;
        }
        let file_name = directory_entry.file_name();
        let Some(file_name) = file_name.to_str() else {
            continue;
        };
        // Agave's exact archive filename parser excludes temporary/in-progress archives.
        if let Ok((slot, _, _)) = parse_full_snapshot_archive_filename(file_name) {
            archives_by_slot.push((slot, directory_entry.path()));
        }
    }

    Ok(archives_by_slot)
}

fn remove_expired_snapshot_archives(
    expired_archive_paths: impl Iterator<Item = PathBuf>,
) -> Result<()> {
    let mut first_removal_error = None;
    for archive_path in expired_archive_paths {
        // On Unix, unlinking read-only files only requires a writable parent directory.
        // No chmod or recursive deletion is needed, and retained archives stay protected.
        match fs::remove_file(&archive_path) {
            Ok(()) => log::info!("Removed old snapshot {}", archive_path.display()),
            Err(error) => {
                log::error!(
                    "Failed to remove old snapshot {}: {error}",
                    archive_path.display()
                );
                first_removal_error.get_or_insert_with(|| {
                    anyhow::Error::new(error)
                        .context(format!("removing old snapshot {}", archive_path.display()))
                });
            }
        }
    }
    first_removal_error.map_or(Ok(()), Err)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SNAPSHOT_HASH: &str = "11111111111111111111111111111111";

    fn full_snapshot_archive_name(slot: u64) -> String {
        format!("snapshot-{slot}-{SNAPSHOT_HASH}.tar.zst")
    }

    #[test]
    fn retains_two_highest_slots_and_ignores_unrelated_entries() {
        let output_dir = tempfile::tempdir().unwrap();
        // Lexical order and insertion order differ from slot order.
        for slot in [100, 9, 20, 10] {
            fs::write(
                output_dir.path().join(full_snapshot_archive_name(slot)),
                b"snapshot",
            )
            .unwrap();
        }
        let preserved_file_names = [
            format!("tmp-snapshot-archive-1-{SNAPSHOT_HASH}.tar.zst"),
            format!("snapshot-1-{SNAPSHOT_HASH}.tar.zst.tmp"),
            format!("incremental-snapshot-1-2-{SNAPSHOT_HASH}.tar.zst"),
            "snapshot-1-invalid.tar.zst".to_owned(),
            "notes.txt".to_owned(),
        ];
        for file_name in &preserved_file_names {
            fs::write(output_dir.path().join(file_name), b"untouched").unwrap();
        }
        let snapshot_named_directory = output_dir.path().join(full_snapshot_archive_name(1));
        fs::create_dir(&snapshot_named_directory).unwrap();
        fs::write(snapshot_named_directory.join("keep"), b"untouched").unwrap();

        enforce_snapshot_retention(output_dir.path()).unwrap();

        for slot in [9, 10] {
            assert!(!output_dir
                .path()
                .join(full_snapshot_archive_name(slot))
                .exists());
        }
        for slot in [20, 100] {
            assert!(output_dir
                .path()
                .join(full_snapshot_archive_name(slot))
                .exists());
        }
        for file_name in preserved_file_names {
            assert_eq!(
                fs::read(output_dir.path().join(file_name)).unwrap(),
                b"untouched"
            );
        }
        assert!(snapshot_named_directory.join("keep").exists());
        enforce_snapshot_retention(output_dir.path()).unwrap();
    }

    #[test]
    fn leaves_fewer_than_limit_unchanged() {
        let output_dir = tempfile::tempdir().unwrap();
        enforce_snapshot_retention(output_dir.path()).unwrap();
        let archive_path = output_dir.path().join(full_snapshot_archive_name(1));
        fs::write(&archive_path, b"snapshot").unwrap();
        enforce_snapshot_retention(output_dir.path()).unwrap();
        assert!(archive_path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn removes_read_only_archives_and_leaves_symlink_targets_untouched() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let output_dir = tempfile::tempdir().unwrap();
        let external_dir = tempfile::tempdir().unwrap();
        let external_archive_path = external_dir.path().join("archive");
        fs::write(&external_archive_path, b"external").unwrap();
        let archive_symlink = output_dir.path().join(full_snapshot_archive_name(999));
        symlink(&external_archive_path, &archive_symlink).unwrap();
        for slot in [1, 2, 3] {
            let archive_path = output_dir.path().join(full_snapshot_archive_name(slot));
            fs::write(&archive_path, b"snapshot").unwrap();
            fs::set_permissions(archive_path, fs::Permissions::from_mode(0o444)).unwrap();
        }

        enforce_snapshot_retention(output_dir.path()).unwrap();

        assert!(!output_dir
            .path()
            .join(full_snapshot_archive_name(1))
            .exists());
        for slot in [2, 3] {
            let retained_archive_metadata =
                fs::metadata(output_dir.path().join(full_snapshot_archive_name(slot))).unwrap();
            assert_eq!(
                retained_archive_metadata.permissions().mode() & 0o777,
                0o444
            );
        }
        assert!(archive_symlink
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(fs::read(external_archive_path).unwrap(), b"external");
    }
}
