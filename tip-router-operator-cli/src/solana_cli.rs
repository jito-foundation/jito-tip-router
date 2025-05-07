use anyhow::{anyhow, Ok, Result};
use solana_cli::cli::process_command;
use solana_cli::cli::CliCommand;
use solana_cli::cli::CliConfig;

pub fn catchup(rpc_url: String, our_localhost_port: u16) -> Result<String> {
    let mut catchup_config = CliConfig {
        json_rpc_url: rpc_url,
        ..CliConfig::default()
    };
    catchup_config.command = CliCommand::Catchup {
        node_json_rpc_url: None,
        node_pubkey: None,
        our_localhost_port: Some(our_localhost_port),
        follow: false,
        log: false,
    };
    let result = process_command(&catchup_config);
    if let Err(e) = result {
        Err(anyhow!("Failed to execute catchup command: {}. Double check the localhost Solana RPC and the provided `localhost_port` argument.", e))
    } else {
        Ok(result.unwrap())
    }
}
