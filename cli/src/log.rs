use std::io::Write;

use chrono::{SecondsFormat, Utc};
use env_logger::{
    fmt::{Color, Formatter, Style, StyledValue},
    Env,
};
use log::Record;
use solana_sdk::{bs58, instruction::Instruction};
pub fn init_logger() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .format(format_log_message)
        .init();
}

fn format_log_message(buf: &mut Formatter, record: &Record) -> std::io::Result<()> {
    let mut style = buf.style();
    let level = colored_level(&mut style, record.level());

    // RFC3339 UTC so log aggregators can parse the timestamp
    let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);

    writeln!(
        buf,
        "[{} {} {}] {}",
        timestamp,
        level,
        record.target(),
        record.args()
    )
}

fn colored_level(style: &mut Style, level: log::Level) -> StyledValue<'_, &'static str> {
    match level {
        log::Level::Trace => style.set_color(Color::Magenta).value("TRACE"),
        log::Level::Debug => style.set_color(Color::Blue).value("DEBUG"),
        log::Level::Info => style.set_color(Color::Green).value("INFO "),
        log::Level::Warn => style.set_color(Color::Yellow).value("WARN "),
        log::Level::Error => style.set_color(Color::Red).value("ERROR"),
    }
}

pub(crate) fn print_base58_tx(ixs: &[Instruction]) {
    ixs.iter().for_each(|ix| {
        log::info!("------ IX ------");

        log::info!("{}", ix.program_id);

        ix.accounts.iter().for_each(|account| {
            let pubkey = format!("{}", account.pubkey);
            let writable = if account.is_writable { "W" } else { "" };
            let signer = if account.is_signer { "S" } else { "" };

            log::info!("{:<44} {:>2} {:>1}", pubkey, writable, signer);
        });

        let base58_string = bs58::encode(&ix.data).into_string();
        log::info!("{}", base58_string);
    });
}
