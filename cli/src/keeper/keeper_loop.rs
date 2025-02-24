use crate::{
    getters::get_guaranteed_epoch_and_slot,
    handler::CliHandler,
    instructions::{
        crank_close_epoch_accounts, crank_distribute, crank_register_vaults, crank_set_weight,
        crank_snapshot, crank_vote, create_epoch_state, update_all_vaults_in_network,
    },
    keeper::{
        keeper_metrics::{emit_epoch_metrics, emit_error, emit_heartbeat, emit_ncn_metrics},
        keeper_state::KeeperState,
    },
    log::{boring_progress_bar, progress_bar},
};
use anyhow::Result;
use jito_tip_router_core::epoch_state::State;
use log::info;

pub async fn progress_epoch(
    handler: &CliHandler,
    is_epoch_completed: bool,
    starting_epoch: u64,
    last_current_epoch: u64,
    keeper_epoch: u64,
    epoch_stall: bool,
) -> (u64, bool) {
    let (current_epoch, _) = get_guaranteed_epoch_and_slot(handler).await;

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
) -> bool {
    if let Err(e) = result {
        let error = format!("{:?}", e);
        let message = format!("Error: [{}] \n{}\n\n", title, error);

        log::error!("{}", message);
        emit_error(title, error, message, keeper_epoch).await;
        timeout_error(error_timeout_ms).await;
        true
    } else {
        false
    }
}

pub async fn timeout_error(duration_ms: u64) {
    progress_bar(duration_ms).await;
}

pub async fn timeout_keeper(duration_ms: u64) {
    boring_progress_bar(duration_ms).await;
}

#[allow(clippy::large_stack_frames)]
pub async fn startup_keeper(
    handler: &CliHandler,
    loop_timeout_ms: u64,
    error_timeout_ms: u64,
    test_vote: bool,
    all_vault_update: bool,
    emit_metrics: bool,
    metrics_only: bool,
) -> Result<()> {
    let mut state: KeeperState = KeeperState::default();
    let mut epoch_stall = false;
    let mut current_epoch = handler.epoch;
    let mut is_new_epoch = true;
    let mut tick = 0;
    let (mut last_current_epoch, _) = get_guaranteed_epoch_and_slot(handler).await;

    let run_operations = !metrics_only;
    let emit_metrics = emit_metrics || metrics_only;

    loop {
        // If there is a new epoch, this will do a full vault update on *all* vaults
        // created with restaking - this adds some extra redundancy
        if is_new_epoch && all_vault_update && run_operations {
            info!("\n\n-2. Update Vaults - {}\n", current_epoch);
            let result = update_all_vaults_in_network(handler).await;

            if check_and_timeout_error(
                "Update Vaults".to_string(),
                &result,
                error_timeout_ms,
                state.epoch,
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
            info!("\n\nA. Progress Epoch - {}\n", current_epoch);
            let starting_epoch = handler.epoch;
            let keeper_epoch = current_epoch;

            let (result, set_is_new_epoch) = progress_epoch(
                handler,
                state.is_epoch_completed,
                starting_epoch,
                last_current_epoch,
                keeper_epoch,
                epoch_stall,
            )
            .await;

            if current_epoch != result {
                info!("\n\nPROGRESS EPOCH: {} -> {}\n\n", current_epoch, result);
            }

            is_new_epoch = set_is_new_epoch;
            current_epoch = result;
            last_current_epoch = last_current_epoch.max(current_epoch);
            epoch_stall = false;
        }

        // Emits metrics for the NCN state
        // This includes validators info, epoch info, ticket states and more
        if emit_metrics {
            info!("\n\nB. Emit NCN Metrics - {}\n", current_epoch);
            let result = emit_ncn_metrics(handler, tick).await;

            check_and_timeout_error(
                "Emit NCN Metrics".to_string(),
                &result,
                error_timeout_ms,
                state.epoch,
            )
            .await;
        }

        // Before any work can be done, if there are any outstanding vaults
        // that need to be registered, this will do it. Since vaults are registered
        // with the Global Vault Registry, timing does not matter
        if run_operations {
            info!("\n\n-1. Register Vaults - {}\n", current_epoch);
            let result = crank_register_vaults(handler).await;

            if check_and_timeout_error(
                "Register Vaults".to_string(),
                &result,
                error_timeout_ms,
                state.epoch,
            )
            .await
            {
                continue;
            }
        }

        // Fetches the current state of the keeper, which holds the Epoch State
        // and other helpful information for the keeper to function
        {
            info!("\n\n0. Fetch Keeper State - {}\n", current_epoch);
            if state.epoch != current_epoch {
                let result = state.fetch(handler, current_epoch).await;

                if check_and_timeout_error(
                    "Update Keeper State".to_string(),
                    &result,
                    error_timeout_ms,
                    state.epoch,
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
            info!("\n\n1. Update Epoch State - {}\n", current_epoch);
            let result = state.update_epoch_state(handler).await;

            if check_and_timeout_error(
                "Update Epoch State".to_string(),
                &result,
                error_timeout_ms,
                state.epoch,
            )
            .await
            {
                continue;
            }
        }

        // If there is no state found for the given epoch, this will create it, or
        // detect if its already been closed. Then the epoch will progress to the next
        if run_operations {
            info!("\n\n2. Create or Complete State - {}\n", current_epoch);

            // If complete, reset loop
            if state.is_epoch_completed {
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
                )
                .await;

                // Go back either way
                continue;
            }
        }

        // This is where the real work is done. Depending on the state, the keeper will crank through
        // whatever is needed to be done for the given epoch.
        if run_operations {
            let current_state = state.current_state().expect("cannot get current state");
            info!(
                "\n\n3. Crank State [{:?}] - {}\n",
                current_state, current_epoch
            );

            let result = match current_state {
                State::SetWeight => crank_set_weight(handler, state.epoch).await,
                State::Snapshot => crank_snapshot(handler, state.epoch).await,
                State::Vote => crank_vote(handler, state.epoch, test_vote).await,
                State::Distribute => crank_distribute(handler, state.epoch).await,
                State::Close => crank_close_epoch_accounts(handler, state.epoch).await,
            };

            if check_and_timeout_error(
                format!("Crank State: {:?}", current_state),
                &result,
                error_timeout_ms,
                state.epoch,
            )
            .await
            {
                continue;
            }
        }

        // Emits metrics for the Epoch State
        if emit_metrics {
            info!("\n\nC. Emit Epoch Metrics - {}\n", current_epoch);
            let result = emit_epoch_metrics(handler, state.epoch).await;

            check_and_timeout_error(
                "Emit NCN Metrics".to_string(),
                &result,
                error_timeout_ms,
                state.epoch,
            )
            .await;
        }

        // Detects a stall in the keeper. More specifically in the Epoch State.
        // For example:
        // Waiting for voting to finish
        // Not enough rewards to distribute
        {
            info!("\n\nD. Detect Stall - {}\n", current_epoch);

            let result = state.detect_stall(handler).await;

            if check_and_timeout_error(
                "Detect Stall".to_string(),
                &result,
                error_timeout_ms,
                state.epoch,
            )
            .await
            {
                continue;
            }

            epoch_stall = result.unwrap() || metrics_only;

            if epoch_stall {
                info!("\n\nSTALL DETECTED FOR {}\n\n", current_epoch);
            }
        }

        // Times out the keeper - this is the main loop timeout
        {
            info!("\n\nE. Timeout - {}\n", current_epoch);

            timeout_keeper(loop_timeout_ms).await;
            emit_heartbeat(tick, metrics_only).await;
            tick += 1;
        }
    }
}
