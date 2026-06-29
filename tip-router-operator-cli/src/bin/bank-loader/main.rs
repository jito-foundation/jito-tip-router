use {
    agave_snapshots::{
        paths::get_full_snapshot_archives,
        snapshot_archive_info::{FullSnapshotArchiveInfo, SnapshotArchiveInfoGetter},
        SnapshotArchiveKind, SnapshotKind, SnapshotVersion,
    },
    anyhow::{bail, ensure, Context, Result},
    clap::Parser,
    env_logger::Env,
    log::info,
    solana_genesis_config::GenesisConfig,
    solana_genesis_utils::{open_genesis_config, MAX_GENESIS_ARCHIVE_UNPACKED_SIZE},
    solana_ledger::blockstore_processor::ProcessOptions,
    solana_runtime::{
        bank::Bank,
        snapshot_bank_utils,
        snapshot_package::SnapshotPackage,
        snapshot_utils::{self, BankSnapshotInfo},
    },
    std::{
        fs,
        path::{Path, PathBuf},
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
        time::Instant,
    },
};

mod cli;

use cli::{Cli, Commands, FastbootFromSnapshotArgs, FromDirLoadOptions, FromFastbootDirArgs};

struct BankCacheConfig<'a> {
    ledger_path: &'a Path,
    slot: u64,
    output_dir: &'a Path,
}

impl<'a> From<&'a Cli> for BankCacheConfig<'a> {
    fn from(cli: &'a Cli) -> Self {
        Self {
            ledger_path: &cli.ledger_path,
            slot: cli.slot,
            output_dir: &cli.output_dir,
        }
    }
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    match &cli.command {
        Commands::FastbootFromSnapshot(args) => handle_fastboot_from_snapshot(&cli, args)?,
        Commands::FromFastbootDir(args) => handle_from_fastboot_dir(&cli, args)?,
    }

    Ok(())
}

fn handle_fastboot_from_snapshot(cli: &Cli, args: &FastbootFromSnapshotArgs) -> Result<()> {
    let config = BankCacheConfig::from(cli);
    let cache_paths = BankCachePaths::new(&config);
    let full_snapshot_archive =
        full_snapshot_archive_at_slot(&args.snapshot_archive_dir, config.slot)?;

    prepare_empty_cache_dirs(&config, &cache_paths)?;

    let load_started = Instant::now();
    let bank = load_bank_from_snapshot_archive(&config, &cache_paths, &full_snapshot_archive)?;
    let load_duration_ms = load_started.elapsed().as_millis();
    ensure_loaded_slot(&bank, config.slot)?;
    ensure_bank_is_frozen(&bank)?;

    let cache_write_started = Instant::now();
    write_fastboot_bank_snapshot(&cache_paths, &bank)?;
    let cache_write_duration_ms = cache_write_started.elapsed().as_millis();

    // This cache is intentionally disk-heavy. Keep the account run and snapshot
    // directories on one filesystem so Agave can hard-link account storage.
    info!("mode: fastboot-from-snapshot");
    info!(
        "snapshot_archive_dir: {}",
        args.snapshot_archive_dir.display()
    );
    info!(
        "snapshot_archive: {}",
        full_snapshot_archive.path().display()
    );
    info!("cache_write_duration_ms: {cache_write_duration_ms}");
    log_loaded_bank_context(&config, &bank, &cache_paths, load_duration_ms);
    Ok(())
}

fn handle_from_fastboot_dir(cli: &Cli, args: &FromFastbootDirArgs) -> Result<()> {
    let config = BankCacheConfig::from(cli);
    let cache_paths = BankCachePaths::new(&config);
    let load_options = FromDirLoadOptions::from(args);
    let load_started = Instant::now();
    let bank = load_bank_from_dir(&config, load_options, &cache_paths)?;
    let load_duration_ms = load_started.elapsed().as_millis();

    info!("mode: from-fastboot-dir");
    info!(
        "skip_initial_hash_calc: {}",
        load_options.skip_initial_hash_calc
    );
    info!("verify_index: {}", load_options.verify_index);
    log_loaded_bank_context(&config, &bank, &cache_paths, load_duration_ms);
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
    config: &BankCacheConfig<'_>,
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
    print_cache_paths(paths);
}

fn print_cache_paths(paths: &BankCachePaths) {
    info!(
        "bank_snapshot_cache_dir: {}",
        paths.bank_snapshot_cache_dir.display()
    );
    info!("bank_snapshot_dir: {}", paths.bank_snapshot_dir.display());
    info!("accounts_cache_dir: {}", paths.accounts_cache_dir.display());
    info!("accounts_run_dir: {}", paths.accounts_run_dir.display());
    info!(
        "accounts_snapshot_slot_dir: {}",
        paths.accounts_snapshot_slot_dir.display()
    );
}

struct BankCachePaths {
    bank_snapshot_cache_dir: PathBuf,
    bank_snapshot_dir: PathBuf,
    accounts_cache_dir: PathBuf,
    accounts_run_dir: PathBuf,
    accounts_snapshot_slot_dir: PathBuf,
}

impl BankCachePaths {
    fn new(config: &BankCacheConfig<'_>) -> Self {
        let bank_snapshot_cache_dir = config.output_dir.join("bank-snapshots");
        let accounts_cache_dir = config.output_dir.join("accounts");

        Self {
            bank_snapshot_dir: bank_snapshot_cache_dir.join(config.slot.to_string()),
            accounts_run_dir: accounts_cache_dir.join("run"),
            accounts_snapshot_slot_dir: accounts_cache_dir
                .join("snapshot")
                .join(config.slot.to_string()),
            bank_snapshot_cache_dir,
            accounts_cache_dir,
        }
    }
}

fn prepare_empty_cache_dirs(config: &BankCacheConfig<'_>, paths: &BankCachePaths) -> Result<()> {
    ensure!(
        !paths.bank_snapshot_dir.exists(),
        "bank snapshot cache already exists for slot {} at {}; refusing to overwrite",
        config.slot,
        paths.bank_snapshot_dir.display()
    );
    ensure!(
        !paths.accounts_snapshot_slot_dir.exists(),
        "accounts snapshot cache already exists for slot {} at {}; refusing to overwrite",
        config.slot,
        paths.accounts_snapshot_slot_dir.display()
    );

    fs::create_dir_all(&paths.bank_snapshot_cache_dir).with_context(|| {
        format!(
            "failed to create bank snapshot cache dir {}",
            paths.bank_snapshot_cache_dir.display()
        )
    })?;
    fs::create_dir_all(&paths.accounts_run_dir).with_context(|| {
        format!(
            "failed to create accounts run dir {}",
            paths.accounts_run_dir.display()
        )
    })?;
    fs::create_dir_all(paths.accounts_cache_dir.join("snapshot")).with_context(|| {
        format!(
            "failed to create accounts snapshot dir under {}",
            paths.accounts_cache_dir.display()
        )
    })?;

    ensure_empty_dir(&paths.accounts_run_dir, "accounts run dir")?;
    Ok(())
}

fn ensure_empty_dir(path: &Path, label: &str) -> Result<()> {
    let mut entries =
        fs::read_dir(path).with_context(|| format!("failed to read {label} {}", path.display()))?;
    ensure!(
        entries.next().is_none(),
        "{label} {} is not empty; refusing to mix cache state",
        path.display()
    );
    Ok(())
}

fn full_snapshot_archive_at_slot(
    snapshot_archive_dir: &Path,
    slot: u64,
) -> Result<FullSnapshotArchiveInfo> {
    let mut matching_archives = get_full_snapshot_archives(snapshot_archive_dir);
    matching_archives.retain(|archive| archive.slot() == slot);

    match matching_archives.len() {
        0 => bail!(
            "no full snapshot archive for slot {} found in {}",
            slot,
            snapshot_archive_dir.display()
        ),
        1 => Ok(matching_archives.remove(0)),
        count => bail!(
            "found {} full snapshot archives for slot {} in {}; expected exactly one",
            count,
            slot,
            snapshot_archive_dir.display()
        ),
    }
}

fn load_bank_from_snapshot_archive(
    config: &BankCacheConfig<'_>,
    paths: &BankCachePaths,
    full_snapshot_archive: &FullSnapshotArchiveInfo,
) -> Result<Bank> {
    let genesis_config = open_genesis(config.ledger_path)?;
    let process_options = ProcessOptions {
        halt_at_slot: Some(config.slot),
        ..Default::default()
    };
    let exit = Arc::new(AtomicBool::new(false));
    let account_paths = vec![paths.accounts_run_dir.clone()];

    let bank = snapshot_bank_utils::bank_from_snapshot_archives(
        &account_paths,
        &paths.bank_snapshot_cache_dir,
        full_snapshot_archive,
        None,
        &genesis_config,
        &process_options.runtime_config,
        process_options.debug_keys.clone(),
        process_options.limit_load_slot_count_from_snapshot,
        process_options.accounts_db_skip_shrink,
        process_options.accounts_db_force_initial_clean,
        process_options.verify_index,
        process_options.accounts_db_config.clone(),
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
        &paths.bank_snapshot_cache_dir,
        SnapshotVersion::default(),
        snapshot_package.bank_snapshot_package,
        snapshot_package.snapshot_storages.as_slice(),
        true,
    )
    .with_context(|| {
        format!(
            "failed to write fastboot bank snapshot for slot {} into {}",
            bank.slot(),
            paths.bank_snapshot_cache_dir.display()
        )
    })?;

    Ok(())
}

fn load_bank_from_dir(
    config: &BankCacheConfig<'_>,
    load_options: FromDirLoadOptions,
    paths: &BankCachePaths,
) -> Result<Bank> {
    fs::create_dir_all(&paths.accounts_run_dir).with_context(|| {
        format!(
            "failed to create accounts run dir {}",
            paths.accounts_run_dir.display()
        )
    })?;

    let bank_snapshot_info =
        BankSnapshotInfo::new_from_dir(&paths.bank_snapshot_cache_dir, config.slot).with_context(
            || {
                format!(
                    "failed to read bank snapshot dir {}",
                    paths.bank_snapshot_dir.display()
                )
            },
        )?;
    let genesis_config = open_genesis(config.ledger_path)?;
    let process_options = ProcessOptions {
        halt_at_slot: Some(config.slot),
        ..Default::default()
    };
    let mut accounts_db_config = process_options.accounts_db_config.clone();
    accounts_db_config.skip_initial_hash_calc = load_options.skip_initial_hash_calc;
    let exit = Arc::new(AtomicBool::new(false));
    let account_paths = vec![paths.accounts_run_dir.clone()];

    let bank = snapshot_bank_utils::bank_from_snapshot_dir(
        &account_paths,
        &bank_snapshot_info,
        &genesis_config,
        &process_options.runtime_config,
        process_options.debug_keys.clone(),
        process_options.limit_load_slot_count_from_snapshot,
        load_options.verify_index,
        accounts_db_config,
        None,
        exit.clone(),
    )
    .with_context(|| {
        format!(
            "failed to load bank from snapshot dir {}",
            paths.bank_snapshot_dir.display()
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
