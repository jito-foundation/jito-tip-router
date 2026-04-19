/// Diagnostic tool for inspecting why certain claim transactions fail.
///
/// Mirrors the fixed production logic from `get_claim_transactions_for_valid_unclaimed`:
///   - fetches TDAs first and uses the ON-CHAIN merkle_root_upload_authority to filter
///     tree nodes (instead of the stale file value)
///   - applies the same claim-status / claimant / amount filters
///   - simulates every remaining candidate and prints the full program logs
///
/// Run:
///   cargo run --example get_claim_transactions_for_valid_unclaimed -- \
///     --rpc-url <RPC> \
///     --save-path <DIR> \
///     --epoch <EPOCH> \
///     --tip-router-program-id <PUBKEY> \
///     --ncn-address <PUBKEY> \
///     --tip-distribution-program-id <PUBKEY> \
///     --priority-fee-distribution-program-id <PUBKEY>
use clap::Parser;
use itertools::Itertools;
use jito_priority_fee_distribution_sdk::PriorityFeeDistributionAccount;
use jito_tip_distribution_sdk::{
    derive_claim_status_account_address, TipDistributionAccount, CONFIG_SEED,
};
use jito_tip_router_client::instructions::ClaimWithPayerBuilder;
use jito_tip_router_core::{account_payer::AccountPayer, config::Config};
use meta_merkle_tree::generated_merkle_tree::GeneratedMerkleTreeCollection;
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_config::RpcSimulateTransactionConfig};
use solana_commitment_config::CommitmentConfig;
use solana_sdk::{account::Account, pubkey::Pubkey, transaction::Transaction};
use solana_system_interface::program as system_program;
use std::{collections::HashMap, path::PathBuf};
use tip_router_operator_cli::{
    merkle_tree_collection_file_name, priority_fees, rpc_utils::get_batched_accounts,
};

#[derive(Parser, Debug)]
#[command(about = "Inspect and simulate failing claim transactions for a given epoch")]
struct Args {
    #[arg(long, env, default_value = "http://localhost:8899")]
    rpc_url: String,

    #[arg(long, env)]
    save_path: PathBuf,

    #[arg(long, env)]
    epoch: u64,

    #[arg(long, env)]
    tip_router_program_id: Pubkey,

    #[arg(long, env)]
    ncn_address: Pubkey,

    #[arg(long, env)]
    tip_distribution_program_id: Pubkey,

    #[arg(long, env)]
    priority_fee_distribution_program_id: Pubkey,

    #[arg(long, env, default_value_t = 5000)]
    min_claim_amount: u64,

    #[arg(long, env, default_value_t = 1)]
    micro_lamports: u64,

    /// Payer pubkey for building transactions. Defaults to the tip-router AccountPayer PDA.
    /// sig_verify=false so no real signing is needed.
    #[arg(long, env)]
    payer: Option<Pubkey>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = Args::parse();

    let rpc_client =
        RpcClient::new_with_commitment(args.rpc_url.clone(), CommitmentConfig::confirmed());

    let tip_router_config_address =
        Config::find_program_address(&args.tip_router_program_id, &args.ncn_address).0;
    let tip_router_account_payer =
        AccountPayer::find_program_address(&args.tip_router_program_id, &args.ncn_address).0;

    let tip_distribution_config =
        Pubkey::find_program_address(&[CONFIG_SEED], &args.tip_distribution_program_id).0;
    let priority_fee_distribution_config = Pubkey::find_program_address(
        &[jito_priority_fee_distribution_sdk::CONFIG_SEED],
        &args.priority_fee_distribution_program_id,
    )
    .0;

    let payer_pubkey = args.payer.unwrap_or(tip_router_account_payer);

    println!("tip_router_config:   {tip_router_config_address}");
    println!("tip_router_payer:    {tip_router_account_payer}");
    println!("epoch:               {}", args.epoch);
    println!();

    // Load merkle tree collection from file
    let merkle_tree_path = args
        .save_path
        .join(merkle_tree_collection_file_name(args.epoch));
    let mut merkle_trees = GeneratedMerkleTreeCollection::new_from_file(&merkle_tree_path)
        .map_err(|e| anyhow::anyhow!("Failed to load merkle tree: {e}"))?;

    println!(
        "Loaded {} trees from {}",
        merkle_trees.generated_merkle_trees.len(),
        merkle_tree_path.display()
    );

    // Mirror the production "fix wrong claim status pubkeys" patch
    for tree in merkle_trees.generated_merkle_trees.iter_mut() {
        if tree.merkle_root_upload_authority != tip_router_config_address {
            continue;
        }
        for node in tree.tree_nodes.iter_mut() {
            let (claim_status_pubkey, claim_status_bump) = derive_claim_status_account_address(
                &tree.distribution_program,
                &node.claimant,
                &tree.distribution_account,
            );
            node.claim_status_pubkey = claim_status_pubkey;
            node.claim_status_bump = claim_status_bump;
        }
    }

    // ── Step 1: fetch ALL TDAs first (mirrors the fixed production logic) ──
    let tda_pubkeys: Vec<Pubkey> = merkle_trees
        .generated_merkle_trees
        .iter()
        .map(|t| t.distribution_account)
        .collect();

    println!("Fetching {} TDAs...", tda_pubkeys.len());
    let tdas: HashMap<Pubkey, Account> = get_batched_accounts(&rpc_client, &tda_pubkeys)
        .await?
        .into_iter()
        .filter_map(|(k, v)| Some((k, v?)))
        .collect();

    println!("TDAs found on-chain: {}", tdas.len());

    // ── Step 2: filter trees using the ON-CHAIN TDA authority (the fix) ──
    let qualifies =
        |tree: &meta_merkle_tree::generated_merkle_tree::GeneratedMerkleTree| -> bool {
            let Some(tda_account) = tdas.get(&tree.distribution_account) else {
                return false;
            };
            if tree
                .distribution_program
                .eq(&args.tip_distribution_program_id)
            {
                match TipDistributionAccount::deserialize(tda_account.data.as_slice()) {
                    Ok(tda) => {
                        tda.merkle_root.is_some()
                            && tda.merkle_root_upload_authority == tip_router_config_address
                    }
                    Err(_) => false,
                }
            } else if tree
                .distribution_program
                .eq(&args.priority_fee_distribution_program_id)
            {
                match PriorityFeeDistributionAccount::deserialize(tda_account.data.as_slice()) {
                    Ok(pfda) => {
                        pfda.merkle_root.is_some()
                            && pfda.merkle_root_upload_authority == tip_router_config_address
                    }
                    Err(_) => false,
                }
            } else {
                false
            }
        };

    let qualifying_nodes: Vec<_> = merkle_trees
        .generated_merkle_trees
        .iter()
        .filter(|t| qualifies(t))
        .flat_map(|t| t.tree_nodes.iter())
        .collect();

    println!("Qualifying nodes (on-chain TDA filter): {}", qualifying_nodes.len());

    // ── Step 3: fetch claimants and claim-status accounts ──
    let claimant_pubkeys: Vec<Pubkey> = qualifying_nodes.iter().map(|n| n.claimant).collect_vec();
    let claim_status_pubkeys: Vec<Pubkey> = qualifying_nodes
        .iter()
        .map(|n| n.claim_status_pubkey)
        .collect_vec();

    println!("Fetching {} claimant accounts...", claimant_pubkeys.len());
    let claimants: HashMap<Pubkey, Account> =
        get_batched_accounts(&rpc_client, &claimant_pubkeys)
            .await?
            .into_iter()
            .filter_map(|(k, v)| Some((k, v?)))
            .collect();

    println!("Fetching {} claim-status accounts...", claim_status_pubkeys.len());
    let claim_statuses: HashMap<Pubkey, Account> =
        get_batched_accounts(&rpc_client, &claim_status_pubkeys)
            .await?
            .into_iter()
            .filter_map(|(k, v)| Some((k, v?)))
            .collect();

    println!(
        "on-chain: claimants={} claim_statuses={}",
        claimants.len(),
        claim_statuses.len()
    );
    println!();

    // ── Step 4: apply all production filters and build candidate transactions ──
    let mut skip_no_tda = 0usize;
    let mut skip_wrong_auth = 0usize;
    let mut skip_no_merkle_root = 0usize;
    let mut skip_no_claimant = 0usize;
    let mut skip_already_claimed = 0usize;
    let mut skip_zero_amount = 0usize;
    let mut skip_below_min = 0usize;

    let mut candidates: Vec<(Pubkey, Transaction)> = Vec::new();

    for tree in &merkle_trees.generated_merkle_trees {
        if tree.max_total_claim == 0 {
            continue;
        }

        let tda_account = match tdas.get(&tree.distribution_account) {
            Some(a) => a,
            None => {
                skip_no_tda += tree.tree_nodes.len();
                continue;
            }
        };

        if tree
            .distribution_program
            .eq(&args.tip_distribution_program_id)
        {
            match TipDistributionAccount::deserialize(tda_account.data.as_slice()) {
                Ok(tda) => {
                    if tda.merkle_root.is_none() {
                        skip_no_merkle_root += tree.tree_nodes.len();
                        continue;
                    }
                    if tda.merkle_root_upload_authority != tip_router_config_address {
                        skip_wrong_auth += tree.tree_nodes.len();
                        continue;
                    }
                }
                Err(_) => {
                    skip_no_tda += tree.tree_nodes.len();
                    continue;
                }
            }
        } else if tree
            .distribution_program
            .eq(&args.priority_fee_distribution_program_id)
        {
            match PriorityFeeDistributionAccount::deserialize(tda_account.data.as_slice()) {
                Ok(pfda) => {
                    if pfda.merkle_root.is_none() {
                        skip_no_merkle_root += tree.tree_nodes.len();
                        continue;
                    }
                    if pfda.merkle_root_upload_authority != tip_router_config_address {
                        skip_wrong_auth += tree.tree_nodes.len();
                        continue;
                    }
                }
                Err(_) => {
                    skip_no_tda += tree.tree_nodes.len();
                    continue;
                }
            }
        } else {
            continue;
        }

        for node in &tree.tree_nodes {
            if !claimants.contains_key(&node.claimant) {
                skip_no_claimant += 1;
                continue;
            }
            if claim_statuses.contains_key(&node.claim_status_pubkey) {
                skip_already_claimed += 1;
                continue;
            }
            if node.amount == 0 {
                skip_zero_amount += 1;
                continue;
            }
            if node.amount < args.min_claim_amount {
                skip_below_min += 1;
                continue;
            }

            let mut builder = ClaimWithPayerBuilder::new();
            builder
                .config(tip_router_config_address)
                .account_payer(tip_router_account_payer)
                .ncn(args.ncn_address)
                .tip_distribution_account(tree.distribution_account)
                .claim_status(node.claim_status_pubkey)
                .claimant(node.claimant)
                .system_program(system_program::id())
                .proof(
                    node.proof
                        .clone()
                        .expect("claimable merkle tree node should include a proof"),
                )
                .amount(node.amount)
                .bump(node.claim_status_bump)
                .tip_distribution_program(tree.distribution_program);

            if tree
                .distribution_program
                .eq(&args.tip_distribution_program_id)
            {
                builder.tip_distribution_config(tip_distribution_config);
            } else {
                builder.tip_distribution_config(priority_fee_distribution_config);
            }

            let claim_ix = builder.instruction();
            let instructions =
                priority_fees::configure_instruction(claim_ix, args.micro_lamports, Some(100_000));
            let tx = Transaction::new_with_payer(&instructions, Some(&payer_pubkey));
            candidates.push((node.claim_status_pubkey, tx));
        }
    }

    println!("=== Filter breakdown ===");
    println!("  skip: TDA not on-chain or parse error  : {skip_no_tda}");
    println!("  skip: wrong merkle_root_upload_authority: {skip_wrong_auth}");
    println!("  skip: no merkle root uploaded           : {skip_no_merkle_root}");
    println!("  skip: claimant not on-chain             : {skip_no_claimant}");
    println!("  skip: already claimed                   : {skip_already_claimed}");
    println!("  skip: amount == 0                       : {skip_zero_amount}");
    println!(
        "  skip: amount < min ({})              : {skip_below_min}",
        args.min_claim_amount
    );
    println!("  ──────────────────────────────────────────");
    println!("  candidates (would be sent)              : {}", candidates.len());
    println!();

    if candidates.is_empty() {
        println!("No candidates — nothing would be sent. The fix is working correctly.");
        return Ok(());
    }

    // ── Step 5: simulate each candidate to expose the actual program error ──
    println!(
        "Simulating {} transactions...",
        candidates.len()
    );
    println!();

    let mut ok_count = 0usize;
    let mut already_in_use_count = 0usize;
    let mut other_err_count = 0usize;

    for (claim_status, tx) in &candidates {
        match rpc_client
            .simulate_transaction_with_config(
                tx,
                RpcSimulateTransactionConfig {
                    sig_verify: false,
                    replace_recent_blockhash: true,
                    commitment: Some(CommitmentConfig::processed()),
                    ..RpcSimulateTransactionConfig::default()
                },
            )
            .await
        {
            Ok(result) => {
                if let Some(err) = result.value.err {
                    let is_already_in_use = matches!(
                        solana_sdk::transaction::TransactionError::from(err.clone()),
                        solana_sdk::transaction::TransactionError::InstructionError(
                            _,
                            solana_sdk::instruction::InstructionError::Custom(0),
                        )
                    );
                    if is_already_in_use {
                        already_in_use_count += 1;
                        println!("claim_status: {claim_status}  [already in use — still a zombie after fix?]");
                    } else {
                        other_err_count += 1;
                        println!("claim_status: {claim_status}");
                        println!("  error: {err:?}");
                        if let Some(logs) = result.value.logs {
                            for log in logs {
                                println!("  log: {log}");
                            }
                        }
                        println!();
                    }
                } else {
                    ok_count += 1;
                }
            }
            Err(e) => {
                other_err_count += 1;
                println!("claim_status: {claim_status}");
                println!("  rpc error: {e}");
                println!();
            }
        }
    }

    println!();
    println!("=== Simulation summary ===");
    println!("  simulation OK (would land)       : {ok_count}");
    println!("  already-in-use / Custom(0)        : {already_in_use_count}");
    println!("  other errors                      : {other_err_count}");

    if already_in_use_count > 0 && ok_count == 0 && other_err_count == 0 {
        println!();
        println!(
            "All {} remaining candidates fail with Custom(0). \
             These are already-claimed accounts that the fix should now correctly detect. \
             If this count is still > 0 after deploying the fix, check whether \
             the file's merkle_root_upload_authority was updated after the on-chain TDA was claimed.",
            already_in_use_count
        );
    }

    Ok(())
}
