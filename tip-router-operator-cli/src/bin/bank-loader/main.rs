use {
    agave_snapshots::{
        paths::get_full_snapshot_archives,
        snapshot_archive_info::{FullSnapshotArchiveInfo, SnapshotArchiveInfoGetter},
        SnapshotArchiveKind, SnapshotKind, SnapshotVersion,
    },
    anyhow::{anyhow, ensure, Context, Result},
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

use bankcache::BankCachePaths;
use cli::{BankCacheConfig, BankCacheFromSnapshotArgs, Cli, Commands, LoadBankCacheArgs};

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
    let cache_paths = BankCachePaths::new(config.output_dir.clone(), config.slot);
    let full_snapshot_archive =
        full_snapshot_archive_at_slot(&args.snapshot_archive_dir, config.slot)?;

    cache_paths.prepare_empty_dirs()?;

    let load_started = Instant::now();
    let bank = load_bank_from_snapshot_archive(config, &cache_paths, &full_snapshot_archive)?;
    let load_duration_ms = load_started.elapsed().as_millis();
    ensure_loaded_slot(&bank, config.slot)?;
    ensure_bank_is_frozen(&bank)?;

    let cache_write_started = Instant::now();
    write_fastboot_bank_snapshot(&cache_paths, &bank)?;
    let cache_write_duration_ms = cache_write_started.elapsed().as_millis();

    // This cache is intentionally disk-heavy. Keep the account run and snapshot
    // directories on one filesystem so Agave can hard-link account storage.
    info!(
        "mode: create-bank-cache snapshot_archive_dir: {} snapshot_archive: {} cache_write_duration_ms: {cache_write_duration_ms}",
        args.snapshot_archive_dir.display(),
        full_snapshot_archive.path().display()
    );
    log_loaded_bank_context(config, &bank, &cache_paths, load_duration_ms);
    Ok(())
}

fn handle_load_bankcache(args: &LoadBankCacheArgs) -> Result<()> {
    let config = &args.cache;
    let cache_paths = BankCachePaths::new(config.output_dir.clone(), config.slot);
    let load_started = Instant::now();
    let bank = load_bank_from_dir(
        config,
        args.skip_initial_hash_calc,
        args.verify_index,
        &cache_paths,
    )?;
    let load_duration_ms = load_started.elapsed().as_millis();

    info!(
        "mode: load-bank-cache skip_initial_hash_calc: {} verify_index: {}",
        args.skip_initial_hash_calc, args.verify_index
    );
    log_loaded_bank_context(config, &bank, &cache_paths, load_duration_ms);
    Ok(())
}

fn ensure_loaded_slot(bank: &Bank, requested_slot: u64) -> Result<()> {
    ensure!(
        bank.slot() == requested_slot,
        "loaded bank slot {} does not match requested slot {}",
        bank.slot(),
        requested_slot
    );
    Ok(())
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
    info!("slot: {}", config.slot);
    info!("bank_hash: {}", bank.hash());
    info!("epoch: {}", bank.epoch());
    info!("load_duration_ms: {load_duration_ms}");
    info!("{paths:?}");
}

fn full_snapshot_archive_at_slot(
    snapshot_archive_dir: &Path,
    slot: u64,
) -> Result<FullSnapshotArchiveInfo> {
    let full_archives = get_full_snapshot_archives(snapshot_archive_dir);

    // There can technically be more than one for a single slot, but we will ignore this for now bc
    // it seems like an extreme edge case
    let snapshot = full_archives.iter().find(|archive| archive.slot() == slot);

    snapshot.cloned().ok_or(anyhow!(
        "no full snapshot archive for slot {} found in {}",
        slot,
        snapshot_archive_dir.display()
    ))
}

fn load_bank_from_snapshot_archive(
    config: &BankCacheConfig,
    paths: &BankCachePaths,
    full_snapshot_archive: &FullSnapshotArchiveInfo,
) -> Result<Bank> {
    let genesis_config = open_genesis(&config.ledger_path)?;
    let exit = Arc::new(AtomicBool::new(false));

    let bank = snapshot_bank_utils::bank_from_snapshot_archives(
        &[paths.accounts_run_dir().to_path_buf()],
        paths.bank_snapshot_cache_dir(),
        full_snapshot_archive,
        None,
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
) -> Result<Bank> {
    paths.ensure_accounts_run_dir()?;

    let bank_snapshot_info =
        BankSnapshotInfo::new_from_dir(paths.bank_snapshot_cache_dir(), config.slot).with_context(
            || {
                format!(
                    "failed to read bank snapshot dir {}",
                    paths.bank_snapshot_dir().display()
                )
            },
        )?;
    let genesis_config = open_genesis(&config.ledger_path)?;
    let exit = Arc::new(AtomicBool::new(false));
    let account_paths = vec![paths.accounts_run_dir().to_path_buf()];

    let bank = snapshot_bank_utils::bank_from_snapshot_dir(
        &account_paths,
        &bank_snapshot_info,
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
    ensure_loaded_slot(&bank, config.slot)?;
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
