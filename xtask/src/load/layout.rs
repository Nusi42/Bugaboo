//! `tools/dev/load.json`: the one place that names the driver of `cargo xtask load`, the variable
//! that carries the address of the database and the defaults of a run.

use std::fs;
use std::path::Path;

use anyhow::Context as _;
use serde::Deserialize;

/// Where the layout lives.
const LAYOUT_FILE: &str = "tools/dev/load.json";

/// What `tools/dev/load.json` says.
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Layout {
    /// The Node program that runs the people, relative to the repository.
    pub(crate) driver: String,
    /// The variable that carries the address of the PostgreSQL database to the driver.
    pub(crate) database_variable: String,
    /// The name of the PostgreSQL database of a run.
    pub(crate) database_name: String,
    /// How many people work at the same time.
    pub(crate) people: u32,
    /// How long the short run takes, the one of `cargo xtask ci`.
    pub(crate) minutes: u32,
    /// How long the nightly run takes: the nightly job passes it with `--minutes`.
    pub(crate) nightly_minutes: u32,
}

impl Layout {
    pub(crate) fn parse(json: &str) -> anyhow::Result<Self> {
        serde_json::from_str(json).with_context(|| format!("{LAYOUT_FILE} is not valid"))
    }

    pub(crate) fn load(root: &Path) -> anyhow::Result<Self> {
        let path = root.join(LAYOUT_FILE);
        let text = fs::read_to_string(&path)
            .with_context(|| format!("could not read {}", path.display()))?;
        Self::parse(&text)
    }
}
