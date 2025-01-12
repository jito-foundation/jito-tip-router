use std::time::Duration;

use crate::{
    handler::CliHandler,
    instructions::{
        crank_distribute, crank_set_weight, crank_setup_router, crank_snapshot, crank_upload,
        crank_vote, create_epoch_state,
    },
    keeper::keeper_state::KeeperState,
};
use anyhow::Result;
use jito_tip_router_core::epoch_state::State;
use log::info;
use tokio::time::sleep;

pub async fn wait_for_epoch(handler: &mut CliHandler, target_epoch: u64) {
    let client = handler.rpc_client();

    loop {
        info!("Waiting for epoch {}", target_epoch);

        let result = client.get_epoch_info().await;

        if check_and_timeout_error("Waiting for epoch", &result).await {
            continue;
        } else if result.unwrap().epoch >= target_epoch {
            break;
        }

        info!("Sleeping for 15 minutes");
        sleep(Duration::from_secs(60 * 15)).await;
    }
}

pub async fn check_and_timeout_error<T, E>(title: &str, result: &Result<T, E>) -> bool
where
    E: std::fmt::Debug,
{
    if let Err(e) = result {
        log::error!("Error: [{}] \n{:?}\n\n", title, e);
        timeout_keeper().await;
        true
    } else {
        false
    }
}
pub async fn timeout_keeper() {
    log::info!("Timeout keeper");
    sleep(Duration::from_secs(1)).await;
}

pub async fn startup_keeper(handler: &mut CliHandler) -> Result<()> {
    run_keeper(handler).await;

    // Will never reach
    Ok(())
}

pub async fn run_keeper(handler: &mut CliHandler) {
    let mut state: KeeperState = KeeperState::default();
    let mut current_epoch = handler.epoch;

    loop {
        // -2. TODO find and register vaults if needed

        // -1. Wait for epoch
        wait_for_epoch(handler, current_epoch).await;

        // 0. Update Keeper State
        if state.epoch != current_epoch {
            let result = state.fetch(handler, current_epoch).await;

            if check_and_timeout_error("Update Keeper State", &result).await {
                continue;
            }
        }

        // 1. Update the epoch state
        {
            let result = state.update_epoch_state(handler).await;

            if check_and_timeout_error("Update Epoch State", &result).await {
                continue;
            }
        }

        // 2. If epoch state DNE, create it
        if state.epoch_state.is_none() {
            let result = create_epoch_state(handler, state.epoch).await;

            let _ = check_and_timeout_error("Create Epoch State", &result).await;

            // Go back either way
            continue;
        }

        // 3. Check state
        {
            let result = match state.current_state().unwrap() {
                State::SetWeight => crank_set_weight(handler, state.epoch).await,
                State::Snapshot => crank_snapshot(handler, state.epoch).await,
                State::Vote => crank_vote(handler, state.epoch).await,
                State::SetupRouter => crank_setup_router(handler, state.epoch).await,
                State::Upload => crank_upload(handler, state.epoch).await,
                State::Distribute => crank_distribute(handler, state.epoch).await,
                State::Done => {
                    info!("Epoch Complete");
                    current_epoch += 1;
                    Ok(())
                }
            };

            if check_and_timeout_error("Managing State", &result).await {
                continue;
            }
        }

        // END. Timeout keeper
        {
            timeout_keeper().await;
        }
    }
}
