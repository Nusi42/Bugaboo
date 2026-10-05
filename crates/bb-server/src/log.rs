//! The server's log: one JSON object per line on standard error (OPS-08).
//!
//! What may go into it is decided where lines are written, not here: IDs only, no names,
//! e-mail addresses or free text, addresses cut short (PRIV-01, DATA-LOGS).

use tracing::Level;

use crate::config::LogLevel;

fn level_of(level: LogLevel) -> Level {
    match level {
        LogLevel::Error => Level::ERROR,
        LogLevel::Warn => Level::WARN,
        LogLevel::Info => Level::INFO,
        LogLevel::Debug => Level::DEBUG,
    }
}

/// Starts logging into a file, for a server that has no console (a service). Lines are added to
/// the end of the file. When the file cannot be opened the log goes to standard error, which is
/// all there is.
#[cfg(windows)]
pub(crate) fn init_file(level: LogLevel, path: &std::path::Path) {
    let Ok(file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    else {
        init(level);
        return;
    };
    let _ = tracing_subscriber::fmt()
        .json()
        .with_max_level(level_of(level))
        .with_current_span(true)
        .with_writer(std::sync::Mutex::new(file))
        .try_init();
}

/// Starts logging at `level`. Calling it twice keeps the first setup.
pub(crate) fn init(level: LogLevel) {
    let level = level_of(level);
    // Fails only when a logger is already installed, which is fine.
    let _ = tracing_subscriber::fmt()
        .json()
        .with_max_level(level)
        .with_current_span(true)
        .with_writer(std::io::stderr)
        .try_init();
}
