use anyhow::Result;
use jito_tip_router_core::{
    account_payer::AccountPayer, base_fee_group::BaseFeeGroup, constants::MAX_OPERATORS,
    epoch_state::AccountStatus, ncn_fee_group::NcnFeeGroup,
};
use solana_metrics::datapoint_info;
use solana_sdk::native_token::lamports_to_sol;

use crate::{
    getters::{
        get_account_payer, get_all_operators_in_ncn, get_all_tickets, get_all_vaults_in_ncn,
        get_ballot_box, get_base_reward_receiver, get_base_reward_router,
        get_current_epoch_and_slot, get_epoch_snapshot, get_epoch_state, get_is_epoch_completed,
        get_ncn_reward_receiver, get_ncn_reward_router, get_operator, get_operator_snapshot,
        get_tip_router_config, get_vault, get_vault_operator_delegation, get_vault_registry,
        get_weight_table,
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
    emit_ncn_metrics_vault_tickets(handler).await?;
    emit_ncn_metrics_vault_operator_delegation(handler).await?;
    emit_ncn_metrics_operators(handler).await?;
    emit_ncn_metrics_vault_registry(handler).await?;
    emit_ncn_metrics_config(handler).await?;
    emit_ncn_metrics_account_payer(handler).await?;
    emit_ncn_metrics_epoch_slot(handler).await?;

    Ok(())
}

pub async fn emit_ncn_metrics_epoch_slot(handler: &CliHandler) -> Result<()> {
    let (current_epoch, current_slot) = get_current_epoch_and_slot(handler).await?;

    datapoint_info!(
        "trk-em-epoch-slot",
        ("current-epoch", current_epoch, i64),
        ("current-slot", current_slot, i64),
    );

    Ok(())
}

pub async fn emit_ncn_metrics_account_payer(handler: &CliHandler) -> Result<()> {
    let (current_epoch, current_slot) = get_current_epoch_and_slot(handler).await?;

    let (account_payer_address, _, _) =
        AccountPayer::find_program_address(&handler.tip_router_program_id, handler.ncn()?);
    let account_payer = get_account_payer(handler).await?;

    datapoint_info!(
        "trk-em-vault-ticket",
        ("current-epoch", current_epoch, i64),
        ("current-slot", current_slot, i64),
        ("account-payer", account_payer_address.to_string(), String),
        ("balance", account_payer.lamports, i64),
        ("balance-sol", lamports_to_sol(account_payer.lamports), f64),
    );

    Ok(())
}

pub async fn emit_ncn_metrics_vault_tickets(handler: &CliHandler) -> Result<()> {
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

pub async fn emit_ncn_metrics_vault_operator_delegation(handler: &CliHandler) -> Result<()> {
    let (current_epoch, current_slot) = get_current_epoch_and_slot(handler).await?;
    let all_operators = get_all_operators_in_ncn(handler).await?;
    let all_vaults = get_all_vaults_in_ncn(handler).await?;

    for operator in all_operators.iter() {
        for vault in all_vaults.iter() {
            let result = get_vault_operator_delegation(handler, vault, operator).await;

            if result.is_err() {
                continue;
            }
            let vault_operator_delegation = result?;

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

pub async fn emit_ncn_metrics_operators(handler: &CliHandler) -> Result<()> {
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

pub async fn emit_ncn_metrics_vault_registry(handler: &CliHandler) -> Result<()> {
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
        if vault.is_empty() {
            continue;
        }

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

pub async fn emit_ncn_metrics_config(handler: &CliHandler) -> Result<()> {
    let (current_epoch, current_slot) = get_current_epoch_and_slot(handler).await?;

    let config = get_tip_router_config(handler).await?;
    let fee_config = config.fee_config;
    let current_fees = fee_config.current_fees(current_epoch);

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
        // Fees
        (
            "block-engine-fee-bps",
            fee_config.block_engine_fee_bps(),
            i64
        ),
        (
            "base-fee-wallet",
            fee_config
                .base_fee_wallet(BaseFeeGroup::default())?
                .to_string(),
            String
        ),
        (
            "base-fee-dao",
            current_fees.base_fee_bps(BaseFeeGroup::dao())?,
            i64
        ),
        (
            "ncn-fee-lst",
            current_fees.ncn_fee_bps(NcnFeeGroup::lst())?,
            i64
        ),
        (
            "ncn-fee-jto",
            current_fees.ncn_fee_bps(NcnFeeGroup::jto())?,
            i64
        ),
        ("total-fees", current_fees.total_fees_bps()?, i64)
    );

    Ok(())
}

pub async fn emit_epoch_metrics(handler: &CliHandler, epoch: u64) -> Result<()> {
    emit_epoch_metrics_state(handler, epoch).await?;
    emit_epoch_metrics_weight_table(handler, epoch).await?;
    emit_epoch_metrics_epoch_snapshot(handler, epoch).await?;
    emit_epoch_metrics_operator_snapshot(handler, epoch).await?;
    emit_epoch_metrics_ballot_box(handler, epoch).await?;
    emit_epoch_metrics_base_rewards(handler, epoch).await?;
    emit_epoch_metrics_ncn_rewards(handler, epoch).await?;

    Ok(())
}

pub async fn emit_epoch_metrics_ncn_rewards(handler: &CliHandler, epoch: u64) -> Result<()> {
    let (current_epoch, current_slot) = get_current_epoch_and_slot(handler).await?;

    let all_operators = get_all_operators_in_ncn(handler).await?;
    for operator in all_operators {
        for group in NcnFeeGroup::all_groups().iter().take(2) {
            let result = get_ncn_reward_router(handler, *group, &operator, epoch).await;

            if let Ok(ncn_reward_router) = result {
                let (ncn_reward_receiver_address, ncn_reward_receiver_account) =
                    get_ncn_reward_receiver(handler, *group, &operator, epoch).await?;

                let total_vault_rewards = ncn_reward_router
                    .vault_reward_routes()
                    .iter()
                    .map(|route| route.rewards())
                    .sum::<u64>();

                for route in ncn_reward_router.vault_reward_routes() {
                    if route.is_empty() {
                        continue;
                    }
                    datapoint_info!(
                        "trk-ee-epoch-ncn-vault-rewards",
                        ("current-epoch", current_epoch, i64),
                        ("current-slot", current_slot, i64),
                        ("keeper-epoch", epoch, i64),
                        ("group", group.group, i64),
                        ("operator", operator.to_string(), String),
                        ("vault", route.vault().to_string(), String),
                        ("rewards", route.rewards(), i64),
                    );
                }

                datapoint_info!(
                    "trk-ee-epoch-ncn-rewards",
                    ("current-epoch", current_epoch, i64),
                    ("current-slot", current_slot, i64),
                    ("keeper-epoch", epoch, i64),
                    ("group", group.group, i64),
                    ("operator", operator.to_string(), String),
                    (
                        "receiver-address",
                        ncn_reward_receiver_address.to_string(),
                        String
                    ),
                    (
                        "receiver-balance",
                        ncn_reward_receiver_account.lamports,
                        i64
                    ),
                    (
                        "receiver-balance-sol",
                        lamports_to_sol(ncn_reward_receiver_account.lamports),
                        f64
                    ),
                    ("still-routing", ncn_reward_router.still_routing(), bool),
                    ("total-rewards", ncn_reward_router.total_rewards(), i64),
                    (
                        "rewards-processed",
                        ncn_reward_router.rewards_processed(),
                        i64
                    ),
                    (
                        "operator-rewards",
                        ncn_reward_router.operator_rewards(),
                        i64
                    ),
                    ("total-vault-rewards", total_vault_rewards, i64),
                );
            }
        }
    }

    Ok(())
}

pub async fn emit_epoch_metrics_base_rewards(handler: &CliHandler, epoch: u64) -> Result<()> {
    let (current_epoch, current_slot) = get_current_epoch_and_slot(handler).await?;

    let result = get_base_reward_router(handler, epoch).await;

    if let Ok(base_reward_router) = result {
        let (base_reward_receiver_address, base_reward_receiver_account) =
            get_base_reward_receiver(handler, epoch).await?;

        datapoint_info!(
            "trk-ee-epoch-base-rewards",
            ("current-epoch", current_epoch, i64),
            ("current-slot", current_slot, i64),
            ("keeper-epoch", epoch, i64),
            (
                "receiver-address",
                base_reward_receiver_address.to_string(),
                String
            ),
            (
                "receiver-balance",
                base_reward_receiver_account.lamports,
                i64
            ),
            (
                "receiver-balance-sol",
                lamports_to_sol(base_reward_receiver_account.lamports),
                f64
            ),
            ("still-routing", base_reward_router.still_routing(), bool),
            ("total-rewards", base_reward_router.total_rewards(), i64),
            (
                "rewards-processed",
                base_reward_router.rewards_processed(),
                i64
            ),
            (
                "dao-rewards",
                base_reward_router.base_fee_group_reward(BaseFeeGroup::dao())?,
                i64
            ),
            (
                "lst-rewards",
                base_reward_router.ncn_fee_group_rewards(NcnFeeGroup::lst())?,
                i64
            ),
            (
                "jto-rewards",
                base_reward_router.ncn_fee_group_rewards(NcnFeeGroup::jto())?,
                i64
            ),
        );
    }

    Ok(())
}

#[allow(clippy::large_stack_frames)]
pub async fn emit_epoch_metrics_ballot_box(handler: &CliHandler, epoch: u64) -> Result<()> {
    let (current_epoch, current_slot) = get_current_epoch_and_slot(handler).await?;
    let valid_slots_after_consensus = {
        let config = get_tip_router_config(handler).await?;

        config.valid_slots_after_consensus()
    };

    let result = get_ballot_box(handler, epoch).await;

    if let Ok(ballot_box) = result {
        for operator_vote in ballot_box.operator_votes() {
            if operator_vote.is_empty() {
                continue;
            }

            let ballot_index = operator_vote.ballot_index();
            let ballot_tally = ballot_box.ballot_tallies()[ballot_index as usize];
            let vote = format!("{:?}", ballot_tally.ballot().root());
            ballot_tally.stake_weights().stake_weight();

            datapoint_info!(
                "trk-ee-ballot-box-votes",
                ("current-epoch", current_epoch, i64),
                ("current-slot", current_slot, i64),
                ("keeper-epoch", epoch, i64),
                ("operator", operator_vote.operator().to_string(), String),
                ("slot-voted", operator_vote.slot_voted(), i64),
                (
                    "operator-stake-weight",
                    operator_vote.stake_weights().stake_weight(),
                    i64
                ),
                (
                    "ballot-stake-weight",
                    ballot_tally.stake_weights().stake_weight(),
                    i64
                ),
                ("vote", vote, String),
            );
        }

        datapoint_info!(
            "trk-ee-ballot-box",
            ("current-epoch", current_epoch, i64),
            ("current-slot", current_slot, i64),
            ("keeper-epoch", epoch, i64),
            ("unique-ballots", ballot_box.unique_ballots(), i64),
            ("operators-voted", ballot_box.operators_voted(), i64),
            ("has-winning-ballot", ballot_box.has_winning_ballot(), bool),
            (
                "is-voting-valid",
                ballot_box.is_voting_valid(current_slot, valid_slots_after_consensus)?,
                bool
            ),
        );
    }

    Ok(())
}

pub async fn emit_epoch_metrics_operator_snapshot(handler: &CliHandler, epoch: u64) -> Result<()> {
    let (current_epoch, current_slot) = get_current_epoch_and_slot(handler).await?;

    let all_operators = get_all_operators_in_ncn(handler).await?;

    for operator in all_operators.iter() {
        let result = get_operator_snapshot(handler, operator, epoch).await;

        if let Ok(operator_snapshot) = result {
            datapoint_info!(
                "trk-ee-operator-snapshot",
                ("current-epoch", current_epoch, i64),
                ("current-slot", current_slot, i64),
                ("keeper-epoch", epoch, i64),
                ("operator", operator.to_string(), String),
                ("is-finalized", operator_snapshot.finalized(), bool),
                ("is-active", operator_snapshot.is_active(), bool),
                (
                    "ncn-operator-index",
                    operator_snapshot.ncn_operator_index(),
                    i64
                ),
                (
                    "operator-fee-bps",
                    operator_snapshot.operator_fee_bps(),
                    i64
                ),
                (
                    "valid-operator-vault-delegations",
                    operator_snapshot.valid_operator_vault_delegations(),
                    i64
                ),
                (
                    "vault-operator-delegation-count",
                    operator_snapshot.vault_operator_delegation_count(),
                    i64
                ),
                (
                    "vault-operator-delegations-registered",
                    operator_snapshot.vault_operator_delegations_registered(),
                    i64
                ),
                (
                    "stake-weight",
                    operator_snapshot.stake_weights().stake_weight(),
                    i64
                ),
                ("slot-finalized", operator_snapshot.slot_finalized(), i64),
            );
        }
    }

    Ok(())
}

pub async fn emit_epoch_metrics_epoch_snapshot(handler: &CliHandler, epoch: u64) -> Result<()> {
    let (current_epoch, current_slot) = get_current_epoch_and_slot(handler).await?;

    let result = get_epoch_snapshot(handler, epoch).await;

    if let Ok(epoch_snapshot) = result {
        let fees = epoch_snapshot.fees();

        datapoint_info!(
            "trk-ee-epoch-snapshot",
            ("current-epoch", current_epoch, i64),
            ("current-slot", current_slot, i64),
            ("keeper-epoch", epoch, i64),
            (
                "total-stake-weight",
                epoch_snapshot.stake_weights().stake_weight(),
                i64
            ),
            (
                "valid-operator-vault-delegations",
                epoch_snapshot.valid_operator_vault_delegations(),
                i64
            ),
            (
                "operators-registered",
                epoch_snapshot.operators_registered(),
                i64
            ),
            ("operator-count", epoch_snapshot.operator_count(), i64),
            ("vault-count", epoch_snapshot.vault_count(), i64),
            (
                "base-fee-bps",
                fees.base_fee_bps(BaseFeeGroup::default())?,
                i64
            ),
            ("base-fee-dao", fees.base_fee_bps(BaseFeeGroup::dao())?, i64),
            ("ncn-fee-lst", fees.ncn_fee_bps(NcnFeeGroup::lst())?, i64),
            ("ncn-fee-jto", fees.ncn_fee_bps(NcnFeeGroup::jto())?, i64),
            ("total-fees", fees.total_fees_bps()?, i64)
        );
    }

    Ok(())
}

pub async fn emit_epoch_metrics_weight_table(handler: &CliHandler, epoch: u64) -> Result<()> {
    let (current_epoch, current_slot) = get_current_epoch_and_slot(handler).await?;

    let result = get_weight_table(handler, epoch).await;

    if let Ok(weight_table) = result {
        for entry in weight_table.table() {
            if entry.is_empty() {
                continue;
            }

            datapoint_info!(
                "trk-ee-weight-table-entry",
                ("current-epoch", current_epoch, i64),
                ("current-slot", current_slot, i64),
                ("keeper-epoch", epoch, i64),
                ("st-mint", entry.st_mint().to_string(), String),
                ("weight", entry.weight() as u64, i64),
            );
        }

        datapoint_info!(
            "trk-ee-weight-table",
            ("current-epoch", current_epoch, i64),
            ("current-slot", current_slot, i64),
            ("keeper-epoch", epoch, i64),
            ("weight-count", weight_table.mint_count(), i64),
            ("vault-count", weight_table.vault_count(), i64),
            ("weight-count", weight_table.weight_count(), i64),
        );
    }

    Ok(())
}

#[allow(clippy::large_stack_frames)]
pub async fn emit_epoch_metrics_state(handler: &CliHandler, epoch: u64) -> Result<()> {
    let (current_epoch, current_slot) = get_current_epoch_and_slot(handler).await?;

    let is_epoch_completed = get_is_epoch_completed(handler, epoch).await?;

    if is_epoch_completed {
        datapoint_info!(
            "trk-ee-state",
            ("current-epoch", current_epoch, i64),
            ("current-slot", current_slot, i64),
            ("keeper-epoch", epoch, i64),
            ("current-state-string", "Complete", String),
            ("current-state", u8::MAX, i64),
            ("is-complete", true, bool),
        );

        return Ok(());
    }

    let state = get_epoch_state(handler, epoch).await?;
    let current_state = {
        let (valid_slots_after_consensus, epochs_after_consensus_before_close) = {
            let config = get_tip_router_config(handler).await?;
            (
                config.valid_slots_after_consensus(),
                config.epochs_after_consensus_before_close(),
            )
        };
        let epoch_schedule = handler.rpc_client().get_epoch_schedule().await?;

        if state.set_weight_progress().tally() > 0 {
            let weight_table = get_weight_table(handler, epoch).await?;
            state.current_state_patched(
                &epoch_schedule,
                valid_slots_after_consensus,
                epochs_after_consensus_before_close,
                weight_table.st_mint_count() as u64,
                current_slot,
            )
        } else {
            state.current_state(
                &epoch_schedule,
                valid_slots_after_consensus,
                epochs_after_consensus_before_close,
                current_slot,
            )
        }
    }?;

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
            "current-state-string",
            format!("{:?}", current_state),
            String
        ),
        ("current-state", current_state as u8, i64),
        ("operator-count", state.operator_count(), i64),
        ("vault-count", state.vault_count(), i64),
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
