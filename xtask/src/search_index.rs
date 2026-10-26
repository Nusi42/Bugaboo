//! `cargo xtask search-index rebuild [--data-dir <folder>]`: builds the index of the text search
//! again from the stored texts (M5 WP-05, ADR-0016): for a database that has none yet, and after
//! the rule that prepares texts changed. xtask runs the `bugaboo` program as a process, so it
//! does not link the store; the program does the work (`bugaboo search-index rebuild`).

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Context as _;

use crate::dev;

/// Rebuilds the index of the data folder `data_dir` (the data folder of `cargo xtask dev` when
/// none is named). The result says whether it went well.
pub(crate) fn rebuild(root: &Path, data_dir: Option<&Path>) -> anyhow::Result<bool> {
    let data_dir: PathBuf = data_dir.map_or_else(|| dev::data_folder(root), Path::to_path_buf);
    let status = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "-p",
            "bb-server",
            "--",
            "search-index",
            "rebuild",
        ])
        .arg("--data-dir")
        .arg(&data_dir)
        // The data folder is the one the command works on: an address of a database in the
        // environment of the developer must not send it to another one.
        .env_remove("BUGABOO_DATABASE_URL")
        .current_dir(root)
        .status()
        .context("could not run cargo")?;
    Ok(status.success())
}
