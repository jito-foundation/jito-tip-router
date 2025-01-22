use crate::{
    handler::CliHandler,
    instructions::{
        crank_close_epoch_accounts, crank_distribute, crank_register_vaults, crank_set_weight,
        crank_setup_router, crank_snapshot, crank_test_vote, crank_upload, create_epoch_state,
    },
    keeper::keeper_state::KeeperState,
    log::{boring_progress_bar, progress_bar},
};
use anyhow::{Ok, Result};
use jito_tip_router_core::epoch_state::State;
use log::info;

pub async fn progress_epoch(
    handler: &CliHandler,
    starting_epoch: u64,
    last_current_epoch: u64,
    keeper_epoch: u64,
    current_loop_done: bool,
) -> Result<u64> {
    let client = handler.rpc_client();
    let current_epoch = client.get_epoch_info().await?.epoch;

    if current_epoch > last_current_epoch {
        // Automatically go to new epoch
        return Ok(current_epoch);
    }

    if current_loop_done {
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
pub async fn check_and_timeout_error<T>(title: String, result: &Result<T>) -> bool {
    if let Err(e) = result {
        log::error!("Error: [{}] \n{:?}\n\n", title, e);
        timeout_error(5000).await;
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
pub async fn startup_keeper(handler: &CliHandler) -> Result<()> {
    run_keeper(handler).await;

    // Will never reach
    Ok(())
}

#[allow(clippy::large_stack_frames)]
pub async fn run_keeper(handler: &CliHandler) {
    let mut state: KeeperState = KeeperState::default();
    let mut current_epoch = handler.epoch;
    let mut last_current_epoch = handler.epoch;

    loop {
        {
            info!("-1. Register Vaults");
            let result = crank_register_vaults(handler).await;

            if check_and_timeout_error("Register Vaults".to_string(), &result).await {
                continue;
            }
        }

        {
            info!("0. Update Keeper State");
            if state.epoch != current_epoch {
                let result = state.fetch(handler, current_epoch).await;

                if check_and_timeout_error("Update Keeper State".to_string(), &result).await {
                    continue;
                }
            }
        }

        {
            info!("1. Update the epoch state");
            let result = state.update_epoch_state(handler).await;

            if check_and_timeout_error("Update Epoch State".to_string(), &result).await {
                continue;
            }
        }

        {
            info!("2. If epoch state DNE, create it");
            if state.epoch_state.is_none() {
                let result = create_epoch_state(handler, state.epoch).await;

                let _ = check_and_timeout_error("Create Epoch State".to_string(), &result).await;

                // Go back either way
                continue;
            }
        }

        {
            info!("3. Progress Epoch");
            let starting_epoch = handler.epoch;
            let keeper_epoch = state.epoch;
            let current_loop_done = state.current_loop_done().unwrap();

            current_epoch = progress_epoch(
                handler,
                starting_epoch,
                last_current_epoch,
                keeper_epoch,
                current_loop_done,
            )
            .await
            .unwrap();

            last_current_epoch = current_epoch;
        }

        {
            let current_state = state.current_state().unwrap();
            info!("4. Crank State: {:?}", current_state);

            let result = match current_state {
                State::SetWeight => crank_set_weight(handler, state.epoch).await,
                State::Snapshot => crank_snapshot(handler, state.epoch).await,
                // State::Vote => crank_vote(handler, state.epoch).await,
                State::Vote => crank_test_vote(handler, state.epoch).await,
                State::SetupRouter => crank_setup_router(handler, state.epoch).await,
                State::Upload => crank_upload(handler, state.epoch).await,
                State::Distribute => crank_distribute(handler, state.epoch).await,
                State::Close => crank_close_epoch_accounts(handler, state.epoch).await,
            };

            if check_and_timeout_error(format!("Crank State: {:?}", current_state), &result).await {
                continue;
            }
        }

        {
            timeout_keeper(10_000).await;
        }
    }
}
