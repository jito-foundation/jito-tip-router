use clap::Parser;
use anyhow::Result;
use log::info;

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    /// Path to operator keypair
    #[arg(short, long)]
    keypair_path: String,

    /// RPC URL
    #[arg(short, long, default_value = "http://localhost:8899")]
    rpc_url: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Start monitoring tips
    Monitor {
        /// NCN address
        #[arg(short, long)]
        ncn_address: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Monitor { ncn_address } => {
            info!("Starting monitor for NCN address: {}", ncn_address);
            info!("Using keypair at: {}", cli.keypair_path);
            info!("Connected to RPC: {}", cli.rpc_url);
            // TODO: Implement monitoring logic
        }
    }

    Ok(())
}