pub mod cast_vote;
pub mod claim_mev_workflow;
pub mod merkle_root_generator_workflow;
pub mod merkle_root_upload_workflow;
pub mod reclaim_rent_workflow;
pub mod snapshot;
pub mod stake_meta_generator_workflow;
pub use crate::cli::{Cli, Commands};
pub mod cli;
pub use crate::process_epoch::process_epoch;
pub mod process_epoch;

use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::BufReader,
    path::PathBuf,
    result::Result,
    sync::Arc,
    time::{Duration, Instant},
};

use anchor_lang::{prelude::*, Id};
use ellipsis_client::EllipsisClient;
use jito_tip_distribution::{
    program::JitoTipDistribution,
    state::{ClaimStatus, TipDistributionAccount},
};
use jito_tip_payment::{
    Config, CONFIG_ACCOUNT_SEED, TIP_ACCOUNT_SEED_0, TIP_ACCOUNT_SEED_1, TIP_ACCOUNT_SEED_2,
    TIP_ACCOUNT_SEED_3, TIP_ACCOUNT_SEED_4, TIP_ACCOUNT_SEED_5, TIP_ACCOUNT_SEED_6,
    TIP_ACCOUNT_SEED_7,
};
use log::{error, *};
use meta_merkle_tree::generated_merkle_tree::TreeNode;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_client::{RpcClient as SyncRpcClient, SerializableTransaction},
};
use solana_merkle_tree::MerkleTree;
use solana_metrics::{datapoint_error, datapoint_warn};
use solana_program::{
    instruction::InstructionError,
    rent::{ACCOUNT_STORAGE_OVERHEAD, DEFAULT_EXEMPTION_THRESHOLD, DEFAULT_LAMPORTS_PER_BYTE_YEAR},
};
use solana_rpc_client_api::{
    client_error::{Error, ErrorKind},
    config::RpcSendTransactionConfig,
    request::{RpcError, RpcResponseErrorData, MAX_MULTIPLE_ACCOUNTS},
    response::RpcSimulateTransactionResult,
};
use solana_runtime::bank::Bank;
use solana_sdk::{
    account::{Account, AccountSharedData, ReadableAccount},
    clock::Slot,
    commitment_config::{CommitmentConfig, CommitmentLevel},
    hash::{Hash, Hasher},
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    stake_history::Epoch,
    transaction::{
        Transaction,
        TransactionError::{self},
    },
};
use solana_transaction_status::TransactionStatus;
use tokio::{sync::Semaphore, time::sleep};

use crate::stake_meta_generator_workflow::StakeMetaGeneratorError::CheckedMathError;

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct GeneratedMerkleTreeCollection {
    pub generated_merkle_trees: Vec<GeneratedMerkleTree>,
    pub bank_hash: String,
    pub epoch: Epoch,
    pub slot: Slot,
}

#[derive(Clone, Eq, Debug, Hash, PartialEq, Deserialize, Serialize)]
pub struct GeneratedMerkleTree {
    #[serde(with = "pubkey_string_conversion")]
    pub tip_distribution_account: Pubkey,
    #[serde(with = "pubkey_string_conversion")]
    pub merkle_root_upload_authority: Pubkey,
    pub merkle_root: Hash,
    pub tree_nodes: Vec<TreeNode>,
    pub max_total_claim: u64,
    pub max_num_nodes: u64,
}

pub struct TipPaymentPubkeys {
    config_pda: Pubkey,
    tip_pdas: Vec<Pubkey>,
}

#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct TipAccountConfig {
    pub authority: Pubkey,
    pub protocol_fee_bps: u16,
    pub bump: u8,
}

fn emit_inconsistent_tree_node_amount_dp(
    tree_nodes: &[TreeNode],
    tip_distribution_account: &Pubkey,
    rpc_client: &SyncRpcClient,
) {
    let actual_claims: u64 = tree_nodes.iter().map(|t| t.amount).sum();
    let tda = rpc_client.get_account(tip_distribution_account).unwrap();
    let min_rent = rpc_client
        .get_minimum_balance_for_rent_exemption(tda.data.len())
        .unwrap();

    let expected_claims = tda.lamports.checked_sub(min_rent).unwrap();
    if actual_claims == expected_claims {
        return;
    }

    if actual_claims > expected_claims {
        datapoint_error!(
            "tip-distributor",
            ("actual_claims_exceeded", format!(
                "tip_distribution_account={tip_distribution_account},actual_claims={actual_claims}, expected_claims={expected_claims}"
            ), String)
        );
    } else {
        datapoint_warn!("tip-distributor", (
            "actual_claims_below",
            format!(
                "tip_distribution_account={tip_distribution_account},actual_claims={actual_claims}, expected_claims={expected_claims}"
            ),
            String
        ));
    }
}

pub fn get_proof(merkle_tree: &MerkleTree, i: usize) -> Vec<[u8; 32]> {
    let mut proof = Vec::new();
    let path = merkle_tree.find_path(i).expect("path to index");
    for branch in path.get_proof_entries() {
        if let Some(hash) = branch.get_left_sibling() {
            proof.push(hash.to_bytes());
        } else if let Some(hash) = branch.get_right_sibling() {
            proof.push(hash.to_bytes());
        } else {
            panic!("expected some hash at each level of the tree");
        }
    }
    proof
}

fn derive_tip_payment_pubkeys(program_id: &Pubkey) -> TipPaymentPubkeys {
    let config_pda = Pubkey::find_program_address(&[CONFIG_ACCOUNT_SEED], program_id).0;
    let tip_pda_0 = Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_0], program_id).0;
    let tip_pda_1 = Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_1], program_id).0;
    let tip_pda_2 = Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_2], program_id).0;
    let tip_pda_3 = Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_3], program_id).0;
    let tip_pda_4 = Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_4], program_id).0;
    let tip_pda_5 = Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_5], program_id).0;
    let tip_pda_6 = Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_6], program_id).0;
    let tip_pda_7 = Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_7], program_id).0;

    TipPaymentPubkeys {
        config_pda,
        tip_pdas: vec![
            tip_pda_0, tip_pda_1, tip_pda_2, tip_pda_3, tip_pda_4, tip_pda_5, tip_pda_6, tip_pda_7,
        ],
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StakeMetaCollection {
    /// List of [StakeMeta].
    pub stake_metas: Vec<StakeMeta>,

    /// base58 encoded tip-distribution program id.
    #[serde(with = "pubkey_string_conversion")]
    pub tip_distribution_program_id: Pubkey,

    /// Base58 encoded bank hash this object was generated at.
    pub bank_hash: String,

    /// Epoch for which this object was generated for.
    pub epoch: Epoch,

    /// Slot at which this object was generated.
    pub slot: Slot,
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct StakeMeta {
    #[serde(with = "pubkey_string_conversion")]
    pub validator_vote_account: Pubkey,

    #[serde(with = "pubkey_string_conversion")]
    pub validator_node_pubkey: Pubkey,

    /// The validator's tip-distribution meta if it exists.
    pub maybe_tip_distribution_meta: Option<TipDistributionMeta>,

    /// Delegations to this validator.
    pub delegations: Vec<Delegation>,

    /// The total amount of delegations to the validator.
    pub total_delegated: u64,

    /// The validator's delegation commission rate as a percentage between 0-100.
    pub commission: u8,
}

impl Ord for StakeMeta {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.validator_vote_account
            .cmp(&other.validator_vote_account)
    }
}

impl PartialOrd<Self> for StakeMeta {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct TipDistributionMeta {
    #[serde(with = "pubkey_string_conversion")]
    pub merkle_root_upload_authority: Pubkey,

    #[serde(with = "pubkey_string_conversion")]
    pub tip_distribution_pubkey: Pubkey,

    /// The validator's total tips in the [TipDistributionAccount].
    pub total_tips: u64,

    /// The validator's cut of tips from [TipDistributionAccount], calculated from the on-chain
    /// commission fee bps.
    pub validator_fee_bps: u16,
}

impl TipDistributionMeta {
    fn from_tda_wrapper(
        tda_wrapper: TipDistributionAccountWrapper,
        // The amount that will be left remaining in the tda to maintain rent exemption status.
        rent_exempt_amount: u64,
    ) -> Result<Self, stake_meta_generator_workflow::StakeMetaGeneratorError> {
        Ok(TipDistributionMeta {
            tip_distribution_pubkey: tda_wrapper.tip_distribution_pubkey,
            total_tips: tda_wrapper
                .account_data
                .lamports()
                .checked_sub(rent_exempt_amount)
                .ok_or(CheckedMathError)?,
            validator_fee_bps: tda_wrapper
                .tip_distribution_account
                .validator_commission_bps,
            merkle_root_upload_authority: tda_wrapper
                .tip_distribution_account
                .merkle_root_upload_authority,
        })
    }
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct Delegation {
    #[serde(with = "pubkey_string_conversion")]
    pub stake_account_pubkey: Pubkey,

    #[serde(with = "pubkey_string_conversion")]
    pub staker_pubkey: Pubkey,

    #[serde(with = "pubkey_string_conversion")]
    pub withdrawer_pubkey: Pubkey,

    /// Lamports delegated by the stake account
    pub lamports_delegated: u64,
}

impl Ord for Delegation {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (
            self.stake_account_pubkey,
            self.withdrawer_pubkey,
            self.staker_pubkey,
            self.lamports_delegated,
        )
            .cmp(&(
                other.stake_account_pubkey,
                other.withdrawer_pubkey,
                other.staker_pubkey,
                other.lamports_delegated,
            ))
    }
}

impl PartialOrd<Self> for Delegation {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Convenience wrapper around [TipDistributionAccount]
pub struct TipDistributionAccountWrapper {
    pub tip_distribution_account: TipDistributionAccount,
    pub account_data: AccountSharedData,
    pub tip_distribution_pubkey: Pubkey,
}

// TODO: move to program's sdk
pub fn derive_tip_distribution_account_address(
    tip_distribution_program_id: &Pubkey,
    vote_pubkey: &Pubkey,
    epoch: Epoch,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            TipDistributionAccount::SEED,
            vote_pubkey.to_bytes().as_ref(),
            epoch.to_le_bytes().as_ref(),
        ],
        tip_distribution_program_id,
    )
}

pub const MAX_RETRIES: usize = 5;
pub const FAIL_DELAY: Duration = Duration::from_millis(100);

pub async fn sign_and_send_transactions_with_retries(
    signer: &Keypair,
    rpc_client: &RpcClient,
    max_concurrent_rpc_get_reqs: usize,
    transactions: Vec<Transaction>,
    txn_send_batch_size: usize,
    max_loop_duration: Duration,
) -> (Vec<Transaction>, HashMap<Signature, Error>) {
    let semaphore = Arc::new(Semaphore::new(max_concurrent_rpc_get_reqs));
    let mut errors = HashMap::default();
    let mut blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .expect("fetch latest blockhash");
    // track unsigned txns
    let mut transactions_to_process = transactions
        .into_iter()
        .map(|txn| (txn.message_data(), txn))
        .collect::<HashMap<Vec<u8>, Transaction>>();

    let start = Instant::now();
    while start.elapsed() < max_loop_duration && !transactions_to_process.is_empty() {
        // ensure we always have a recent blockhash
        // blockhashes last max 150 blocks
        // finalized commitment is ~32 slots behind tip
        // assuming 0% skip rate (every slot has a block), we’d have roughly 120 slots
        // or (120*0.4s) = 48s to land a tx before it expires
        // if we’re refreshing every 30s, then any txs sent immediately before the refresh would likely expire
        if start.elapsed() > Duration::from_secs(1) {
            blockhash = rpc_client
                .get_latest_blockhash()
                .await
                .expect("fetch latest blockhash");
        }
        info!(
            "Sending {txn_send_batch_size} of {} transactions to claim mev tips",
            transactions_to_process.len()
        );
        let send_futs = transactions_to_process
            .iter()
            .take(txn_send_batch_size)
            .map(|(hash, txn)| {
                let semaphore = semaphore.clone();
                async move {
                    let _permit = semaphore.acquire_owned().await.unwrap(); // wait until our turn
                    let (txn, res) = signed_send(signer, rpc_client, blockhash, txn.clone()).await;
                    (hash.clone(), txn, res)
                }
            });

        let send_res = futures::future::join_all(send_futs).await;
        let new_errors = send_res
            .into_iter()
            .filter_map(|(hash, txn, result)| match result {
                Err(e) => Some((txn.signatures[0], e)),
                Ok(..) => {
                    let _ = transactions_to_process.remove(&hash);
                    None
                }
            })
            .collect::<HashMap<_, _>>();

        errors.extend(new_errors);
    }

    (transactions_to_process.values().cloned().collect(), errors)
}

pub async fn send_until_blockhash_expires(
    rpc_client: &RpcClient,
    transactions: Vec<Transaction>,
    blockhash: Hash,
    keypair: &Arc<Keypair>,
) -> solana_rpc_client_api::client_error::Result<()> {
    let mut claim_transactions: HashMap<Signature, Transaction> = transactions
        .into_iter()
        .map(|mut tx| {
            tx.sign(&[&keypair], blockhash);
            (*tx.get_signature(), tx)
        })
        .collect();

    let txs_requesting_send = claim_transactions.len();

    while rpc_client
        .is_blockhash_valid(&blockhash, CommitmentConfig::processed())
        .await?
    {
        let mut check_signatures = HashSet::with_capacity(claim_transactions.len());
        let mut already_processed = HashSet::with_capacity(claim_transactions.len());
        let mut is_blockhash_not_found = false;

        for (signature, tx) in &claim_transactions {
            match rpc_client
                .send_transaction_with_config(
                    tx,
                    RpcSendTransactionConfig {
                        skip_preflight: false,
                        preflight_commitment: Some(CommitmentLevel::Confirmed),
                        max_retries: Some(2),
                        ..RpcSendTransactionConfig::default()
                    },
                )
                .await
            {
                Ok(_) => {
                    check_signatures.insert(*signature);
                }
                Err(e) => match e.get_transaction_error() {
                    Some(TransactionError::BlockhashNotFound) => {
                        is_blockhash_not_found = true;
                        break;
                    }
                    Some(TransactionError::AlreadyProcessed) => {
                        already_processed.insert(*tx.get_signature());
                    }
                    Some(e) => {
                        warn!(
                            "TransactionError sending signature: {} error: {:?} tx: {:?}",
                            tx.get_signature(),
                            e,
                            tx
                        );
                    }
                    None => {
                        warn!(
                            "Unknown error sending transaction signature: {} error: {:?}",
                            tx.get_signature(),
                            e
                        );
                    }
                },
            }
        }

        sleep(Duration::from_secs(10)).await;

        let signatures: Vec<Signature> = check_signatures.iter().cloned().collect();
        let statuses = get_batched_signatures_statuses(rpc_client, &signatures).await?;

        for (signature, maybe_status) in &statuses {
            if let Some(_status) = maybe_status {
                claim_transactions.remove(signature);
                check_signatures.remove(signature);
            }
        }

        for signature in already_processed {
            claim_transactions.remove(&signature);
        }

        if claim_transactions.is_empty() || is_blockhash_not_found {
            break;
        }
    }

    let num_landed = txs_requesting_send
        .checked_sub(claim_transactions.len())
        .unwrap();
    info!("num_landed: {:?}", num_landed);

    Ok(())
}

pub async fn get_batched_signatures_statuses(
    rpc_client: &RpcClient,
    signatures: &[Signature],
) -> solana_rpc_client_api::client_error::Result<Vec<(Signature, Option<TransactionStatus>)>> {
    let mut signature_statuses = Vec::new();

    for signatures_batch in signatures.chunks(100) {
        // was using get_signature_statuses_with_history, but it blocks if the signatures don't exist
        // bigtable calls to read signatures that don't exist block forever w/o --rpc-bigtable-timeout argument set
        // get_signature_statuses looks in status_cache, which only has a 150 block history
        // may have false negative, but for this workflow it doesn't matter
        let statuses = rpc_client.get_signature_statuses(signatures_batch).await?;
        signature_statuses.extend(signatures_batch.iter().cloned().zip(statuses.value));
    }
    Ok(signature_statuses)
}

/// Just in time sign and send transaction to RPC
async fn signed_send(
    signer: &Keypair,
    rpc_client: &RpcClient,
    blockhash: Hash,
    mut txn: Transaction,
) -> (Transaction, solana_rpc_client_api::client_error::Result<()>) {
    txn.sign(&[signer], blockhash); // just in time signing
    let res = match rpc_client.send_and_confirm_transaction(&txn).await {
        Ok(_) => Ok(()),
        Err(e) => {
            match e.kind {
                // Already claimed, skip.
                ErrorKind::TransactionError(TransactionError::AlreadyProcessed)
                | ErrorKind::TransactionError(TransactionError::InstructionError(
                    0,
                    InstructionError::Custom(0),
                ))
                | ErrorKind::RpcError(RpcError::RpcResponseError {
                    data:
                        RpcResponseErrorData::SendTransactionPreflightFailure(
                            RpcSimulateTransactionResult {
                                err:
                                    Some(TransactionError::InstructionError(
                                        0,
                                        InstructionError::Custom(0),
                                    )),
                                ..
                            },
                        ),
                    ..
                }) => Ok(()),

                // transaction got held up too long and blockhash expired. retry txn
                ErrorKind::TransactionError(TransactionError::BlockhashNotFound) => Err(e),

                // unexpected error, warn and retry
                _ => {
                    error!(
                        "Error sending transaction. Signature: {}, Error: {e:?}",
                        txn.signatures[0]
                    );
                    Err(e)
                }
            }
        }
    };

    (txn, res)
}

async fn get_batched_accounts(
    rpc_client: &RpcClient,
    pubkeys: &[Pubkey],
) -> solana_rpc_client_api::client_error::Result<HashMap<Pubkey, Option<Account>>> {
    let mut batched_accounts = HashMap::new();

    for pubkeys_chunk in pubkeys.chunks(MAX_MULTIPLE_ACCOUNTS) {
        let accounts = rpc_client.get_multiple_accounts(pubkeys_chunk).await?;
        batched_accounts.extend(pubkeys_chunk.iter().cloned().zip(accounts));
    }
    Ok(batched_accounts)
}

/// Calculates the minimum balance needed to be rent-exempt
/// taken from: https://github.com/jito-foundation/jito-solana/blob/d1ba42180d0093dd59480a77132477323a8e3f88/sdk/program/src/rent.rs#L78
pub fn minimum_balance(data_len: usize) -> u64 {
    ((ACCOUNT_STORAGE_OVERHEAD
        .checked_add(data_len as u64)
        .unwrap()
        .checked_mul(DEFAULT_LAMPORTS_PER_BYTE_YEAR)
        .unwrap() as f64)
        * DEFAULT_EXEMPTION_THRESHOLD) as u64
}

mod pubkey_string_conversion {
    use ::{
        serde::{self, Deserialize, Deserializer, Serializer},
        solana_sdk::pubkey::Pubkey,
        std::str::FromStr,
    };

    pub(crate) fn serialize<S>(pubkey: &Pubkey, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&pubkey.to_string())
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Pubkey, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Pubkey::from_str(&s).map_err(serde::de::Error::custom)
    }
}

pub fn read_json_from_file<T>(path: &PathBuf) -> serde_json::Result<T>
where
    T: DeserializeOwned,
{
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    serde_json::from_reader(reader)
}
