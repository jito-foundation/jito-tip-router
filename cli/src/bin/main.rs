use clap::Parser;
use jito_tip_router_cli::cli_args::Args;

fn main() {
    let args = Args::parse();
    println!("Hello!");
}
