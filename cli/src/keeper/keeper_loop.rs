use std::time::Duration;

use crate::{handler::CliHandler, keeper::keeper_state::KeeperState};
use anyhow::Result;
use tokio::time::sleep;

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
    let mut state = KeeperState::default();

    state.fetch(handler).await.expect("Could not fetch state");

    loop {
        timeout_keeper().await;
    }
}
