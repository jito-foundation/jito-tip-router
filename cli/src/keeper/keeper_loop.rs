use crate::{
    getters::get_current_epoch,
    handler::CliHandler,
    instructions::{
        crank_close_epoch_accounts, crank_distribute, crank_register_vaults, crank_set_weight,
        crank_snapshot, crank_vote, create_epoch_state,
    },
    keeper::keeper_state::KeeperState,
    log::{boring_progress_bar, progress_bar},
};
use anyhow::{Ok, Result};
use jito_tip_router_core::epoch_state::State;
use log::info;

pub async fn progress_epoch(
    handler: &CliHandler,
    is_epoch_completed: bool,
    starting_epoch: u64,
    last_current_epoch: u64,
    keeper_epoch: u64,
    epoch_stall: bool,
) -> Result<u64> {
    let current_epoch = get_current_epoch(handler).await?;

    if current_epoch > last_current_epoch {
        // Automatically go to new epoch
        return Ok(current_epoch);
    }

    if is_epoch_completed || epoch_stall {
        // Reset to starting epoch
        if keeper_epoch == current_epoch {
            return Ok(starting_epoch);
        }

        // Increment keeper epoch
        return Ok(keeper_epoch + 1);
    }

    Ok(keeper_epoch)
}

#[allow(clippy::future_not_send)]
pub async fn check_and_timeout_error<T>(
    title: String,
    result: &Result<T>,
    error_timeout_ms: u64,
) -> bool {
    if let Err(e) = result {
        log::error!("Error: [{}] \n{:?}\n\n", title, e);
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
) -> Result<()> {
    run_keeper(handler, loop_timeout_ms, error_timeout_ms, test_vote).await;

    // Will never reach
    Ok(())
}

#[allow(clippy::large_stack_frames)]
pub async fn run_keeper(
    handler: &CliHandler,
    loop_timeout_ms: u64,
    error_timeout_ms: u64,
    test_vote: bool,
) {
    let mut state: KeeperState = KeeperState::default();
    let mut epoch_stall = false;
    let mut current_epoch = handler.epoch;
    let mut last_current_epoch = get_current_epoch(handler)
        .await
        .expect("Could not get epoch");

    loop {
        {
            info!("-1. Register Vaults");
            let result = crank_register_vaults(handler).await;

            if check_and_timeout_error("Register Vaults".to_string(), &result, error_timeout_ms)
                .await
            {
                continue;
            }
        }

        {
            info!("0. Progress Epoch");
            let starting_epoch = handler.epoch;
            let keeper_epoch = current_epoch;

            let result = progress_epoch(
                handler,
                state.is_epoch_completed,
                starting_epoch,
                last_current_epoch,
                keeper_epoch,
                epoch_stall,
            )
            .await;

            if check_and_timeout_error("Progress Epoch".to_string(), &result, error_timeout_ms)
                .await
            {
                continue;
            }

            current_epoch = result.unwrap();
            last_current_epoch = last_current_epoch.max(current_epoch);
            epoch_stall = false;
        }

        {
            info!("1. Update Keeper State - {}", current_epoch);
            if state.epoch != current_epoch {
                let result = state.fetch(handler, current_epoch).await;

                if check_and_timeout_error(
                    "Update Keeper State".to_string(),
                    &result,
                    error_timeout_ms,
                )
                .await
                {
                    continue;
                }
            }
        }

        {
            info!("2. Update the epoch state - {}", current_epoch);
            let result = state.update_epoch_state(handler).await;

            if check_and_timeout_error("Update Epoch State".to_string(), &result, error_timeout_ms)
                .await
            {
                continue;
            }
        }

        {
            info!("3. Create or Complete State - {}", current_epoch);

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
                )
                .await;

                // Go back either way
                continue;
            }
        }

        {
            let current_state = state.current_state().unwrap();
            info!("5. Crank State [{:?}] - {}", current_state, current_epoch);

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
            )
            .await
            {
                continue;
            }
        }

        {
            info!("5. Detect Stall - {}", current_epoch);

            let result = state.detect_stall(handler).await;

            if check_and_timeout_error("Detect Stall".to_string(), &result, error_timeout_ms).await
            {
                continue;
            }

            epoch_stall = result.unwrap();
        }

        {
            timeout_keeper(loop_timeout_ms).await;
        }
    }
}
