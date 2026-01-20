use anyhow::Result;
use jito_priority_fee_distribution_sdk::PriorityFeeDistributionAccount;
use jito_tip_distribution_sdk::TipDistributionAccount;
use log::info;
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
};
use solana_sdk::pubkey::Pubkey;

pub struct TipDistributionStats {
    pub account_pubkey: Pubkey,
    pub validator_vote_account: Pubkey,
    pub total_lamports: u64,
    pub is_priority_fee: bool,
    pub validator_commission_bps: u16,
    // Merkle root details
    pub has_merkle_root: bool,
    pub max_total_claim: u64,
    pub total_funds_claimed: u64,
    pub max_num_nodes: u64,
    pub num_nodes_claimed: u64,
}

pub async fn get_tip_distribution_stats(
    rpc_client: &RpcClient,
    tip_distribution_program_id: &Pubkey,
    priority_fee_distribution_program_id: &Pubkey,
    epoch: u64,
) -> Result<()> {
    info!(
        "=== Starting tip distribution stats for epoch {} ===",
        epoch
    );

    let rpc_client_with_timeout =
        RpcClient::new_with_timeout(rpc_client.url(), std::time::Duration::from_secs(1800));

    info!("[1/5] Fetching tip distribution accounts from RPC (this may take a while)...");
    let start = std::time::Instant::now();
    let tip_distribution_accounts = get_tip_distribution_accounts_for_epoch(
        &rpc_client_with_timeout,
        tip_distribution_program_id,
        epoch,
    )
    .await?;
    info!(
        "[1/5] Found {} tip distribution accounts for epoch {} in {:.2}s",
        tip_distribution_accounts.len(),
        epoch,
        start.elapsed().as_secs_f64()
    );

    info!("[2/5] Fetching priority fee distribution accounts from RPC...");
    let start = std::time::Instant::now();
    let priority_fee_distribution_accounts = get_priority_fee_distribution_accounts_for_epoch(
        &rpc_client_with_timeout,
        priority_fee_distribution_program_id,
        epoch,
    )
    .await?;
    info!(
        "[2/5] Found {} priority fee distribution accounts for epoch {} in {:.2}s",
        priority_fee_distribution_accounts.len(),
        epoch,
        start.elapsed().as_secs_f64()
    );

    let mut all_stats = Vec::new();
    let total_tip_accounts = tip_distribution_accounts.len();
    let total_pf_accounts = priority_fee_distribution_accounts.len();

    info!(
        "[3/5] Processing {} tip distribution accounts (fetching balances)...",
        total_tip_accounts
    );
    let start = std::time::Instant::now();
    for (i, (pubkey, account)) in tip_distribution_accounts.into_iter().enumerate() {
        if (i + 1) % 100 == 0 || i + 1 == total_tip_accounts {
            info!(
                "       Processing tip account {}/{} ({:.1}%)",
                i + 1,
                total_tip_accounts,
                ((i + 1) as f64 / total_tip_accounts as f64) * 100.0
            );
        }
        let stats =
            process_tip_distribution_account(&rpc_client_with_timeout, &pubkey, &account, false)
                .await?;
        all_stats.push(stats);
    }
    info!(
        "[3/5] Processed {} tip distribution accounts in {:.2}s",
        total_tip_accounts,
        start.elapsed().as_secs_f64()
    );

    info!(
        "[4/5] Processing {} priority fee distribution accounts (fetching balances)...",
        total_pf_accounts
    );
    let start = std::time::Instant::now();
    for (i, (pubkey, account)) in priority_fee_distribution_accounts.into_iter().enumerate() {
        if (i + 1) % 100 == 0 || i + 1 == total_pf_accounts {
            info!(
                "       Processing priority fee account {}/{} ({:.1}%)",
                i + 1,
                total_pf_accounts,
                ((i + 1) as f64 / total_pf_accounts as f64) * 100.0
            );
        }
        let stats = process_priority_fee_distribution_account(
            &rpc_client_with_timeout,
            &pubkey,
            &account,
            true,
        )
        .await?;
        all_stats.push(stats);
    }
    info!(
        "[4/5] Processed {} priority fee distribution accounts in {:.2}s",
        total_pf_accounts,
        start.elapsed().as_secs_f64()
    );

    info!("[5/5] Generating summary...");
    print_tip_distribution_summary(&all_stats, epoch);

    Ok(())
}

async fn get_tip_distribution_accounts_for_epoch(
    rpc_client: &RpcClient,
    tip_distribution_program_id: &Pubkey,
    epoch: u64,
) -> Result<Vec<(Pubkey, TipDistributionAccount)>> {
    info!("       Calling getProgramAccounts for tip distribution program...");
    let accounts = rpc_client
        .get_program_accounts_with_config(
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

    info!(
        "       Got {} total accounts, filtering for epoch {}...",
        accounts.len(),
        epoch
    );

    let mut result = Vec::new();
    let mut skipped_wrong_epoch = 0;
    let mut skipped_deserialize = 0;

    for (pubkey, account) in accounts {
        if let Ok(tip_distribution_account) = TipDistributionAccount::deserialize(&account.data) {
            if tip_distribution_account.epoch_created_at == epoch {
                result.push((pubkey, tip_distribution_account));
            } else {
                skipped_wrong_epoch += 1;
            }
        } else {
            skipped_deserialize += 1;
        }
    }

    info!(
        "       Filtered: {} matching epoch {}, {} other epochs, {} non-TDA accounts",
        result.len(),
        epoch,
        skipped_wrong_epoch,
        skipped_deserialize
    );

    Ok(result)
}

async fn get_priority_fee_distribution_accounts_for_epoch(
    rpc_client: &RpcClient,
    priority_fee_distribution_program_id: &Pubkey,
    epoch: u64,
) -> Result<Vec<(Pubkey, PriorityFeeDistributionAccount)>> {
    info!("       Calling getProgramAccounts for priority fee distribution program...");
    let accounts = rpc_client
        .get_program_accounts_with_config(
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

    info!(
        "       Got {} total accounts, filtering for epoch {}...",
        accounts.len(),
        epoch
    );

    let mut result = Vec::new();
    let mut skipped_wrong_epoch = 0;
    let mut skipped_deserialize = 0;

    for (pubkey, account) in accounts {
        if let Ok(priority_fee_distribution_account) =
            PriorityFeeDistributionAccount::deserialize(&account.data)
        {
            if priority_fee_distribution_account.epoch_created_at == epoch {
                result.push((pubkey, priority_fee_distribution_account));
            } else {
                skipped_wrong_epoch += 1;
            }
        } else {
            skipped_deserialize += 1;
        }
    }

    info!(
        "       Filtered: {} matching epoch {}, {} other epochs, {} non-PFDA accounts",
        result.len(),
        epoch,
        skipped_wrong_epoch,
        skipped_deserialize
    );

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

    // Extract merkle root details
    let (has_merkle_root, max_total_claim, total_funds_claimed, max_num_nodes, num_nodes_claimed) =
        if let Some(ref merkle_root) = tip_distribution_account.merkle_root {
            (
                true,
                merkle_root.max_total_claim,
                merkle_root.total_funds_claimed,
                merkle_root.max_num_nodes,
                merkle_root.num_nodes_claimed,
            )
        } else {
            (false, 0, 0, 0, 0)
        };

    Ok(TipDistributionStats {
        account_pubkey: *account_pubkey,
        validator_vote_account: tip_distribution_account.validator_vote_account,
        total_lamports,
        is_priority_fee,
        validator_commission_bps: tip_distribution_account.validator_commission_bps,
        has_merkle_root,
        max_total_claim,
        total_funds_claimed,
        max_num_nodes,
        num_nodes_claimed,
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

    // Extract merkle root details
    let (has_merkle_root, max_total_claim, total_funds_claimed, max_num_nodes, num_nodes_claimed) =
        if let Some(ref merkle_root) = priority_fee_distribution_account.merkle_root {
            (
                true,
                merkle_root.max_total_claim,
                merkle_root.total_funds_claimed,
                merkle_root.max_num_nodes,
                merkle_root.num_nodes_claimed,
            )
        } else {
            (false, 0, 0, 0, 0)
        };

    Ok(TipDistributionStats {
        account_pubkey: *account_pubkey,
        validator_vote_account: priority_fee_distribution_account.validator_vote_account,
        total_lamports,
        is_priority_fee,
        validator_commission_bps: priority_fee_distribution_account.validator_commission_bps,
        has_merkle_root,
        max_total_claim,
        total_funds_claimed,
        max_num_nodes,
        num_nodes_claimed,
    })
}

fn print_tip_distribution_summary(stats: &[TipDistributionStats], epoch: u64) {
    // Filter for only unclaimed TDAs:
    // - has merkle root
    // - not all nodes have claimed yet (num_nodes_claimed < max_num_nodes)
    let unclaimed_stats: Vec<_> = stats
        .iter()
        .filter(|s| s.has_merkle_root && s.num_nodes_claimed < s.max_num_nodes)
        .collect();

    info!("\n=== Epoch {} Tip Distribution Statistics ===", epoch);
    info!(
        "Total TDAs: {}, Unclaimed TDAs: {}",
        stats.len(),
        unclaimed_stats.len()
    );
    info!("");
    info!(
        "{:<46} {:<12} {:<10} {:<18} {:<18} {:<12}",
        "Account", "Remaining", "Type", "Claimed/Max", "Nodes", "Commission"
    );
    info!("{:-<120}", "");

    let mut total_remaining = 0u64;
    let mut total_unclaimed_funds = 0u64;

    for stat in &unclaimed_stats {
        let remaining_sol = stat.total_lamports as f64 / 1_000_000_000.0;
        let account_type = if stat.is_priority_fee {
            "Priority"
        } else {
            "Tip"
        };
        let commission_pct = stat.validator_commission_bps as f64 / 100.0;

        let claimed_sol = stat.total_funds_claimed as f64 / 1_000_000_000.0;
        let max_sol = stat.max_total_claim as f64 / 1_000_000_000.0;
        let unclaimed = stat
            .max_total_claim
            .saturating_sub(stat.total_funds_claimed);

        info!(
            "{:<46} {:<12.6} {:<10} {:<18} {:<18} {:<12.2}%",
            format!("{}", stat.account_pubkey),
            remaining_sol,
            account_type,
            format!("{:.4}/{:.4}", claimed_sol, max_sol),
            format!("{}/{}", stat.num_nodes_claimed, stat.max_num_nodes),
            commission_pct
        );

        total_remaining += stat.total_lamports;
        total_unclaimed_funds += unclaimed;
    }

    info!("{:-<120}", "");
    let total_remaining_sol = total_remaining as f64 / 1_000_000_000.0;
    let total_unclaimed_sol = total_unclaimed_funds as f64 / 1_000_000_000.0;

    info!(
        "{:<46} {:<12.6} {:<10} {:<18}",
        "TOTAL REMAINING (account balance)", total_remaining_sol, "SOL", ""
    );
    info!(
        "{:<46} {:<12.6} {:<10} {:<18}",
        "TOTAL UNCLAIMED (max - claimed)", total_unclaimed_sol, "SOL", ""
    );

    // Also show TDAs without merkle root
    let no_merkle_root: Vec<_> = stats.iter().filter(|s| !s.has_merkle_root).collect();
    if !no_merkle_root.is_empty() {
        info!("");
        info!(
            "=== TDAs without merkle root (not yet uploaded): {} ===",
            no_merkle_root.len()
        );
        for stat in &no_merkle_root {
            let account_type = if stat.is_priority_fee {
                "Priority"
            } else {
                "Tip"
            };
            info!("  {} ({})", stat.account_pubkey, account_type);
        }
    }
}
