use {
    clap::{Args, Parser, Subcommand},
    std::path::PathBuf,
};

#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about = "Build and reuse fastboot bank caches for faster iteration",
    long_about = "\
Build and reuse a durable Agave fastboot bank cache for rapid testing of bank-processing code.

Summary:
  fastboot-from-snapshot loads a full snapshot archive once, writes a durable fastboot bank snapshot dir,
  and hard-links account storages into the cache.

  from-fastboot-dir loads a fresh Bank from that cache repeatedly, skipping archive untar/decompression
  while still rebuilding the in-memory accounts index.

The ledger path is used only to read genesis. Snapshot archives and one output cache root are supplied separately."
)]
pub(crate) struct Cli {
    /// Validator ledger directory used; solely to read genesis.
    #[arg(long, env)]
    pub(crate) ledger_path: PathBuf,

    /// Bank slot to prepare or load from the durable cache.
    #[arg(long, env)]
    pub(crate) slot: u64,

    /// Output root for the durable fastboot cache.
    #[arg(long, env)]
    pub(crate) output_dir: PathBuf,

    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Commands {
    /// Prepare a durable fastboot bank cache from a snapshot archive.
    FastbootFromSnapshot(FastbootFromSnapshotArgs),

    /// Load from an existing durable fastboot bank cache.
    FromFastbootDir(FromFastbootDirArgs),
}

#[derive(Debug, Args)]
pub(crate) struct FastbootFromSnapshotArgs {
    /// Directory containing the source full snapshot archive.
    #[arg(long, env)]
    pub(crate) snapshot_archive_dir: PathBuf,
}

#[derive(Debug, Args)]
pub(crate) struct FromFastbootDirArgs {
    /// Skip the post-load accounts hash verification pass.
    #[arg(long, env, default_value_t = false)]
    pub(crate) skip_initial_hash_calc: bool,

    /// Run the startup accounts index verification pass.
    #[arg(long, env, default_value_t = false)]
    pub(crate) verify_index: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct FromDirLoadOptions {
    pub(crate) skip_initial_hash_calc: bool,
    pub(crate) verify_index: bool,
}

impl From<&FromFastbootDirArgs> for FromDirLoadOptions {
    fn from(args: &FromFastbootDirArgs) -> Self {
        Self {
            skip_initial_hash_calc: args.skip_initial_hash_calc,
            verify_index: args.verify_index,
        }
    }
}
