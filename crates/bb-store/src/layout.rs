//! Where the store keeps its files inside a data folder: written once here, read by the program
//! that runs a server (`bb-server`) and by the developer tool that makes a sample organization
//! (`bb-sample`), so the two cannot disagree about where a database or a key lies.

/// The built-in SQLite database, directly in the data folder.
pub const DATABASE_FILE: &str = "bugaboo.db";
/// The folder with the key files (`MASTER_KEY_FILE` and the others).
pub const KEYS_FOLDER: &str = "keys";
/// The folder for what the security log keeps outside the database.
pub const LOGS_FOLDER: &str = "logs";
/// The file in [`LOGS_FOLDER`] where the security log writes down its end (SEC-15).
pub const SECURITY_LOG_ANCHOR_FILE: &str = "security-head.log";
