use anyhow::Result;
use clap::Parser;
use clap_markdown::MarkdownOptions;
use env_logger::Env;

use jito_tip_router_cli::{args::Args, handler::CliHandler};
use log::info;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

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

    let handler = CliHandler::from_args(&args).await?;

    info!("{}\n", args);

    handler.handle(args.command).await?;

    Ok(())
}
