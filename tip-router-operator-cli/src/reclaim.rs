use std::{sync::Arc, time::Instant};

use anyhow::Result;
use jito_priority_fee_distribution_sdk::{
    instruction::{
        close_claim_status_ix as close_pf_claim_status_ix,
        close_priority_fee_distribution_account_ix,
    },
    ClaimStatus as PriorityFeeDistributionClaimStatus, Config as PriorityFeeDistributionConfig,
    PriorityFeeDistributionAccount,
};
use jito_tip_distribution_sdk::{
    instruction::{
        close_claim_status_ix as close_tip_claim_status_ix, close_tip_distribution_account_ix,
    },
    ClaimStatus as TipDistributionClaimStatus, Config as TipDistributionConfig,
    TipDistributionAccount,
};
use log::{error, info};
use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_sdk::signature::Signer;

use crate::{rpc_utils, tx_utils::pack_transactions};
use rand::seq::SliceRandom;
use solana_metrics::datapoint_info;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, transaction::Transaction};

const MAX_TRANSACTION_SIZE: usize = 1232;

pub async fn close_expired_accounts(
    rpc_url: &str,
    tip_distribution_program_id: Pubkey,
    priority_fee_distribution_program_id: Pubkey,
    signer: Arc<Keypair>,
    num_monitored_epochs: u64,
) -> Result<()> {
    info!("Closing expired distribution accounts");
    close_expired_distribution_accounts(
        rpc_url,
        tip_distribution_program_id,
        priority_fee_distribution_program_id,
        signer.clone(),
        num_monitored_epochs,
    )
    .await?;
    info!("Closing expired claim status accounts");
    close_expired_claims(
        rpc_url,
        tip_distribution_program_id,
        priority_fee_distribution_program_id,
        signer.clone(),
        num_monitored_epochs,
    )
    .await?;
    Ok(())
}

pub async fn close_expired_claims(
    rpc_url: &str,
    tip_distribution_program_id: Pubkey,
    priority_fee_distribution_program_id: Pubkey,
    signer: Arc<Keypair>,
    num_monitored_epochs: u64,
) -> Result<()> {
    let epochs_to_process = {
        // Use default timeout and commitment config for fetching the current epoch
        let rpc_client = rpc_utils::new_rpc_client(rpc_url);
        let current_epoch = rpc_client.get_epoch_info().await?.epoch;
        (current_epoch - num_monitored_epochs)..current_epoch
    };
    for epoch in epochs_to_process {
        let rpc_client = rpc_utils::new_high_timeout_rpc_client(rpc_url);
        info!("Fetching claim status accounts expiring in epoch {}", epoch);
        let start = Instant::now();
        let (tip_distribution_claim_accounts, priority_fee_distribution_claim_accounts) =
            fetch_expired_claim_statuses(
                &rpc_client,
                tip_distribution_program_id,
                priority_fee_distribution_program_id,
                epoch,
            )
            .await?;
        let duration = start.elapsed();
        datapoint_info!(
            "tip_router_cli.expired_claim_statuses",
            (
                "expired_tip_claim_statuses",
                tip_distribution_claim_accounts.len(),
                i64
            ),
            (
                "expired_pf_claim_statuses",
                priority_fee_distribution_claim_accounts.len(),
                i64
            ),
            ("duration", duration.as_secs(), i64),
            "epoch" => epoch.to_string(),
        );

        let close_tip_claim_transactions = close_tip_claim_transactions(
            &tip_distribution_claim_accounts,
            tip_distribution_program_id,
            signer.pubkey(),
        );
        let close_priority_fee_claim_transactions = close_priority_fee_claim_transactions(
            &priority_fee_distribution_claim_accounts,
            priority_fee_distribution_program_id,
            signer.pubkey(),
        );
        let mut transactions = [
            close_tip_claim_transactions,
            close_priority_fee_claim_transactions,
        ]
        .concat();

        info!("Processing {} close claim transactions", transactions.len());
        let rpc_client = rpc_utils::new_rpc_client(rpc_url);
        transactions.shuffle(&mut rand::thread_rng());
        for batch in transactions.chunks_mut(100_000) {
            let start = Instant::now();
            let mut blockhash = rpc_client.get_latest_blockhash().await?;
            for transaction in batch.iter_mut() {
                transaction.sign(&[&signer], blockhash);
                let maybe_signature = rpc_client.send_transaction(transaction).await;
                if let Err(e) = maybe_signature {
                    // Fetch a new blockhash if the transaction failed
                    blockhash = rpc_client.get_latest_blockhash().await?;
                    error!("Error sending transaction: {:?}", e);
                }
            }
            let duration = start.elapsed();
            info!(
                "Processed batch of {} close claim transactions in {:?} seconds",
                batch.len(),
                duration.as_secs()
            );
        }
    }
    Ok(())
}

pub async fn close_expired_distribution_accounts(
    rpc_url: &str,
    tip_distribution_program_id: Pubkey,
    priority_fee_distribution_program_id: Pubkey,
    signer: Arc<Keypair>,
    num_monitored_epochs: u64,
) -> Result<()> {
    let epochs_to_process = {
        // Use default timeout and commitment config for fetching the current epoch
        let rpc_client = rpc_utils::new_rpc_client(rpc_url);
        let current_epoch = rpc_client.get_epoch_info().await?.epoch;
        (current_epoch - num_monitored_epochs)..current_epoch
    };
    for epoch in epochs_to_process {
        let rpc_client = rpc_utils::new_high_timeout_rpc_client(rpc_url);
        info!("Fetching distribution accounts expiring in epoch {}", epoch);
        let start = Instant::now();
        let (tip_distribution_accounts, priority_fee_distribution_accounts) =
            fetch_expired_distribution_accounts(
                &rpc_client,
                tip_distribution_program_id,
                priority_fee_distribution_program_id,
                epoch,
            )
            .await?;
        let duration = start.elapsed();
        datapoint_info!(
            "tip_router_cli.expired_distribution_accounts",
            (
                "expired_tip_distribution_accounts",
                tip_distribution_accounts.len(),
                i64
            ),
            (
                "expired_priority_fee_distribution_accounts",
                priority_fee_distribution_accounts.len(),
                i64
            ),
            ("duration", duration.as_secs(), i64),
            "epoch" => epoch.to_string(),
        );

        if tip_distribution_accounts.is_empty() && priority_fee_distribution_accounts.is_empty() {
            info!("No expired distribution accounts found in epoch {}", epoch);
            continue;
        }
        let close_tip_claim_transactions = close_tip_distribution_account_transactions(
            &rpc_client,
            &tip_distribution_accounts,
            tip_distribution_program_id,
            signer.pubkey(),
        )
        .await?;
        let close_priority_fee_claim_transactions =
            close_priority_fee_distribution_account_transactions(
                &rpc_client,
                &priority_fee_distribution_accounts,
                priority_fee_distribution_program_id,
                signer.pubkey(),
            )
            .await?;
        let mut transactions = [
            close_tip_claim_transactions,
            close_priority_fee_claim_transactions,
        ]
        .concat();

        info!(
            "Processing {} close distribution account transactions",
            transactions.len()
        );
        let rpc_client = rpc_utils::new_rpc_client(rpc_url);
        transactions.shuffle(&mut rand::thread_rng());
        for batch in transactions.chunks_mut(100_000) {
            let start = Instant::now();
            let mut blockhash = rpc_client.get_latest_blockhash().await?;
            for transaction in batch.iter_mut() {
                transaction.sign(&[&signer], blockhash);
                let maybe_signature = rpc_client.send_transaction(transaction).await;
                if let Err(e) = maybe_signature {
                    // Fetch a new blockhash if the transaction failed
                    blockhash = rpc_client.get_latest_blockhash().await?;
                    error!("Error sending transaction: {:?}", e);
                }
            }
            let duration = start.elapsed();
            info!(
                "Processed batch of {} close distribution account transactions in {:?} seconds",
                batch.len(),
                duration.as_secs()
            );
        }
    }
    Ok(())
}

fn close_tip_claim_transactions(
    accounts: &[(Pubkey, TipDistributionClaimStatus)],
    tip_distribution_program_id: Pubkey,
    payer: Pubkey,
) -> Vec<Transaction> {
    let config_pubkey =
        jito_tip_distribution_sdk::derive_config_account_address(&tip_distribution_program_id).0;

    let instructions: Vec<_> = accounts
        .iter()
        .map(|(pubkey, account)| {
            close_tip_claim_status_ix(config_pubkey, *pubkey, account.claim_status_payer)
        })
        .collect();

    pack_transactions(instructions, payer, MAX_TRANSACTION_SIZE)
}

fn close_priority_fee_claim_transactions(
    accounts: &[(Pubkey, PriorityFeeDistributionClaimStatus)],
    priority_fee_distribution_program_id: Pubkey,
    payer: Pubkey,
) -> Vec<Transaction> {
    let config_pubkey = jito_priority_fee_distribution_sdk::derive_config_account_address(
        &priority_fee_distribution_program_id,
    )
    .0;

    let instructions: Vec<_> = accounts
        .iter()
        .map(|(pubkey, account)| {
            close_pf_claim_status_ix(config_pubkey, *pubkey, account.claim_status_payer)
        })
        .collect();

    pack_transactions(instructions, payer, MAX_TRANSACTION_SIZE)
}

async fn close_tip_distribution_account_transactions(
    rpc_client: &RpcClient,
    accounts: &[(Pubkey, TipDistributionAccount)],
    tip_distribution_program_id: Pubkey,
    payer: Pubkey,
) -> Result<Vec<Transaction>> {
    let config_pubkey =
        jito_tip_distribution_sdk::derive_config_account_address(&tip_distribution_program_id).0;

    let config_account = rpc_client
        .get_account_with_config(
            &config_pubkey,
            RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                ..RpcAccountInfoConfig::default()
            },
        )
        .await?
        .value
        .ok_or_else(|| anyhow::anyhow!("Config account not found"))?;

    let tip_distribution_config = TipDistributionConfig::deserialize(&config_account.data)?;

    let instructions: Vec<_> = accounts
        .iter()
        .map(|(pubkey, account)| {
            close_tip_distribution_account_ix(
                config_pubkey,
                *pubkey,
                tip_distribution_config.expired_funds_account,
                account.validator_vote_account,
                payer,
                account.epoch_created_at,
            )
        })
        .collect();

    Ok(pack_transactions(instructions, payer, MAX_TRANSACTION_SIZE))
}

async fn close_priority_fee_distribution_account_transactions(
    rpc_client: &RpcClient,
    accounts: &[(Pubkey, PriorityFeeDistributionAccount)],
    priority_fee_distribution_program_id: Pubkey,
    payer: Pubkey,
) -> Result<Vec<Transaction>> {
    let config_pubkey = jito_priority_fee_distribution_sdk::derive_config_account_address(
        &priority_fee_distribution_program_id,
    )
    .0;
    let config_account = rpc_client
        .get_account_with_config(
            &config_pubkey,
            RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                ..RpcAccountInfoConfig::default()
            },
        )
        .await?
        .value
        .ok_or_else(|| anyhow::anyhow!("Config account not found"))?;

    let priority_fee_distribution_config =
        PriorityFeeDistributionConfig::deserialize(&config_account.data)?;

    let instructions: Vec<_> = accounts
        .iter()
        .map(|(pubkey, account)| {
            close_priority_fee_distribution_account_ix(
                config_pubkey,
                *pubkey,
                priority_fee_distribution_config.expired_funds_account,
                account.validator_vote_account,
                payer,
                account.epoch_created_at,
            )
        })
        .collect();

    Ok(pack_transactions(instructions, payer, MAX_TRANSACTION_SIZE))
}

pub async fn fetch_expired_distribution_accounts(
    rpc_client: &RpcClient,
    tip_distribution_program_id: Pubkey,
    priority_fee_distribution_program_id: Pubkey,
    target_epoch: u64,
) -> Result<(
    Vec<(Pubkey, TipDistributionAccount)>,
    Vec<(Pubkey, PriorityFeeDistributionAccount)>,
)> {
    let tda_filter = vec![
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            0,
            TipDistributionAccount::DISCRIMINATOR.to_vec(),
        )),
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            8 // Discriminator
            + 32 // vote_account
            + 32 // merkle_root_upload_authority
            + 1  // Option<MerkleRoot>
            + 32 // merkle_root.root
            + 8  // merkle_root.max_total_claim
            + 8  // merkle_root.max_num_nodes
            + 8  // merkle_root.total_funds_claimed
            + 8  // merkle_root.num_nodes_claimed
            + 8  // epoch_created_at
            + 2, // commission_bps
            target_epoch.to_le_bytes().to_vec(),
        )),
    ];
    let tda_accounts = rpc_client.get_program_accounts_with_config(
        &tip_distribution_program_id,
        RpcProgramAccountsConfig {
            filters: Some(tda_filter),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                ..RpcAccountInfoConfig::default()
            },
            ..RpcProgramAccountsConfig::default()
        },
    );

    let pfda_filter = vec![
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            0,
            PriorityFeeDistributionAccount::DISCRIMINATOR.to_vec(),
        )),
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            8 // Discriminator
            + 32 // vote_account
            + 32 // merkle_root_upload_authority
            + 1  // Option<MerkleRoot>
            + 32 // merkle_root.root
            + 8  // merkle_root.max_total_claim
            + 8  // merkle_root.max_num_nodes
            + 8  // merkle_root.total_funds_claimed
            + 8  // merkle_root.num_nodes_claimed
            + 8  // epoch_created_at
            + 2, // commission_bps
            target_epoch.to_le_bytes().to_vec(),
        )),
    ];
    let pfda_accounts = rpc_client.get_program_accounts_with_config(
        &priority_fee_distribution_program_id,
        RpcProgramAccountsConfig {
            filters: Some(pfda_filter),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                ..RpcAccountInfoConfig::default()
            },
            ..RpcProgramAccountsConfig::default()
        },
    );

    let (tda_accounts, pfda_accounts) = tokio::join!(tda_accounts, pfda_accounts);

    info!(
        "Fetched {} expired tip distribution accounts and {} expired priority fee distribution accounts",
        tda_accounts.as_ref().map_or(0, |v| v.len()),
        pfda_accounts.as_ref().map_or(0, |v| v.len()),
    );

    let tda_accounts = tda_accounts?
        .iter()
        .flat_map(|(pubkey, account)| {
            let tip_distribution_account =
                TipDistributionAccount::deserialize(account.data.as_slice());
            tip_distribution_account.map_or_else(
                |_| vec![],
                |tip_distribution_account| vec![(*pubkey, tip_distribution_account)],
            )
        })
        .collect();
    let pfda_accounts = pfda_accounts?
        .iter()
        .flat_map(|(pubkey, account)| {
            let priority_fee_distribution_account =
                PriorityFeeDistributionAccount::deserialize(&account.data);
            priority_fee_distribution_account.map_or_else(
                |_| vec![],
                |priority_fee_distribution_account| {
                    vec![(*pubkey, priority_fee_distribution_account)]
                },
            )
        })
        .collect();
    Ok((tda_accounts, pfda_accounts))
}

async fn fetch_expired_claim_statuses(
    rpc_client: &RpcClient,
    tip_distribution_program_id: Pubkey,
    priority_fee_distribution_program_id: Pubkey,
    target_epoch: u64,
) -> Result<(
    Vec<(Pubkey, TipDistributionClaimStatus)>,
    Vec<(Pubkey, PriorityFeeDistributionClaimStatus)>,
)> {
    let tip_distribution_claim_filters = vec![
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            0,
            jito_tip_distribution_sdk::ClaimStatus::DISCRIMINATOR.to_vec(),
        )),
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            8 // Discriminator
            + 1 // is_claimed
            + 32 // claimant
            + 32 // claim_status_payer
            + 8 // slot_claimed_at
            + 8, // amount
            target_epoch.to_le_bytes().to_vec(),
        )),
    ];

    let tip_distribution_claim_accounts = rpc_client.get_program_accounts_with_config(
        &tip_distribution_program_id,
        RpcProgramAccountsConfig {
            filters: Some(tip_distribution_claim_filters),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                ..RpcAccountInfoConfig::default()
            },
            ..RpcProgramAccountsConfig::default()
        },
    );

    let priority_fee_distribution_claim_filters = vec![
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            0,
            jito_priority_fee_distribution_sdk::ClaimStatus::DISCRIMINATOR.to_vec(),
        )),
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            8 // Discriminator
            + 32, // claim_status_payer
            target_epoch.to_le_bytes().to_vec(),
        )),
    ];
    let priority_fee_distribution_claim_accounts = rpc_client.get_program_accounts_with_config(
        &priority_fee_distribution_program_id,
        RpcProgramAccountsConfig {
            filters: Some(priority_fee_distribution_claim_filters),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                ..RpcAccountInfoConfig::default()
            },
            ..RpcProgramAccountsConfig::default()
        },
    );

    let (tip_distribution_claim_accounts, priority_fee_distribution_claim_accounts) = tokio::join!(
        tip_distribution_claim_accounts,
        priority_fee_distribution_claim_accounts
    );

    let tip_distribution_claim_accounts = tip_distribution_claim_accounts?
        .iter()
        .flat_map(|(pubkey, account)| {
            let tip_distribution_claim_status =
                TipDistributionClaimStatus::deserialize(account.data.as_slice());
            tip_distribution_claim_status.map_or_else(
                |_| vec![],
                |tip_distribution_claim_status| vec![(*pubkey, tip_distribution_claim_status)],
            )
        })
        .collect();

    let priority_fee_distribution_claim_accounts = priority_fee_distribution_claim_accounts?
        .iter()
        .flat_map(|(pubkey, account)| {
            let priority_fee_distribution_claim_status =
                PriorityFeeDistributionClaimStatus::deserialize(&account.data);
            priority_fee_distribution_claim_status.map_or_else(
                |_| vec![],
                |priority_fee_distribution_claim_status| {
                    vec![(*pubkey, priority_fee_distribution_claim_status)]
                },
            )
        })
        .collect();

    Ok((
        tip_distribution_claim_accounts,
        priority_fee_distribution_claim_accounts,
    ))
}
