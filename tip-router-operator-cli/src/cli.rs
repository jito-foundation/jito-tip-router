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

    #[arg(long, env, default_value = "false")]
    pub submit_as_memo: bool,

    /// The price to pay for priority fee when claiming tips
    #[arg(long, env, default_value_t = 1)]
    pub claim_microlamports: u64,

    /// The price to pay for priority fee when voting
    #[arg(long, env, default_value_t = 1000000)]
    pub vote_microlamports: u64,

    #[arg(long, env, help = "Path to save data (formerly meta-merkle-tree-dir)")]
    pub save_path: Option<PathBuf>,

    #[arg(long, env, default_value = "/tmp/claim_tips_epoch.txt")]
    pub claim_tips_epoch_filepath: PathBuf,

    #[arg(short, long, env, help = "Path to save data (deprecated)")]
    #[deprecated(since = "1.1.0", note = "use --save-path instead")]
    pub meta_merkle_tree_dir: Option<PathBuf>,

    #[arg(long, env, default_value = "mainnet")]
    pub cluster: String,

    #[arg(long, env, default_value = "local")]
    pub region: String,

    #[arg(long, env, default_value = "8899")]
    pub localhost_port: u16,

    #[arg(long, env, default_value = "900")]
    pub heartbeat_interval_seconds: u64,

    #[command(subcommand)]
    pub command: Commands,
}

#[allow(unused_assignments)]
#[allow(deprecated)]
impl Cli {
    pub fn as_legacy(&self) -> legacy_tip_router_operator_cli::Cli {
        legacy_tip_router_operator_cli::Cli {
            keypair_path: self.keypair_path.clone(),
            operator_address: self.operator_address.clone(),
            rpc_url: self.rpc_url.clone(),
            ledger_path: self.ledger_path.clone(),
            full_snapshots_path: self.full_snapshots_path.clone(),
            backup_snapshots_dir: self.backup_snapshots_dir.clone(),
            snapshot_output_dir: self.snapshot_output_dir.clone(),
            submit_as_memo: self.submit_as_memo,
            claim_microlamports: self.claim_microlamports,
            vote_microlamports: self.vote_microlamports,
            save_path: self.save_path.clone(),
            claim_tips_epoch_filepath: self.claim_tips_epoch_filepath.clone(),
            meta_merkle_tree_dir: self.meta_merkle_tree_dir.clone(),
            cluster: self.cluster.clone(),
            region: self.region.clone(),
            localhost_port: self.localhost_port,
            heartbeat_interval_seconds: self.heartbeat_interval_seconds,
            command: self.command.clone().as_legacy(),
        }
    }

    #[allow(deprecated)]
    pub fn get_save_path(&self) -> PathBuf {
        self.save_path.to_owned().map_or_else(
            || {
                self.meta_merkle_tree_dir.to_owned().map_or_else(
                    || {
                        panic!("--save-path argument must be set");
                    },
                    |save_path| save_path,
                )
            },
            |save_path| save_path,
        )
    }

    pub fn create_save_path(&self) {
        let save_path = self.get_save_path();
        if !save_path.exists() {
            info!(
                "Creating Tip Router save directory at {}",
                save_path.display()
            );
            std::fs::create_dir_all(&save_path).unwrap();
        }
    }

    pub fn get_snapshot_paths(&self) -> SnapshotPaths {
        let ledger_path = self.ledger_path.clone();
        let account_paths = None;
        let account_paths = account_paths.map_or_else(|| vec![ledger_path.clone()], |paths| paths);
        let full_snapshots_path = self.full_snapshots_path.clone();
        let full_snapshots_path = full_snapshots_path.map_or(ledger_path.clone(), |path| path);
        let incremental_snapshots_path = self.backup_snapshots_dir.clone();
        SnapshotPaths {
            ledger_path,
            account_paths,
            full_snapshots_path,
            incremental_snapshots_path,
            backup_snapshots_dir: self.backup_snapshots_dir.clone(),
        }
    }

    pub fn force_different_backup_snapshot_dir(&self) {
        let snapshot_paths = self.get_snapshot_paths();
        assert_ne!(
            snapshot_paths.full_snapshots_path,
            snapshot_paths.backup_snapshots_dir
        );
    }
}

pub struct SnapshotPaths {
    pub ledger_path: PathBuf,
    pub account_paths: Vec<PathBuf>,
    pub full_snapshots_path: PathBuf,
    pub incremental_snapshots_path: PathBuf,
    /// Used when storing or loading snapshots that the operator CLI is workign with
    pub backup_snapshots_dir: PathBuf,
}

#[derive(clap::Subcommand, Clone)]
pub enum Commands {
    Run {
        #[arg(short, long, env)]
        ncn_address: Pubkey,

        #[arg(long, env)]
        tip_distribution_program_id: Pubkey,

        #[arg(long, env)]
        priority_fee_distribution_program_id: Pubkey,

        #[arg(long, env)]
        tip_payment_program_id: Pubkey,

        #[arg(long, env)]
        tip_router_program_id: Pubkey,

        #[arg(long, env, default_value = "3")]
        num_monitored_epochs: u64,

        #[arg(long, env)]
        override_target_slot: Option<u64>,

        #[arg(long, env, default_value = "false")]
        set_merkle_roots: bool,

        #[arg(long, env, default_value = "false")]
        claim_tips: bool,

        #[arg(long, env, default_value = "false")]
        claim_tips_metrics: bool,

        #[arg(long, env, default_value_t = 3)]
        claim_tips_epoch_lookback: u64,

        #[arg(long, env, default_value = "wait-for-next-epoch")]
        starting_stage: OperatorState,

        #[arg(long, env, default_value = "true")]
        save_stages: bool,

        #[arg(
            long,
            env,
            alias = "enable-snapshots",
            help = "Flag to enable storing created snapshots (formerly enable-snapshots)",
            default_value = "false"
        )]
        save_snapshot: bool,
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
        priority_fee_distribution_program_id: Pubkey,

        #[arg(long, env)]
        tip_router_program_id: Pubkey,

        #[arg(long, env)]
        epoch: u64,

        #[arg(long, env, default_value = "false")]
        set_merkle_roots: bool,
    },
    ClaimTips {
        #[arg(long, env)]
        tip_router_program_id: Pubkey,

        /// Tip distribution program ID
        #[arg(long, env)]
        tip_distribution_program_id: Pubkey,

        /// Priority fee distribution program ID
        #[arg(long, env)]
        priority_fee_distribution_program_id: Pubkey,

        #[arg(short, long, env)]
        ncn_address: Pubkey,

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
        priority_fee_distribution_program_id: Pubkey,

        #[arg(long, env)]
        tip_payment_program_id: Pubkey,

        #[arg(long, env, default_value = "true")]
        save: bool,
    },
    CreateMerkleTreeCollection {
        #[arg(long, env)]
        tip_router_program_id: Pubkey,

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
    GetTipDistributionStats {
        #[arg(long, env)]
        tip_distribution_program_id: Pubkey,

        #[arg(long, env)]
        priority_fee_distribution_program_id: Pubkey,

        #[arg(long, env)]
        epoch: u64,
    },
}

impl Commands {
    pub const fn as_legacy(self) -> legacy_tip_router_operator_cli::Commands {
        match self {
            Self::Run {
                ncn_address,
                tip_distribution_program_id,
                priority_fee_distribution_program_id: _,
                tip_payment_program_id,
                tip_router_program_id,
                num_monitored_epochs,
                override_target_slot,
                set_merkle_roots,
                claim_tips,
                claim_tips_metrics,
                claim_tips_epoch_lookback,
                starting_stage,
                save_stages,
                save_snapshot,
            } => legacy_tip_router_operator_cli::Commands::Run {
                ncn_address,
                tip_distribution_program_id,
                tip_payment_program_id,
                tip_router_program_id,
                num_monitored_epochs,
                override_target_slot,
                set_merkle_roots,
                claim_tips,
                claim_tips_metrics,
                claim_tips_epoch_lookback,
                starting_stage: starting_stage.as_legacy(),
                save_stages,
                save_snapshot,
            },
            Self::SnapshotSlot { slot } => {
                legacy_tip_router_operator_cli::Commands::SnapshotSlot { slot }
            }
            Self::SubmitEpoch {
                ncn_address,
                tip_distribution_program_id,
                priority_fee_distribution_program_id: _,
                tip_router_program_id,
                epoch,
                set_merkle_roots,
            } => legacy_tip_router_operator_cli::Commands::SubmitEpoch {
                ncn_address,
                tip_distribution_program_id,
                tip_router_program_id,
                epoch,
                set_merkle_roots,
            },
            Self::ClaimTips {
                tip_router_program_id,
                tip_distribution_program_id,
                priority_fee_distribution_program_id: _,
                ncn_address,
                epoch,
            } => legacy_tip_router_operator_cli::Commands::ClaimTips {
                tip_router_program_id,
                tip_distribution_program_id,
                ncn_address,
                epoch,
            },
            Self::CreateStakeMeta {
                epoch,
                slot,
                tip_distribution_program_id,
                priority_fee_distribution_program_id: _,
                tip_payment_program_id,
                save,
            } => legacy_tip_router_operator_cli::Commands::CreateStakeMeta {
                epoch,
                slot,
                tip_distribution_program_id,
                tip_payment_program_id,
                save,
            },
            Self::CreateMerkleTreeCollection {
                tip_router_program_id,
                ncn_address,
                epoch,
                save,
            } => legacy_tip_router_operator_cli::Commands::CreateMerkleTreeCollection {
                tip_router_program_id,
                ncn_address,
                epoch,
                save,
            },
            Self::CreateMetaMerkleTree { epoch, save } => {
                legacy_tip_router_operator_cli::Commands::CreateMetaMerkleTree { epoch, save }
            }
            Self::GetTipDistributionStats { .. } => {
                // This command doesn't exist in legacy, so we'll panic
                panic!("GetTipDistributionStats command not supported in legacy mode")
            }
        }
    }
}
