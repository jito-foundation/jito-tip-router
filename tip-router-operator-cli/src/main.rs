use clap::Parser;
use anyhow::Result;
use log::info;
use snapshot::SnapshotCreator;
use std::path::PathBuf;
use solana_sdk::signer::keypair::read_keypair_file;
use tip_router_operator_cli::*;  // Add this line to use your library crate

#[cfg(test)]
mod tests;

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    /// Path to operator keypair
    #[arg(short, long)]
    keypair_path: String,

    /// RPC URL
    #[arg(short, long, default_value = "http://localhost:8899")]
    rpc_url: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Start monitoring tips
    Monitor {
        /// NCN address
        #[arg(short, long)]
        ncn_address: String,
    },
    /// Create and validate snapshots at epoch boundaries
    Snapshot {
        /// Output directory for snapshots
        #[arg(short, long)]
        output_dir: String,

        /// Maximum number of snapshots to retain
        #[arg(short, long, default_value = "2")]
        max_snapshots: u32,

        /// Snapshot compression type (none, bzip2, gzip, zstd)
        #[arg(short, long, default_value = "zstd")]
        compression: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Monitor { ncn_address } => {
            info!("Starting monitor for NCN address: {}", ncn_address);
            info!("Using keypair at: {}", cli.keypair_path);
            info!("Connected to RPC: {}", cli.rpc_url);

            let keypair = read_keypair_file(&cli.keypair_path).map_err(|e|
                anyhow::Error::msg(e.to_string())
            )?;

            // let merkle_tree_generator = merkle_tree::MerkleTreeGenerator::new(
            //     &cli.rpc_url,
            //     keypair,
            //     ncn_address.parse()?,
            //     PathBuf::from("output") // Configure this
            // )?;

            // loop {
            //     let current_epoch = merkle_tree_generator.wait_for_epoch_boundary().await?;
            //     info!("Starting workflow for epoch {}", current_epoch);
            //     let start = Instant::now();

            //     // Generate and upload regular merkle trees
            //     let stake_meta = merkle_tree_generator.generate_stake_meta(current_epoch).await?;
            //     let merkle_trees =
            //         merkle_tree_generator.generate_and_upload_merkle_trees(stake_meta).await?;

            //     // Generate and upload meta merkle tree
            //     let meta_tree = merkle_tree_generator.generate_meta_merkle_tree(
            //         &merkle_trees
            //     ).await?;
            //     merkle_tree_generator.upload_to_ncn(&meta_tree).await?;

            //     let elapsed = start.elapsed();
            //     datapoint_info!(
            //         "tip_router_workflow",
            //         ("epoch", current_epoch, i64),
            //         ("elapsed_ms", elapsed.as_millis(), i64)
            //     );
            // }
        }
        Commands::Snapshot { output_dir, max_snapshots, compression } => {
            info!("Starting snapshot creator");
            let keypair = read_keypair_file(&cli.keypair_path).map_err(|e|
                anyhow::Error::msg(e.to_string())
            )?;
            let snapshot_creator = SnapshotCreator::new(
                &cli.rpc_url,
                output_dir,
                max_snapshots,
                compression,
                keypair,
                PathBuf::from("blockstore") // You'll need to provide the correct path
            )?;
            snapshot_creator.monitor_epoch_boundary().await?;
        }
    }

    Ok(())
}
