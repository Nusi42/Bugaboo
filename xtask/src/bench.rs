//! `cargo xtask bench [--database sqlite|postgres]`: runs the scenarios of
//! `web/tooling/bench/scenarios.ts` against a release server on the large dataset, on both
//! databases unless one is named, and judges the numbers against `tools/checks/perf-bench.toml`
//! (PERF-BENCH). The numbers of a run go to `.work/bench/<date>.json`.
//!
//! The driver is a Node program (like the one of `cargo xtask load`): it starts the server on the
//! dataset of one database and writes the numbers of that database. xtask starts the database of
//! the run, judges the numbers and keeps them.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Context as _;
use bb_dev_words::database::DatabaseKind;
use jiff::Timestamp;
use jiff::tz::TimeZone;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::bench_budgets::{BUDGETS_PATH, Budgets, Measured, over};
use crate::dataset::{in_folder, load_layout};
use crate::dataset_large::{dataset_folder, index_is_current, mark_index_current};
use crate::db;
use crate::output;
use crate::repo::Repo;
use crate::source::read_text;
use crate::steps::{self, Profile};

/// The driver, relative to the repository.
const DRIVER: &str = "web/tooling/bench/main.ts";
/// The variable that carries the address of the PostgreSQL database to the driver.
const DATABASE_VARIABLE: &str = "BB_BENCH_DATABASE_URL";

/// What the driver writes for one database.
#[derive(Debug, Deserialize)]
struct DriverResult {
    build: String,
    scenarios: BTreeMap<String, Measured>,
}

/// Today as `YYYY-MM-DD`, in UTC.
pub(crate) fn today() -> String {
    Timestamp::now().to_zoned(TimeZone::UTC).date().to_string()
}

/// Builds the index of the text search of a dataset whose marker is older than the rule that
/// prepares a text (`bugaboo search-index rebuild`; see `dataset_large`). A dataset made by
/// `dataset large` has it and its marker already. The program does not migrate: a dataset made for
/// an older schema is refused with a sentence that says so, and made again by `dataset large`.
fn ensure_text_index(server: &Path, folder: &Path, url: Option<&str>) -> anyhow::Result<bool> {
    if index_is_current(folder) {
        return Ok(true);
    }
    let mut command = Command::new(server);
    command
        .args(["search-index", "rebuild", "--data-dir"])
        .arg(folder.join("data"))
        .env_remove("BUGABOO_DATABASE_URL");
    if let Some(url) = url {
        command.env("BUGABOO_DATABASE_URL", url);
    }
    let built = command
        .status()
        .context("could not start the server program to build the text index")?
        .success();
    if built {
        mark_index_current(folder)?;
    }
    Ok(built)
}

fn run_driver(
    root: &Path,
    server: &Path,
    folder: &Path,
    database: DatabaseKind,
    result: &Path,
    url: Option<&str>,
) -> anyhow::Result<bool> {
    let mut command = Command::new("node");
    command
        .arg(DRIVER)
        .args(["--folder"])
        .arg(folder)
        .args(["--database", database.code(), "--result"])
        .arg(result)
        .current_dir(root)
        .env("BB_SERVER_BIN", server);
    if let Some(url) = url {
        command.env(DATABASE_VARIABLE, url);
    }
    Ok(command
        .status()
        .context("could not start node; run `cargo xtask setup`")?
        .success())
}

/// Prints the numbers of a database next to their budgets.
fn table(budgets: &Budgets, database: DatabaseKind, result: &DriverResult) {
    output::info(&format!(
        "bench on {} ({} build): scenario, p50, p95, p99 in ms, budget",
        database.code(),
        result.build
    ));
    for (name, measured) in &result.scenarios {
        output::info(&format!(
            "  {name}: p50 {:.1}, p95 {:.1}, p99 {:.1}; budget {} ms ({})",
            measured.p50,
            measured.p95,
            measured.p99,
            budgets.p95_ms(&measured.budget).unwrap_or_default(),
            measured.budget
        ));
    }
}

pub(crate) fn run(repo: &Repo, databases: &[DatabaseKind]) -> anyhow::Result<bool> {
    let _lock = crate::nightly::FullRun::take(repo)?;
    let root = repo.root();
    let layout = load_layout(root)?;
    let budgets = Budgets::parse(&read_text(repo, BUDGETS_PATH)?)?;
    let mut plan = vec![steps::server_build(Profile::Release)];
    if !root
        .join(steps::WEB_APP)
        .join("dist")
        .join("index.html")
        .is_file()
    {
        plan.push(steps::web_build(root));
    }
    let built = steps::run(root, &plan)?;
    let Some(server) = built.executable.filter(|_| built.passed) else {
        return Ok(false);
    };
    let databases = if databases.is_empty() {
        DatabaseKind::ALL.to_vec()
    } else {
        databases.to_vec()
    };
    let bench_folder = in_folder(root, &layout.large.bench_out);
    fs::create_dir_all(&bench_folder)?;
    let mut all_held = true;
    let mut kept: BTreeMap<String, Value> = BTreeMap::new();
    for database in databases {
        let folder = dataset_folder(repo, &layout, database);
        if !folder.join("data").is_dir() {
            output::info(&format!(
                "bench: there is no dataset for {}: run `cargo xtask dataset large --database {}`.",
                database.code(),
                database.code()
            ));
            all_held = false;
            continue;
        }
        let result_file: PathBuf = bench_folder.join(format!("run-{}.json", database.code()));
        let started = match database {
            DatabaseKind::Sqlite => {
                ensure_text_index(&server, &folder, None)?
                    && run_driver(root, &server, &folder, database, &result_file, None)?
            }
            DatabaseKind::Postgres => db::with_postgres(repo, |postgres| {
                let url = postgres.url_of(&layout.large.database)?;
                Ok(ensure_text_index(&server, &folder, Some(&url))?
                    && run_driver(root, &server, &folder, database, &result_file, Some(&url))?)
            })?,
        };
        if !started {
            all_held = false;
            continue;
        }
        let text = fs::read_to_string(&result_file)?;
        fs::remove_file(&result_file)?;
        let result: DriverResult = serde_json::from_str(&text)?;
        table(&budgets, database, &result);
        for sentence in over(&budgets, &result.scenarios) {
            output::info(&format!("PERF-BENCH: {sentence}"));
            all_held = false;
        }
        kept.insert(database.code().to_owned(), serde_json::from_str(&text)?);
    }
    let file = bench_folder.join(format!("{}.json", today()));
    fs::write(
        &file,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&json!({ "databases": kept }))?
        ),
    )?;
    output::info(&format!("bench: the numbers are in {}.", file.display()));
    Ok(all_held)
}
