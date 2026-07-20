use anyhow::Result;
use clap::Parser;
use clap_markdown::MarkdownOptions;
use dotenv::dotenv;

use jito_tip_router_cli::{args::Args, handler::CliHandler, log::init_logger};
use log::info;

#[tokio::main]
async fn main() {
    dotenv().ok();
    init_logger();

    if let Err(error) = run().await {
        log::error!("{error:#}");
        std::process::exit(1);
    }
}

#[allow(clippy::large_stack_frames)]
async fn run() -> Result<()> {
    let args: Args = Args::parse();

    if args.markdown_help {
        let markdown = clap_markdown::help_markdown_custom::<Args>(
            &MarkdownOptions::new().show_table_of_contents(false),
        );
        println!("---");
        println!("title: CLI");
        println!("category: Jekyll");
        println!("layout: post");
        println!("weight: 1");
        println!("---");
        println!();
        println!("{}", markdown);
        return Ok(());
    }

    // if let ProgramCommand::Keeper { .. } = args.command {
    info!("\n{}", args);
    // }

    let handler = CliHandler::from_args(&args).await?;
    handler.handle(args.command).await?;

    Ok(())
}
