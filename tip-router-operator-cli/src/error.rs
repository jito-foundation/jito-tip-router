use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Keypair error: {0}")]
    Keypair(String),

    #[error("RPC error: {0}")]
    Rpc(String),
}