pub mod ledger_tool;
pub mod solana_client;

use anyhow::Result;
use clap::Parser;
use ledger_tool::LedgerTool;
use solana_client::SolanaRpcClient;
use std::path::PathBuf;

#[derive(Parser)]
struct Cli {
    #[clap(short, long, default_value = "http://127.0.0.1:8899")]
    rpc_url: String,

    #[clap(short, long)]
    ledger_path: PathBuf,

    #[clap(short, long)]
    output_dir: PathBuf,

    #[clap(long, default_value = "agave-ledger-tool")]
    ledger_tool_bin: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    let cli = Cli::parse();

    log::info!("Starting snapshot daemon");

    let ledger_tool = LedgerTool::new(cli.ledger_tool_bin, cli.ledger_path);
    let version = ledger_tool.version().await?;
    log::info!("Ledger tool version: {version}");
    let solana_client = SolanaRpcClient::new(cli.rpc_url);

    loop {
        let boundary = solana_client.wait_for_epoch_boundary_final().await?;
        let slot = boundary.snapshot_slot;

        log::info!(
            "Creating snapshot after epoch {} at finalized slot {slot}",
            boundary.epoch
        );
        if let Err(error) = ledger_tool
            .create_full_snapshot(cli.output_dir.clone(), slot)
            .await
        {
            log::error!(
                "Failed to create epoch {} snapshot at slot {slot}: {error}",
                boundary.epoch
            );
        }
    }
}
