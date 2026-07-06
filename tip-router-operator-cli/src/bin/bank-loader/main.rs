use {
    agave_snapshots::{
        snapshot_archive_info::{
            FullSnapshotArchiveInfo, IncrementalSnapshotArchiveInfo, SnapshotArchiveInfoGetter,
        },
        SnapshotArchiveKind, SnapshotKind, SnapshotVersion,
    },
    anyhow::{ensure, Context, Result},
    clap::Parser,
    env_logger::Env,
    log::info,
    solana_accounts_db::accounts_db::AccountsDbConfig,
    solana_genesis_config::GenesisConfig,
    solana_genesis_utils::{open_genesis_config, MAX_GENESIS_ARCHIVE_UNPACKED_SIZE},
    solana_runtime::{
        bank::Bank,
        runtime_config::RuntimeConfig,
        snapshot_bank_utils,
        snapshot_package::SnapshotPackage,
        snapshot_utils::{self, BankSnapshotInfo},
    },
    std::{
        path::Path,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
        time::Instant,
    },
};

mod bankcache;
mod cli;
mod stake_meta;
mod stake_meta_generator;

use bankcache::BankCachePaths;
use cli::{BankCacheConfig, BankCacheFromSnapshotArgs, Cli, Commands, LoadBankCacheArgs};
use stake_meta::StakeMetaConfig;

fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    match &cli.command {
        Commands::CreateBankCache(args) => handle_create_bank_cache(args)?,
        Commands::LoadBankCache(args) => handle_load_bankcache(args)?,
    }

    Ok(())
}

fn handle_create_bank_cache(args: &BankCacheFromSnapshotArgs) -> Result<()> {
    let config = &args.cache;
    let full_snapshot_archive = FullSnapshotArchiveInfo::new_from_path(args.full_snapshot.clone())?;

    let incremental_snapshot_archive = args
        .incremental_snapshot
        .as_ref()
        .map(|path| IncrementalSnapshotArchiveInfo::new_from_path(path.clone()))
        .transpose()?;

    let slot = cache_slot_from_snapshot_archives(
        &full_snapshot_archive,
        incremental_snapshot_archive.as_ref(),
    )?;

    let cache_paths = BankCachePaths::new(config.output_dir.clone(), slot);
    cache_paths.prepare_empty_dirs()?;

    let load_started = Instant::now();
    let bank = load_bank_from_snapshot_archive(
        config,
        &cache_paths,
        &full_snapshot_archive,
        incremental_snapshot_archive.as_ref(),
    )?;
    let load_duration_ms = load_started.elapsed().as_millis();
    ensure_loaded_slot(&bank, slot)?;
    ensure_bank_is_frozen(&bank)?;

    let cache_write_started = Instant::now();
    write_fastboot_bank_snapshot(&cache_paths, &bank)?;
    let cache_write_duration_ms = cache_write_started.elapsed().as_millis();

    // This cache is intentionally disk-heavy. Keep the account run and snapshot
    // directories on one filesystem so Agave can hard-link account storage.
    info!(
        "mode: create-bank-cache slot: {slot} full_snapshot: {} incremental_snapshot: {} cache_write_duration_ms: {cache_write_duration_ms}",
        full_snapshot_archive.path().display(),
        incremental_snapshot_archive
            .as_ref()
            .map(|incremental| incremental.path().display().to_string())
            .unwrap_or_else(|| "<none>".to_string()),
    );
    log_loaded_bank_context(config, &bank, &cache_paths, load_duration_ms);

    Ok(())
}

fn handle_load_bankcache(args: &LoadBankCacheArgs) -> Result<()> {
    let config = &args.cache;
    let bank_snapshot = detect_cached_bank_snapshot(&config.output_dir)?;
    let slot = bank_snapshot.slot;
    let cache_paths = BankCachePaths::new(config.output_dir.clone(), slot);
    let load_started = Instant::now();
    let bank = load_bank_from_dir(
        config,
        args.skip_initial_hash_calc,
        args.verify_index,
        &cache_paths,
        &bank_snapshot,
    )?;
    let load_duration_ms = load_started.elapsed().as_millis();

    info!(
        "mode: load-bank-cache slot: {slot} skip_initial_hash_calc: {} verify_index: {}",
        args.skip_initial_hash_calc, args.verify_index
    );
    ensure_bank_is_frozen(&bank)?;
    log_loaded_bank_context(config, &bank, &cache_paths, load_duration_ms);
    maybe_run_stake_meta_generation(bank, &args.stake_meta, &config.output_dir)?;
    Ok(())
}

/// Locate the single fastboot bank snapshot in a cache dir. Errors unless exactly one exists.
fn detect_cached_bank_snapshot(output_dir: &Path) -> Result<BankSnapshotInfo> {
    let root = output_dir.join("bank-snapshots");
    let mut snapshots = snapshot_utils::get_bank_snapshots(&root);

    ensure!(
        snapshots.len() == 1,
        "expected exactly one bank snapshot under {}, found {}",
        root.display(),
        snapshots.len()
    );
    Ok(snapshots.pop().expect("snapshot count checked before pop"))
}

fn maybe_run_stake_meta_generation(
    bank: Bank,
    args: &cli::StakeMetaArgs,
    default_output_dir: &Path,
) -> Result<()> {
    if !args.generate_stake_meta {
        return Ok(());
    }

    let program_ids = args.stake_meta_cluster.program_ids();
    let config = StakeMetaConfig {
        output_dir: args.output_dir_or_default(default_output_dir),
        cluster: args.stake_meta_cluster,
        tip_distribution_program_id: program_ids.tip_distribution_program_id,
        priority_fee_distribution_program_id: program_ids.priority_fee_distribution_program_id,
        tip_payment_program_id: program_ids.tip_payment_program_id,
    };
    stake_meta::generate(bank, &config)?;
    Ok(())
}

fn ensure_loaded_slot(bank: &Bank, requested_slot: u64) -> Result<()> {
    ensure!(
        bank.slot() == requested_slot,
        "loaded bank slot {} does not match expected slot {}",
        bank.slot(),
        requested_slot
    );
    Ok(())
}

fn cache_slot_from_snapshot_archives(
    full_snapshot_archive: &FullSnapshotArchiveInfo,
    incremental_snapshot_archive: Option<&IncrementalSnapshotArchiveInfo>,
) -> Result<u64> {
    let Some(incremental_snapshot_archive) = incremental_snapshot_archive else {
        return Ok(full_snapshot_archive.slot());
    };

    ensure!(
        incremental_snapshot_archive.base_slot() == full_snapshot_archive.slot(),
        "incremental base slot {} does not match full snapshot slot {}; not a matching pair",
        incremental_snapshot_archive.base_slot(),
        full_snapshot_archive.slot()
    );
    ensure!(
        incremental_snapshot_archive.slot() > full_snapshot_archive.slot(),
        "incremental tip slot {} must be greater than full snapshot slot {}",
        incremental_snapshot_archive.slot(),
        full_snapshot_archive.slot()
    );

    Ok(incremental_snapshot_archive.slot())
}

fn ensure_bank_is_frozen(bank: &Bank) -> Result<()> {
    ensure!(
        bank.is_frozen(),
        "loaded bank at slot {} is not frozen",
        bank.slot()
    );
    Ok(())
}

fn log_loaded_bank_context(
    config: &BankCacheConfig,
    bank: &Bank,
    paths: &BankCachePaths,
    load_duration_ms: u128,
) {
    info!("ledger_path: {}", config.ledger_path.display());
    info!("output_dir: {}", config.output_dir.display());
    info!("slot: {}", bank.slot());
    info!("bank_hash: {}", bank.hash());
    info!("epoch: {}", bank.epoch());
    info!("load_duration_ms: {load_duration_ms}");
    info!("{paths:?}");
}

fn load_bank_from_snapshot_archive(
    config: &BankCacheConfig,
    paths: &BankCachePaths,
    full_snapshot_archive: &FullSnapshotArchiveInfo,
    incremental_snapshot_archive: Option<&IncrementalSnapshotArchiveInfo>,
) -> Result<Bank> {
    let genesis_config = open_genesis(&config.ledger_path)?;
    let exit = Arc::new(AtomicBool::new(false));

    let bank = snapshot_bank_utils::bank_from_snapshot_archives(
        &[paths.accounts_run_dir().to_path_buf()],
        paths.bank_snapshot_cache_dir(),
        full_snapshot_archive,
        incremental_snapshot_archive,
        &genesis_config,
        &RuntimeConfig::default(),
        None,
        None,
        false,
        false,
        false,
        AccountsDbConfig::default(),
        None,
        exit.clone(),
    )
    .with_context(|| {
        format!(
            "failed to load bank from snapshot archive {}",
            full_snapshot_archive.path().display()
        )
    })?;
    exit.store(true, Ordering::Relaxed);
    Ok(bank)
}

fn write_fastboot_bank_snapshot(paths: &BankCachePaths, bank: &Bank) -> Result<()> {
    ensure!(
        bank.is_complete(),
        "bank at slot {} is not complete; refusing to serialize fastboot snapshot",
        bank.slot()
    );

    bank.rc
        .accounts
        .accounts_db
        .set_latest_full_snapshot_slot(bank.slot());
    bank.squash();
    bank.rehash();
    bank.force_flush_accounts_cache();

    let snapshot_package = SnapshotPackage::new(
        SnapshotKind::Archive(SnapshotArchiveKind::Full),
        bank,
        bank.get_snapshot_storages(None),
        bank.status_cache.read().unwrap().root_slot_deltas(),
    );

    snapshot_utils::serialize_snapshot(
        paths.bank_snapshot_cache_dir(),
        SnapshotVersion::default(),
        snapshot_package.bank_snapshot_package,
        snapshot_package.snapshot_storages.as_slice(),
        true,
    )
    .with_context(|| {
        format!(
            "failed to write fastboot bank snapshot for slot {} into {}",
            bank.slot(),
            paths.bank_snapshot_cache_dir().display()
        )
    })?;

    Ok(())
}

fn load_bank_from_dir(
    config: &BankCacheConfig,
    skip_initial_hash_calc: bool,
    verify_index: bool,
    paths: &BankCachePaths,
    bank_snapshot_info: &BankSnapshotInfo,
) -> Result<Bank> {
    paths.ensure_accounts_run_dir()?;

    let genesis_config = open_genesis(&config.ledger_path)?;
    let exit = Arc::new(AtomicBool::new(false));
    let account_paths = vec![paths.accounts_run_dir().to_path_buf()];

    let bank = snapshot_bank_utils::bank_from_snapshot_dir(
        &account_paths,
        bank_snapshot_info,
        &genesis_config,
        &RuntimeConfig::default(),
        None,
        None,
        verify_index,
        AccountsDbConfig {
            skip_initial_hash_calc,
            ..AccountsDbConfig::default()
        },
        None,
        exit.clone(),
    )
    .with_context(|| {
        format!(
            "failed to load bank from snapshot dir {}",
            paths.bank_snapshot_dir().display()
        )
    })?;
    exit.store(true, Ordering::Relaxed);
    ensure_loaded_slot(&bank, bank_snapshot_info.slot)?;
    ensure_bank_is_frozen(&bank)?;

    Ok(bank)
}

fn open_genesis(ledger_path: &Path) -> Result<GenesisConfig> {
    open_genesis_config(ledger_path, MAX_GENESIS_ARCHIVE_UNPACKED_SIZE).with_context(|| {
        format!(
            "failed to open genesis config from {}",
            ledger_path.display()
        )
    })
}
