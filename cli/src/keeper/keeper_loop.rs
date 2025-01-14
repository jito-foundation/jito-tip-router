use crate::{
    handler::CliHandler,
    instructions::{
        crank_distribute, crank_register_vaults, crank_set_weight, crank_setup_router,
        crank_snapshot, crank_test_vote, crank_upload, create_epoch_state,
    },
    keeper::keeper_state::KeeperState,
    log::progress_bar,
};
use anyhow::Result;
use jito_tip_router_core::epoch_state::State;
use log::info;

pub async fn wait_for_epoch(handler: &CliHandler, target_epoch: u64) {
    let client = handler.rpc_client();

    loop {
        let result = client.get_epoch_info().await.map_err(Into::into);

        if check_and_timeout_error("Waiting for epoch".to_string(), &result).await {
            continue;
        }

        let current_epoch = result.unwrap().epoch;
        if current_epoch >= target_epoch {
            break;
        }

        info!("Waiting for epoch {}/{}", current_epoch, target_epoch);
        timeout_keeper(1000 * 60 * 15).await;
    }
}

#[allow(clippy::future_not_send)]
pub async fn check_and_timeout_error<T>(title: String, result: &Result<T>) -> bool {
    if let Err(e) = result {
        log::error!("Error: [{}] \n{:?}\n\n", title, e);
        timeout_keeper(5000).await;
        true
    } else {
        false
    }
}

pub async fn timeout_keeper(duration_ms: u64) {
    // boring_progress_bar(duration_ms).await;
    progress_bar(duration_ms).await;
}

pub async fn startup_keeper(handler: &CliHandler) -> Result<()> {
    run_keeper(handler).await;

    // Will never reach
    Ok(())
}

#[allow(clippy::large_stack_frames)]
pub async fn run_keeper(handler: &CliHandler) {
    let mut state: KeeperState = KeeperState::default();
    let mut current_epoch = handler.epoch;

    loop {
        {
            info!("-3. Start snapshot");
            // wait_for_epoch(handler, current_epoch).await;
        }

        {
            info!("-2. Register Vaults");
            let result = crank_register_vaults(handler).await;

            if check_and_timeout_error("Register Vaults".to_string(), &result).await {
                continue;
            }
        }

        {
            info!("-1. Wait for epoch");
            wait_for_epoch(handler, current_epoch).await;
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
            let current_state = state.current_state().unwrap();
            info!("3. Crank State: {:?}", current_state);

            let result = match current_state {
                State::SetWeight => crank_set_weight(handler, state.epoch).await,
                State::Snapshot => crank_snapshot(handler, state.epoch).await,
                // State::Vote => crank_vote(handler, state.epoch).await,
                State::Vote => crank_test_vote(handler, state.epoch).await,
                State::SetupRouter => crank_setup_router(handler, state.epoch).await,
                State::Upload => crank_upload(handler, state.epoch).await,
                State::Distribute => crank_distribute(handler, state.epoch).await,
                State::Done => {
                    info!("Epoch Complete");
                    current_epoch += 1;
                    Ok(())
                }
            };

            if check_and_timeout_error(format!("Managing State: {:?}", current_state), &result)
                .await
            {
                continue;
            }
        }

        {
            timeout_keeper(10_000).await;
        }
    }
}
