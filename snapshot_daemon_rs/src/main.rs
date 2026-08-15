pub mod ledger_tool;
pub mod solana_client;

use anyhow::{anyhow, Result};
use clap::Parser;
use ledger_tool::LedgerTool;
use solana_client::SolanaRpcClient;
use std::path::{Path, PathBuf};

const STARTUP_SNAPSHOT_SLOT_OFFSET: u64 = 100;

#[derive(Parser)]
struct Cli {
    #[clap(short, long, default_value = "http://127.0.0.1:8899")]
    rpc_url: String,

    #[clap(short, long)]
    ledger_path: PathBuf,

    #[clap(short, long)]
    output_dir: PathBuf,

    /// Create a startup test snapshot 100 slots after this slot.
    #[clap(long)]
    start_slot: Option<u64>,

    #[clap(long, default_value = "agave-ledger-tool")]
    ledger_tool_bin: PathBuf,

    /// Directory containing full snapshot archives. Defaults to --ledger-path.
    #[clap(long)]
    full_snapshot_archive_path: Option<PathBuf>,

    /// Directory containing incremental snapshot archives. Defaults to the full snapshot path.
    #[clap(long)]
    incremental_snapshot_archive_path: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    let cli = Cli::parse();

    log::info!("Starting snapshot daemon");

    let full_snapshot_archive_path = cli
        .full_snapshot_archive_path
        .unwrap_or_else(|| cli.ledger_path.clone());
    let incremental_snapshot_archive_path = cli
        .incremental_snapshot_archive_path
        .unwrap_or_else(|| full_snapshot_archive_path.clone());
    let ledger_tool = LedgerTool::new(
        cli.ledger_tool_bin,
        cli.ledger_path,
        full_snapshot_archive_path,
        incremental_snapshot_archive_path,
    );
    let version = ledger_tool.version().await?;
    log::info!("Ledger tool version: {version}");
    let solana_client = SolanaRpcClient::new(cli.rpc_url);

    if let Some(start_slot) = cli.start_slot {
        let target_slot = startup_snapshot_target(start_slot)?;
        log::info!(
            "Creating startup test snapshot at slot {target_slot} ({STARTUP_SNAPSHOT_SLOT_OFFSET} slots after start slot {start_slot})"
        );
        if let Err(error) = ledger_tool
            .create_full_snapshot(cli.output_dir.clone(), target_slot)
            .await
        {
            log::error!("Failed to create startup test snapshot at slot {target_slot}: {error}");
        }
    }

    loop {
        let boundary = solana_client.wait_for_epoch_boundary_final().await?;
        create_snapshot(&ledger_tool, &cli.output_dir, boundary).await;
    }
}

fn startup_snapshot_target(start_slot: u64) -> Result<u64> {
    start_slot
        .checked_add(STARTUP_SNAPSHOT_SLOT_OFFSET)
        .ok_or_else(|| {
            anyhow!(
                "cannot add startup snapshot offset {STARTUP_SNAPSHOT_SLOT_OFFSET} to slot {start_slot}"
            )
        })
}

async fn create_snapshot(
    ledger_tool: &LedgerTool,
    output_dir: &Path,
    boundary: solana_client::CompletedEpochBoundary,
) {
    let slot = boundary.snapshot_slot;
    log::info!(
        "Creating snapshot after epoch {} at slot {slot}",
        boundary.epoch
    );
    if let Err(error) = ledger_tool
        .create_full_snapshot(output_dir.to_path_buf(), slot)
        .await
    {
        log::error!(
            "Failed to create epoch {} snapshot at slot {slot}: {error}",
            boundary.epoch
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_the_startup_snapshot_target_by_100_slots() {
        assert_eq!(startup_snapshot_target(439_343_998).unwrap(), 439_344_098);
    }

    #[test]
    fn rejects_a_startup_snapshot_target_that_overflows() {
        assert!(startup_snapshot_target(u64::MAX).is_err());
    }
}
