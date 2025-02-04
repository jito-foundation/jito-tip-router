use std::ffi::OsString;
use std::path::PathBuf;

use clap_old::{App, AppSettings, Arg, ArgMatches, SubCommand};
use solana_clap_utils::{
    hidden_unless_forced,
    input_validators::{
        is_parsable, is_pow2, is_pubkey, is_pubkey_or_keypair, is_slot, is_valid_percentage,
    },
};
use solana_ledger::use_snapshot_archives_at_startup;
use solana_runtime::snapshot_utils::{
    SnapshotVersion, DEFAULT_ARCHIVE_COMPRESSION, SUPPORTED_ARCHIVE_COMPRESSION,
};
use solana_sdk::{clock::Slot, rent::Rent};

// pub fn create_snapshot_arg_matches<'a, 'b>(
//     full_snapshots_archives_dir: PathBuf,
//     incremental_snapshots_archives_dir: PathBuf,
//     account_paths: Vec<PathBuf>,
// ) -> ArgMatches<'a> {
//     let mut app = App::new("tip-router-operator-cli");
//     ledger_tool_arg_app(&mut app);
//     let args: Vec<OsString> = vec![
//         "tip-router-operator-cli".into(),
//         "--full-snapshot-archive-path".into(),
//         full_snapshots_archives_dir.into(),
//         "--incremental-snapshot-archive-path".into(),
//         incremental_snapshots_archives_dir.into(),
//         "--accounts".into(),
//         account_paths
//             .iter()
//             .map(|p| p.to_string_lossy().to_string())
//             .collect::<Vec<_>>()
//             .join(",")
//             .into(),
//     ];

//     app.get_matches_from(args)
// }

pub fn set_ledger_tool_arg_matches(
    arg_matches: &mut ArgMatches<'_>,
    full_snapshots_archives_dir: PathBuf,
    incremental_snapshots_archives_dir: PathBuf,
    _account_paths: Vec<PathBuf>,
) {
    let _rent = Rent::default();
    // let default_bootstrap_validator_lamports = sol_to_lamports(500.0)
    //     .max(VoteState::get_rent_exempt_reserve(&rent))
    //     .to_string();
    // let default_bootstrap_validator_stake_lamports = sol_to_lamports(0.5)
    //     .max(rent.minimum_balance(StakeStateV2::size_of()))
    //     .to_string();

    let load_genesis_config_arg = load_genesis_arg();
    let accounts_db_config_args = accounts_db_args();
    let snapshot_config_args = snapshot_args();

    let _accounts_db_test_hash_calculation_arg =
        Arg::with_name("accounts_db_test_hash_calculation")
            .long("accounts-db-test-hash-calculation")
            .help("Enable hash calculation test");
    let _halt_at_slot_arg = Arg::with_name("halt_at_slot")
        .long("halt-at-slot")
        .value_name("SLOT")
        .validator(is_slot)
        .takes_value(true)
        .help("Halt processing at the given slot");
    let _os_memory_stats_reporting_arg = Arg::with_name("os_memory_stats_reporting")
        .long("os-memory-stats-reporting")
        .help("Enable reporting of OS memory statistics.");
    let _halt_at_slot_store_hash_raw_data = Arg::with_name("halt_at_slot_store_hash_raw_data")
        .long("halt-at-slot-store-hash-raw-data")
        .help(
            "After halting at slot, run an accounts hash calculation and store the raw hash data \
         for debugging.",
        )
        .hidden(hidden_unless_forced());
    let _verify_index_arg = Arg::with_name("verify_accounts_index")
        .long("verify-accounts-index")
        .takes_value(false)
        .help("For debugging and tests on accounts index.");
    let _limit_load_slot_count_from_snapshot_arg =
        Arg::with_name("limit_load_slot_count_from_snapshot")
            .long("limit-load-slot-count-from-snapshot")
            .value_name("SLOT")
            .validator(is_slot)
            .takes_value(true)
            .help(
                "For debugging and profiling with large snapshots, artificially limit how many \
             slots are loaded from a snapshot.",
            );
    let hard_forks_arg = Arg::with_name("hard_forks")
        .long("hard-fork")
        .value_name("SLOT")
        .validator(is_slot)
        .multiple(true)
        .takes_value(true)
        .help("Add a hard fork at this slot");
    let _allow_dead_slots_arg = Arg::with_name("allow_dead_slots")
        .long("allow-dead-slots")
        .takes_value(false)
        .help("Output dead slots as well");
    let hashes_per_tick = Arg::with_name("hashes_per_tick")
        .long("hashes-per-tick")
        .value_name("NUM_HASHES|\"sleep\"")
        .takes_value(true)
        .help(
            "How many PoH hashes to roll before emitting the next tick. If \"sleep\", for \
         development sleep for the target tick duration instead of hashing",
        );
    let snapshot_version_arg = Arg::with_name("snapshot_version")
        .long("snapshot-version")
        .value_name("SNAPSHOT_VERSION")
        .validator(is_parsable::<SnapshotVersion>)
        .takes_value(true)
        .default_value(SnapshotVersion::default().into())
        .help("Output snapshot version");
    let _debug_key_arg = Arg::with_name("debug_key")
        .long("debug-key")
        .validator(is_pubkey)
        .value_name("ADDRESS")
        .multiple(true)
        .takes_value(true)
        .help("Log when transactions are processed that reference the given key(s).");

    let geyser_plugin_args = Arg::with_name("geyser_plugin_config")
        .long("geyser-plugin-config")
        .value_name("FILE")
        .takes_value(true)
        .multiple(true)
        .help("Specify the configuration file for the Geyser plugin.");

    let log_messages_bytes_limit_arg = Arg::with_name("log_messages_bytes_limit")
        .long("log-messages-bytes-limit")
        .takes_value(true)
        .validator(is_parsable::<usize>)
        .value_name("BYTES")
        .help("Maximum number of bytes written to the program log before truncation");

    let _accounts_data_encoding_arg = Arg::with_name("encoding")
        .long("encoding")
        .takes_value(true)
        .possible_values(&["base64", "base64+zstd", "jsonParsed"])
        .default_value("base64")
        .help("Print account data in specified format when printing account contents.");

    let app = App::new("tip-router-operator-cli")
        .about("Tip Router Operator CLI")
        .version("0.1.0")
        .global_setting(AppSettings::ColoredHelp)
        .global_setting(AppSettings::InferSubcommands)
        .global_setting(AppSettings::UnifiedHelpMessage)
        .global_setting(AppSettings::VersionlessSubcommands)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg(
            Arg::with_name("ledger_path")
                .short("l")
                .long("ledger")
                .value_name("DIR")
                .takes_value(true)
                .global(true)
                .default_value("ledger")
                .help("Use DIR as ledger location"),
        )
        .arg(
            Arg::with_name("wal_recovery_mode")
                .long("wal-recovery-mode")
                .value_name("MODE")
                .takes_value(true)
                .global(true)
                .possible_values(&[
                    "tolerate_corrupted_tail_records",
                    "absolute_consistency",
                    "point_in_time",
                    "skip_any_corrupted_record",
                ])
                .help("Mode to recovery the ledger db write ahead log"),
        )
        .arg(
            Arg::with_name("force_update_to_open")
                .long("force-update-to-open")
                .takes_value(false)
                .global(true)
                .help(
                    "Allow commands that would otherwise not alter the blockstore to make \
                 necessary updates in order to open it",
                ),
        )
        .arg(
            Arg::with_name("ignore_ulimit_nofile_error")
                .long("ignore-ulimit-nofile-error")
                .takes_value(false)
                .global(true)
                .help(
                    "Allow opening the blockstore to succeed even if the desired open file \
                 descriptor limit cannot be configured. Use with caution as some commands may \
                 run fine with a reduced file descriptor limit while others will not",
                ),
        )
        .arg(
            Arg::with_name("block_verification_method")
                .long("block-verification-method")
                .value_name("METHOD")
                .takes_value(true)
                // .possible_values(BlockVerificationMethod::cli_names())
                .global(true), // .help(BlockVerificationMethod::cli_message()),
        )
        .arg(
            Arg::with_name("unified_scheduler_handler_threads")
                .long("unified-scheduler-handler-threads")
                .value_name("COUNT")
                .takes_value(true)
                // .validator(|s| is_within_range(s, 1..))
                .global(true), // .help(DefaultSchedulerPool::cli_message()),
        )
        .arg(
            Arg::with_name("output_format")
                .long("output")
                .value_name("FORMAT")
                .global(true)
                .takes_value(true)
                .possible_values(&["json", "json-compact"])
                .help(
                    "Return information in specified output format, currently only available for \
                 bigtable and program subcommands",
                ),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .global(true)
                .multiple(true)
                .takes_value(false)
                .help("Show additional information where supported"),
        )
        .subcommand(
            SubCommand::with_name("create-snapshot")
                .about("Create a new ledger snapshot")
                .arg(&load_genesis_config_arg)
                .args(&accounts_db_config_args)
                .args(&snapshot_config_args)
                .arg(&hard_forks_arg)
                .arg(&snapshot_version_arg)
                .arg(&geyser_plugin_args)
                .arg(&log_messages_bytes_limit_arg)
                .arg(
                    Arg::with_name("snapshot_slot")
                        .index(1)
                        .value_name("SLOT")
                        .validator(|value| {
                            if value.parse::<Slot>().is_ok() || value == "ROOT" {
                                Ok(())
                            } else {
                                Err(format!(
                                    "Unable to parse as a number or the keyword ROOT, provided: \
                                 {value}"
                                ))
                            }
                        })
                        .takes_value(true)
                        .help(
                            "Slot at which to create the snapshot; accepts keyword ROOT for the \
                         highest root",
                        ),
                )
                .arg(
                    Arg::with_name("output_directory")
                        .index(2)
                        .value_name("DIR")
                        .takes_value(true)
                        .help(
                            "Output directory for the snapshot \
                        [default: --snapshot-archive-path if present else --ledger directory]",
                        ),
                )
                .arg(
                    Arg::with_name("warp_slot")
                        .required(false)
                        .long("warp-slot")
                        .takes_value(true)
                        .value_name("WARP_SLOT")
                        .validator(is_slot)
                        .help(
                            "After loading the snapshot slot warp the ledger to WARP_SLOT, which \
                         could be a slot in a galaxy far far away",
                        ),
                )
                .arg(
                    Arg::with_name("faucet_lamports")
                        .short("t")
                        .long("faucet-lamports")
                        .value_name("LAMPORTS")
                        .takes_value(true)
                        .requires("faucet_pubkey")
                        .help("Number of lamports to assign to the faucet"),
                )
                .arg(
                    Arg::with_name("faucet_pubkey")
                        .short("m")
                        .long("faucet-pubkey")
                        .value_name("PUBKEY")
                        .takes_value(true)
                        .validator(is_pubkey_or_keypair)
                        .requires("faucet_lamports")
                        .help("Path to file containing the faucet's pubkey"),
                )
                .arg(
                    Arg::with_name("bootstrap_validator")
                        .short("b")
                        .long("bootstrap-validator")
                        .value_name("IDENTITY_PUBKEY VOTE_PUBKEY STAKE_PUBKEY")
                        .takes_value(true)
                        .validator(is_pubkey_or_keypair)
                        .number_of_values(3)
                        .multiple(true)
                        .help("The bootstrap validator's identity, vote and stake pubkeys"),
                )
                .arg(
                    Arg::with_name("bootstrap_stake_authorized_pubkey")
                        .long("bootstrap-stake-authorized-pubkey")
                        .value_name("BOOTSTRAP STAKE AUTHORIZED PUBKEY")
                        .takes_value(true)
                        .validator(is_pubkey_or_keypair)
                        .help(
                            "Path to file containing the pubkey authorized to manage the \
                         bootstrap validator's stake
                         [default: --bootstrap-validator IDENTITY_PUBKEY]",
                        ),
                )
                // .arg(
                //     Arg::with_name("bootstrap_validator_lamports")
                //         .long("bootstrap-validator-lamports")
                //         .value_name("LAMPORTS")
                //         .takes_value(true)
                //         .default_value(&default_bootstrap_validator_lamports)
                //         .help("Number of lamports to assign to the bootstrap validator"),
                // )
                // .arg(
                //     Arg::with_name("bootstrap_validator_stake_lamports")
                //         .long("bootstrap-validator-stake-lamports")
                //         .value_name("LAMPORTS")
                //         .takes_value(true)
                //         .default_value(&default_bootstrap_validator_stake_lamports)
                //         .help(
                //             "Number of lamports to assign to the bootstrap validator's stake \
                //          account",
                //         ),
                // )
                .arg(
                    Arg::with_name("rent_burn_percentage")
                        .long("rent-burn-percentage")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .help("Adjust percentage of collected rent to burn")
                        .validator(is_valid_percentage),
                )
                .arg(&hashes_per_tick)
                .arg(
                    Arg::with_name("accounts_to_remove")
                        .required(false)
                        .long("remove-account")
                        .takes_value(true)
                        .value_name("PUBKEY")
                        .validator(is_pubkey)
                        .multiple(true)
                        .help("List of accounts to remove while creating the snapshot"),
                )
                .arg(
                    Arg::with_name("feature_gates_to_deactivate")
                        .required(false)
                        .long("deactivate-feature-gate")
                        .takes_value(true)
                        .value_name("PUBKEY")
                        .validator(is_pubkey)
                        .multiple(true)
                        .help("List of feature gates to deactivate while creating the snapshot"),
                )
                .arg(
                    Arg::with_name("vote_accounts_to_destake")
                        .required(false)
                        .long("destake-vote-account")
                        .takes_value(true)
                        .value_name("PUBKEY")
                        .validator(is_pubkey)
                        .multiple(true)
                        .help("List of validator vote accounts to destake"),
                )
                .arg(
                    Arg::with_name("remove_stake_accounts")
                        .required(false)
                        .long("remove-stake-accounts")
                        .takes_value(false)
                        .help("Remove all existing stake accounts from the new snapshot"),
                )
                .arg(
                    Arg::with_name("incremental")
                        .long("incremental")
                        .takes_value(false)
                        .help(
                            "Create an incremental snapshot instead of a full snapshot. This \
                         requires that the ledger is loaded from a full snapshot, which will \
                         be used as the base for the incremental snapshot.",
                        )
                        .conflicts_with("no_snapshot"),
                )
                .arg(
                    Arg::with_name("minimized")
                        .long("minimized")
                        .takes_value(false)
                        .help(
                            "Create a minimized snapshot instead of a full snapshot. This \
                         snapshot will only include information needed to replay the ledger \
                         from the snapshot slot to the ending slot.",
                        )
                        .conflicts_with("incremental")
                        .requires("ending_slot"),
                )
                .arg(
                    Arg::with_name("ending_slot")
                        .long("ending-slot")
                        .takes_value(true)
                        .value_name("ENDING_SLOT")
                        .help("Ending slot for minimized snapshot creation"),
                )
                .arg(
                    Arg::with_name("snapshot_archive_format")
                        .long("snapshot-archive-format")
                        .possible_values(SUPPORTED_ARCHIVE_COMPRESSION)
                        .default_value(DEFAULT_ARCHIVE_COMPRESSION)
                        .value_name("ARCHIVE_TYPE")
                        .takes_value(true)
                        .help("Snapshot archive format to use.")
                        .conflicts_with("no_snapshot"),
                )
                .arg(
                    Arg::with_name("enable_capitalization_change")
                        .long("enable-capitalization-change")
                        .takes_value(false)
                        .help("If snapshot creation should succeed with a capitalization delta."),
                ),
        );

    let args: Vec<OsString> = vec![
        "tip-router-operator-cli".into(),
        "create-snapshot".into(),
        "--full-snapshot-archive-path".into(),
        full_snapshots_archives_dir.into(),
        "--incremental-snapshot-archive-path".into(),
        incremental_snapshots_archives_dir.into(),
        // "--accounts".into(),
        // account_paths
        //     .iter()
        //     .map(|p| p.to_string_lossy().to_string())
        //     .collect::<Vec<_>>()
        //     .join(",")
        //     .into(),
    ];

    *arg_matches = app.get_matches_from(args);
}

/// Returns the arguments that configure AccountsDb
pub fn accounts_db_args<'a, 'b>() -> Box<[Arg<'a, 'b>]> {
    vec![
        Arg::with_name("account_paths")
            .long("accounts")
            .value_name("PATHS")
            .takes_value(true)
            .help(
                "Persistent accounts location. May be specified multiple times. \
                [default: <LEDGER>/accounts]",
            ),
        Arg::with_name("accounts_index_path")
            .long("accounts-index-path")
            .value_name("PATH")
            .takes_value(true)
            .multiple(true)
            .help(
                "Persistent accounts-index location. May be specified multiple times. \
                [default: <LEDGER>/accounts_index]",
            ),
        Arg::with_name("accounts_hash_cache_path")
            .long("accounts-hash-cache-path")
            .value_name("PATH")
            .takes_value(true)
            .help(
                "Use PATH as accounts hash cache location [default: <LEDGER>/accounts_hash_cache]",
            ),
        Arg::with_name("accounts_index_bins")
            .long("accounts-index-bins")
            .value_name("BINS")
            .validator(is_pow2)
            .takes_value(true)
            .help("Number of bins to divide the accounts index into"),
        Arg::with_name("accounts_index_memory_limit_mb")
            .long("accounts-index-memory-limit-mb")
            .value_name("MEGABYTES")
            .validator(is_parsable::<usize>)
            .takes_value(true)
            .help(
                "How much memory the accounts index can consume. If this is exceeded, some \
                 account index entries will be stored on disk.",
            ),
        Arg::with_name("disable_accounts_disk_index")
            .long("disable-accounts-disk-index")
            .help(
                "Disable the disk-based accounts index. It is enabled by default. The entire \
                 accounts index will be kept in memory.",
            )
            .conflicts_with("accounts_index_memory_limit_mb"),
        Arg::with_name("accounts_db_skip_shrink")
            .long("accounts-db-skip-shrink")
            .help(
                "Enables faster starting of ledger-tool by skipping shrink. This option is for \
                use during testing.",
            ),
        Arg::with_name("accounts_db_verify_refcounts")
            .long("accounts-db-verify-refcounts")
            .help(
                "Debug option to scan all AppendVecs and verify account index refcounts prior to \
                clean",
            )
            .hidden(hidden_unless_forced()),
        Arg::with_name("accounts_db_test_skip_rewrites")
            .long("accounts-db-test-skip-rewrites")
            .help(
                "Debug option to skip rewrites for rent-exempt accounts but still add them in \
                 bank delta hash calculation",
            )
            .hidden(hidden_unless_forced()),
        Arg::with_name("accounts_db_skip_initial_hash_calculation")
            .long("accounts-db-skip-initial-hash-calculation")
            .help("Do not verify accounts hash at startup.")
            .hidden(hidden_unless_forced()),
        Arg::with_name("accounts_db_ancient_append_vecs")
            .long("accounts-db-ancient-append-vecs")
            .value_name("SLOT-OFFSET")
            .validator(is_parsable::<i64>)
            .takes_value(true)
            .help(
                "AppendVecs that are older than (slots_per_epoch - SLOT-OFFSET) are squashed \
                 together.",
            )
            .hidden(hidden_unless_forced()),
    ]
    .into_boxed_slice()
}

// For our current version of CLAP, the value passed to Arg::default_value()
// must be a &str. But, we can't convert an integer to a &str at compile time.
// So, declare this constant and enforce equality with the following unit test
// test_max_genesis_archive_unpacked_size_constant
const MAX_GENESIS_ARCHIVE_UNPACKED_SIZE_STR: &str = "10485760";

/// Returns the arguments that configure loading genesis
pub fn load_genesis_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("max_genesis_archive_unpacked_size")
        .long("max-genesis-archive-unpacked-size")
        .value_name("NUMBER")
        .takes_value(true)
        .default_value(MAX_GENESIS_ARCHIVE_UNPACKED_SIZE_STR)
        .help("maximum total uncompressed size of unpacked genesis archive")
}

/// Returns the arguments that configure snapshot loading
pub fn snapshot_args<'a, 'b>() -> Box<[Arg<'a, 'b>]> {
    vec![
        Arg::with_name("no_snapshot")
            .long("no-snapshot")
            .takes_value(false)
            .help("Do not start from a local snapshot if present"),
        Arg::with_name("snapshots")
            .long("snapshots")
            .alias("snapshot-archive-path")
            .alias("full-snapshot-archive-path")
            .value_name("DIR")
            .takes_value(true)
            .global(true)
            .help("Use DIR for snapshot location [default: --ledger value]"),
        Arg::with_name("incremental_snapshot_archive_path")
            .long("incremental-snapshot-archive-path")
            .value_name("DIR")
            .takes_value(true)
            .global(true)
            .help("Use DIR for separate incremental snapshot location"),
        Arg::with_name(use_snapshot_archives_at_startup::cli::NAME)
            .long(use_snapshot_archives_at_startup::cli::LONG_ARG)
            .takes_value(true)
            .possible_values(use_snapshot_archives_at_startup::cli::POSSIBLE_VALUES)
            .default_value(use_snapshot_archives_at_startup::cli::default_value_for_ledger_tool())
            .help(use_snapshot_archives_at_startup::cli::HELP)
            .long_help(use_snapshot_archives_at_startup::cli::LONG_HELP),
    ]
    .into_boxed_slice()
}
