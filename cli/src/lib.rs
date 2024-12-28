use solana_sdk::{commitment_config::CommitmentConfig, signature::Keypair};

pub mod cli_args;
pub mod log;
pub mod tip_router;
pub mod tip_router_handler;

pub struct CliConfig {
    pub rpc_url: String,

    pub commitment: CommitmentConfig,

    pub keypair: Option<Keypair>,
}
