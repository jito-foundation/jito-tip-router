pub mod ledger_tool;
pub mod solana_client;

use anyhow::Result;
use clap::Parser;
use ledger_tool::LedgerTool;
use solana_client::SolanaRpcClient;
use std::path::{Path, PathBuf};

#[derive(Parser)]
struct Cli {
    #[clap(short, long, default_value = "http://127.0.0.1:8899")]
    rpc_url: String,

    #[clap(short, long)]
    ledger_path: PathBuf,

    #[clap(short, long)]
    output_dir: PathBuf,

    /// Create snapshots for epoch boundaries passed after this slot at startup.
    #[clap(long, conflicts_with = "test_slot_ahead")]
    start_slot: Option<u64>,

    /// Wait this many finalized slots after startup, then create a test snapshot.
    #[clap(long, value_name = "N SLOTS", conflicts_with = "start_slot")]
    test_slot_ahead: Option<u64>,

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

    if let Some(slots_ahead) = cli.test_slot_ahead {
        let target_slot = solana_client
            .wait_for_finalized_slots_ahead(slots_ahead)
            .await?;
        create_test_snapshot(&ledger_tool, &cli.output_dir, target_slot).await;
    } else if let Some(start_slot) = cli.start_slot {
        let missed_boundaries = solana_client
            .completed_epoch_boundaries_since(start_slot)
            .await?;
        log::info!(
            "Found {} completed epoch boundaries after start slot {start_slot}",
            missed_boundaries.len()
        );

        for boundary in missed_boundaries {
            create_boundary_snapshot(&solana_client, &ledger_tool, &cli.output_dir, boundary).await;
        }
    }

    loop {
        let boundary = solana_client.wait_for_epoch_boundary_final().await?;
        create_boundary_snapshot(&solana_client, &ledger_tool, &cli.output_dir, boundary).await;
    }
}

async fn create_boundary_snapshot(
    solana_client: &SolanaRpcClient,
    ledger_tool: &LedgerTool,
    output_dir: &Path,
    boundary: solana_client::CompletedEpochBoundary,
) {
    let slot = match solana_client
        .find_latest_finalized_block_slot(&boundary)
        .await
    {
        Ok(Some(slot)) => slot,
        Ok(None) => {
            log::error!(
                "No finalized boundary bank found near the end of epoch {}",
                boundary.epoch
            );
            return;
        }
        Err(error) => {
            log::error!(
                "Failed to select a snapshot slot for epoch {}: {error}",
                boundary.epoch
            );
            return;
        }
    };

    log::info!(
        "Creating epoch {} boundary snapshot at slot {slot}",
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

async fn create_test_snapshot(ledger_tool: &LedgerTool, output_dir: &Path, slot: u64) {
    log::info!("Creating startup test snapshot at finalized slot {slot}");
    if let Err(error) = ledger_tool
        .create_full_snapshot(output_dir.to_path_buf(), slot)
        .await
    {
        log::error!("Failed to create startup test snapshot at slot {slot}: {error}");
    }
}
