use anyhow::Result;
use jito_priority_fee_distribution_sdk::{
    ClaimStatus as PriorityFeeClaimStatus, PriorityFeeDistributionAccount,
};
use jito_tip_distribution_sdk::{ClaimStatus as TipClaimStatus, TipDistributionAccount};
use log::info;
use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_sdk::pubkey::Pubkey;

use crate::rpc_utils;

pub struct TipDistributionStats {
    pub account_pubkey: Pubkey,
    pub validator_vote_account: Pubkey,
    pub merkle_root_upload_authority: Pubkey,
    pub total_lamports: u64,
    pub is_priority_fee: bool,
    pub validator_commission_bps: u16,
}

struct ClaimStatusTotals {
    tip_claim_status_accounts: usize,
    priority_fee_claim_status_accounts: usize,
}

pub async fn get_tip_distribution_stats(
    rpc_client: &RpcClient,
    tip_distribution_program_id: &Pubkey,
    priority_fee_distribution_program_id: &Pubkey,
    epoch: u64,
) -> Result<()> {
    info!("Fetching tip distribution accounts for epoch {}...", epoch);

    let rpc_client_with_timeout =
        RpcClient::new_with_timeout(rpc_client.url(), std::time::Duration::from_secs(1800));

    let tip_distribution_accounts = get_tip_distribution_accounts_for_epoch(
        &rpc_client_with_timeout,
        tip_distribution_program_id,
        epoch,
    )
    .await?;

    let priority_fee_distribution_accounts = get_priority_fee_distribution_accounts_for_epoch(
        &rpc_client_with_timeout,
        priority_fee_distribution_program_id,
        epoch,
    )
    .await?;

    info!(
        "Found {} tip distribution accounts and {} priority fee distribution accounts",
        tip_distribution_accounts.len(),
        priority_fee_distribution_accounts.len()
    );

    info!("Fetching claim status account totals...");
    let claim_status_totals = get_claim_status_totals(
        &rpc_client_with_timeout,
        tip_distribution_program_id,
        priority_fee_distribution_program_id,
    )
    .await?;

    let mut all_stats = Vec::new();

    for (pubkey, account) in tip_distribution_accounts {
        let stats =
            process_tip_distribution_account(&rpc_client_with_timeout, &pubkey, &account, false)
                .await?;
        all_stats.push(stats);
    }

    for (pubkey, account) in priority_fee_distribution_accounts {
        let stats = process_priority_fee_distribution_account(
            &rpc_client_with_timeout,
            &pubkey,
            &account,
            true,
        )
        .await?;
        all_stats.push(stats);
    }

    print_tip_distribution_summary(&all_stats, epoch, &claim_status_totals);

    Ok(())
}

async fn get_tip_distribution_accounts_for_epoch(
    rpc_client: &RpcClient,
    tip_distribution_program_id: &Pubkey,
    epoch: u64,
) -> Result<Vec<(Pubkey, TipDistributionAccount)>> {
    let accounts = rpc_utils::get_program_accounts_with_config(
        rpc_client,
        tip_distribution_program_id,
        RpcProgramAccountsConfig {
            filters: None,
            account_config: RpcAccountInfoConfig {
                encoding: Some(solana_account_decoder::UiAccountEncoding::Base64),
                ..RpcAccountInfoConfig::default()
            },
            ..RpcProgramAccountsConfig::default()
        },
    )
    .await?;

    let mut result = Vec::new();
    for (pubkey, account) in accounts {
        if let Ok(tip_distribution_account) = TipDistributionAccount::deserialize(&account.data) {
            if tip_distribution_account.epoch_created_at == epoch {
                result.push((pubkey, tip_distribution_account));
            }
        }
    }

    Ok(result)
}

async fn get_priority_fee_distribution_accounts_for_epoch(
    rpc_client: &RpcClient,
    priority_fee_distribution_program_id: &Pubkey,
    epoch: u64,
) -> Result<Vec<(Pubkey, PriorityFeeDistributionAccount)>> {
    let accounts = rpc_utils::get_program_accounts_with_config(
        rpc_client,
        priority_fee_distribution_program_id,
        RpcProgramAccountsConfig {
            filters: None,
            account_config: RpcAccountInfoConfig {
                encoding: Some(solana_account_decoder::UiAccountEncoding::Base64),
                ..RpcAccountInfoConfig::default()
            },
            ..RpcProgramAccountsConfig::default()
        },
    )
    .await?;

    let mut result = Vec::new();
    for (pubkey, account) in accounts {
        if let Ok(priority_fee_distribution_account) =
            PriorityFeeDistributionAccount::deserialize(&account.data)
        {
            if priority_fee_distribution_account.epoch_created_at == epoch {
                result.push((pubkey, priority_fee_distribution_account));
            }
        }
    }

    Ok(result)
}

async fn process_tip_distribution_account(
    rpc_client: &RpcClient,
    account_pubkey: &Pubkey,
    tip_distribution_account: &TipDistributionAccount,
    is_priority_fee: bool,
) -> Result<TipDistributionStats> {
    // Get the account data to calculate total lamports
    let account_info = rpc_client.get_account(account_pubkey).await?;
    let rent_exempt_amount = rpc_client
        .get_minimum_balance_for_rent_exemption(account_info.data.len())
        .await?;
    let total_lamports = account_info.lamports.saturating_sub(rent_exempt_amount);

    Ok(TipDistributionStats {
        account_pubkey: *account_pubkey,
        validator_vote_account: tip_distribution_account.validator_vote_account,
        merkle_root_upload_authority: tip_distribution_account.merkle_root_upload_authority,
        total_lamports,
        is_priority_fee,
        validator_commission_bps: tip_distribution_account.validator_commission_bps,
    })
}

async fn process_priority_fee_distribution_account(
    rpc_client: &RpcClient,
    account_pubkey: &Pubkey,
    priority_fee_distribution_account: &PriorityFeeDistributionAccount,
    is_priority_fee: bool,
) -> Result<TipDistributionStats> {
    // Get the account data to calculate total lamports
    let account_info = rpc_client.get_account(account_pubkey).await?;
    let rent_exempt_amount = rpc_client
        .get_minimum_balance_for_rent_exemption(account_info.data.len())
        .await?;
    let total_lamports = account_info.lamports.saturating_sub(rent_exempt_amount);

    Ok(TipDistributionStats {
        account_pubkey: *account_pubkey,
        validator_vote_account: priority_fee_distribution_account.validator_vote_account,
        merkle_root_upload_authority: priority_fee_distribution_account
            .merkle_root_upload_authority,
        total_lamports,
        is_priority_fee,
        validator_commission_bps: priority_fee_distribution_account.validator_commission_bps,
    })
}

async fn get_claim_status_totals(
    rpc_client: &RpcClient,
    tip_distribution_program_id: &Pubkey,
    priority_fee_distribution_program_id: &Pubkey,
) -> Result<ClaimStatusTotals> {
    // Fetch all tip distribution claim status accounts
    let tip_claim_filters = vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
        0,
        TipClaimStatus::DISCRIMINATOR.to_vec(),
    ))];

    let tip_claim_accounts = rpc_utils::get_program_accounts_with_config(
        rpc_client,
        tip_distribution_program_id,
        RpcProgramAccountsConfig {
            filters: Some(tip_claim_filters),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                ..RpcAccountInfoConfig::default()
            },
            ..RpcProgramAccountsConfig::default()
        },
    )
    .await?;

    // Fetch all priority fee distribution claim status accounts
    let pf_claim_filters = vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
        0,
        PriorityFeeClaimStatus::DISCRIMINATOR.to_vec(),
    ))];

    let pf_claim_accounts = rpc_utils::get_program_accounts_with_config(
        rpc_client,
        priority_fee_distribution_program_id,
        RpcProgramAccountsConfig {
            filters: Some(pf_claim_filters),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                ..RpcAccountInfoConfig::default()
            },
            ..RpcProgramAccountsConfig::default()
        },
    )
    .await?;

    Ok(ClaimStatusTotals {
        tip_claim_status_accounts: tip_claim_accounts.len(),
        priority_fee_claim_status_accounts: pf_claim_accounts.len(),
    })
}

fn print_tip_distribution_summary(
    stats: &[TipDistributionStats],
    epoch: u64,
    claim_status_totals: &ClaimStatusTotals,
) {
    info!("\n=== Epoch {} Tip Distribution Statistics ===", epoch);
    info!(
        "Claim status totals: tip={}, priority_fee={}",
        claim_status_totals.tip_claim_status_accounts,
        claim_status_totals.priority_fee_claim_status_accounts
    );
    info!(
        "Per-account claim counts are omitted because ClaimStatus PDAs cannot be attributed to a single distribution account without claimant data."
    );
    info!(
        "{:<50} {:<50} {:<15} {:<10} {:<10}",
        "Account", "Merkle Root Upload Authority", "Total (SOL)", "Type", "Commission"
    );
    info!("{:-<133}", "");

    let mut total_total = 0u64;

    for stat in stats {
        let total_sol = stat.total_lamports as f64 / 1_000_000_000.0;
        let account_type = if stat.is_priority_fee {
            "Priority"
        } else {
            "Tip"
        };
        let commission_pct = stat.validator_commission_bps as f64 / 100.0;

        info!(
            "{:<50} {:<50} {:<15.6} {:<10} {:<10.2}",
            format!("{}", stat.account_pubkey),
            format!("{}", stat.merkle_root_upload_authority),
            total_sol,
            account_type,
            commission_pct,
        );

        total_total += stat.total_lamports;
    }

    info!("{:-<133}", "");
    let total_total_sol = total_total as f64 / 1_000_000_000.0;

    info!(
        "{:<50} {:<50} {:<15.6} {:<10} {:<10}",
        "TOTAL", "", total_total_sol, "ALL", ""
    );
}
