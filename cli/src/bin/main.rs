use anyhow::Result;
use clap::Parser;
use clap_markdown::MarkdownOptions;
use dotenv::dotenv;

use jito_tip_router_cli::{args::Args, handler::CliHandler, log::init_logger};

#[tokio::main]
#[allow(clippy::large_stack_frames)]
async fn main() -> Result<()> {
    dotenv().ok();
    init_logger();

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

    // info!("{}\n", args);

    let handler = CliHandler::from_args(&args).await?;
    handler.handle(args.command).await?;

    Ok(())
}
