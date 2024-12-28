use clap::{command, Subcommand};
use solana_sdk::pubkey::Pubkey;

/// The CLI handler for the Jito Tip Router program
#[derive(Subcommand)]
pub enum TipRouterCommands {
    /// Initialize, get, and set the config struct
    Config {
        #[command(subcommand)]
        action: ConfigActions,
    },
}

/// The actions that can be performed on the Jito Tip Router config
#[derive(Subcommand)]
pub enum ConfigActions {
    /// Initialize the config
    Initialize {
        ncn: Pubkey,
        dao_fee_bps: u16,
        default_ncn_fee_bps: u16,
        block_engine_fee_bps: u16,
        epochs_before_stall: u64,
        valid_slots_after_consensus: u64,
    },

    /// Get the config
    Get { ncn: Pubkey },
}
