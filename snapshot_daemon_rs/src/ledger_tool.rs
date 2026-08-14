use std::{fmt, path::PathBuf, str::FromStr};

use anyhow::{anyhow, Result};
use nom::{
    bytes::complete::{tag, take_till1, take_until},
    character::complete::{digit1, space1},
    combinator::{all_consuming, map_res, opt},
    sequence::{delimited, preceded},
    IResult, Parser,
};

pub struct LedgerTool {
    ledger_tool_binary: PathBuf,
    ledger_path: PathBuf,
}

impl LedgerTool {
    pub fn new(ledger_tool_binary: PathBuf, ledger_path: PathBuf) -> Self {
        Self {
            ledger_tool_binary,
            ledger_path,
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
        let status = tokio::process::Command::new(&self.ledger_tool_binary)
            .arg("--ledger")
            .arg(&self.ledger_path)
            .arg("create-snapshot")
            .arg(slot.to_string())
            .arg(&snapshot_output_dir)
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
}

/// Parsed `agave-ledger-tool --version` output
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerToolVersion {
    pub binary: String,
    pub version: String,
    pub source_revision: Option<String>,
    pub feature_set: Option<u64>,
    pub client: Option<String>,
}

impl fmt::Display for LedgerToolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.binary, self.version)?;

        if let (Some(source_revision), Some(feature_set), Some(client)) = (
            &self.source_revision,
            self.feature_set,
            &self.client,
        ) {
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
                preceded(tag("; feat:"), map_res(digit1, |s: &str| s.parse::<u64>())),
                delimited(tag(", client:"), take_until(")"), tag(")")),
            ),
        )),
    )
        .parse(input)?;

    let (source_revision, feature_set, client) = match build_metadata {
        Some((source_revision, feature_set, client)) => (
            Some(source_revision.to_string()),
            Some(feature_set),
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
    use super::LedgerToolVersion;

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
            "agave-ledger-tool 4.2.1 (src:20853fb1; feat:21, client:JitoLabs)"
                .parse()
                .unwrap();

        assert_eq!(version.source_revision.as_deref(), Some("20853fb1"));
        assert_eq!(version.feature_set, Some(21));
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
