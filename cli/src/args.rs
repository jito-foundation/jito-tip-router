use std::fmt;

use clap::{Parser, Subcommand};
use solana_sdk::clock::DEFAULT_SLOTS_PER_EPOCH;

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
        env = "PRIORITY_FEE_MICRO_LAMPORTS",
        default_value_t = 1,
        help = "Priority fee in micro lamports"
    )]
    pub priority_fee_micro_lamports: u64,

    #[arg(
        long,
        global = true,
        env = "TRANSACTION_RETRIES",
        default_value_t = 0,
        help = "Amount of times to retry a transaction"
    )]
    pub transaction_retries: u64,

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
        env = "TOKEN_PROGRAM_ID",
        default_value_t = spl_token::id().to_string(),
        help = "Token Program ID"
    )]
    pub token_program_id: String,

    #[arg(long, global = true, env = "NCN", help = "NCN Account Address")]
    pub ncn: Option<String>,

    #[arg(
        long,
        global = true,
        env = "EPOCH",
        help = "Epoch - defaults to current epoch"
    )]
    pub epoch: Option<u64>,

    #[arg(long, global = true, env = "KEYPAIR_PATH", help = "keypair path")]
    pub keypair_path: Option<String>,

    #[arg(long, global = true, help = "Verbose mode")]
    pub verbose: bool,

    #[arg(long, global = true, hide = true)]
    pub markdown_help: bool,
}

#[derive(Subcommand)]
pub enum ProgramCommand {
    /// Keeper
    Keeper {
        #[arg(
            long,
            env,
            default_value_t = 600_000, // 10 minutes
            help = "Keeper error timeout in milliseconds"
        )]
        loop_timeout_ms: u64,
        #[arg(
            long,
            env,
            default_value_t = 10_000, // 10 seconds
            help = "Keeper error timeout in milliseconds"
        )]
        error_timeout_ms: u64,
        #[arg(long, help = "calls test vote, instead of waiting for a real vote")]
        test_vote: bool,
    },

    /// Admin
    AdminCreateConfig {
        #[arg(long, default_value_t = 10 as u64, help = "Epochs before tie breaker can set consensus")]
        epochs_before_stall: u64,
        #[arg(long, default_value_t = (DEFAULT_SLOTS_PER_EPOCH as f64 * 0.1) as u64, help = "Valid slots after consensus")]
        valid_slots_after_consensus: u64,
        #[arg(
            long,
            default_value_t = 10,
            help = "Epochs after consensus before accounts can be closed"
        )]
        epochs_after_consensus_before_close: u64,
        #[arg(long, default_value_t = 300, help = "DAO fee in basis points")]
        dao_fee_bps: u16,
        #[arg(long, default_value_t = 100, help = "Block engine fee in basis points")]
        block_engine_fee_bps: u16,
        #[arg(long, default_value_t = 100, help = "Default NCN fee in basis points")]
        default_ncn_fee_bps: u16,
        #[arg(long, help = "Fee wallet address")]
        fee_wallet: Option<String>,
        #[arg(long, help = "Tie breaker admin address")]
        tie_breaker_admin: Option<String>,
    },
    AdminRegisterStMint {
        #[arg(long, help = "Vault address")]
        vault: String,
        #[arg(long, default_value_t = 0, help = "NCN fee group")]
        ncn_fee_group: u8,
        #[arg(
            long,
            default_value_t = 100,
            help = "Reward multiplier in basis points"
        )]
        reward_multiplier_bps: u64,
        #[arg(long, help = "Switchboard feed address")]
        switchboard_feed: Option<String>,
        #[arg(long, help = "Weight when no feed is available")]
        no_feed_weight: Option<u128>,
    },
    AdminSetWeight {
        #[arg(long, help = "Vault address")]
        vault: String,
        #[arg(long, help = "Weight value")]
        weight: u128,
    },
    AdminSetTieBreaker {
        #[arg(long, help = "Meta merkle root")]
        meta_merkle_root: String,
    },
    AdminSetParameters {
        #[arg(long, help = "Epochs before tie breaker can set consensus")]
        epochs_before_stall: Option<u64>,
        #[arg(long, help = "Epochs after consensus before accounts can be closed")]
        epochs_after_consensus_before_close: Option<u64>,
        #[arg(long, help = "Slots to which voting is allowed after consensus")]
        valid_slots_after_consensus: Option<u64>,
        #[arg(long, help = "Starting valid epoch")]
        starting_valid_epoch: Option<u64>,
    },
    AdminSetConfigFees {
        #[arg(long, help = "New block engine fee in basis points")]
        new_block_engine_fee_bps: Option<u16>,
        #[arg(long, help = "Base fee group")]
        base_fee_group: Option<u8>,
        #[arg(long, help = "New base fee wallet")]
        new_base_fee_wallet: Option<String>,
        #[arg(long, help = "New base fee in basis points")]
        new_base_fee_bps: Option<u16>,
        #[arg(long, help = "NCN fee group")]
        ncn_fee_group: Option<u8>,
        #[arg(long, help = "New NCN fee in basis points")]
        new_ncn_fee_bps: Option<u16>,
    },
    AdminSetNewAdmin {
        #[arg(long, help = "New admin address")]
        new_admin: String,
        #[arg(long, help = "Set fee admin")]
        set_fee_admin: bool,
        #[arg(long, help = "Set tie breaker admin")]
        set_tie_breaker_admin: bool,
    },
    AdminFundAccountPayer {
        #[arg(long, help = "Amount of SOL to fund")]
        amount_in_sol: f64,
    },

    /// Instructions
    CreateVaultRegistry,

    RegisterVault {
        #[arg(long, help = "Vault address")]
        vault: String,
    },

    CreateEpochState,

    CreateWeightTable,

    CrankSwitchboard {
        #[arg(long, help = "Switchboard feed address")]
        switchboard_feed: String,
    },

    SetWeight {
        #[arg(long, help = "Vault address")]
        vault: String,
    },

    CreateEpochSnapshot,

    CreateOperatorSnapshot {
        #[arg(long, help = "Operator address")]
        operator: String,
    },

    SnapshotVaultOperatorDelegation {
        #[arg(long, help = "Vault address")]
        vault: String,
        #[arg(long, help = "Operator address")]
        operator: String,
    },

    CreateBallotBox,

    OperatorCastVote {
        #[arg(long, help = "Operator address")]
        operator: String,
        #[arg(long, help = "Meta merkle root")]
        meta_merkle_root: String,
    },

    CreateBaseRewardRouter,

    CreateNcnRewardRouter {
        #[arg(long, help = "Operator address")]
        operator: String,
        #[arg(long, default_value_t = 0, help = "NCN fee group")]
        ncn_fee_group: u8,
    },

    RouteBaseRewards,

    RouteNcnRewards {
        #[arg(long, help = "Operator address")]
        operator: String,
        #[arg(long, default_value_t = 0, help = "NCN fee group")]
        ncn_fee_group: u8,
    },

    DistributeBaseNcnRewards {
        #[arg(long, help = "Operator address")]
        operator: String,
        #[arg(long, default_value_t = 0, help = "NCN fee group")]
        ncn_fee_group: u8,
    },

    /// Getters
    GetNcn,
    GetNcnOperatorState {
        #[arg(long, env = "OPERATOR", help = "Operator Account Address")]
        operator: String,
    },
    GetVaultNcnTicket {
        #[arg(long, env = "VAULT", help = "Vault Account Address")]
        vault: String,
    },
    GetNcnVaultTicket {
        #[arg(long, env = "VAULT", help = "Vault Account Address")]
        vault: String,
    },
    GetVaultOperatorDelegation {
        #[arg(long, env = "VAULT", help = "Vault Account Address")]
        vault: String,
        #[arg(long, env = "OPERATOR", help = "Operator Account Address")]
        operator: String,
    },
    GetAllTickets,
    GetAllOperatorsInNcn,
    GetAllVaultsInNcn,
    GetTipRouterConfig,
    GetVaultRegistry,
    GetWeightTable,
    GetEpochState,
    GetEpochSnapshot,
    GetOperatorSnapshot {
        #[arg(long, env = "OPERATOR", help = "Operator Account Address")]
        operator: String,
    },
    GetBallotBox,
    GetBaseRewardRouter,
    GetBaseRewardReceiverAddress,
    GetNcnRewardRouter {
        #[arg(long, env = "OPERATOR", help = "Operator Account Address")]
        operator: String,
        #[arg(long, default_value_t = 0, help = "NCN fee group")]
        ncn_fee_group: u8,
    },
    GetAllNcnRewardRouters,
    GetAccountPayer,
    GetTotalEpochRentCost,
    GetStakePool,

    /// TESTS
    Test,
    CreateTestNcn,
    CreateAndAddTestOperator {
        #[arg(
            long,
            env = "OPERATOR_FEE_BPS",
            default_value_t = 100,
            help = "Operator Fee BPS"
        )]
        operator_fee_bps: u16,
    },
    CreateAndAddTestVault {
        #[arg(
            long,
            env = "VAULT_DEPOSIT_FEE",
            default_value_t = 100,
            help = "Deposit fee BPS"
        )]
        deposit_fee_bps: u16,
        #[arg(
            long,
            env = "VAULT_WITHDRAWAL_FEE",
            default_value_t = 100,
            help = "Withdrawal fee BPS"
        )]
        withdrawal_fee_bps: u16,
        #[arg(
            long,
            env = "VAULT_REWARD_FEE",
            default_value_t = 100,
            help = "Reward fee BPS"
        )]
        reward_fee_bps: u16,
    },
}

#[rustfmt::skip]
impl fmt::Display for Args {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "\nMEV Tip Distribution NCN CLI Configuration")?;
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
        writeln!(f, "  • Token:             {}", self.token_program_id)?;
        writeln!(f, "  • Tip Distribution:  {}", self.tip_distribution_program_id)?;

        // Solana Settings
        writeln!(f, "\n◎  Solana Settings:")?;
        writeln!(f, "  • Keypair Path:  {}", self.keypair_path.as_deref().unwrap_or("Not Set"))?;
        writeln!(f, "  • NCN:  {}", self.ncn.as_deref().unwrap_or("Not Set"))?;
        writeln!(f, "  • Epoch: {}", if self.epoch.is_some() { format!("{}", self.epoch.unwrap()) } else { "Current".to_string() })?;

        // Optional Settings
        writeln!(f, "\n⚙️  Additional Settings:")?;
        writeln!(f, "  • Verbose Mode:  {}", if self.verbose { "Enabled" } else { "Disabled" })?;
        writeln!(f, "  • Markdown Help: {}", if self.markdown_help { "Enabled" } else { "Disabled" })?;

        writeln!(f, "\n")?;

        Ok(())
    }
}
