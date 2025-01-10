use crate::handler::CliHandler;
use anyhow::Result;

pub async fn startup_keeper(handler: &CliHandler) -> Result<()> {
    println!("Hello, world!");

    run_keeper(handler).await
}

pub async fn run_keeper(_handler: &CliHandler) -> Result<()> {
    todo!("Return correct state")
}
