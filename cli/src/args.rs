use std::fmt;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about = "A CLI for creating and managing the MEV Tip Distribution NCN", long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: ProgramCommand,

    #[arg(
        long,
        global = true,
        env = "RPC_URL",
        default_value = "https://api.mainnet-beta.solana.com",
        help = "RPC URL to use"
    )]
    pub rpc_url: String,

    #[arg(
        long,
        global = true,
        env = "COMMITMENT",
        default_value = "confirmed",
        help = "Commitment level"
    )]
    pub commitment: String,

    #[arg(
        long,
        global = true,
        env = "TIP_ROUTER_PROGRAM_ID",
        default_value_t = jito_tip_router_program::id().to_string(),
        help = "Tip router program ID"
    )]
    pub tip_router_program_id: String,

    #[arg(
        long, 
        global = true, 
        env = "RESTAKING_PROGRAM_ID",
        default_value_t = jito_restaking_program::id().to_string(),
        help = "Restaking program ID"
    )]
    pub restaking_program_id: String,

    #[arg(
        long,
        global = true, 
        env = "VAULT_PROGRAM_ID", 
        default_value_t = jito_vault_program::id().to_string(), 
        help = "Vault program ID"
    )]
    pub vault_program_id: String,

    #[arg(
        long, 
        global = true, 
        env = "TIP_DISTRIBUTION_PROGRAM_ID",
        default_value_t = jito_tip_distribution_sdk::jito_tip_distribution::ID.to_string(),
        help = "Tip distribution program ID"
    )]
    pub tip_distribution_program_id: String,

    #[arg(
        long, 
        global = true, 
        env = "NCN",
        help = "NCN Account Address"
    )]
    pub ncn: Option<String>,

    #[arg(
        long, 
        global = true, 
        env = "EPOCH",
        help = "Epoch - defaults to current epoch"
    )]
    pub epoch: Option<u64>,

    #[arg(long, global = true, help = "keypair path")]
    pub keypair_path: Option<String>,

    #[arg(long, global = true, help = "Verbose mode")]
    pub verbose: bool,

    #[arg(long, global = true, hide = true)]
    pub markdown_help: bool,
}

#[derive(Subcommand)]
pub enum ProgramCommand {
    /// Create a new NCN
    Test,
}

impl fmt::Display for Args {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "MEV Tip Distribution NCN CLI Configuration")?;
        writeln!(f, "═══════════════════════════════════════")?;
        
        // Network Configuration
        writeln!(f, "\n📡 Network Settings:")?;
        writeln!(f, "  • RPC URL:     {}", self.rpc_url)?;
        writeln!(f, "  • Commitment:  {}", self.commitment)?;
        
        // Program IDs
        writeln!(f, "\n🔑 Program IDs:")?;
        writeln!(f, "  • Tip Router:        {}", self.tip_router_program_id)?;
        writeln!(f, "  • Restaking:         {}", self.restaking_program_id)?;
        writeln!(f, "  • Vault:             {}", self.vault_program_id)?;
        writeln!(f, "  • Tip Distribution:  {}", self.tip_distribution_program_id)?;
        
        // Optional Settings
        writeln!(f, "\n◎  Solana Settings:")?;
        writeln!(f, "  • Keypair Path:  {}", self.keypair_path.as_deref().unwrap_or("Not Set"))?;
        writeln!(f, "  • NCN:  {}", self.ncn.as_deref().unwrap_or("Not Set"))?;
        writeln!(f, "  • Epoch: {}", if self.epoch.is_some() { format!("{}", self.epoch.unwrap()) } else { "Current".to_string() })?;
        

        // Optional Settings
        writeln!(f, "\n⚙️  Additional Settings:")?;
        writeln!(f, "  • Verbose Mode:  {}", if self.verbose { "Enabled" } else { "Disabled" })?;
        writeln!(f, "  • Markdown Help: {}", if self.markdown_help { "Enabled" } else { "Disabled" })?;

        // Command
        writeln!(f, "\n🔧 Active Command:")?;
        match &self.command {
            cmd => writeln!(f, "  • {}", match cmd {
                ProgramCommand::Test => "Test",
            })?,
        }
        
        writeln!(f, "")?;

        Ok(())
    }
}