use std::path::PathBuf;

use clap::Parser;
use solana_sdk::pubkey::Pubkey;

#[derive(Parser)]
#[command(author, version, about)]
pub struct Cli {
    #[arg(short, long, env)]
    pub keypair_path: String,

    #[arg(short, long, env)]
    pub operator_address: String,

    #[arg(short, long, env, default_value = "http://localhost:8899")]
    pub rpc_url: String,

    #[arg(short, long, env)]
    pub ledger_path: PathBuf,

    #[arg(short, long, env)]
    pub account_paths: Option<Vec<PathBuf>>,

    #[arg(short, long, env)]
    pub full_snapshots_path: Option<PathBuf>,

    #[arg(short, long, env)]
    pub snapshot_output_dir: PathBuf,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(clap::Subcommand)]
pub enum Commands {
    Run {
        #[arg(short, long, env)]
        ncn_address: Pubkey,

        #[arg(long, env)]
        tip_distribution_program_id: Pubkey,

        #[arg(long, env)]
        tip_payment_program_id: Pubkey,

        #[arg(long, env, default_value = "false")]
        enable_snapshots: bool,
    },
    SnapshotSlot {
        #[arg(short, long, env)]
        ncn_address: Pubkey,

        #[arg(long, env)]
        tip_distribution_program_id: Pubkey,

        #[arg(long, env)]
        tip_payment_program_id: Pubkey,

        #[arg(long, env, default_value = "false")]
        enable_snapshots: bool,

        #[arg(long, env)]
        slot: u64,
    },
}
