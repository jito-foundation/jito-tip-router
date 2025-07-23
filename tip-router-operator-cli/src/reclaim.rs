use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use anchor_lang::{AccountDeserialize, Discriminator};
use anyhow::Result;
use jito_priority_fee_distribution_sdk::{
    instruction::close_claim_status_ix as close_pf_claim_status_ix,
    jito_priority_fee_distribution::accounts::ClaimStatus as PriorityFeeDistributionClaimStatus,
    PriorityFeeDistributionAccount,
};
use jito_tip_distribution_sdk::{
    instruction::close_claim_status_ix as close_tip_claim_status_ix,
    jito_tip_distribution::accounts::ClaimStatus as TipDistributionClaimStatus,
    TipDistributionAccount,
};
use log::{debug, info};
use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_sdk::signature::Signer;

use crate::{rpc_utils, tx_utils::pack_transactions};
use solana_metrics::datapoint_info;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Keypair,
    transaction::Transaction,
};

const MAX_TRANSACTION_SIZE: usize = 1232;

pub fn create_bulk_transaction_rpc_client(rpc_url: &str) -> RpcClient {
    RpcClient::new_with_timeout_and_commitment(
        rpc_url.to_string(),
        Duration::from_secs(1800),
        CommitmentConfig::processed(),
    )
}

pub async fn close_expired_accounts(
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
        info!("Fetching TipDistribution and PriorityFeeDistribution Claim Statuses expiring in epoch {}", epoch);
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
        info!("Found {} TipDistribution Claim Statuses and {} PriorityFeeDistribution claim statuses with expiration epoch {} in {:?}", tip_distribution_claim_accounts.len(), priority_fee_distribution_claim_accounts.len(), epoch, duration);
        datapoint_info!(
            "tip_router_cli.expired_claim_statuses",
            ("epoch", epoch, i64),
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
        );

        let rpc_client = rpc_utils::new_high_timeout_rpc_client(rpc_url);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(300)).await;
                let start = Instant::now();
                let (tip_distribution_claim_accounts, priority_fee_distribution_claim_accounts) =
                    fetch_expired_claim_statuses(
                        &rpc_client,
                        tip_distribution_program_id,
                        priority_fee_distribution_program_id,
                        epoch,
                    )
                    .await
                    .unwrap_or_else(|e| {
                        debug!("Error fetching expired claim statuses: {:?}", e);
                        (vec![], vec![])
                    });
                let duration = start.elapsed();
                datapoint_info!(
                    "tip_router_cli.expired_claim_statuses",
                    ("epoch", epoch, i64),
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
                );
            }
        });

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
        for batch in transactions.chunks_mut(5000) {
            let start = Instant::now();
            let mut blockhash = rpc_client.get_latest_blockhash().await?;
            for transaction in batch.iter_mut() {
                transaction.sign(&[&signer], blockhash);
                let maybe_signature = rpc_client.send_transaction(transaction).await;
                if let Err(e) = maybe_signature {
                    // Fetch a new blockhash if the transaction failed
                    blockhash = rpc_client.get_latest_blockhash().await?;
                    debug!("Error sending transaction: {:?}", e);
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
        .map(|(pubkey, _account)| {
            // For priority fee distribution, we need to derive the claim_status_payer
            // This is typically the claimant who created the claim status account
            // For now, we'll use the signer as the payer since we don't have access to the original claimant
            close_pf_claim_status_ix(config_pubkey, *pubkey, payer)
        })
        .collect();

    pack_transactions(instructions, payer, MAX_TRANSACTION_SIZE)
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
            137,
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
            137,
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

    let tda_accounts = tda_accounts?
        .iter()
        .flat_map(|(pubkey, account)| {
            let tip_distribution_account =
                TipDistributionAccount::try_deserialize(&mut account.data.as_slice());
            if let Ok(tip_distribution_account) = tip_distribution_account {
                vec![(*pubkey, tip_distribution_account)]
            } else {
                vec![]
            }
        })
        .collect();
    let pfda_accounts = pfda_accounts?
        .iter()
        .flat_map(|(pubkey, account)| {
            let priority_fee_distribution_account =
                PriorityFeeDistributionAccount::try_deserialize(&mut account.data.as_slice());
            if let Ok(priority_fee_distribution_account) = priority_fee_distribution_account {
                vec![(*pubkey, priority_fee_distribution_account)]
            } else {
                vec![]
            }
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
            jito_tip_distribution_sdk::jito_tip_distribution::accounts::ClaimStatus::DISCRIMINATOR
                .to_vec(),
        )),
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            89,
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
            jito_priority_fee_distribution_sdk::jito_priority_fee_distribution::accounts::ClaimStatus::DISCRIMINATOR.to_vec(),
        )),
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            40,
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
                TipDistributionClaimStatus::try_deserialize(&mut account.data.as_slice());
            if let Ok(tip_distribution_claim_status) = tip_distribution_claim_status {
                vec![(*pubkey, tip_distribution_claim_status)]
            } else {
                vec![]
            }
        })
        .collect();

    let priority_fee_distribution_claim_accounts = priority_fee_distribution_claim_accounts?
        .iter()
        .flat_map(|(pubkey, account)| {
            let priority_fee_distribution_claim_status =
                PriorityFeeDistributionClaimStatus::try_deserialize(&mut account.data.as_slice());
            if let Ok(priority_fee_distribution_claim_status) =
                priority_fee_distribution_claim_status
            {
                vec![(*pubkey, priority_fee_distribution_claim_status)]
            } else {
                vec![]
            }
        })
        .collect();

    Ok((
        tip_distribution_claim_accounts,
        priority_fee_distribution_claim_accounts,
    ))
}
