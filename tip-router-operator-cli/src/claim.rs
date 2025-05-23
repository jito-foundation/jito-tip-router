use anchor_lang::AccountDeserialize;
use itertools::Itertools;
use jito_tip_distribution_sdk::{
    derive_claim_status_account_address, jito_tip_distribution::accounts::ClaimStatus,
    TipDistributionAccount, CLAIM_STATUS_SIZE, CONFIG_SEED,
};
use jito_tip_router_client::instructions::ClaimWithPayerBuilder;
use jito_tip_router_core::{account_payer::AccountPayer, config::Config};
use log::{info, warn};
use meta_merkle_tree::generated_merkle_tree::GeneratedMerkleTreeCollection;
use rand::{prelude::SliceRandom, thread_rng};
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_config::RpcSimulateTransactionConfig};
use solana_metrics::{datapoint_error, datapoint_info};
use solana_sdk::{
    account::Account,
    commitment_config::CommitmentConfig,
    compute_budget::ComputeBudgetInstruction,
    fee_calculator::DEFAULT_TARGET_LAMPORTS_PER_SIGNATURE,
    native_token::{lamports_to_sol, LAMPORTS_PER_SOL},
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
    signer::Signer,
    system_program,
    transaction::Transaction,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use thiserror::Error;
use tokio::fs::File;
use tokio::fs::OpenOptions;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use tokio::sync::Mutex;

use crate::{
    merkle_tree_collection_file_name, priority_fees,
    rpc_utils::{get_batched_accounts, send_until_blockhash_expires},
    Cli,
};

#[derive(Error, Debug)]
pub enum ClaimMevError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    JsonError(#[from] serde_json::Error),

    #[error(transparent)]
    AnchorError(anchor_lang::error::Error),

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

    let all_claim_transactions = get_claim_transactions_for_valid_unclaimed(
        &rpc_client,
        &merkle_trees,
        tip_distribution_program_id,
        tip_router_program_id,
        ncn,
        0,
        Pubkey::new_unique(),
        &cli.operator_address,
        &cli.cluster,
    )
    .await?;

    datapoint_info!(
        "tip_router_cli.claim_mev_tips-send_summary",
        ("claim_transactions_left", all_claim_transactions.len(), i64),
        ("epoch", epoch, i64),
        "cluster" => &cli.cluster,
    );

    if all_claim_transactions.is_empty() {
        add_completed_epoch(epoch, current_epoch, file_path, file_mutex).await?;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn claim_mev_tips_with_emit(
    cli: &Cli,
    epoch: u64,
    tip_distribution_program_id: Pubkey,
    tip_router_program_id: Pubkey,
    ncn: Pubkey,
    max_loop_duration: Duration,
    file_path: &PathBuf,
    file_mutex: &Arc<Mutex<()>>,
) -> Result<(), anyhow::Error> {
    let keypair = read_keypair_file(cli.keypair_path.clone())
        .map_err(|e| anyhow::anyhow!("Failed to read keypair file: {:?}", e))?;
    let keypair = Arc::new(keypair);
    let meta_merkle_tree_dir = cli.get_save_path().clone();
    let rpc_url = cli.rpc_url.clone();
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
                &tip_distribution_program_id,
                &node.claimant,
                &tree.tip_distribution_account,
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
        tip_router_program_id,
        ncn,
        &keypair,
        max_loop_duration,
        cli.micro_lamports,
        file_path,
        file_mutex,
        &cli.operator_address,
        &cli.cluster,
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

    let claimer_balance = get_claimer_balance(rpc_url, &keypair).await?;
    datapoint_info!(
        "claimer_info",
        ("claimer", keypair.pubkey().to_string(), String),
        ("epoch", epoch, i64),
        ("lamport_balance", claimer_balance, i64),
        ("sol_balance", lamports_to_sol(claimer_balance), f64),
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

#[allow(clippy::too_many_arguments)]
pub async fn claim_mev_tips(
    merkle_trees: &GeneratedMerkleTreeCollection,
    rpc_url: String,
    rpc_sender_url: String,
    tip_distribution_program_id: Pubkey,
    tip_router_program_id: Pubkey,
    ncn: Pubkey,
    keypair: &Arc<Keypair>,
    max_loop_duration: Duration,
    micro_lamports: u64,
    file_path: &PathBuf,
    file_mutex: &Arc<Mutex<()>>,
    operator_address: &String,
    cluster: &str,
) -> Result<(), ClaimMevError> {
    let rpc_client = RpcClient::new_with_timeout_and_commitment(
        rpc_url,
        Duration::from_secs(1800),
        CommitmentConfig::confirmed(),
    );
    let rpc_sender_client = RpcClient::new(rpc_sender_url);

    let epoch = merkle_trees.epoch;
    let current_epoch = rpc_client.get_epoch_info().await?.epoch;
    if is_epoch_completed(epoch, current_epoch, file_path, file_mutex).await? {
        return Ok(());
    }

    let start = Instant::now();
    while start.elapsed() <= max_loop_duration {
        let mut all_claim_transactions = get_claim_transactions_for_valid_unclaimed(
            &rpc_client,
            merkle_trees,
            tip_distribution_program_id,
            tip_router_program_id,
            ncn,
            micro_lamports,
            keypair.pubkey(),
            operator_address,
            cluster,
        )
        .await?;

        datapoint_info!(
            "tip_router_cli.claim_mev_tips-send_summary",
            ("claim_transactions_left", all_claim_transactions.len(), i64),
            ("epoch", epoch, i64),
            ("operator", operator_address, String),
            "cluster" => cluster,
        );

        if all_claim_transactions.is_empty() {
            add_completed_epoch(epoch, current_epoch, file_path, file_mutex).await?;
            return Ok(());
        }

        all_claim_transactions.shuffle(&mut thread_rng());

        for transactions in all_claim_transactions.chunks(2_000) {
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

    let transactions = get_claim_transactions_for_valid_unclaimed(
        &rpc_client,
        merkle_trees,
        tip_distribution_program_id,
        tip_router_program_id,
        ncn,
        micro_lamports,
        keypair.pubkey(),
        operator_address,
        cluster,
    )
    .await?;
    if transactions.is_empty() {
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
    tip_router_program_id: Pubkey,
    ncn: Pubkey,
    micro_lamports: u64,
    payer_pubkey: Pubkey,
    operator_address: &String,
    cluster: &str,
) -> Result<Vec<Transaction>, ClaimMevError> {
    let epoch = merkle_trees.epoch;
    let tip_router_config_address = Config::find_program_address(&tip_router_program_id, &ncn).0;

    let tree_nodes = merkle_trees
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

    info!(
        "reading {} tip distribution related accounts for epoch {}",
        tree_nodes.len(),
        epoch
    );

    let start = Instant::now();

    let tda_pubkeys = merkle_trees
        .generated_merkle_trees
        .iter()
        .map(|tree| tree.tip_distribution_account)
        .collect_vec();

    let tdas: HashMap<Pubkey, Account> = get_batched_accounts(rpc_client, &tda_pubkeys)
        .await?
        .into_iter()
        .filter_map(|(pubkey, a)| Some((pubkey, a?)))
        .collect();

    let claimant_pubkeys = tree_nodes
        .iter()
        .map(|tree_node| tree_node.claimant)
        .collect_vec();
    let claimants: HashMap<Pubkey, Account> = get_batched_accounts(rpc_client, &claimant_pubkeys)
        .await?
        .into_iter()
        .filter_map(|(pubkey, a)| Some((pubkey, a?)))
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

    // can be helpful for determining mismatch in state between requested and read
    datapoint_info!(
        "tip_router_cli.get_claim_transactions_account_data",
        ("elapsed_us", elapsed_us, i64),
        ("tdas", tda_pubkeys.len(), i64),
        ("tdas_onchain", tdas.len(), i64),
        ("claimants", claimant_pubkeys.len(), i64),
        ("claimants_onchain", claimants.len(), i64),
        ("claim_statuses", claim_status_pubkeys.len(), i64),
        ("claim_statuses_onchain", claim_statuses.len(), i64),
        ("epoch", epoch, i64),
        ("operator", operator_address, String),
        "cluster" => cluster,
    );

    let transactions = build_mev_claim_transactions(
        tip_distribution_program_id,
        tip_router_program_id,
        merkle_trees,
        tdas,
        claimants,
        claim_statuses,
        micro_lamports,
        payer_pubkey,
        ncn,
        cluster,
    );

    Ok(transactions)
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
    tip_router_program_id: Pubkey,
    merkle_trees: &GeneratedMerkleTreeCollection,
    tdas: HashMap<Pubkey, Account>,
    claimants: HashMap<Pubkey, Account>,
    claim_status: HashMap<Pubkey, Account>,
    micro_lamports: u64,
    payer_pubkey: Pubkey,
    ncn_address: Pubkey,
    cluster: &str,
) -> Vec<Transaction> {
    let epoch = merkle_trees.epoch;
    let tip_router_config_address =
        Config::find_program_address(&tip_router_program_id, &ncn_address).0;
    let tip_router_account_payer =
        AccountPayer::find_program_address(&tip_router_program_id, &ncn_address).0;

    let tip_distribution_accounts: HashMap<Pubkey, TipDistributionAccount> = tdas
        .iter()
        .filter_map(|(pubkey, account)| {
            Some((
                *pubkey,
                TipDistributionAccount::try_deserialize(&mut account.data.as_slice()).ok()?,
            ))
        })
        .collect();

    let claim_statuses: HashMap<Pubkey, ClaimStatus> = claim_status
        .iter()
        .filter_map(|(pubkey, account)| {
            Some((
                *pubkey,
                ClaimStatus::try_deserialize(&mut account.data.as_slice()).ok()?,
            ))
        })
        .collect();

    let tip_distribution_config =
        Pubkey::find_program_address(&[CONFIG_SEED], &tip_distribution_program_id).0;

    let mut zero_amount_claimants = 0;

    let mut instructions = Vec::with_capacity(claimants.len());
    for tree in &merkle_trees.generated_merkle_trees {
        if tree.max_total_claim == 0 {
            continue;
        }

        // if unwrap panics, there's a bug in the merkle tree code because the merkle tree code relies on the state
        // of the chain to claim.
        let tip_distribution_account = tip_distribution_accounts
            .get(&tree.tip_distribution_account)
            .unwrap();

        // can continue here, as there might be tip distribution accounts this account doesn't upload for
        if tip_distribution_account.merkle_root.is_none()
            || tip_distribution_account.merkle_root_upload_authority != tip_router_config_address
        {
            continue;
        }

        for node in &tree.tree_nodes {
            // doesn't make sense to claim for claimants that don't exist anymore
            // can't claim for something already claimed
            // don't need to claim for claimants that get 0 MEV
            if !claimants.contains_key(&node.claimant)
                || claim_statuses.contains_key(&node.claim_status_pubkey)
                || node.amount == 0
            {
                if node.amount == 0 {
                    zero_amount_claimants += 1;
                }
                continue;
            }

            let claim_with_payer_ix = ClaimWithPayerBuilder::new()
                .account_payer(tip_router_account_payer)
                .ncn(ncn_address)
                .config(tip_router_config_address)
                .tip_distribution_program(tip_distribution_program_id)
                .tip_distribution_config(tip_distribution_config)
                .tip_distribution_account(tree.tip_distribution_account)
                .claim_status(node.claim_status_pubkey)
                .claimant(node.claimant)
                .system_program(system_program::id())
                .proof(node.proof.clone().unwrap())
                .amount(node.amount)
                .bump(node.claim_status_bump)
                .instruction();

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

    info!("zero amount claimants: {}", zero_amount_claimants);
    datapoint_info!(
        "tip_router_cli.build_mev_claim_transactions",
        (
            "tip_distribution_accounts",
            tip_distribution_accounts.len(),
            i64
        ),
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
                .unwrap(),
        )
        .unwrap();
    if start_balance < desired_balance {
        let sol_to_deposit = desired_balance
            .checked_sub(start_balance)
            .unwrap()
            .checked_add(LAMPORTS_PER_SOL)
            .unwrap()
            .checked_sub(1)
            .unwrap()
            .checked_div(LAMPORTS_PER_SOL)
            .unwrap(); // rounds up to nearest sol
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
