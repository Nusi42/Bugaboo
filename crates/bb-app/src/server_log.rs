//! The port of the log file of a server that has no console (M1-F61): a server that runs as a
//! Windows service writes its log to `logs/server.log`, and a job starts a new file when the old
//! one is big. The program that wires the server implements it, because only it holds the file.

/// The log file of the server.
pub trait ServerLogFile: Send + Sync {
    /// Starts a new file when the current one is bigger than `max_bytes`, and keeps `keep_files`
    /// old ones next to it. Returns whether it started a new file.
    ///
    /// # Errors
    /// The error of the file system.
    fn rotate(&self, max_bytes: u64, keep_files: u32) -> std::io::Result<bool>;
}
