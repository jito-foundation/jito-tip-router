use clap::Parser;
use jito_tip_router_cli::{
    args::{Args, ProgramCommand},
    getters::get_tip_distribution_accounts_to_migrate,
    handler::CliHandler,
};
use solana_sdk::pubkey::Pubkey;

#[derive(Parser)]
#[command(
    about = "List TipDistributionAccounts whose merkle_root_upload_authority needs migration"
)]
struct ExampleArgs {
    /// RPC endpoint (or set RPC_URL env var)
    #[arg(long, env = "RPC_URL")]
    rpc_url: String,

    /// Path to the payer keypair (or set KEYPAIR_PATH env var, or falls back to Solana CLI config)
    #[arg(long, env = "KEYPAIR_PATH")]
    keypair_path: Option<String>,

    /// The upload authority being replaced
    #[arg(long)]
    old_authority: Pubkey,

    /// Epoch to inspect (defaults to the current epoch)
    #[arg(long)]
    epoch: Option<u64>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let ex = ExampleArgs::parse();

    let args = Args {
        command: ProgramCommand::CreateVaultRegistry,
        config_file: None,
        rpc_url: Some(ex.rpc_url),
        commitment: "confirmed".to_string(),
        priority_fee_micro_lamports: 1,
        transaction_retries: 0,
        tip_router_program_id: jito_tip_router_program::id().to_string(),
        restaking_program_id: jito_restaking_program::id().to_string(),
        vault_program_id: jito_vault_program::id().to_string(),
        tip_distribution_program_id: jito_tip_distribution_sdk::id().to_string(),
        token_program_id: spl_token_interface::id().to_string(),
        ncn: None,
        epoch: ex.epoch,
        keypair_path: ex.keypair_path,
        verbose: false,
        print_tx: false,
        markdown_help: false,
    };

    let handler = CliHandler::from_args(&args).await?;

    let accounts = get_tip_distribution_accounts_to_migrate(
        &handler,
        &handler.tip_distribution_program_id,
        &ex.old_authority,
        handler.epoch,
    )
    .await?;

    println!("TDAs to migrate: {}", accounts.len());
    for pubkey in &accounts {
        println!("  {pubkey}");
    }

    Ok(())
}
