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
use serde::Deserialize;
use serde_json::{Value, json};

use crate::bench_budgets::{BUDGETS_PATH, Budgets, Measured, over};
use crate::dataset::{in_folder, load_layout};
use crate::dataset_large::dataset_folder;
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

/// The day of `seconds` since 1970 as `YYYY-MM-DD` (the civil date of the proleptic Gregorian
/// calendar, in UTC).
fn day_of(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400) + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = if month_part < 10 {
        month_part + 3
    } else {
        month_part - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

pub(crate) fn today() -> anyhow::Result<String> {
    let seconds = crate::dataset::now()?;
    Ok(day_of(seconds))
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
                run_driver(root, &server, &folder, database, &result_file, None)?
            }
            DatabaseKind::Postgres => db::with_postgres(repo, |postgres| {
                let url = postgres.url_of(&layout.large.database)?;
                run_driver(root, &server, &folder, database, &result_file, Some(&url))
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
    let file = bench_folder.join(format!("{}.json", today()?));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_day_is_written_as_year_month_day() {
        assert_eq!(day_of(0), "1970-01-01");
        assert_eq!(day_of(1_800_000_000), "2027-01-15");
        assert_eq!(day_of(951_782_400), "2000-02-29");
    }
}
