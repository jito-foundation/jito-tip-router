use std::str::FromStr;

use anyhow::anyhow;
use clap::Parser;
use jito_restaking_client::programs::JITO_RESTAKING_ID;
use jito_tip_router_cli::{cli_args::Cli, tip_router_handler::TipRouterCliHandler, CliConfig};
use jito_tip_router_client::programs::JITO_TIP_ROUTER_ID;
use jito_vault_client::programs::JITO_VAULT_ID;
use solana_sdk::{
    commitment_config::CommitmentConfig, pubkey::Pubkey, signature::read_keypair_file,
};

fn get_cli_config(args: &Cli) -> Result<CliConfig, anyhow::Error> {
    let cli_config = if let Some(config_file) = &args.config_file {
        let config = solana_cli_config::Config::load(config_file.as_os_str().to_str().unwrap())?;
        CliConfig {
            rpc_url: config.json_rpc_url,
            commitment: CommitmentConfig::from_str(&config.commitment)?,
            keypair: Some(
                read_keypair_file(config.keypair_path).map_err(|e| anyhow!(e.to_string()))?,
            ),
        }
    } else {
        let config_file = solana_cli_config::CONFIG_FILE
            .as_ref()
            .ok_or_else(|| anyhow!("unable to get config file path"))?;
        if let Ok(config) = solana_cli_config::Config::load(config_file) {
            let keypair = if let Some(keypair_path) = &args.keypair {
                read_keypair_file(keypair_path)
            } else {
                read_keypair_file(config.keypair_path)
            }
            .map_err(|e| anyhow!(e.to_string()))?;
            let rpc = if let Some(rpc) = &args.rpc_url {
                rpc.to_string()
            } else {
                config.json_rpc_url
            };

            CliConfig {
                rpc_url: rpc,
                commitment: CommitmentConfig::from_str(&config.commitment)?,
                keypair: Some(keypair),
            }
        } else {
            CliConfig {
                rpc_url: args
                    .rpc_url
                    .as_ref()
                    .ok_or_else(|| anyhow!("RPC URL not provided"))?
                    .to_string(),
                commitment: if let Some(commitment) = &args.commitment {
                    CommitmentConfig::from_str(commitment)?
                } else {
                    CommitmentConfig::confirmed()
                },
                keypair: if let Some(keypair) = &args.keypair {
                    Some(read_keypair_file(keypair).map_err(|e| anyhow!(e.to_string()))?)
                } else {
                    None
                },
            }
        }
    };

    Ok(cli_config)
}

#[tokio::main]
async fn main() -> anyhow::Result<(), anyhow::Error> {
    jito_tip_router_cli::log::init_logger();

    let args: Cli = Cli::parse();

    let cli_config = get_cli_config(&args)?;

    let restaking_program_id = if let Some(restaking_program_id) = &args.restaking_program_id {
        Pubkey::from_str(restaking_program_id)?
    } else {
        JITO_RESTAKING_ID
    };

    let _vault_program_id = if let Some(vault_program_id) = &args.vault_program_id {
        Pubkey::from_str(vault_program_id)?
    } else {
        JITO_VAULT_ID
    };

    let tip_router_program_id = if let Some(tip_router_program_id) = &args.tip_router_program_id {
        Pubkey::from_str(tip_router_program_id)?
    } else {
        JITO_TIP_ROUTER_ID
    };

    let action = args.action.expect("Action not found");
    TipRouterCliHandler::new(cli_config, restaking_program_id, tip_router_program_id)
        .handle(action)
        .await?;

    Ok(())
}
