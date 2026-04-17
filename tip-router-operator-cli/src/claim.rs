use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use itertools::Itertools;
use jito_priority_fee_distribution_sdk::PriorityFeeDistributionAccount;
use jito_tip_distribution_sdk::{
    derive_claim_status_account_address, ClaimStatus, TipDistributionAccount, CLAIM_STATUS_SIZE,
    CONFIG_SEED,
};
use jito_tip_router_client::instructions::ClaimWithPayerBuilder;
use jito_tip_router_core::{account_payer::AccountPayer, config::Config};
use log::{info, warn};
use meta_merkle_tree::generated_merkle_tree::{GeneratedMerkleTreeCollection, TreeNode};
use rand::{prelude::SliceRandom, thread_rng};
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_config::RpcSimulateTransactionConfig};
use solana_commitment_config::CommitmentConfig;
use solana_metrics::{datapoint_error, datapoint_info};
#[allow(deprecated)]
use solana_sdk::{
    account::Account, fee_calculator::DEFAULT_TARGET_LAMPORTS_PER_SIGNATURE,
    native_token::LAMPORTS_PER_SOL, pubkey::Pubkey, signature::Keypair, signer::Signer,
    transaction::Transaction,
};
use solana_system_interface::program as system_program;
use thiserror::Error;
use tokio::fs::File;
use tokio::fs::OpenOptions;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use tokio::sync::Mutex;

use crate::{
    claim::claim_processor::ClaimProcessor,
    get_epoch_percentage, merkle_tree_collection_file_name, priority_fees,
    rpc_utils::{get_batched_accounts, send_until_blockhash_expires},
    Cli,
};

pub mod claim_processor;

#[derive(Error, Debug)]
pub enum ClaimMevError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    JsonError(#[from] serde_json::Error),

    #[error("Failed to deserialize account data: {0}")]
    AnchorError(String),

    #[error(transparent)]
    RpcError(#[from] solana_rpc_client_api::client_error::Error),

    #[error("Expected to have at least {desired_balance} lamports in {payer:?}. Current balance is {start_balance} lamports. Deposit {sol_to_deposit} SOL to continue.")]
    InsufficientBalance {
        desired_balance: u64,
        payer: Pubkey,
        start_balance: u64,
        sol_to_deposit: u64,
    },

    #[error("Not finished with job, transactions left {transactions_left}")]
    NotFinished { transactions_left: usize },

    #[error("Failed to check or update completed epochs: {0}")]
    CompletedEpochsError(String),

    #[error("UncaughtError {e:?}")]
    UncaughtError { e: String },
}

#[allow(clippy::too_many_arguments)]
pub async fn emit_claim_mev_tips_metrics(
    cli: &Cli,
    epoch: u64,
    tip_distribution_program_id: Pubkey,
    priority_fee_distribution_program_id: Pubkey,
    tip_router_program_id: Pubkey,
    ncn: Pubkey,
    file_path: &PathBuf,
    file_mutex: &Arc<Mutex<()>>,
) -> Result<(), anyhow::Error> {
    let meta_merkle_tree_dir = cli.get_save_path().clone();
    let merkle_tree_coll_path = meta_merkle_tree_dir.join(merkle_tree_collection_file_name(epoch));
    let merkle_trees = GeneratedMerkleTreeCollection::new_from_file(&merkle_tree_coll_path)
        .map_err(|e| anyhow::anyhow!(e))?;

    let rpc_url = cli.rpc_url.clone();
    let rpc_client = RpcClient::new_with_timeout_and_commitment(
        rpc_url,
        Duration::from_secs(1800),
        CommitmentConfig::confirmed(),
    );

    let epoch = merkle_trees.epoch;
    let current_epoch = rpc_client.get_epoch_info().await?.epoch;
    if is_epoch_completed(epoch, current_epoch, file_path, file_mutex).await? {
        return Ok(());
    }

    let (claims_to_process, validators_processed, _) = get_claim_transactions_for_valid_unclaimed(
        &rpc_client,
        &merkle_trees,
        tip_distribution_program_id,
        priority_fee_distribution_program_id,
        tip_router_program_id,
        ncn,
        0,
        Pubkey::new_unique(),
        cli.min_claim_amount,
        &cli.operator_address,
        &cli.cluster,
        &HashSet::new(),
    )
    .await?;

    if validators_processed {
        match get_epoch_percentage(&rpc_client).await {
            Ok(epoch_percentage) => {
                datapoint_info!(
                    "tip_router_cli.claim_mev_tips-metrics_only",
                    ("claim_transactions_left", claims_to_process.len(), i64),
                    ("epoch", epoch, i64),
                    ("epoch_percentage", epoch_percentage, f64),
                    "cluster" => &cli.cluster,
                );
            }
            Err(e) => {
                warn!("Failed to fetch epoch percentage for claims: {e:?}");
            }
        }
    }

    if validators_processed && claims_to_process.is_empty() {
        add_completed_epoch(epoch, current_epoch, file_path, file_mutex).await?;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn handle_claim_mev_tips(
    cli: &Cli,
    epoch: u64,
    tip_distribution_program_id: Pubkey,
    priority_fee_distribution_program_id: Pubkey,
    tip_router_program_id: Pubkey,
    ncn: Pubkey,
    max_loop_duration: Duration,
    file_path: &PathBuf,
    file_mutex: &Arc<Mutex<()>>,
    keypair: &Arc<Keypair>,
    rpc_url: String,
    processor: &ClaimProcessor,
) -> Result<(), anyhow::Error> {
    let meta_merkle_tree_dir = cli.get_save_path().clone();
    let merkle_tree_coll_path = meta_merkle_tree_dir.join(merkle_tree_collection_file_name(epoch));
    let mut merkle_tree_coll = GeneratedMerkleTreeCollection::new_from_file(&merkle_tree_coll_path)
        .map_err(|e| anyhow::anyhow!(e))?;

    let tip_router_config_address = Config::find_program_address(&tip_router_program_id, &ncn).0;

    // Fix wrong claim status pubkeys for 1 epoch -- noop if already correct
    for tree in merkle_tree_coll.generated_merkle_trees.iter_mut() {
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

    let start = Instant::now();

    match claim_mev_tips(
        &merkle_tree_coll,
        rpc_url.clone(),
        rpc_url.clone(),
        tip_distribution_program_id,
        priority_fee_distribution_program_id,
        tip_router_program_id,
        ncn,
        keypair,
        max_loop_duration,
        cli.claim_microlamports,
        cli.min_claim_amount,
        file_path,
        file_mutex,
        &cli.operator_address,
        &cli.cluster,
        processor,
    )
    .await
    {
        Ok(()) => {
            datapoint_info!(
                "claim_mev_workflow",
                ("operator", cli.operator_address, String),
                ("epoch", epoch, i64),
                ("transactions_left", 0, i64),
                ("elapsed_us", start.elapsed().as_micros(), i64),
                "cluster" => &cli.cluster,
            );
        }
        Err(ClaimMevError::NotFinished { transactions_left }) => {
            datapoint_info!(
                "claim_mev_workflow",
                ("operator", cli.operator_address, String),
                ("epoch", epoch, i64),
                ("transactions_left", transactions_left, i64),
                ("elapsed_us", start.elapsed().as_micros(), i64),
                "cluster" => &cli.cluster,
            );
        }
        Err(e) => {
            datapoint_error!(
                "claim_mev_workflow",
                ("operator", cli.operator_address, String),
                ("epoch", epoch, i64),
                ("error", e.to_string(), String),
                ("elapsed_us", start.elapsed().as_micros(), i64),
                "cluster" => &cli.cluster,
            );
        }
    }

    let claimer_balance = get_claimer_balance(rpc_url, keypair).await?;
    datapoint_info!(
        "claimer_info",
        ("claimer", keypair.pubkey().to_string(), String),
        ("epoch", epoch, i64),
        ("lamport_balance", claimer_balance, i64),
        ("sol_balance", (claimer_balance as f64 / LAMPORTS_PER_SOL as f64), f64),
        "cluster" => &cli.cluster,
    );
    Ok(())
}

pub async fn get_claimer_balance(
    rpc_url: String,
    keypair: &Arc<Keypair>,
) -> Result<u64, ClaimMevError> {
    let rpc_client = RpcClient::new(rpc_url);
    let balance = rpc_client.get_balance(&keypair.pubkey()).await?;
    Ok(balance)
}

#[allow(clippy::cognitive_complexity)]
#[allow(clippy::too_many_arguments)]
pub async fn claim_mev_tips(
    merkle_trees: &GeneratedMerkleTreeCollection,
    rpc_url: String,
    rpc_sender_url: String,
    tip_distribution_program_id: Pubkey,
    priority_fee_distribution_program_id: Pubkey,
    tip_router_program_id: Pubkey,
    ncn: Pubkey,
    keypair: &Arc<Keypair>,
    max_loop_duration: Duration,
    micro_lamports: u64,
    min_claim_amount: u64,
    file_path: &PathBuf,
    file_mutex: &Arc<Mutex<()>>,
    operator_address: &String,
    cluster: &str,
    processor: &ClaimProcessor,
) -> Result<(), ClaimMevError> {
    let rpc_client = Arc::new(RpcClient::new_with_timeout_and_commitment(
        rpc_url,
        Duration::from_secs(1800),
        CommitmentConfig::confirmed(),
    ));
    let rpc_sender_client = RpcClient::new(rpc_sender_url);

    let epoch = merkle_trees.epoch;
    let current_epoch = rpc_client.get_epoch_info().await?.epoch;
    if is_epoch_completed(epoch, current_epoch, file_path, file_mutex).await? {
        return Ok(());
    }

    let start = Instant::now();
    while start.elapsed() <= max_loop_duration {
        let epoch_skipped = processor.get_epoch_skipped(epoch);
        let (mut claims_to_process, validators_processed, none_claimants) =
            get_claim_transactions_for_valid_unclaimed(
                &rpc_client,
                merkle_trees,
                tip_distribution_program_id,
                priority_fee_distribution_program_id,
                tip_router_program_id,
                ncn,
                micro_lamports,
                keypair.pubkey(),
                min_claim_amount,
                operator_address,
                cluster,
                &epoch_skipped,
            )
            .await?;

        if validators_processed {
            match get_epoch_percentage(&rpc_client).await {
                Ok(epoch_percentage) => {
                    datapoint_info!(
                        "tip_router_cli.claim_mev_tips-send_summary",
                        ("claim_transactions_left", claims_to_process.len(), i64),
                        ("epoch", epoch, i64),
                        ("operator", operator_address, String),
                        ("epoch_percentage", epoch_percentage, f64),
                        "cluster" => cluster,
                    );
                }
                Err(e) => {
                    warn!("Failed to fetch epoch percentage for claims: {:?}", e);
                }
            }
        }

        if validators_processed && claims_to_process.is_empty() {
            processor.extend_epoch_skipped(epoch, none_claimants);
            add_completed_epoch(epoch, current_epoch, file_path, file_mutex).await?;
            return Ok(());
        }

        claims_to_process.shuffle(&mut thread_rng());

        for transactions in claims_to_process.chunks(2_000) {
            let transactions: Vec<_> = transactions.to_vec();
            // only check balance for the ones we need to currently send since reclaim rent running in parallel
            if let Some((start_balance, desired_balance, sol_to_deposit)) =
                is_sufficient_balance(&keypair.pubkey(), &rpc_client, transactions.len() as u64)
                    .await
            {
                return Err(ClaimMevError::InsufficientBalance {
                    desired_balance,
                    payer: keypair.pubkey(),
                    start_balance,
                    sol_to_deposit,
                });
            }

            let blockhash = rpc_client.get_latest_blockhash().await?;
            if let Err(e) = send_until_blockhash_expires(
                &rpc_client,
                &rpc_sender_client,
                transactions,
                blockhash,
                keypair,
            )
            .await
            {
                info!("send_until_blockhash_expires failed: {:?}", e);
            }
        }
    }

    let epoch_skipped = processor.get_epoch_skipped(epoch);
    let (transactions, validators_processed, none_claimants) =
        get_claim_transactions_for_valid_unclaimed(
            &rpc_client,
            merkle_trees,
            tip_distribution_program_id,
            priority_fee_distribution_program_id,
            tip_router_program_id,
            ncn,
            micro_lamports,
            keypair.pubkey(),
            min_claim_amount,
            operator_address,
            cluster,
            &epoch_skipped,
        )
        .await?;

    if validators_processed && transactions.is_empty() {
        processor.extend_epoch_skipped(epoch, none_claimants);
        add_completed_epoch(epoch, current_epoch, file_path, file_mutex).await?;
        return Ok(());
    }

    // if more transactions left, we'll simulate them all to make sure its not an uncaught error
    let mut is_error = false;
    let mut error_str = String::new();
    for tx in &transactions {
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
            Ok(_) => {}
            Err(e) => {
                error_str = e.to_string();
                is_error = true;

                match e.get_transaction_error() {
                    None => {
                        break;
                    }
                    Some(e) => {
                        warn!("transaction error. tx: {:?} error: {:?}", tx, e);
                        break;
                    }
                }
            }
        }
    }

    if is_error {
        Err(ClaimMevError::UncaughtError { e: error_str })
    } else {
        info!(
            "Not finished claiming for epoch {}, transactions left {}",
            epoch,
            transactions.len()
        );
        Err(ClaimMevError::NotFinished {
            transactions_left: transactions.len(),
        })
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn get_claim_transactions_for_valid_unclaimed(
    rpc_client: &RpcClient,
    merkle_trees: &GeneratedMerkleTreeCollection,
    tip_distribution_program_id: Pubkey,
    priority_fee_distribution_program_id: Pubkey,
    tip_router_program_id: Pubkey,
    ncn: Pubkey,
    micro_lamports: u64,
    payer_pubkey: Pubkey,
    min_claim_amount: u64,
    operator_address: &String,
    cluster: &str,
    epoch_skipped: &HashSet<Pubkey>,
) -> Result<(Vec<Transaction>, bool, HashSet<Pubkey>), ClaimMevError> {
    let epoch = merkle_trees.epoch;
    let tip_router_config_address = Config::find_program_address(&tip_router_program_id, &ncn).0;

    let all_tree_nodes = merkle_trees
        .generated_merkle_trees
        .iter()
        .filter_map(|tree| {
            if tree.merkle_root_upload_authority != tip_router_config_address {
                return None;
            }

            Some(&tree.tree_nodes)
        })
        .flatten()
        .collect_vec();

    let validator_tree_nodes = merkle_trees
        .generated_merkle_trees
        .iter()
        .filter_map(|tree| {
            if tree.merkle_root_upload_authority != tip_router_config_address {
                return None;
            }

            Some(vec![&tree.tree_nodes[1]])
        })
        .flatten()
        .collect_vec();

    let remaining_validator_claims =
        get_unprocessed_claims_for_validators(rpc_client, &validator_tree_nodes).await?;

    let validators_processed = remaining_validator_claims.is_empty();
    let tree_nodes = if validators_processed {
        all_tree_nodes.to_owned()
    } else {
        validator_tree_nodes.to_owned()
    };

    info!(
        "reading {} tip distribution related accounts for epoch {}",
        all_tree_nodes.len(),
        epoch
    );

    let start = Instant::now();

    let tda_pubkeys = merkle_trees
        .generated_merkle_trees
        .iter()
        .map(|tree| tree.distribution_account)
        .collect_vec();

    let tdas: HashMap<Pubkey, Account> = get_batched_accounts(rpc_client, &tda_pubkeys)
        .await?
        .into_iter()
        .filter_map(|(pubkey, a)| Some((pubkey, a?)))
        .collect();

    let claimant_pubkeys: Vec<Pubkey> = tree_nodes
        .iter()
        .map(|tree_node| tree_node.claimant)
        .filter(|pk| !epoch_skipped.contains(pk))
        .collect_vec();

    let fetched_claimants = get_batched_accounts(rpc_client, &claimant_pubkeys).await?;

    let claimants: HashMap<Pubkey, Account> = fetched_claimants
        .iter()
        .filter_map(|(pubkey, a)| Some((*pubkey, a.clone()?)))
        .collect();

    let claim_status_pubkeys = tree_nodes
        .iter()
        .map(|tree_node| tree_node.claim_status_pubkey)
        .collect_vec();

    let claim_statuses: HashMap<Pubkey, Account> =
        get_batched_accounts(rpc_client, &claim_status_pubkeys)
            .await?
            .into_iter()
            .filter_map(|(pubkey, a)| Some((pubkey, a?)))
            .collect();

    let elapsed_us = start.elapsed().as_micros();

    if validators_processed {
        // can be helpful for determining mismatch in state between requested and read
        datapoint_info!(
            "tip_router_cli.get_claim_transactions_account_data",
            ("elapsed_us", elapsed_us, i64),
            ("tdas", tda_pubkeys.len(), i64),
            ("tdas_onchain", tdas.len(), i64),
            ("epoch", epoch, i64),
            ("claimants", claimant_pubkeys.len(), i64),
            ("claim_statuses", claim_status_pubkeys.len(), i64),
            ("claimants_onchain", claimants.len(), i64),
            ("claim_statuses_onchain", claim_statuses.len(), i64),
            ("operator", operator_address, String),
            "cluster" => cluster,
        );
    }

    let transactions = build_mev_claim_transactions(
        tip_distribution_program_id,
        priority_fee_distribution_program_id,
        tip_router_program_id,
        merkle_trees,
        tdas,
        claimants,
        claim_statuses,
        micro_lamports,
        payer_pubkey,
        ncn,
        min_claim_amount,
        cluster,
    );

    let none_claimants: HashSet<Pubkey> = if transactions.is_empty() {
        // Collect claimants not found on-chain this iteration. We do NOT write to
        // skipped_claimants here — the caller promotes these only when 0 claims
        // remain, ensuring a single RPC drop doesn't permanently blacklist a valid
        // account.
        fetched_claimants
            .iter()
            .filter_map(|(pk, acct)| acct.is_none().then_some(*pk))
            .collect()
    } else {
        HashSet::new()
    };

    Ok((transactions, validators_processed, none_claimants))
}

pub async fn get_unprocessed_claims_for_validators(
    rpc_client: &RpcClient,
    tree_nodes: &[&TreeNode],
) -> Result<Vec<Account>, ClaimMevError> {
    let claim_status_pubkeys = tree_nodes
        .iter()
        .map(|tree_node| tree_node.claim_status_pubkey)
        .collect_vec();

    let claim_statuses: HashMap<Pubkey, Account> =
        get_batched_accounts(rpc_client, &claim_status_pubkeys)
            .await?
            .into_iter()
            .filter_map(|(pubkey, a)| Some((pubkey, a?)))
            .collect();

    let deserialized_claim_statuses = claim_statuses.values().map(|a| {
        (
            ClaimStatus::deserialize(&a.data).expect("claim status account should deserialize"),
            a,
        )
    });

    let unprocessed_claim_statuses = deserialized_claim_statuses
        .filter(|(c, _)| !c.is_claimed)
        .map(|(_, a)| a.clone())
        .collect();

    Ok(unprocessed_claim_statuses)
}

/// Returns a list of claim transactions for valid, unclaimed MEV tips
/// A valid, unclaimed transaction consists of the following:
/// - there must be lamports to claim for the tip distribution account.
/// - there must be a merkle root.
/// - the claimant (typically a stake account) must exist.
/// - the claimant (typically a stake account) must have a non-zero amount of tips to claim
/// - the claimant must have enough lamports post-claim to be rent-exempt.
///   - note: there aren't any rent exempt accounts on solana mainnet anymore.
/// - it must not have already been claimed.
#[allow(clippy::too_many_arguments)]
fn build_mev_claim_transactions(
    tip_distribution_program_id: Pubkey,
    priority_fee_distribution_program_id: Pubkey,
    tip_router_program_id: Pubkey,
    merkle_trees: &GeneratedMerkleTreeCollection,
    tdas: HashMap<Pubkey, Account>,
    claimants: HashMap<Pubkey, Account>,
    claim_statuses: HashMap<Pubkey, Account>,
    micro_lamports: u64,
    payer_pubkey: Pubkey,
    ncn_address: Pubkey,
    min_claim_amount: u64,
    cluster: &str,
) -> Vec<Transaction> {
    let epoch = merkle_trees.epoch;
    let tip_router_config_address =
        Config::find_program_address(&tip_router_program_id, &ncn_address).0;
    let tip_router_account_payer =
        AccountPayer::find_program_address(&tip_router_program_id, &ncn_address).0;

    let tip_distribution_config =
        Pubkey::find_program_address(&[CONFIG_SEED], &tip_distribution_program_id).0;

    let priority_fee_distribution_config =
        Pubkey::find_program_address(&[CONFIG_SEED], &priority_fee_distribution_program_id).0;

    let mut under_min_amount_claimants = 0;

    let mut instructions = Vec::with_capacity(claimants.len());
    for tree in &merkle_trees.generated_merkle_trees {
        if tree.max_total_claim == 0 {
            continue;
        }

        // if unwrap panics, there's a bug in the merkle tree code because the merkle tree code relies on the state
        // of the chain to claim.
        let distribution_account = tdas
            .get(&tree.distribution_account)
            .expect("merkle tree distribution account should exist in fetched account set");
        if tree.distribution_program.eq(&tip_distribution_program_id) {
            let tda = TipDistributionAccount::deserialize(distribution_account.data.as_slice());
            match tda {
                Ok(tda) => {
                    // can continue here, as there might be tip distribution accounts this account doesn't upload for
                    if tda.merkle_root.is_none()
                        || tda.merkle_root_upload_authority != tip_router_config_address
                    {
                        continue;
                    }
                }
                Err(_) => continue,
            }
        } else if tree
            .distribution_program
            .eq(&priority_fee_distribution_program_id)
        {
            let pfda =
                PriorityFeeDistributionAccount::deserialize(distribution_account.data.as_slice());
            match pfda {
                Ok(pfda) => {
                    // can continue here, as there might be tip distribution accounts this account doesn't upload for
                    if pfda.merkle_root.is_none()
                        || pfda.merkle_root_upload_authority != tip_router_config_address
                    {
                        continue;
                    }
                }
                Err(_) => continue,
            }
        } else {
            panic!("Unknown distribution program for tree");
        }

        for node in &tree.tree_nodes {
            // doesn't make sense to claim for claimants that don't exist anymore
            // can't claim for something already claimed
            // don't need to claim for claimants that get 0 MEV
            // skip claims below min_claim_amount threshold
            if !claimants.contains_key(&node.claimant)
                || claim_statuses.contains_key(&node.claim_status_pubkey)
                || node.amount == 0
                || node.amount < min_claim_amount
            {
                if node.amount == 0 {
                    under_min_amount_claimants += 1;
                }
                continue;
            }

            let mut claim_with_payer_builder = ClaimWithPayerBuilder::new();
            claim_with_payer_builder
                .config(tip_router_config_address)
                .account_payer(tip_router_account_payer)
                .ncn(ncn_address)
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

            if tree.distribution_program.eq(&tip_distribution_program_id) {
                claim_with_payer_builder.tip_distribution_config(tip_distribution_config);
            } else if tree
                .distribution_program
                .eq(&priority_fee_distribution_program_id)
            {
                claim_with_payer_builder.tip_distribution_config(priority_fee_distribution_config);
            } else {
                panic!("Unknown distribution program for tree");
            }
            let claim_with_payer_ix = claim_with_payer_builder.instruction();

            instructions.push(claim_with_payer_ix);
        }
    }

    // TODO (LB): see if we can do >1 claim here
    let transactions: Vec<Transaction> = instructions
        .into_iter()
        .map(|claim_ix| {
            let instructions = priority_fees::configure_instruction(
                claim_ix,
                micro_lamports,
                Some(100_000), // helps get txs into block easier since default is 400k CUs
            );
            Transaction::new_with_payer(&instructions, Some(&payer_pubkey))
        })
        .collect();

    info!("Under min amount claimants: {under_min_amount_claimants}");

    datapoint_info!(
        "tip_router_cli.build_mev_claim_transactions",
        ("distribution_accounts", tdas.len(), i64),
        ("claim_statuses", claim_statuses.len(), i64),
        ("claim_transactions", transactions.len(), i64),
        ("epoch", epoch, i64),
        "cluster" => cluster,
    );

    transactions
}

/// heuristic to make sure we have enough funds to cover the rent costs if epoch has many validators
/// If insufficient funds, returns start balance, desired balance, and amount of sol to deposit
async fn is_sufficient_balance(
    payer: &Pubkey,
    rpc_client: &RpcClient,
    instruction_count: u64,
) -> Option<(u64, u64, u64)> {
    let start_balance = rpc_client
        .get_balance(payer)
        .await
        .expect("Failed to get starting balance");
    // most amounts are for 0 lamports. had 1736 non-zero claims out of 164742
    let min_rent_per_claim = rpc_client
        .get_minimum_balance_for_rent_exemption(CLAIM_STATUS_SIZE)
        .await
        .expect("Failed to calculate min rent");
    let desired_balance = instruction_count
        .checked_mul(
            min_rent_per_claim
                .checked_add(DEFAULT_TARGET_LAMPORTS_PER_SIGNATURE)
                .expect("rent plus signature target should fit in u64"),
        )
        .expect("desired claim balance should fit in u64");
    if start_balance < desired_balance {
        let sol_to_deposit = desired_balance
            .checked_sub(start_balance)
            .expect("desired balance should exceed start balance in insufficient branch")
            .checked_add(LAMPORTS_PER_SOL)
            .expect("buffered deposit should fit in u64")
            .checked_sub(1)
            .expect("buffered deposit should be at least one lamport")
            .checked_div(LAMPORTS_PER_SOL)
            .expect("deposit rounding division should succeed"); // rounds up to nearest sol
        Some((start_balance, desired_balance, sol_to_deposit))
    } else {
        None
    }
}

/// Helper function to check if an epoch is in the completed_claim_epochs.txt file
pub async fn is_epoch_completed(
    epoch: u64,
    current_epoch: u64,
    file_path: &PathBuf,
    file_mutex: &Arc<Mutex<()>>,
) -> Result<bool, ClaimMevError> {
    // If we're still on the current epoch, it can't be completed
    let current_claim_epoch =
        current_epoch
            .checked_sub(1)
            .ok_or(ClaimMevError::CompletedEpochsError(
                "Epoch underflow".to_string(),
            ))?;

    if current_claim_epoch == epoch {
        info!("Do not skip the current claim epoch ( {} )", epoch);
        return Ok(false);
    }

    // Acquire the mutex lock before file operations
    let _lock = file_mutex.lock().await;

    // If file doesn't exist, no epochs are completed
    if !file_path.exists() {
        info!("No completed epochs file found - creating empty");
        drop(_lock);
        add_completed_epoch(0, current_epoch, file_path, file_mutex).await?;

        return Ok(false);
    }

    // Open and read file
    let file = File::open(file_path).await.map_err(|e| {
        ClaimMevError::CompletedEpochsError(format!("Failed to open completed epochs file: {}", e))
    })?;

    let mut reader = BufReader::new(file);
    let mut line = String::new();

    // Read lines asynchronously
    while reader.read_line(&mut line).await.map_err(|e| {
        ClaimMevError::CompletedEpochsError(format!("Failed to read line from epochs file: {}", e))
    })? > 0
    {
        // Try to parse the line as a u64 and compare with our epoch
        if let Ok(completed_epoch) = line.trim().parse::<u64>() {
            if completed_epoch == epoch {
                info!("Skipping epoch {} ( already completed )", epoch);
                return Ok(true);
            }
        }

        // Clear the line for the next iteration
        line.clear();
    }

    info!("Epoch {} not found in completed epochs file", epoch);
    Ok(false)
}

/// Helper function to add an epoch to the completed_claim_epochs.txt file
pub async fn add_completed_epoch(
    epoch: u64,
    current_epoch: u64,
    file_path: &PathBuf,
    file_mutex: &Arc<Mutex<()>>,
) -> Result<(), ClaimMevError> {
    // If we're still on the current epoch, it can't be completed
    let current_claim_epoch =
        current_epoch
            .checked_sub(1)
            .ok_or(ClaimMevError::CompletedEpochsError(
                "Epoch underflow".to_string(),
            ))?;

    if current_claim_epoch == epoch {
        info!("Do not write file for current epoch ( {} )", epoch);
        return Ok(());
    }

    // Acquire the mutex lock before file operations
    let _lock = file_mutex.lock().await;

    // Create or open file in append mode
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)
        .await
        .map_err(|e| {
            ClaimMevError::CompletedEpochsError(format!(
                "Failed to open epochs file for writing: {}",
                e
            ))
        })?;

    // Write epoch followed by newline
    file.write_all(format!("{}\n", epoch).as_bytes())
        .await
        .map_err(|e| {
            ClaimMevError::CompletedEpochsError(format!("Failed to write epoch to file: {}", e))
        })?;

    info!(
        "Epoch {} added to completed epochs file ( {} )",
        epoch,
        file_path.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use jito_tip_distribution_sdk::{MerkleRoot, TipDistributionAccount};
    use jito_tip_router_core::config::Config;
    use meta_merkle_tree::generated_merkle_tree::{
        GeneratedMerkleTree, GeneratedMerkleTreeCollection, TreeNode,
    };
    use solana_sdk::hash::Hash;

    use super::*;

    /// Serialize a TipDistributionAccount with its 8-byte discriminator prefix,
    /// padded to the expected on-chain account size.
    fn serialize_tda(tda: &TipDistributionAccount) -> Vec<u8> {
        let mut data = TipDistributionAccount::DISCRIMINATOR.to_vec();
        data.extend_from_slice(&borsh::to_vec(tda).unwrap());
        data.resize(jito_tip_distribution_sdk::TIP_DISTRIBUTION_SIZE, 0);
        data
    }

    /// Build a minimal test fixture for `build_mev_claim_transactions`.
    ///
    /// Returns (merkle_trees, tdas, claimants, claim_statuses, tip_distribution_program_id,
    ///          priority_fee_distribution_program_id, tip_router_program_id, ncn, payer)
    fn setup_test_fixture(
        tree_nodes: Vec<TreeNode>,
        max_total_claim: u64,
    ) -> (
        GeneratedMerkleTreeCollection,
        HashMap<Pubkey, Account>,
        HashMap<Pubkey, Account>,
        HashMap<Pubkey, Account>,
        Pubkey,
        Pubkey,
        Pubkey,
        Pubkey,
        Pubkey,
    ) {
        let tip_distribution_program_id = jito_tip_distribution_sdk::id();
        let priority_fee_distribution_program_id = jito_priority_fee_distribution_sdk::id();
        let tip_router_program_id = Pubkey::new_unique();
        let ncn = Pubkey::new_unique();
        let payer = Pubkey::new_unique();
        let distribution_account_pubkey = Pubkey::new_unique();

        let tip_router_config_address =
            Config::find_program_address(&tip_router_program_id, &ncn).0;

        let tda = TipDistributionAccount {
            validator_vote_account: Pubkey::new_unique(),
            merkle_root_upload_authority: tip_router_config_address,
            merkle_root: Some(MerkleRoot {
                root: [1u8; 32],
                max_total_claim,
                max_num_nodes: tree_nodes.len() as u64,
                total_funds_claimed: 0,
                num_nodes_claimed: 0,
            }),
            epoch_created_at: 100,
            validator_commission_bps: 0,
            expires_at: 200,
            bump: 0,
        };

        let tda_data = serialize_tda(&tda);
        let tda_account = Account {
            lamports: 1_000_000,
            data: tda_data,
            owner: tip_distribution_program_id,
            executable: false,
            rent_epoch: 0,
        };

        let mut tdas = HashMap::new();
        tdas.insert(distribution_account_pubkey, tda_account);

        let mut claimants = HashMap::new();
        for node in &tree_nodes {
            claimants.insert(
                node.claimant,
                Account {
                    lamports: 1_000_000,
                    data: vec![],
                    owner: Pubkey::new_unique(),
                    executable: false,
                    rent_epoch: 0,
                },
            );
        }

        let tree = GeneratedMerkleTree {
            distribution_program: tip_distribution_program_id,
            distribution_account: distribution_account_pubkey,
            merkle_root_upload_authority: tip_router_config_address,
            merkle_root: Hash::new_unique(),
            tree_nodes,
            max_total_claim,
            max_num_nodes: 0,
        };

        let merkle_trees = GeneratedMerkleTreeCollection {
            generated_merkle_trees: vec![tree],
            bank_hash: "test".to_string(),
            epoch: 100,
            slot: 1000,
        };

        let claim_statuses = HashMap::new();

        (
            merkle_trees,
            tdas,
            claimants,
            claim_statuses,
            tip_distribution_program_id,
            priority_fee_distribution_program_id,
            tip_router_program_id,
            ncn,
            payer,
        )
    }

    fn make_tree_node(amount: u64) -> TreeNode {
        TreeNode {
            claimant: Pubkey::new_unique(),
            claim_status_pubkey: Pubkey::new_unique(),
            claim_status_bump: 0,
            staker_pubkey: Pubkey::new_unique(),
            withdrawer_pubkey: Pubkey::new_unique(),
            amount,
            proof: Some(vec![]),
        }
    }

    #[test]
    fn test_min_claim_amount_filters_small_claims() {
        let nodes = vec![
            make_tree_node(10_000), // above threshold
            make_tree_node(3_000),  // below threshold
            make_tree_node(5_000),  // at threshold
            make_tree_node(1_000),  // below threshold
        ];
        let total = nodes.iter().map(|n| n.amount).sum();

        let (
            merkle_trees,
            tdas,
            claimants,
            claim_statuses,
            tip_dist_id,
            pf_dist_id,
            router_id,
            ncn,
            payer,
        ) = setup_test_fixture(nodes, total);

        let txs = build_mev_claim_transactions(
            tip_dist_id,
            pf_dist_id,
            router_id,
            &merkle_trees,
            tdas,
            claimants,
            claim_statuses,
            0,
            payer,
            ncn,
            5_000, // min_claim_amount
            "test",
        );

        // Only 10_000 and 5_000 should pass (3_000 and 1_000 are below threshold)
        assert_eq!(txs.len(), 2);
    }

    #[test]
    fn test_min_claim_amount_zero_allows_all() {
        let nodes = vec![
            make_tree_node(100),
            make_tree_node(1),
            make_tree_node(50_000),
        ];
        let total = nodes.iter().map(|n| n.amount).sum();

        let (
            merkle_trees,
            tdas,
            claimants,
            claim_statuses,
            tip_dist_id,
            pf_dist_id,
            router_id,
            ncn,
            payer,
        ) = setup_test_fixture(nodes, total);

        let txs = build_mev_claim_transactions(
            tip_dist_id,
            pf_dist_id,
            router_id,
            &merkle_trees,
            tdas,
            claimants,
            claim_statuses,
            0,
            payer,
            ncn,
            0, // no minimum
            "test",
        );

        assert_eq!(txs.len(), 3);
    }

    #[test]
    fn test_zero_amount_claims_are_skipped() {
        let nodes = vec![
            make_tree_node(10_000),
            make_tree_node(0), // zero amount
        ];
        let total = 10_000;

        let (
            merkle_trees,
            tdas,
            claimants,
            claim_statuses,
            tip_dist_id,
            pf_dist_id,
            router_id,
            ncn,
            payer,
        ) = setup_test_fixture(nodes, total);

        let txs = build_mev_claim_transactions(
            tip_dist_id,
            pf_dist_id,
            router_id,
            &merkle_trees,
            tdas,
            claimants,
            claim_statuses,
            0,
            payer,
            ncn,
            0,
            "test",
        );

        assert_eq!(txs.len(), 1);
    }

    #[test]
    fn test_already_claimed_are_skipped() {
        let nodes = vec![make_tree_node(10_000), make_tree_node(20_000)];
        let total = nodes.iter().map(|n| n.amount).sum();

        let (
            merkle_trees,
            tdas,
            claimants,
            mut claim_statuses,
            tip_dist_id,
            pf_dist_id,
            router_id,
            ncn,
            payer,
        ) = setup_test_fixture(nodes, total);

        // Mark the first node as already claimed
        let first_claim_status =
            merkle_trees.generated_merkle_trees[0].tree_nodes[0].claim_status_pubkey;
        claim_statuses.insert(
            first_claim_status,
            Account {
                lamports: 1_000_000,
                data: vec![],
                owner: Pubkey::new_unique(),
                executable: false,
                rent_epoch: 0,
            },
        );

        let txs = build_mev_claim_transactions(
            tip_dist_id,
            pf_dist_id,
            router_id,
            &merkle_trees,
            tdas,
            claimants,
            claim_statuses,
            0,
            payer,
            ncn,
            0,
            "test",
        );

        assert_eq!(txs.len(), 1);
    }

    #[test]
    fn test_missing_claimant_is_skipped() {
        let nodes = vec![make_tree_node(10_000), make_tree_node(20_000)];
        let total = nodes.iter().map(|n| n.amount).sum();

        let (
            merkle_trees,
            tdas,
            mut claimants,
            claim_statuses,
            tip_dist_id,
            pf_dist_id,
            router_id,
            ncn,
            payer,
        ) = setup_test_fixture(nodes, total);

        // Remove the first claimant
        let first_claimant = merkle_trees.generated_merkle_trees[0].tree_nodes[0].claimant;
        claimants.remove(&first_claimant);

        let txs = build_mev_claim_transactions(
            tip_dist_id,
            pf_dist_id,
            router_id,
            &merkle_trees,
            tdas,
            claimants,
            claim_statuses,
            0,
            payer,
            ncn,
            0,
            "test",
        );

        assert_eq!(txs.len(), 1);
    }

    #[test]
    fn test_high_min_claim_amount_filters_everything() {
        let nodes = vec![
            make_tree_node(1_000),
            make_tree_node(2_000),
            make_tree_node(3_000),
        ];
        let total = nodes.iter().map(|n| n.amount).sum();

        let (
            merkle_trees,
            tdas,
            claimants,
            claim_statuses,
            tip_dist_id,
            pf_dist_id,
            router_id,
            ncn,
            payer,
        ) = setup_test_fixture(nodes, total);

        let txs = build_mev_claim_transactions(
            tip_dist_id,
            pf_dist_id,
            router_id,
            &merkle_trees,
            tdas,
            claimants,
            claim_statuses,
            0,
            payer,
            ncn,
            100_000, // higher than all claims
            "test",
        );

        assert_eq!(txs.len(), 0);
    }
}
