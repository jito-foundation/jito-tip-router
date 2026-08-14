use std::{fmt, path::PathBuf, str::FromStr};

use anyhow::{anyhow, Result};
use nom::{
    bytes::complete::{tag, take_till1, take_until},
    character::complete::{digit1, space1},
    combinator::{all_consuming, map_res},
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
    pub source_revision: String,
    pub feature_set: u64,
    pub client: String,
}

impl fmt::Display for LedgerToolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} (src:{}; feat:{}, client:{})",
            self.binary, self.version, self.source_revision, self.feature_set, self.client
        )
    }
}

fn parse_ledger_tool_version(input: &str) -> IResult<&str, LedgerToolVersion> {
    let (input, (binary, _, version, _, source_revision, feature_set, client)) = (
        take_till1(|c: char| c.is_whitespace()),
        space1,
        take_till1(|c: char| c.is_whitespace()),
        space1,
        preceded(tag("(src:"), take_until(";")),
        preceded(tag("; feat:"), map_res(digit1, |s: &str| s.parse::<u64>())),
        delimited(tag(", client:"), take_until(")"), tag(")")),
    )
        .parse(input)?;

    Ok((
        input,
        LedgerToolVersion {
            binary: binary.to_string(),
            version: version.to_string(),
            source_revision: source_revision.to_string(),
            feature_set,
            client: client.to_string(),
        },
    ))
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
