use std::{
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc},
};

use {
    agave_snapshots::{
        snapshot_archive_info::FullSnapshotArchiveInfo, snapshot_config::SnapshotConfig,
    },
    anyhow::{Context, Result},
    clap::Parser,
    solana_accounts_db::accounts_db::AccountsDbConfig,
    solana_genesis_utils::{open_genesis_config, MAX_GENESIS_ARCHIVE_UNPACKED_SIZE},
    solana_runtime::{runtime_config::RuntimeConfig, snapshot_bank_utils},
    solana_sdk::pubkey::Pubkey,
    stake_meta_generator::stake_meta::generate_stake_meta_collection,
};

#[derive(Debug, Parser)]
#[command(about = "Generate stake-meta artifacts from a full snapshot")]
struct Args {
    /// Full snapshot archive to load.
    #[arg(long)]
    full_snapshot_archive: PathBuf,

    /// Ledger directory containing the genesis archive for the snapshot's cluster.
    #[arg(long)]
    ledger_path: PathBuf,

    /// Writable AccountsDb storage path. Repeat this option to supply multiple paths.
    #[arg(long = "account-path", required = true)]
    account_paths: Vec<PathBuf>,

    /// Writable scratch directory for the unpacked bank snapshot.
    #[arg(long)]
    bank_snapshots_dir: PathBuf,

    /// Tip distribution program used to derive distribution accounts.
    #[arg(long, default_value_t = jito_tip_distribution_sdk::id())]
    tip_distribution_program_id: Pubkey,

    /// Priority-fee distribution program used to derive distribution accounts.
    #[arg(long, default_value_t = jito_priority_fee_distribution_sdk::id())]
    priority_fee_distribution_program_id: Pubkey,

    /// Tip payment program containing the configuration and tip accounts.
    #[arg(long, default_value_t = jito_tip_payment_sdk::id())]
    tip_payment_program_id: Pubkey,

    /// Destination for the generated stake-meta JSON artifact.
    #[arg(long)]
    output: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    std::fs::create_dir_all(&args.bank_snapshots_dir).with_context(|| {
        format!(
            "failed to create bank snapshots directory {}",
            args.bank_snapshots_dir.display()
        )
    })?;

    let snapshot_archive_dir = args
        .full_snapshot_archive
        .parent()
        .context("full snapshot archive path has no parent directory")?
        .to_path_buf();
    let full_snapshot = FullSnapshotArchiveInfo::new_from_path(args.full_snapshot_archive)
        .context("failed to read full snapshot archive info")?;
    let genesis_config = open_genesis_config(&args.ledger_path, MAX_GENESIS_ARCHIVE_UNPACKED_SIZE)
        .context("failed to load the genesis config")?;
    let snapshot_config = SnapshotConfig {
        full_snapshot_archives_dir: snapshot_archive_dir.clone(),
        incremental_snapshot_archives_dir: snapshot_archive_dir,
        bank_snapshots_dir: args.bank_snapshots_dir,
        ..SnapshotConfig::new_load_only()
    };

    let bank = snapshot_bank_utils::bank_from_snapshot_archives(
        &args.account_paths,
        &full_snapshot,
        None,
        &snapshot_config,
        &genesis_config,
        &RuntimeConfig::default(),
        None,
        None,
        None,
        false,
        false,
        true,
        AccountsDbConfig::default(),
        None,
        Arc::new(AtomicBool::new(false)),
    )
    .context("failed to load bank from full snapshot archive")?;

    println!("bank hash: {}", bank.hash());

    let stake_meta = generate_stake_meta_collection(
        Arc::new(bank),
        &args.tip_distribution_program_id,
        &args.priority_fee_distribution_program_id,
        &args.tip_payment_program_id,
    )
    .context("failed to calculate stake metadata")?;
    let output_parent = args
        .output
        .parent()
        .context("output path has no parent directory")?;
    std::fs::create_dir_all(output_parent).with_context(|| {
        format!(
            "failed to create artifact output directory {}",
            output_parent.display()
        )
    })?;
    let output_file = std::fs::File::create(&args.output)
        .with_context(|| format!("failed to create artifact {}", args.output.display()))?;
    serde_json::to_writer_pretty(output_file, &stake_meta)
        .context("failed to serialize stake-meta artifact")?;
    println!("wrote stake-meta artifact: {}", args.output.display());
    Ok(())
}
