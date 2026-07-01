use {
    clap::{Args, Parser, Subcommand, ValueEnum},
    solana_sdk::pubkey::Pubkey,
    std::{
        path::{Path, PathBuf},
        str::FromStr,
    },
};

#[derive(Debug, Parser)]
#[command(
    name = "bank-loader",
    author,
    version,
    about = "Build and reuse fastboot bank caches for faster iteration",
    long_about = "\
Build and reuse a durable Agave fastboot bank cache for rapid testing of bank-processing code.

Summary:
  create-bank-cache loads a full snapshot archive once, writes a durable fastboot bank snapshot dir,
  and hard-links account storages into the cache.

  load-bank-cache loads a fresh Bank from that cache repeatedly, skipping archive untar/decompression
  while still rebuilding the in-memory accounts index.

The ledger path is used only to read genesis. Snapshot archives and one output cache root are supplied separately.",
    after_long_help = "\
Canonical examples:
  bank-loader create-bank-cache --ledger-path <LEDGER_DIR> --slot <SLOT> --output-dir <CACHE_DIR> --snapshot-archive-dir <SNAPSHOT_DIR>
  bank-loader load-bank-cache --ledger-path <LEDGER_DIR> --slot <SLOT> --output-dir <CACHE_DIR> --skip-initial-hash-calc"
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Debug, Args)]
pub(crate) struct BankCacheConfig {
    /// Validator ledger directory used; solely to read genesis.
    #[arg(long, env, value_name = "LEDGER_DIR")]
    pub(crate) ledger_path: PathBuf,

    /// Bank slot to prepare or load from the durable cache.
    #[arg(long, env, value_name = "SLOT")]
    pub(crate) slot: u64,

    /// Output root for the durable bank cache.
    #[arg(long, env, value_name = "CACHE_DIR")]
    pub(crate) output_dir: PathBuf,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Commands {
    /// Prepare a durable bank cache from a snapshot archive.
    CreateBankCache(BankCacheFromSnapshotArgs),

    /// Load from an existing durable bank cache.
    LoadBankCache(LoadBankCacheArgs),
}

#[derive(Debug, Args)]
pub(crate) struct BankCacheFromSnapshotArgs {
    #[command(flatten)]
    pub(crate) cache: BankCacheConfig,

    /// Directory containing the source full snapshot archive.
    #[arg(long, env, value_name = "SNAPSHOT_DIR")]
    pub(crate) snapshot_archive_dir: PathBuf,
}

#[derive(Debug, Args)]
pub(crate) struct LoadBankCacheArgs {
    #[command(flatten)]
    pub(crate) cache: BankCacheConfig,

    #[command(flatten)]
    pub(crate) stake_meta: StakeMetaArgs,

    /// Skip the post-load accounts hash verification pass.
    #[arg(long, env, default_value_t = true)]
    pub(crate) skip_initial_hash_calc: bool,

    /// Run the startup accounts index verification pass.
    #[arg(long, env, default_value_t = false)]
    pub(crate) verify_index: bool,
}

#[derive(Debug, Args)]
pub(crate) struct StakeMetaArgs {
    /// Generate and write a StakeMetaCollection after the bank is loaded.
    #[arg(long, env, default_value_t = true)]
    pub(crate) generate_stake_meta: bool,

    /// Output directory for the generated StakeMetaCollection file. Defaults to --output-dir.
    #[arg(long, env, value_name = "DIR")]
    pub(crate) stake_meta_output_dir: Option<PathBuf>,

    /// Program id set to use for StakeMetaCollection generation.
    #[arg(long, env, value_enum, default_value_t = StakeMetaCluster::Mainnet)]
    pub(crate) stake_meta_cluster: StakeMetaCluster,
}

impl StakeMetaArgs {
    pub(crate) fn output_dir_or_default(&self, default_output_dir: &Path) -> PathBuf {
        self.stake_meta_output_dir
            .clone()
            .unwrap_or_else(|| default_output_dir.to_path_buf())
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum StakeMetaCluster {
    Mainnet,
    Testnet,
}

impl std::fmt::Display for StakeMetaCluster {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mainnet => write!(f, "mainnet"),
            Self::Testnet => write!(f, "testnet"),
        }
    }
}

impl StakeMetaCluster {
    pub(crate) fn program_ids(self) -> StakeMetaProgramIds {
        match self {
            Self::Mainnet => StakeMetaProgramIds {
                tip_distribution_program_id: pubkey("4R3gSG8BpU4t19KYj8CfnbtRpnT8gtk4dvTHxVRwc2r7"),
                priority_fee_distribution_program_id: pubkey(
                    "Priority6weCZ5HwDn29NxLFpb7TDp2iLZ6XKc5e8d3",
                ),
                tip_payment_program_id: pubkey("T1pyyaTNZsKv2WcRAB8oVnk93mLJw2XzjtVYqCsaHqt"),
            },
            Self::Testnet => StakeMetaProgramIds {
                tip_distribution_program_id: pubkey("DzvGET57TAgEDxvm3ERUM4GNcsAJdqjDLCne9sdfY4wf"),
                priority_fee_distribution_program_id: pubkey(
                    "9yw8YAKz16nFmA9EvHzKyVCYErHAJ6ZKtmK6adDBvmuU",
                ),
                tip_payment_program_id: pubkey("GJHtFqM9agxPmkeKjHny6qiRKrXZALvvFGiKf11QE7hy"),
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct StakeMetaProgramIds {
    pub(crate) tip_distribution_program_id: Pubkey,
    pub(crate) priority_fee_distribution_program_id: Pubkey,
    pub(crate) tip_payment_program_id: Pubkey,
}

fn pubkey(value: &str) -> Pubkey {
    Pubkey::from_str(value).expect("stake meta program id constant is a valid pubkey")
}
