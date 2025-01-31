use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use log::{info, warn};
use solana_client::{
    nonblocking::rpc_client::RpcClient, rpc_client::SerializableTransaction,
    rpc_config::RpcSendTransactionConfig, rpc_request::MAX_MULTIPLE_ACCOUNTS,
};
use solana_sdk::{
    account::Account,
    commitment_config::{CommitmentConfig, CommitmentLevel},
    hash::Hash,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    transaction::{Transaction, TransactionError},
};
use solana_transaction_status::TransactionStatus;
use tokio::time::sleep;

pub async fn get_batched_accounts(
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

pub async fn send_until_blockhash_expires(
    rpc_client: &RpcClient,
    rpc_sender_client: &RpcClient,
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
            match rpc_sender_client
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
