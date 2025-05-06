use log::{error, info};
use solana_cli::cli::process_command;
use solana_cli::cli::CliCommand;
use solana_cli::cli::CliConfig;

pub fn catchup(rpc_url: &str, port: u16) {
    let mut catchup_config = CliConfig::default();
    catchup_config.command = CliCommand::Catchup {
        node_json_rpc_url: Some(rpc_url.to_owned()),
        node_pubkey: None,
        our_localhost_port: Some(port),
        follow: false,
        log: false,
    };
    match process_command(&catchup_config) {
        Ok(r) => info!("Catchup command executed successfully:\n{}", r),
        Err(e) => error!("Error executing catchup command: {}", e),
    }
}
