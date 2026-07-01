use {
    clap::{Args, Parser, Subcommand},
    std::path::{Path, PathBuf},
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
}

impl StakeMetaArgs {
    pub(crate) fn output_dir_or_default(&self, default_output_dir: &Path) -> PathBuf {
        self.stake_meta_output_dir
            .clone()
            .unwrap_or_else(|| default_output_dir.to_path_buf())
    }
}
