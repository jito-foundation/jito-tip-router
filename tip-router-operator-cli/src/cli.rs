use std::path::PathBuf;

use clap::Parser;
use solana_sdk::pubkey::Pubkey;

#[derive(Parser)]
#[command(author, version, about)]
pub struct Cli {
    #[arg(short, long)]
    pub keypair_path: String,

    #[arg(short, long)]
    pub operator_address: String,

    #[arg(short, long, default_value = "http://localhost:8899")]
    pub rpc_url: String,

    #[arg(short, long)]
    pub ledger_path: PathBuf,

    #[arg(short, long)]
    pub account_paths: Option<Vec<PathBuf>>,

    #[arg(short, long)]
    pub full_snapshots_path: Option<PathBuf>,

    #[arg(short, long)]
    pub snapshot_output_dir: PathBuf,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(clap::Subcommand)]
pub enum Commands {
    Run {
        #[arg(short, long)]
        ncn_address: Pubkey,

        #[arg(long)]
        tip_distribution_program_id: Pubkey,

        #[arg(long)]
        tip_payment_program_id: Pubkey,

        #[arg(long, default_value = "false")]
        enable_snapshots: bool,
    },
}
