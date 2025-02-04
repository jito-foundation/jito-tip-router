use std::path::PathBuf;

use clap::Parser;
use log::info;
use solana_sdk::pubkey::Pubkey;

use crate::OperatorState;

#[derive(Clone, Parser)]
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
    pub full_snapshots_path: Option<PathBuf>,

    #[arg(short, long, env)]
    pub backup_snapshots_dir: PathBuf,

    #[arg(short, long, env)]
    pub snapshot_output_dir: PathBuf,

    #[arg(short, long, env)]
    pub meta_merkle_tree_dir: PathBuf,

    #[arg(long, env)]
    pub save_path: PathBuf,

    #[command(subcommand)]
    pub command: Commands,
}

impl Cli {
    pub fn create_save_path(&self) {
        if !self.save_path.exists() {
            info!(
                "Creating Tip Router save directory at {}",
                self.save_path.display()
            );
            std::fs::create_dir_all(&self.save_path).unwrap();
        }
    }
}

#[derive(clap::Subcommand, Clone)]
pub enum Commands {
    Run {
        #[arg(short, long, env)]
        ncn_address: Pubkey,

        #[arg(long, env)]
        tip_distribution_program_id: Pubkey,

        #[arg(long, env)]
        tip_payment_program_id: Pubkey,

        #[arg(long, env)]
        tip_router_program_id: Pubkey,

        #[arg(long, env, default_value = "false")]
        enable_snapshots: bool,

        #[arg(long, env, default_value = "3")]
        num_monitored_epochs: u64,

        #[arg(long, env)]
        override_target_slot: Option<u64>,

        #[arg(long, env, default_value = "wait-for-next-epoch")]
        starting_stage: OperatorState,

        #[arg(long, env, default_value = "true")]
        save_stages: bool,
    },
    SnapshotSlot {
        #[arg(long, env)]
        slot: u64,
    },
    SubmitEpoch {
        #[arg(short, long, env)]
        ncn_address: Pubkey,

        #[arg(long, env)]
        tip_distribution_program_id: Pubkey,

        #[arg(long, env)]
        tip_router_program_id: Pubkey,

        #[arg(long, env)]
        epoch: u64,
    },
    ClaimTips {
        /// Tip distribution program ID
        #[arg(long, env)]
        tip_distribution_program_id: Pubkey,

        /// The price to pay for priority fee
        #[arg(long, env, default_value_t = 1)]
        micro_lamports: u64,

        /// The epoch to Claim tips for
        #[arg(long, env)]
        epoch: u64,
    },
    CreateStakeMeta {
        #[arg(long, env)]
        slot: u64,

        #[arg(long, env)]
        epoch: u64,

        #[arg(long, env)]
        tip_distribution_program_id: Pubkey,

        #[arg(long, env)]
        tip_payment_program_id: Pubkey,

        #[arg(long, env, default_value = "true")]
        save: bool,
    },
    CreateMerkleTreeCollection {
        #[arg(short, long, env)]
        ncn_address: Pubkey,

        #[arg(long, env)]
        epoch: u64,

        #[arg(long, env, default_value = "true")]
        save: bool,
    },
    CreateMetaMerkleTree {
        #[arg(long, env)]
        epoch: u64,

        #[arg(long, env, default_value = "true")]
        save: bool,
    },
}
