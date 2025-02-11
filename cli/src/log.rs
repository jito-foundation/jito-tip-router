#![allow(clippy::integer_division)]
use std::{io::Write, time::Duration};

use chrono::Local;
use env_logger::{
    fmt::{Color, Formatter, Style, StyledValue},
    Env,
};
use log::Record;
use tokio::time::{sleep, Instant};
pub fn init_logger() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .format(format_log_message)
        .init();
}

fn format_log_message(buf: &mut Formatter, record: &Record) -> std::io::Result<()> {
    let mut style = buf.style();
    let level = colored_level(&mut style, record.level());

    let _timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");

    writeln!(
        buf,
        // "[{} {} {}] {}",
        "[{} {}] {}",
        // timestamp,
        level,
        record.target(),
        record.args()
    )
}

fn colored_level(style: &mut Style, level: log::Level) -> StyledValue<&'static str> {
    match level {
        log::Level::Trace => style.set_color(Color::Magenta).value("TRACE"),
        log::Level::Debug => style.set_color(Color::Blue).value("DEBUG"),
        log::Level::Info => style.set_color(Color::Green).value("INFO "),
        log::Level::Warn => style.set_color(Color::Yellow).value("WARN "),
        log::Level::Error => style.set_color(Color::Red).value("ERROR"),
    }
}

pub async fn boring_progress_bar(duration_ms: u64) {
    let start = Instant::now();
    let duration = Duration::from_millis(duration_ms);
    let clock_faces = [
        "🕐", "🕑", "🕒", "🕓", "🕔", "🕕", "🕖", "🕗", "🕘", "🕙", "🕚", "🕛",
    ];
    let bar_width = 30; // Standard width since we're using single-width characters now

    print!("\x1B[s");

    loop {
        let elapsed = start.elapsed();
        if elapsed >= duration {
            print!("\x1B[u\x1B[2K");
            break;
        }

        let progress = elapsed.as_millis() as f64 / duration_ms as f64;
        let filled_width = (progress * bar_width as f64) as usize;
        let clock_idx = ((elapsed.as_millis() % 1000) as f64 / 1000.0 * 12.0) as usize % 12;

        let progress_bar = format!(
            "[{}{}]",
            "█".repeat(filled_width),
            "░".repeat(bar_width - filled_width)
        );

        // Calculate remaining time
        let remaining = duration - elapsed;
        let remaining_secs = remaining.as_secs();

        let time_str = if remaining_secs >= 60 {
            let minutes = (remaining_secs / 60).min(99);
            let seconds = remaining_secs % 60;
            format!("{:02}:{:02}", minutes, seconds)
        } else {
            let decaseconds = (remaining.as_millis() % 1000) / 10;
            format!("{:02}:{:02}", remaining_secs, decaseconds)
        };

        print!(
            "\x1B[u\x1B[2K{} {} {} ",
            clock_faces[clock_idx], progress_bar, time_str,
        );
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        sleep(Duration::from_millis(10)).await;
    }

    // Clean up: restore cursor position, clear line, and show cursor
    print!("\x1B[u\x1B[2K\x1B[?25h");
    let _ = std::io::stdout().flush();
}

pub async fn progress_bar(duration_ms: u64) {
    let start = Instant::now();
    let duration = Duration::from_millis(duration_ms);
    let clock_faces = [
        "🕐", "🕑", "🕒", "🕓", "🕔", "🕕", "🕖", "🕗", "🕘", "🕙", "🕚", "🕛",
    ];
    // Reduce bar_width since each fire emoji takes 2 spaces
    let bar_width = 34; // This will effectively be 30 spaces wide due to double-width emojis

    print!("\x1B[s");

    loop {
        let elapsed = start.elapsed();
        if elapsed >= duration {
            print!("\x1B[u\x1B[2K");
            break;
        }

        let progress = elapsed.as_millis() as f64 / duration_ms as f64;
        let dino_position = ((1.0 - progress) * (bar_width - 2) as f64) as usize;

        let clock_idx = ((elapsed.as_millis() % 1000) as f64 / 1000.0 * 12.0) as usize % 12;

        let mut progress_bar = String::with_capacity(bar_width + 2);
        progress_bar.push('[');
        progress_bar.push_str("🏝️");

        // Add dots up to dino position
        progress_bar.push_str(&" ".repeat(dino_position));

        // Add dino
        progress_bar.push('🦕');

        // Add fire (each 🔥 counts as 2 spaces)
        if dino_position % 2 != 0 {
            progress_bar.push(' ');
        }
        progress_bar.push_str(&"🔥".repeat((bar_width - 2 - dino_position) / 2));
        progress_bar.push('🌋');
        progress_bar.push(']');

        // Calculate remaining time
        let remaining = duration - elapsed;
        let remaining_secs = remaining.as_secs();

        let time_str = if remaining_secs >= 60 {
            let minutes = (remaining_secs / 60).min(99);
            let seconds = remaining_secs % 60;
            format!("{:02}:{:02}", minutes, seconds)
        } else {
            let decaseconds = (remaining.as_millis() % 1000) / 10;
            format!("{:02}:{:02}", remaining_secs, decaseconds)
        };

        print!(
            "\x1B[u\x1B[2K{} {} {} ",
            clock_faces[clock_idx], progress_bar, time_str,
        );
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        sleep(Duration::from_millis(10)).await;
    }

    // Clean up: restore cursor position, clear line, and show cursor
    print!("\x1B[u\x1B[2K\x1B[?25h");
    let _ = std::io::stdout().flush();
}
