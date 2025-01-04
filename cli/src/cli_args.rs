use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about = "A CLI for creating and managing the MEV Tip Distribution NCN", long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<ProgramCommand>,

    #[arg(long, global = true, help = "Path to the configuration file")]
    pub config_file: Option<PathBuf>,

    #[arg(long, global = true, help = "RPC URL to use")]
    pub rpc_url: Option<String>,

    #[arg(long, global = true, help = "Commitment level")]
    pub commitment: Option<String>,

    #[arg(long, global = true, help = "Tip router program ID")]
    pub tip_router_program_id: Option<String>,

    #[arg(long, global = true, help = "Restaking program ID")]
    pub restaking_program_id: Option<String>,

    #[arg(long, global = true, help = "Vault program ID")]
    pub vault_program_id: Option<String>,

    #[arg(long, global = true, help = "Keypair")]
    pub keypair: Option<String>,

    #[arg(long, global = true, help = "Verbose mode")]
    pub verbose: bool,

    #[arg(long, global = true, hide = true)]
    pub markdown_help: bool,
}

#[derive(Subcommand)]
pub enum ProgramCommand {}
