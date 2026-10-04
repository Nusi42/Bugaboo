//! The server's log: one JSON object per line on standard error (OPS-08).
//!
//! What may go into it is decided where lines are written, not here: IDs only, no names,
//! e-mail addresses or free text, addresses cut short (PRIV-01, DATA-LOGS).

use tracing::Level;

use crate::config::LogLevel;

/// Starts logging at `level`. Calling it twice keeps the first setup.
pub(crate) fn init(level: LogLevel) {
    let level = match level {
        LogLevel::Error => Level::ERROR,
        LogLevel::Warn => Level::WARN,
        LogLevel::Info => Level::INFO,
        LogLevel::Debug => Level::DEBUG,
    };
    // Fails only when a logger is already installed, which is fine.
    let _ = tracing_subscriber::fmt()
        .json()
        .with_max_level(level)
        .with_current_span(true)
        .with_writer(std::io::stderr)
        .try_init();
}
