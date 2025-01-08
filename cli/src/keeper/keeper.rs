use crate::handler::CliHandler;
use anyhow::Result;
use jito_tip_router_core::state::TipRouterState;

pub async fn startup_keeper(handler: &CliHandler) -> Result<()> {
    println!("Hello, world!");

    run_keeper(handler).await
}

pub async fn run_keeper(handler: &CliHandler) -> Result<()> {
    todo!("Return correct state")
}
