//! Generate IDL type information for SwitchboardQuote
//!
//! Run with:
//! ```bash
//! cargo run --example generate_idl --features anchor,idl-build
//! ```

use switchboard_on_demand::on_demand::oracle_quote::quote_account::SwitchboardQuote;

#[cfg(feature = "idl-build")]
fn main() {
    use anchor_lang::IdlBuild;

    println!("Generating IDL for SwitchboardQuote...\n");

    // Generate the IDL type
    match SwitchboardQuote::create_type() {
        Some(idl_type) => {
            println!("IDL Type Definition:");
            println!("{:#?}", idl_type);
        }
        None => {
            println!("No IDL type generated (returned None)");
        }
    }
}

#[cfg(not(feature = "idl-build"))]
fn main() {
    eprintln!("Error: Must build with --features anchor,idl-build");
    eprintln!("\nRun with:");
    eprintln!("  cargo run --example generate_idl --features anchor,idl-build");
    std::process::exit(1);
}
