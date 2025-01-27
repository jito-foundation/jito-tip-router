use anyhow::Result;
use jito_tip_router_core::{
    constants::MAX_OPERATORS, epoch_state::AccountStatus, ncn_fee_group::NcnFeeGroup,
};
use solana_metrics::datapoint_info;

use crate::{
    getters::{
        get_all_operators_in_ncn, get_all_tickets, get_all_vaults_in_ncn,
        get_current_epoch_and_slot, get_epoch_state, get_is_epoch_completed, get_operator,
        get_tip_router_config, get_vault, get_vault_operator_delegation, get_vault_registry,
    },
    handler::CliHandler,
};

pub async fn emit_error(title: String, error: String, message: String, keeper_epoch: u64) {
    datapoint_info!(
        "trk-error",
        ("command-title", title, String),
        ("error", error, String),
        ("message", message, String),
        ("keeper-epoch", keeper_epoch, i64),
    );
}

pub async fn emit_ncn_metrics(handler: &CliHandler) -> Result<()> {
    emit_epoch_metrics_vault_tickets(handler).await?;
    emit_epoch_metrics_vault_operator_delegation(handler).await?;
    emit_epoch_metrics_operators(handler).await?;
    emit_epoch_metrics_vault_registry(handler).await?;
    emit_epoch_metrics_config(handler).await?;

    Ok(())
}

pub async fn emit_epoch_metrics_vault_tickets(handler: &CliHandler) -> Result<()> {
    let (current_epoch, current_slot) = get_current_epoch_and_slot(handler).await?;
    let all_tickets = get_all_tickets(handler).await?;

    for ticket in all_tickets {
        datapoint_info!(
            "trk-em-vault-ticket",
            ("current-epoch", current_epoch, i64),
            ("current-slot", current_slot, i64),
            ("operator", ticket.operator.to_string(), String),
            ("vault", ticket.vault.to_string(), String),
            ("ncn-vault", ticket.ncn_vault(), i64),
            ("vault-ncn", ticket.vault_ncn(), i64),
            ("ncn-operator", ticket.ncn_operator(), i64),
            ("operator-ncn", ticket.operator_ncn(), i64),
            ("operator-vault", ticket.operator_vault(), i64),
            ("vault-operator", ticket.vault_operator(), i64),
        );
    }

    Ok(())
}

pub async fn emit_epoch_metrics_vault_operator_delegation(handler: &CliHandler) -> Result<()> {
    let (current_epoch, current_slot) = get_current_epoch_and_slot(handler).await?;
    let all_operators = get_all_operators_in_ncn(handler).await?;
    let all_vaults = get_all_vaults_in_ncn(handler).await?;

    for operator in all_operators.iter() {
        for vault in all_vaults.iter() {
            let vault_operator_delegation =
                get_vault_operator_delegation(handler, &vault, &operator).await?;

            //TODO add delegation?
            datapoint_info!(
                "trk-em-vault-operator-delegation",
                ("current-epoch", current_epoch, i64),
                ("current-slot", current_slot, i64),
                ("vault", vault.to_string(), String),
                ("operator", operator.to_string(), String),
                (
                    "delegation",
                    vault_operator_delegation
                        .delegation_state
                        .total_security()?,
                    i64
                ),
            );
        }
    }

    Ok(())
}

pub async fn emit_epoch_metrics_operators(handler: &CliHandler) -> Result<()> {
    let (current_epoch, current_slot) = get_current_epoch_and_slot(handler).await?;
    let all_operators = get_all_operators_in_ncn(handler).await?;

    for operator in all_operators {
        let operator_account = get_operator(handler, &operator).await?;

        datapoint_info!(
            "trk-em-operator",
            ("current-epoch", current_epoch, i64),
            ("current-slot", current_slot, i64),
            ("operator", operator.to_string(), String),
            (
                "fee",
                Into::<u16>::into(operator_account.operator_fee_bps) as i64,
                i64
            ),
            ("vault-count", operator_account.vault_count(), i64),
            ("ncn-count", operator_account.ncn_count(), i64),
        );
    }

    Ok(())
}

pub async fn emit_epoch_metrics_vault_registry(handler: &CliHandler) -> Result<()> {
    let (current_epoch, current_slot) = get_current_epoch_and_slot(handler).await?;
    let vault_registry = get_vault_registry(handler).await?;

    datapoint_info!(
        "trk-em-vault-registry",
        ("current-epoch", current_epoch, i64),
        ("current-slot", current_slot, i64),
        ("st-mints", vault_registry.st_mint_count(), i64),
        ("vaults", vault_registry.vault_count(), i64)
    );

    for vault in vault_registry.vault_list {
        let vault_account = get_vault(handler, vault.vault()).await?;

        datapoint_info!(
            "trk-em-vault-registry-vault",
            ("current-epoch", current_epoch, i64),
            ("current-slot", current_slot, i64),
            ("vault", vault.vault().to_string(), String),
            ("st-mint", vault.st_mint().to_string(), String),
            ("index", vault.vault_index(), i64),
            ("tokens-deposited", vault_account.tokens_deposited(), i64),
            ("vrt-supply", vault_account.vrt_supply(), i64),
            ("operator-count", vault_account.operator_count(), i64),
            ("ncn-count", vault_account.ncn_count(), i64),
        );
    }

    for st_mint in vault_registry.st_mint_list {
        datapoint_info!(
            "trk-em-vault-registry-st-mint",
            ("current-epoch", current_epoch, i64),
            ("current-slot", current_slot, i64),
            ("st-mint", st_mint.st_mint().to_string(), String),
            ("ncn-fee-group", st_mint.ncn_fee_group().group, i64),
            (
                "switchboard-feed",
                st_mint.switchboard_feed().to_string(),
                String
            ),
            (
                "no-feed-weight",
                st_mint.no_feed_weight().to_string(),
                String
            ),
            (
                "reward-multiplier-bps",
                st_mint.reward_multiplier_bps(),
                i64
            ),
        );
    }

    Ok(())
}

pub async fn emit_epoch_metrics_config(handler: &CliHandler) -> Result<()> {
    let (current_epoch, current_slot) = get_current_epoch_and_slot(handler).await?;
    let config = get_tip_router_config(handler).await?;

    datapoint_info!(
        "trk-em-config",
        ("current-epoch", current_epoch, i64),
        ("current-slot", current_slot, i64),
        (
            "epochs-after-consensus-before-close",
            config.epochs_after_consensus_before_close(),
            i64
        ),
        ("epochs-before-stall", config.epochs_before_stall(), i64),
        ("starting-valid-epoch", config.starting_valid_epoch(), i64),
        (
            "valid-slots-after-consensus",
            config.valid_slots_after_consensus(),
            i64
        ),
        ("fee-admin", config.fee_admin.to_string(), String),
        (
            "tie-breaker-admin",
            config.tie_breaker_admin.to_string(),
            String
        ),
    );

    Ok(())
}

pub async fn emit_epoch_metrics(handler: &CliHandler, epoch: u64) -> Result<()> {
    let (current_epoch, current_slot) = get_current_epoch_and_slot(handler).await?;

    let is_epoch_completed = get_is_epoch_completed(handler, epoch).await?;

    if is_epoch_completed {
        datapoint_info!(
            "trk-ee-state",
            ("current-epoch", current_epoch, i64),
            ("current-slot", current_slot, i64),
            ("keeper-epoch", epoch, i64),
            ("is-complete", true, bool),
        );
    }

    let state = get_epoch_state(handler, epoch).await?;

    let mut operator_snapshot_dne = 0;
    let mut operator_snapshot_open = 0;
    let mut operator_snapshot_closed = 0;
    let mut ncn_router_dne = 0;
    let mut ncn_router_open = 0;
    let mut ncn_router_closed = 0;
    for i in 0..MAX_OPERATORS {
        let operator_snapshot_status = state.account_status().operator_snapshot(i)?;

        match operator_snapshot_status {
            AccountStatus::DNE => operator_snapshot_dne += 1,
            AccountStatus::Closed => operator_snapshot_closed += 1,
            _ => operator_snapshot_open += 1,
        }

        for group in NcnFeeGroup::all_groups() {
            let ncn_fee_group_status = state.account_status().ncn_reward_router(i, group)?;

            match ncn_fee_group_status {
                AccountStatus::DNE => ncn_router_dne += 1,
                AccountStatus::Closed => ncn_router_closed += 1,
                _ => ncn_router_open += 1,
            }
        }
    }

    datapoint_info!(
        "trk-ee-state",
        ("current-epoch", current_epoch, i64),
        ("current-slot", current_slot, i64),
        ("keeper-epoch", epoch, i64),
        ("is-complete", false, bool),
        (
            "set-weight-progress-tally",
            state.set_weight_progress().tally(),
            i64
        ),
        (
            "set-weight-progress-total",
            state.set_weight_progress().total(),
            i64
        ),
        (
            "epoch-snapshot-progress-tally",
            state.epoch_snapshot_progress().tally(),
            i64
        ),
        (
            "epoch-snapshot-progress-total",
            state.epoch_snapshot_progress().total(),
            i64
        ),
        (
            "voting-progress-tally",
            state.voting_progress().tally(),
            i64
        ),
        (
            "voting-progress-total",
            state.voting_progress().total(),
            i64
        ),
        (
            "validation-progress-tally",
            state.validation_progress().tally(),
            i64
        ),
        (
            "validation-progress-total",
            state.validation_progress().total(),
            i64
        ),
        (
            "upload-progress-tally",
            state.upload_progress().tally(),
            i64
        ),
        (
            "upload-progress-total",
            state.upload_progress().total(),
            i64
        ),
        (
            "total-distribution-progress-tally",
            state.total_distribution_progress().tally(),
            i64
        ),
        (
            "total-distribution-progress-total",
            state.total_distribution_progress().total(),
            i64
        ),
        (
            "base-distribution-progress-tally",
            state.base_distribution_progress().tally(),
            i64
        ),
        (
            "base-distribution-progress-total",
            state.base_distribution_progress().total(),
            i64
        ),
        // Account status
        (
            "epoch-state-account-status",
            state.account_status().epoch_state()?,
            i64
        ),
        (
            "weight-table-account-status",
            state.account_status().weight_table()?,
            i64
        ),
        (
            "epoch-snapshot-account-status",
            state.account_status().epoch_snapshot()?,
            i64
        ),
        (
            "ballot-box-account-status",
            state.account_status().ballot_box()?,
            i64
        ),
        (
            "base-reward-router-account-status",
            state.account_status().base_reward_router()?,
            i64
        ),
        ("operator-snapshot-account-dne", operator_snapshot_dne, i64),
        (
            "operator-snapshot-account-open",
            operator_snapshot_open,
            i64
        ),
        (
            "operator-snapshot-account-closed",
            operator_snapshot_closed,
            i64
        ),
        ("ncn-reward-router-account-dne", ncn_router_dne, i64),
        ("ncn-reward-router-account-open", ncn_router_open, i64),
        ("ncn-reward-router-account-closed", ncn_router_closed, i64),
    );

    Ok(())
}
