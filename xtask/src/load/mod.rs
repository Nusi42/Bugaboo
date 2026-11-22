//! `cargo xtask load --people 10 [--minutes N | --long [LENGTH]] [--database sqlite|postgres]`: ten
//! people work on the same issues through the API at speed, and the invariants of architecture §11
//! are checked afterwards (TEST-CONCURRENT). `--long 2h` is the long run, made by hand at the exit
//! of a milestone on a quiet computer.
//!
//! xtask builds the programs, starts the database of the run and runs the driver, a Node program
//! (`tools/dev/load.json` names it). The driver starts the server of the load test on the port
//! `loadTestServer` of `tools/dev/ports.json`, lets the people work and writes their book to a
//! file. When the server has stopped, xtask runs `bb-load-check` on that book and the database of
//! the run: the invariants are written there once, as they are for the test of the server. Then it
//! restores the backup the administrator made at the middle of the run into a second database with
//! `bugaboo restore` and runs `bb-load-check --restored` on that copy (architecture §11, "The long
//! run"). Without `--database` the run is made on both.

mod layout;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context as _;
use bb_core::data_folder::{BACKUPS_FOLDER, KEYS_FOLDER, PRIVACY_FOLDER};
use bb_dev_words::database::{DatabaseKind, parse as parse_database};

use crate::db::{self, SERVER_DATABASE_VARIABLE};
use crate::output;
use crate::repo::Repo;
use crate::steps::{self, Profile};

use layout::Layout;

/// The command line of `cargo xtask load`.
#[derive(clap::Args)]
pub(crate) struct Arguments {
    /// How many people work at the same time (default: tools/dev/load.json).
    #[arg(long)]
    people: Option<u32>,
    /// How long they work, in minutes (default: the short run of tools/dev/load.json).
    #[arg(long, conflicts_with = "long")]
    minutes: Option<u32>,
    /// The long run instead of the short one, for a length like `2h` or `90m` (default: the long
    /// run of tools/dev/load.json).
    #[arg(long, num_args = 0..=1, default_missing_value = "", value_parser = parse_length)]
    long: Option<Length>,
    /// Only this database (default: both).
    #[arg(long, value_parser = parse_database)]
    database: Option<DatabaseKind>,
    /// The number the choices of the people are made from, to run a failure again.
    #[arg(long)]
    seed: Option<u64>,
}

impl Arguments {
    /// What the command line asks for.
    pub(crate) fn request(&self) -> Request {
        Request {
            people: self.people,
            minutes: self.minutes,
            long: self.long,
            databases: self.database.into_iter().collect(),
            seed: self.seed,
        }
    }
}

/// What `cargo xtask load` was asked to do.
pub(crate) struct Request {
    pub(crate) people: Option<u32>,
    pub(crate) minutes: Option<u32>,
    /// The long run, and how long it is.
    pub(crate) long: Option<Length>,
    /// The databases to run on; both when empty.
    pub(crate) databases: Vec<DatabaseKind>,
    pub(crate) seed: Option<u64>,
}

/// How long the long run is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Length {
    /// The long run of tools/dev/load.json.
    OfTheLayout,
    /// This many minutes.
    Minutes(u32),
}

/// The length of the long run: `2h`, `90m` or `90` minutes, or nothing for the one of the layout.
fn parse_length(text: &str) -> Result<Length, String> {
    if text.is_empty() {
        return Ok(Length::OfTheLayout);
    }
    let (number, minutes_each) = match text.strip_suffix('h') {
        Some(hours) => (hours, 60),
        None => (text.strip_suffix('m').unwrap_or(text), 1),
    };
    number
        .parse::<u32>()
        .ok()
        .and_then(|count| count.checked_mul(minutes_each))
        .filter(|minutes| *minutes > 0)
        .map(Length::Minutes)
        .ok_or_else(|| format!("{text} is no length: write it like 2h, 90m or 90"))
}

/// The folder of one run: the data folder of the server and the book of the people. It goes away
/// with the run.
struct RunFolder(PathBuf);

impl RunFolder {
    fn create(database: DatabaseKind) -> anyhow::Result<Self> {
        let folder = std::env::temp_dir().join(format!(
            "bugaboo-load-{}-{}",
            database.code(),
            std::process::id()
        ));
        // A folder of an earlier run that stopped half way must not be read as this run's.
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder)
            .with_context(|| format!("could not make {}", folder.display()))?;
        Ok(Self(folder))
    }

    fn data(&self) -> PathBuf {
        self.0.join("data")
    }

    fn book(&self) -> PathBuf {
        self.0.join("book.json")
    }

    /// The data folder the backup of the run is restored into.
    fn restored(&self) -> PathBuf {
        self.0.join("restored")
    }
}

impl Drop for RunFolder {
    fn drop(&mut self) {
        // The folder is a leftover at worst; a failure to remove it must not hide how the run went.
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// The arguments of the driver for one run on one database.
fn driver_arguments(
    layout: &Layout,
    request: &Request,
    database: DatabaseKind,
    folder: &RunFolder,
) -> Vec<String> {
    let people = request.people.unwrap_or(layout.people);
    let minutes = match request.long {
        Some(Length::OfTheLayout) => layout.long_minutes,
        Some(Length::Minutes(minutes)) => minutes,
        None => request.minutes.unwrap_or(layout.minutes),
    };
    let mut arguments = vec![
        layout.driver.clone(),
        "--people".to_owned(),
        people.to_string(),
        "--seconds".to_owned(),
        (u64::from(minutes) * 60).to_string(),
        "--database".to_owned(),
        database.code().to_owned(),
        "--data-dir".to_owned(),
        folder.data().display().to_string(),
        "--book".to_owned(),
        folder.book().display().to_string(),
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
    let mut command = crate::tools::command("node");
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

/// Which database a judge reads: the one of the run, or the copy its backup was restored into.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Judged {
    Run,
    Restored,
}

/// Judges the run: `bb-load-check` reads the book and the database of the run, or with
/// `--restored` the copy the backup of the run was restored into.
fn run_check(
    root: &Path,
    folder: &RunFolder,
    checker: &Path,
    url: Option<&str>,
    judged: Judged,
) -> anyhow::Result<bool> {
    let mut command = crate::tools::command(checker);
    command.arg("--book").arg(folder.book()).current_dir(root);
    let data = match judged {
        Judged::Run => folder.data(),
        Judged::Restored => {
            command.arg("--restored");
            folder.restored()
        }
    };
    match url {
        Some(url) => command.arg("--database-url").arg(url),
        None => command.arg("--data-dir").arg(data),
    };
    Ok(command
        .status()
        .with_context(|| format!("could not start {}", checker.display()))?
        .success())
}

/// The name of the backup the book says was made at the middle of the run.
fn backup_of(folder: &RunFolder) -> anyhow::Result<String> {
    let book: serde_json::Value = serde_json::from_str(&fs::read_to_string(folder.book())?)
        .context("the book of the run is not valid")?;
    book["backup"]["name"]
        .as_str()
        .map(str::to_owned)
        .context("the book of the run names no backup made while the people worked")
}

/// Copies the files of a folder of the data folder of the run (not its sub-folders) into the
/// same folder of the copy: the built-in database, the keys that seal the backup, or the replay
/// list.
fn copy_files(from: &Path, to: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(to).with_context(|| format!("could not make {}", to.display()))?;
    let Ok(entries) = fs::read_dir(from) else {
        return Ok(());
    };
    for entry in entries {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            fs::copy(entry.path(), to.join(entry.file_name()))
                .with_context(|| format!("could not copy {}", entry.path().display()))?;
        }
    }
    Ok(())
}

/// Restores the backup of the run into a second database with `bugaboo restore`, as an
/// administrator does onto the installation of the run: a copy of its data folder (without the
/// files of its sub-folders but the keys and the replay list) and of its database (the file, or
/// the PostgreSQL database `url`).
fn restore(
    root: &Path,
    folder: &RunFolder,
    server: &Path,
    url: Option<&str>,
) -> anyhow::Result<bool> {
    let name = backup_of(folder)?;
    let second = folder.restored();
    // A copy of the installation of the run, as the one an administrator restores onto: its files
    // (the built-in database among them), the keys and the replay list, and its other folders
    // empty.
    copy_files(&folder.data(), &second)?;
    for entry in fs::read_dir(folder.data())? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let part = entry.file_name();
            let to = second.join(&part);
            if part == KEYS_FOLDER || part == PRIVACY_FOLDER {
                copy_files(&entry.path(), &to)?;
            } else {
                fs::create_dir_all(&to)
                    .with_context(|| format!("could not make {}", to.display()))?;
            }
        }
    }
    let mut command = crate::tools::command(server);
    command
        .arg("restore")
        .arg(folder.data().join(BACKUPS_FOLDER).join(name))
        .arg("--data-dir")
        .arg(&second)
        .current_dir(root)
        .env_remove(SERVER_DATABASE_VARIABLE);
    if let Some(url) = url {
        command.env(SERVER_DATABASE_VARIABLE, url);
    }
    Ok(command
        .status()
        .context("could not start the server program to restore the backup")?
        .success())
}

/// What makes the second database of a run once it is over: none on SQLite, whose database is the
/// file of the copy of the data folder.
type SecondDatabase<'s> = &'s dyn Fn() -> anyhow::Result<Option<String>>;

/// The programs a run uses.
struct Programs {
    server: PathBuf,
    checker: PathBuf,
}

/// One run on one database: the people work, then the run is judged. PostgreSQL is the throwaway
/// cluster of `cargo xtask db`, started for the run and stopped afterwards when this run started
/// it.
fn run_on(
    repo: &Repo,
    layout: &Layout,
    request: &Request,
    database: DatabaseKind,
    programs: &Programs,
) -> anyhow::Result<bool> {
    output::info(&format!("load: {} ...", database.code()));
    let folder = RunFolder::create(database)?;
    let arguments = driver_arguments(layout, request, database, &folder);
    let root = repo.root();
    // The run is judged, then its backup is restored into the second database and judged there.
    // `second` makes that database once the run is over and nobody is connected to it any more
    // (none on SQLite: the file of the copy of the data folder).
    let judge = |url: Option<&str>, second: SecondDatabase<'_>| -> anyhow::Result<bool> {
        if !(run_driver(root, layout, &arguments, &programs.server, url)?
            && run_check(root, &folder, &programs.checker, url, Judged::Run)?)
        {
            return Ok(false);
        }
        let restored = second()?;
        let restored = restored.as_deref();
        Ok(restore(root, &folder, &programs.server, restored)?
            && run_check(root, &folder, &programs.checker, restored, Judged::Restored)?)
    };
    match database {
        DatabaseKind::Sqlite => judge(None, &|| Ok(None)),
        DatabaseKind::Postgres => db::with_postgres(repo, |postgres| {
            let url = postgres.empty_database(&layout.database_name)?;
            judge(Some(&url), &|| {
                postgres
                    .copy_of(&layout.database_name, &layout.restored_name)
                    .map(Some)
            })
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
    let Some(server) = built.executable.filter(|_| built.passed) else {
        return Ok(false);
    };
    let judge = steps::run(repo.root(), &[steps::load_check_build(Profile::Debug)])?;
    let Some(checker) = judge.executable.filter(|_| judge.passed) else {
        return Ok(false);
    };
    let programs = Programs { server, checker };
    let databases = if request.databases.is_empty() {
        DatabaseKind::ALL.to_vec()
    } else {
        request.databases.clone()
    };
    let mut all_held = true;
    for database in databases {
        // Every database is tried, so one failure does not hide the other.
        all_held &= run_on(repo, &layout, request, database, &programs)?;
    }
    Ok(all_held)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> Layout {
        Layout::parse(
            r#"{"driver":"driver.ts","databaseVariable":"V","databaseName":"d","people":10,"minutes":2,"longMinutes":120,"restoredName":"r"}"#,
        )
        .unwrap()
    }

    fn folder() -> RunFolder {
        // Not made on disk: the test only reads the names. The drop of a missing folder is harmless.
        RunFolder(PathBuf::from("a-run-folder-that-is-never-made"))
    }

    fn request(people: Option<u32>, minutes: Option<u32>) -> Request {
        Request {
            people,
            minutes,
            long: None,
            databases: Vec::new(),
            seed: None,
        }
    }

    /// The part of the arguments that says where the data and the book go.
    fn places() -> [String; 4] {
        [
            "--data-dir".to_owned(),
            folder().data().display().to_string(),
            "--book".to_owned(),
            folder().book().display().to_string(),
        ]
    }

    #[test]
    fn the_defaults_are_ten_people_for_two_minutes() {
        let arguments = driver_arguments(
            &layout(),
            &request(None, None),
            DatabaseKind::Sqlite,
            &folder(),
        );
        assert_eq!(
            arguments[..7],
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
        assert_eq!(arguments[7..], places());
    }

    #[test]
    fn the_request_wins_over_the_defaults_and_the_seed_is_passed_on() {
        let mut asked = request(Some(4), Some(30));
        asked.seed = Some(7);
        let arguments = driver_arguments(&layout(), &asked, DatabaseKind::Postgres, &folder());
        assert_eq!(arguments[2], "4");
        assert_eq!(arguments[4], "1800");
        assert_eq!(arguments[6], "postgres");
        assert_eq!(arguments[7..11], places());
        assert_eq!(arguments[11..], ["--seed", "7"]);
    }

    #[test]
    fn test_concurrent_the_long_run_is_two_hours_or_the_length_asked_for() {
        let mut asked = request(None, None);
        asked.long = Some(Length::OfTheLayout);
        let arguments = driver_arguments(&layout(), &asked, DatabaseKind::Sqlite, &folder());
        assert_eq!(arguments[4], "7200");
        asked.long = Some(parse_length("90m").unwrap());
        let arguments = driver_arguments(&layout(), &asked, DatabaseKind::Sqlite, &folder());
        assert_eq!(arguments[4], "5400");
    }

    #[test]
    fn a_length_is_read_in_hours_or_minutes() {
        assert_eq!(parse_length("2h"), Ok(Length::Minutes(120)));
        assert_eq!(parse_length("90m"), Ok(Length::Minutes(90)));
        assert_eq!(parse_length("45"), Ok(Length::Minutes(45)));
        assert_eq!(parse_length(""), Ok(Length::OfTheLayout));
        for wrong in ["0h", "two hours", "h", "-1m"] {
            assert!(parse_length(wrong).is_err(), "{wrong}");
        }
    }

    #[test]
    fn the_layout_of_this_repository_is_valid_and_its_driver_exists() {
        let repo = Repo::discover().unwrap();
        let layout = Layout::load(repo.root()).unwrap();
        assert!(repo.root().join(&layout.driver).is_file());
        assert!(layout.minutes < layout.long_minutes);
    }
}
