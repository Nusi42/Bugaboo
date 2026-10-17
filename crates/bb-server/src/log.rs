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

/// The log file of a server that runs as a service, once it is open: the job `server-log.rotate`
/// starts a new file of it when it is big (M1-F61).
#[cfg(windows)]
static SERVER_LOG: std::sync::OnceLock<std::sync::Arc<crate::log_file::RotatingFile>> =
    std::sync::OnceLock::new();

/// The log file the job that rotates it asks, if the server logs to a file.
#[cfg(windows)]
pub(crate) fn server_log_file() -> Option<std::sync::Arc<dyn bb_app::server_log::ServerLogFile>> {
    SERVER_LOG.get().map(|file| {
        std::sync::Arc::clone(file) as std::sync::Arc<dyn bb_app::server_log::ServerLogFile>
    })
}

/// A server that logs to its console has no file to rotate.
#[cfg(not(windows))]
pub(crate) fn server_log_file() -> Option<std::sync::Arc<dyn bb_app::server_log::ServerLogFile>> {
    None
}

/// Starts logging into a file, for a server that has no console (a service). Lines are added to
/// the end of the file. When the file cannot be opened the log goes to standard error, which is
/// all there is.
#[cfg(windows)]
pub(crate) fn init_file(level: LogLevel, path: &std::path::Path) {
    let Ok(file) = crate::log_file::RotatingFile::open(path) else {
        init(level);
        return;
    };
    let _ = SERVER_LOG.set(std::sync::Arc::clone(&file));
    let _ = tracing_subscriber::fmt()
        .json()
        .with_max_level(level_of(level))
        .with_current_span(true)
        .with_writer(crate::log_file::RotatingHandle(file))
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
