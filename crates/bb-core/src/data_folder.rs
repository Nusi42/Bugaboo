//! The names of what Bugaboo keeps directly in a data folder that more than one program reads:
//! the server (`bb-server`, `bb-store`) writes them, and the try-out of the install
//! (`cargo xtask try-install`) looks for them on a clean computer.

/// The built-in SQLite database.
pub const DATABASE_FILE: &str = "bugaboo.db";
/// The folder with the key files.
pub const KEYS_FOLDER: &str = "keys";
/// The folder of the backups, also of the one made before an upgrade.
pub const BACKUPS_FOLDER: &str = "backups";
/// The folder of the replay list: what a restore must not undo (ADR-0044, decision 8).
pub const PRIVACY_FOLDER: &str = "privacy";
/// The replay list, in [`PRIVACY_FOLDER`].
pub const REPLAY_LIST_FILE: &str = "replay.list";
/// The setup code of the first start, for a server without a console.
pub const SETUP_CODE_FILE: &str = "setup-code.txt";
/// The version of the program that last started on the data.
pub const VERSION_FILE: &str = "version.txt";
