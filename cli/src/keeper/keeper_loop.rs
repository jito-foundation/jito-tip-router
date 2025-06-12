use std::time::Duration;

use crate::{
    getters::get_guaranteed_epoch_and_slot,
    handler::CliHandler,
    instructions::{
        crank_close_epoch_accounts, crank_distribute, crank_post_vote_cooldown,
        crank_register_vaults, crank_set_weight, crank_snapshot, crank_vote, create_epoch_state,
        migrate_tda_merkle_root_upload_authorities, update_all_vaults_in_network,
    },
    keeper::{
        keeper_metrics::{emit_epoch_metrics, emit_error, emit_heartbeat, emit_ncn_metrics},
        keeper_state::KeeperState,
    },
};
use anyhow::Result;
use jito_tip_router_core::epoch_state::State;
use log::info;
use solana_metrics::set_host_id;
use std::process::Command;
use tokio::time::sleep;

pub async fn progress_epoch(
    is_epoch_completed: bool,
    current_epoch: u64,
    starting_epoch: u64,
    last_current_epoch: u64,
    keeper_epoch: u64,
    epoch_stall: bool,
) -> (u64, bool) {
    if current_epoch > last_current_epoch {
        // Automatically go to new epoch
        return (current_epoch, true);
    }

    if is_epoch_completed || epoch_stall {
        // Reset to starting epoch
        if keeper_epoch == current_epoch {
            return (starting_epoch, false);
        }

        // Increment keeper epoch
        return (keeper_epoch + 1, false);
    }

    (keeper_epoch, false)
}

#[allow(clippy::future_not_send)]
pub async fn check_and_timeout_error<T>(
    title: String,
    result: &Result<T>,
    error_timeout_ms: u64,
    keeper_epoch: u64,
    cluster_name: &str,
) -> bool {
    if let Err(e) = result {
        let error = format!("{:?}", e);
        let message = format!("Error: [{}] \n{}\n\n", title, error);

        log::error!("{}", message);
        emit_error(title, error, message, keeper_epoch, cluster_name).await;
        timeout_error(error_timeout_ms).await;
        true
    } else {
        false
    }
}

pub async fn timeout_error(duration_ms: u64) {
    info!("Error Timeout for {}s", duration_ms as f64 / 1000.0);
    sleep(Duration::from_millis(duration_ms)).await;
    // progress_bar(duration_ms).await;
}

pub async fn timeout_keeper(duration_ms: u64) {
    info!("Keeper Timeout for {}s", duration_ms as f64 / 1000.0);
    sleep(Duration::from_millis(duration_ms)).await;
    // boring_progress_bar(duration_ms).await;
}

#[allow(clippy::large_stack_frames)]
#[allow(clippy::too_many_arguments)]
pub async fn startup_keeper(
    handler: &CliHandler,
    loop_timeout_ms: u64,
    error_timeout_ms: u64,
    test_vote: bool,
    all_vault_update: bool,
    emit_metrics: bool,
    metrics_only: bool,
    run_migration: bool,
    cluster_name: String,
    region: String,
) -> Result<()> {
    assert!(handler.ncn().is_ok(), "missing NCN address!");

    let mut state: KeeperState = KeeperState::default();
    let mut epoch_stall = false;
    let mut current_keeper_epoch = handler.epoch;
    let mut is_new_epoch = true;
    let mut tick = 0;
    let (mut last_current_epoch, _) = get_guaranteed_epoch_and_slot(handler).await;

    let mut start_of_loop;
    let mut end_of_loop;

    let run_operations = !metrics_only && !run_migration;
    let emit_metrics = emit_metrics || metrics_only;

    let hostname_cmd = Command::new("hostname")
        .output()
        .expect("Failed to execute hostname command");

    let hostname = String::from_utf8_lossy(&hostname_cmd.stdout)
        .trim()
        .to_string();

    set_host_id(format!(
        "tip-router-keeper_{}_{}_{}",
        region, cluster_name, hostname
    ));

    loop {
        // If there is a new epoch, this will do a full vault update on *all* vaults
        // created with restaking - this adds some extra redundancy
        if is_new_epoch && all_vault_update && run_operations {
            info!("\n\n-2. Update Vaults - {}\n", current_keeper_epoch);
            let result = update_all_vaults_in_network(handler).await;

            if check_and_timeout_error(
                "Update Vaults".to_string(),
                &result,
                error_timeout_ms,
                state.epoch,
                &cluster_name,
            )
            .await
            {
                continue;
            }
        }

        // This will progress the epoch:
        // If a new Epoch turns over, it will automatically progress to it
        // If there has been a stall, it will automatically progress to the next epoch
        // If there is still work to be done on the given epoch, it will stay
        // Note: This will loop around and start back at the beginning
        {
            info!("\n\nA. Progress Epoch - {}\n", current_keeper_epoch);
            let starting_epoch = handler.epoch;
            let keeper_epoch = current_keeper_epoch;

            let (current_epoch, _) = get_guaranteed_epoch_and_slot(handler).await;
            let (result, set_is_new_epoch) = progress_epoch(
                state.is_epoch_completed,
                current_epoch,
                starting_epoch,
                last_current_epoch,
                keeper_epoch,
                epoch_stall,
            )
            .await;

            if current_keeper_epoch != result {
                info!(
                    "\n\nPROGRESS EPOCH: {} -> {}\n\n",
                    current_keeper_epoch, result
                );
            }

            is_new_epoch = set_is_new_epoch;
            current_keeper_epoch = result;
            last_current_epoch = last_current_epoch.max(current_keeper_epoch);
            epoch_stall = false;
            start_of_loop = current_keeper_epoch == handler.epoch;
            end_of_loop = current_keeper_epoch == current_epoch;
        }

        // Emits metrics for the NCN state
        // This includes validators info, epoch info, ticket states and more
        if emit_metrics {
            info!("\n\nB. Emit NCN Metrics - {}\n", current_keeper_epoch);
            let result = emit_ncn_metrics(handler, start_of_loop, &cluster_name).await;

            check_and_timeout_error(
                "Emit NCN Metrics".to_string(),
                &result,
                error_timeout_ms,
                state.epoch,
                &cluster_name,
            )
            .await;
        }

        // Before any work can be done, if there are any outstanding vaults
        // that need to be registered, this will do it. Since vaults are registered
        // with the Global Vault Registry, timing does not matter
        if run_operations {
            info!("\n\n-1. Register Vaults - {}\n", current_keeper_epoch);
            let result = crank_register_vaults(handler).await;

            if check_and_timeout_error(
                "Register Vaults".to_string(),
                &result,
                error_timeout_ms,
                state.epoch,
                &cluster_name,
            )
            .await
            {
                continue;
            }
        }

        // Fetches the current state of the keeper, which holds the Epoch State
        // and other helpful information for the keeper to function
        {
            info!("\n\n0. Fetch Keeper State - {}\n", current_keeper_epoch);
            if state.epoch != current_keeper_epoch {
                let result = state.fetch(handler, current_keeper_epoch).await;

                if check_and_timeout_error(
                    "Update Keeper State".to_string(),
                    &result,
                    error_timeout_ms,
                    state.epoch,
                    &cluster_name,
                )
                .await
                {
                    continue;
                }
            }
        }

        // Updates the Epoch State - pulls from the Epoch State account from on chain
        // and further updates the keeper state
        {
            info!("\n\n1. Update Epoch State - {}\n", current_keeper_epoch);
            let result = state.update_epoch_state(handler).await;

            if check_and_timeout_error(
                "Update Epoch State".to_string(),
                &result,
                error_timeout_ms,
                state.epoch,
                &cluster_name,
            )
            .await
            {
                continue;
            }
        }

        // If there is no state found for the given epoch, this will create it, or
        // detect if its already been closed. Then the epoch will progress to the next
        if run_operations {
            info!(
                "\n\n2. Create or Complete State - {}\n",
                current_keeper_epoch
            );

            // If complete, reset loop
            if state.is_epoch_completed {
                info!("Epoch {} is complete", state.epoch);
                continue;
            }

            // Else, if no epoch state, create it
            if state.epoch_state.is_none() {
                let result = create_epoch_state(handler, state.epoch).await;

                check_and_timeout_error(
                    "Create Epoch State".to_string(),
                    &result,
                    error_timeout_ms,
                    state.epoch,
                    &cluster_name,
                )
                .await;

                // Go back either way
                continue;
            }
        }

        // Calls the migrate TDA Merkle Root
        if run_migration {
            info!(
                "\n\nC. Migrate TDA Merkle Root Upload Authorities - {}\n",
                current_keeper_epoch
            );

            // If complete, reset loop
            if state.is_epoch_completed {
                continue;
            }

            let result =
                migrate_tda_merkle_root_upload_authorities(handler, current_keeper_epoch).await;

            check_and_timeout_error(
                "Migrate TDA Merkle Root Upload Authorities".to_string(),
                &result,
                error_timeout_ms,
                state.epoch,
                &cluster_name,
            )
            .await;
        }

        // This is where the real work is done. Depending on the state, the keeper will crank through
        // whatever is needed to be done for the given epoch.
        if run_operations {
            let current_state = state.current_state().expect("cannot get current state");
            info!(
                "\n\n3. Crank State [{:?}] - {}\n",
                current_state, current_keeper_epoch
            );

            let result = match current_state {
                State::SetWeight => crank_set_weight(handler, state.epoch).await,
                State::Snapshot => crank_snapshot(handler, state.epoch).await,
                State::Vote => crank_vote(handler, state.epoch, test_vote).await,
                State::PostVoteCooldown => crank_post_vote_cooldown(handler, state.epoch).await,
                State::Distribute => crank_distribute(handler, state.epoch).await,
                State::Close => crank_close_epoch_accounts(handler, state.epoch).await,
            };

            if check_and_timeout_error(
                format!("Crank State: {:?}", current_state),
                &result,
                error_timeout_ms,
                state.epoch,
                &cluster_name,
            )
            .await
            {
                continue;
            }
        }

        // Emits metrics for the Epoch State
        if emit_metrics {
            info!("\n\nD. Emit Epoch Metrics - {}\n", current_keeper_epoch);
            let result = emit_epoch_metrics(handler, state.epoch, &cluster_name).await;

            check_and_timeout_error(
                "Emit NCN Metrics".to_string(),
                &result,
                error_timeout_ms,
                state.epoch,
                &cluster_name,
            )
            .await;
        }

        // Detects a stall in the keeper. More specifically in the Epoch State.
        // For example:
        // Waiting for voting to finish
        // Not enough rewards to distribute
        {
            info!("\n\nE. Detect Stall - {}\n", current_keeper_epoch);

            let result = state.detect_stall(handler).await;

            if check_and_timeout_error(
                "Detect Stall".to_string(),
                &result,
                error_timeout_ms,
                state.epoch,
                &cluster_name,
            )
            .await
            {
                continue;
            }

            epoch_stall = !run_operations || result.unwrap();

            emit_heartbeat(
                tick,
                run_operations,
                emit_metrics,
                run_migration,
                &cluster_name,
            )
            .await;

            if epoch_stall {
                info!("\n\nSTALL DETECTED FOR {}\n\n", current_keeper_epoch);
            }
        }

        // Times out the keeper - this is the main loop timeout
        if end_of_loop && epoch_stall {
            info!("\n\nF. Timeout - {}\n", current_keeper_epoch);

            timeout_keeper(loop_timeout_ms).await;
            tick += 1;
        }
    }
}
