use crate::handler::CliHandler;
use anyhow::{anyhow, Ok, Result};

pub async fn startup_keeper(handler: &CliHandler) -> Result<()> {
    println!("Hello, world!");

    run_keeper(handler).await
}

pub async fn run_keeper(handler: &CliHandler) -> Result<()> {
    println!("Hello, world!");

    Ok(())
}
