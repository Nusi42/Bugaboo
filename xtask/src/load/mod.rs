//! `cargo xtask load --people 10 [--minutes N] [--database sqlite|postgres]`: ten people work on
//! the same issues through the API at speed, and the invariants of architecture §11 are checked
//! afterwards (TEST-CONCURRENT).
//!
//! xtask builds the program, starts the database of the run and runs the driver, a Node program
//! (`tools/dev/load.json` names it). The driver starts the server of the load test on the port
//! `loadTestServer` of `tools/dev/ports.json`, lets the people work and checks what must hold; it
//! ends with a failure when something did not. Without `--database` the run is made on both.

mod layout;

use std::path::Path;
use std::process::Command;

use anyhow::Context as _;
use bb_dev_words::database::DatabaseKind;

use crate::db;
use crate::output;
use crate::repo::Repo;
use crate::steps::{self, Profile};

use layout::Layout;

/// What `cargo xtask load` was asked to do.
pub(crate) struct Request {
    pub(crate) people: Option<u32>,
    pub(crate) minutes: Option<u32>,
    /// The long run of the nightly job.
    pub(crate) nightly: bool,
    /// The databases to run on; both when empty.
    pub(crate) databases: Vec<DatabaseKind>,
    pub(crate) seed: Option<u64>,
}

/// The arguments of the driver for one run on one database.
fn driver_arguments(layout: &Layout, request: &Request, database: DatabaseKind) -> Vec<String> {
    let people = request.people.unwrap_or(layout.people);
    let minutes = if request.nightly {
        layout.nightly_minutes
    } else {
        request.minutes.unwrap_or(layout.minutes)
    };
    let mut arguments = vec![
        layout.driver.clone(),
        "--people".to_owned(),
        people.to_string(),
        "--seconds".to_owned(),
        (u64::from(minutes) * 60).to_string(),
        "--database".to_owned(),
        database.code().to_owned(),
    ];
    if let Some(seed) = request.seed {
        arguments.push("--seed".to_owned());
        arguments.push(seed.to_string());
    }
    arguments
}

/// Runs the driver once; the address of the PostgreSQL database goes in the environment, never on
/// the command line (it holds a password).
fn run_driver(
    root: &Path,
    layout: &Layout,
    arguments: &[String],
    program: &Path,
    url: Option<&str>,
) -> anyhow::Result<bool> {
    let mut command = Command::new("node");
    command
        .args(arguments)
        .current_dir(root)
        .env("BB_SERVER_BIN", program);
    if let Some(url) = url {
        command.env(&layout.database_variable, url);
    }
    Ok(command
        .status()
        .context("could not start node; run `cargo xtask setup`")?
        .success())
}

/// One run on one database. PostgreSQL is the throwaway cluster of `cargo xtask db`, started for
/// the run and stopped afterwards when this run started it.
fn run_on(
    repo: &Repo,
    layout: &Layout,
    request: &Request,
    database: DatabaseKind,
    program: &Path,
) -> anyhow::Result<bool> {
    output::info(&format!("load: {} ...", database.code()));
    let arguments = driver_arguments(layout, request, database);
    match database {
        DatabaseKind::Sqlite => run_driver(repo.root(), layout, &arguments, program, None),
        DatabaseKind::Postgres => db::with_postgres(repo, |postgres| {
            let url = postgres.empty_database(&layout.database_name)?;
            run_driver(repo.root(), layout, &arguments, program, Some(&url))
        }),
    }
}

/// Builds what the run needs, then runs on every database asked for; true when all of them held.
pub(crate) fn run(repo: &Repo, request: &Request) -> anyhow::Result<bool> {
    let layout = Layout::load(repo.root())?;
    let mut plan = vec![steps::server_build(Profile::Debug)];
    let app = repo
        .root()
        .join(steps::WEB_APP)
        .join("dist")
        .join("index.html");
    if !app.is_file() {
        plan.push(steps::web_build(repo.root()));
    }
    let built = steps::run(repo.root(), &plan)?;
    let Some(program) = built.executable.filter(|_| built.passed) else {
        return Ok(false);
    };
    let databases = if request.databases.is_empty() {
        DatabaseKind::ALL.to_vec()
    } else {
        request.databases.clone()
    };
    let mut all_held = true;
    for database in databases {
        // Every database is tried, so one failure does not hide the other.
        all_held &= run_on(repo, &layout, request, database, &program)?;
    }
    Ok(all_held)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> Layout {
        Layout::parse(
            r#"{"driver":"driver.ts","databaseVariable":"V","databaseName":"d","people":10,"minutes":2,"nightlyMinutes":30}"#,
        )
        .unwrap()
    }

    fn request(people: Option<u32>, minutes: Option<u32>) -> Request {
        Request {
            people,
            minutes,
            nightly: false,
            databases: Vec::new(),
            seed: None,
        }
    }

    #[test]
    fn the_defaults_are_ten_people_for_two_minutes() {
        let arguments = driver_arguments(&layout(), &request(None, None), DatabaseKind::Sqlite);
        assert_eq!(
            arguments,
            [
                "driver.ts",
                "--people",
                "10",
                "--seconds",
                "120",
                "--database",
                "sqlite"
            ]
        );
    }

    #[test]
    fn the_request_wins_over_the_defaults_and_the_seed_is_passed_on() {
        let mut asked = request(Some(4), Some(30));
        asked.seed = Some(7);
        let arguments = driver_arguments(&layout(), &asked, DatabaseKind::Postgres);
        assert_eq!(arguments[2], "4");
        assert_eq!(arguments[4], "1800");
        assert_eq!(arguments[6], "postgres");
        assert_eq!(arguments[7..], ["--seed", "7"]);
    }

    #[test]
    fn the_nightly_run_is_the_long_one() {
        let mut asked = request(None, None);
        asked.nightly = true;
        let arguments = driver_arguments(&layout(), &asked, DatabaseKind::Sqlite);
        assert_eq!(arguments[4], "1800");
    }

    #[test]
    fn the_layout_of_this_repository_is_valid_and_its_driver_exists() {
        let repo = Repo::discover().unwrap();
        let layout = Layout::load(repo.root()).unwrap();
        assert!(repo.root().join(&layout.driver).is_file());
        assert!(layout.minutes < layout.nightly_minutes);
    }
}
