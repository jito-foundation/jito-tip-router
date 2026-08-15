use std::{
    ffi::OsString,
    fmt, fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use agave_snapshots::{
    paths::{full_snapshot_archives_iter, incremental_snapshot_archives_iter},
    snapshot_archive_info::SnapshotArchiveInfoGetter,
};
use anyhow::{anyhow, Result};
use nom::{
    bytes::complete::{tag, take_till1, take_until},
    character::complete::space1,
    combinator::{all_consuming, opt},
    sequence::{delimited, preceded},
    IResult, Parser,
};

pub struct LedgerTool {
    ledger_tool_binary: PathBuf,
    ledger_path: PathBuf,
    full_snapshot_archive_path: PathBuf,
    incremental_snapshot_archive_path: PathBuf,
}

impl LedgerTool {
    pub const fn new(
        ledger_tool_binary: PathBuf,
        ledger_path: PathBuf,
        full_snapshot_archive_path: PathBuf,
        incremental_snapshot_archive_path: PathBuf,
    ) -> Self {
        Self {
            ledger_tool_binary,
            ledger_path,
            full_snapshot_archive_path,
            incremental_snapshot_archive_path,
        }
    }

    pub async fn version(&self) -> Result<LedgerToolVersion> {
        let output = tokio::process::Command::new(&self.ledger_tool_binary)
            .arg("--version")
            .output()
            .await?;
        let version = String::from_utf8_lossy(&output.stdout);
        version
            .parse()
            .map_err(|error| anyhow!("failed to parse ledger tool version: {error}"))
    }

    pub async fn create_full_snapshot(
        &self,
        snapshot_output_dir: PathBuf,
        slot: u64,
    ) -> Result<()> {
        let snapshot_archives = SnapshotArchiveView::new(
            &self.full_snapshot_archive_path,
            &self.incremental_snapshot_archive_path,
            slot,
        )?;
        let status = tokio::process::Command::new(&self.ledger_tool_binary)
            .args(self.create_snapshot_args(&snapshot_output_dir, slot, &snapshot_archives))
            .spawn()?
            .wait()
            .await?;

        if !status.success() {
            return Err(anyhow!(
                "failed to create full snapshot at slot {slot}: ledger tool exited with {status}"
            ));
        }

        Ok(())
    }

    fn create_snapshot_args(
        &self,
        snapshot_output_dir: &Path,
        slot: u64,
        snapshot_archives: &SnapshotArchiveView,
    ) -> Vec<OsString> {
        vec![
            "--ledger".into(),
            self.ledger_path.as_os_str().to_owned(),
            "--ignore-ulimit-nofile-error".into(),
            "create-snapshot".into(),
            "--full-snapshot-archive-path".into(),
            snapshot_archives
                .full_snapshot_archive_path()
                .as_os_str()
                .to_owned(),
            "--incremental-snapshot-archive-path".into(),
            snapshot_archives
                .incremental_snapshot_archive_path()
                .as_os_str()
                .to_owned(),
            "--use-snapshot-archives-at-startup".into(),
            "always".into(),
            slot.to_string().into(),
            snapshot_output_dir.as_os_str().to_owned(),
        ]
    }
}

struct SnapshotArchiveView {
    _temp_dir: tempfile::TempDir,
    full_snapshot_archive_path: PathBuf,
    incremental_snapshot_archive_path: PathBuf,
}

impl SnapshotArchiveView {
    fn new(
        full_snapshot_archive_path: &Path,
        incremental_snapshot_archive_path: &Path,
        maximum_slot: u64,
    ) -> Result<Self> {
        let full_snapshot_archive = full_snapshot_archives_iter(full_snapshot_archive_path)
            .filter(|archive| archive.slot() <= maximum_slot)
            .max()
            .ok_or_else(|| {
                anyhow!(
                    "no full snapshot archive at or before slot {maximum_slot} in {}",
                    full_snapshot_archive_path.display()
                )
            })?;
        let incremental_snapshot_archive =
            incremental_snapshot_archives_iter(incremental_snapshot_archive_path)
                .filter(|archive| archive.base_slot() == full_snapshot_archive.slot())
                .filter(|archive| archive.slot() <= maximum_slot)
                .max();

        let temp_dir = tempfile::Builder::new()
            .prefix("snapshot-daemon-")
            .tempdir()?;
        let selected_full_snapshot_archive_path = temp_dir.path().join("full");
        let selected_incremental_snapshot_archive_path = temp_dir.path().join("incremental");
        fs::create_dir(&selected_full_snapshot_archive_path)?;
        fs::create_dir(&selected_incremental_snapshot_archive_path)?;

        symlink_snapshot_archive(
            full_snapshot_archive.path(),
            &selected_full_snapshot_archive_path,
        )?;
        log::info!(
            "Selected full snapshot archive at slot {} for target slot {maximum_slot}",
            full_snapshot_archive.slot()
        );
        if let Some(incremental_snapshot_archive) = incremental_snapshot_archive {
            symlink_snapshot_archive(
                incremental_snapshot_archive.path(),
                &selected_incremental_snapshot_archive_path,
            )?;
            log::info!(
                "Selected incremental snapshot archive at slot {} with base slot {} for target slot {maximum_slot}",
                incremental_snapshot_archive.slot(),
                incremental_snapshot_archive.base_slot()
            );
        }

        Ok(Self {
            _temp_dir: temp_dir,
            full_snapshot_archive_path: selected_full_snapshot_archive_path,
            incremental_snapshot_archive_path: selected_incremental_snapshot_archive_path,
        })
    }

    fn full_snapshot_archive_path(&self) -> &Path {
        &self.full_snapshot_archive_path
    }

    fn incremental_snapshot_archive_path(&self) -> &Path {
        &self.incremental_snapshot_archive_path
    }
}

fn symlink_snapshot_archive(snapshot_archive_path: &Path, destination_dir: &Path) -> Result<()> {
    let file_name = snapshot_archive_path.file_name().ok_or_else(|| {
        anyhow!(
            "snapshot archive path has no filename: {}",
            snapshot_archive_path.display()
        )
    })?;
    let canonical_snapshot_archive_path = snapshot_archive_path.canonicalize()?;
    std::os::unix::fs::symlink(
        canonical_snapshot_archive_path,
        destination_dir.join(file_name),
    )?;
    Ok(())
}

/// Parsed `agave-ledger-tool --version` output
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerToolVersion {
    pub binary: String,
    pub version: String,
    pub source_revision: Option<String>,
    pub feature_set: Option<String>,
    pub client: Option<String>,
}

impl fmt::Display for LedgerToolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.binary, self.version)?;

        if let (Some(source_revision), Some(feature_set), Some(client)) =
            (&self.source_revision, &self.feature_set, &self.client)
        {
            write!(
                f,
                " (src:{source_revision}; feat:{feature_set}, client:{client})"
            )?;
        }

        Ok(())
    }
}

fn parse_ledger_tool_version(input: &str) -> IResult<&str, LedgerToolVersion> {
    let (input, (binary, _, version, build_metadata)) = (
        take_till1(|c: char| c.is_whitespace()),
        space1,
        take_till1(|c: char| c.is_whitespace()),
        opt(preceded(
            space1,
            (
                preceded(tag("(src:"), take_until(";")),
                preceded(tag("; feat:"), take_until(", client:")),
                delimited(tag(", client:"), take_until(")"), tag(")")),
            ),
        )),
    )
        .parse(input)?;

    let (source_revision, feature_set, client) = match build_metadata {
        Some((source_revision, feature_set, client)) => (
            Some(source_revision.to_string()),
            Some(feature_set.to_string()),
            Some(client.to_string()),
        ),
        None => (None, None, None),
    };

    Ok((
        input,
        LedgerToolVersion {
            binary: binary.to_string(),
            version: version.to_string(),
            source_revision,
            feature_set,
            client,
        },
    ))
}

#[cfg(test)]
mod tests {
    use {super::*, std::collections::HashSet};

    const SNAPSHOT_HASH: &str = "11111111111111111111111111111111";

    #[test]
    fn create_snapshot_args_place_global_flags_before_subcommand() {
        let source_dir = tempfile::tempdir().unwrap();
        fs::write(
            source_dir
                .path()
                .join(format!("snapshot-100-{SNAPSHOT_HASH}.tar.zst")),
            [],
        )
        .unwrap();
        let snapshot_archives =
            SnapshotArchiveView::new(source_dir.path(), source_dir.path(), 100).unwrap();
        let ledger_tool = LedgerTool::new(
            "agave-ledger-tool".into(),
            "/ledger".into(),
            source_dir.path().into(),
            source_dir.path().into(),
        );

        assert_eq!(
            ledger_tool.create_snapshot_args(Path::new("/output"), 100, &snapshot_archives),
            vec![
                OsString::from("--ledger"),
                OsString::from("/ledger"),
                OsString::from("--ignore-ulimit-nofile-error"),
                OsString::from("create-snapshot"),
                OsString::from("--full-snapshot-archive-path"),
                snapshot_archives
                    .full_snapshot_archive_path()
                    .as_os_str()
                    .to_owned(),
                OsString::from("--incremental-snapshot-archive-path"),
                snapshot_archives
                    .incremental_snapshot_archive_path()
                    .as_os_str()
                    .to_owned(),
                OsString::from("--use-snapshot-archives-at-startup"),
                OsString::from("always"),
                OsString::from("100"),
                OsString::from("/output"),
            ]
        );
    }

    #[test]
    fn snapshot_archive_view_symlinks_only_archives_at_or_before_target() {
        let source_dir = tempfile::tempdir().unwrap();
        for slot in [100, 200, 300] {
            fs::write(
                source_dir
                    .path()
                    .join(format!("snapshot-{slot}-{SNAPSHOT_HASH}.tar.zst")),
                [],
            )
            .unwrap();
        }
        for (base_slot, slot) in [(100, 150), (200, 225), (200, 275)] {
            fs::write(
                source_dir.path().join(format!(
                    "incremental-snapshot-{base_slot}-{slot}-{SNAPSHOT_HASH}.tar.zst"
                )),
                [],
            )
            .unwrap();
        }

        let snapshot_archives =
            SnapshotArchiveView::new(source_dir.path(), source_dir.path(), 250).unwrap();

        assert_eq!(
            archive_names(snapshot_archives.full_snapshot_archive_path()),
            HashSet::from([format!("snapshot-200-{SNAPSHOT_HASH}.tar.zst")])
        );
        assert_eq!(
            archive_names(snapshot_archives.incremental_snapshot_archive_path()),
            HashSet::from([format!(
                "incremental-snapshot-200-225-{SNAPSHOT_HASH}.tar.zst"
            )])
        );
        for path in [
            snapshot_archives.full_snapshot_archive_path(),
            snapshot_archives.incremental_snapshot_archive_path(),
        ] {
            assert!(fs::read_dir(path).unwrap().all(|entry| entry
                .unwrap()
                .file_type()
                .unwrap()
                .is_symlink()));
        }
    }

    fn archive_names(path: &Path) -> HashSet<String> {
        fs::read_dir(path)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect()
    }

    #[test]
    fn parses_version_without_build_metadata() {
        let version: LedgerToolVersion = "agave-ledger-tool 4.2.1".parse().unwrap();

        assert_eq!(version.binary, "agave-ledger-tool");
        assert_eq!(version.version, "4.2.1");
        assert_eq!(version.source_revision, None);
        assert_eq!(version.feature_set, None);
        assert_eq!(version.client, None);
        assert_eq!(version.to_string(), "agave-ledger-tool 4.2.1");
    }

    #[test]
    fn parses_version_with_build_metadata() {
        let version: LedgerToolVersion =
            "agave-ledger-tool 4.2.1 (src:20853fb1; feat:21b0d33a, client:JitoLabs)"
                .parse()
                .unwrap();

        assert_eq!(version.source_revision.as_deref(), Some("20853fb1"));
        assert_eq!(version.feature_set.as_deref(), Some("21b0d33a"));
        assert_eq!(version.client.as_deref(), Some("JitoLabs"));
    }
}

impl FromStr for LedgerToolVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        all_consuming(parse_ledger_tool_version)
            .parse(s.trim())
            .map(|(_, version)| version)
            .map_err(|e| e.to_string())
    }
}
