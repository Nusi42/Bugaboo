//! `cargo xtask dataset sample [--seed N] [--database sqlite|postgres] [--out <folder>] [--check]`:
//! makes the sample organization (architecture §11), or checks that it is made the same every
//! time (TEST-SAMPLE-DATA).
//!
//! xtask runs the `bb-sample` program as a process, so xtask itself depends on no product crate.
//! The program never reads the clock: this file hands it the start moment (now; a fixed moment
//! for `--check`, so the runs are comparable).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, bail};

use crate::checks::test_sample_data::{self, NAMES_FILE, NameList, Run};
use crate::db;
use crate::output;
use crate::repo::Repo;
use crate::steps::{self, Profile};

/// Where the sample is made when `--out` is not given.
const SQLITE_OUT: &str = "target/sample";
const POSTGRES_OUT: &str = "target/sample-postgres";
/// Where `cargo xtask dev --sample` keeps its sample.
const DEV_OUT: &str = "target/dev-sample";
/// Where `--check` makes its runs, one folder each.
const CHECK_OUT: &str = "target/sample-check";
/// The data folder inside an output folder (`bb-sample` has the same name; the program says where
/// it put it when it is done).
const DATA_FOLDER: &str = "data";
/// The moment every run of `--check` ends at: 2027-01-15 08:00:00 UTC, an arbitrary fixed one.
const CHECK_START: i64 = 1_800_000_000;
/// The environment variable of the program that holds the address of its PostgreSQL database
/// (a test keeps it in step with `bb-sample`).
const DATABASE_URL_VARIABLE: &str = "BB_SAMPLE_DATABASE_URL";
/// What `--check` makes of the tables of a PostgreSQL run, so the scan for secrets covers them.
const POSTGRES_DUMP_FILE: &str = "postgres.sql";
/// The runs of `--check`: twice on each database.
const CHECK_RUNS: [(&str, DatabaseKind); 4] = [
    ("sqlite-1", DatabaseKind::Sqlite),
    ("sqlite-2", DatabaseKind::Sqlite),
    ("postgres-1", DatabaseKind::Postgres),
    ("postgres-2", DatabaseKind::Postgres),
];

/// Which database the sample is made on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DatabaseKind {
    /// The built-in database: a file in the data folder.
    Sqlite,
    /// The throwaway PostgreSQL (`cargo xtask db`).
    Postgres,
}

impl DatabaseKind {
    const fn name(self) -> &'static str {
        match self {
            Self::Sqlite => "sqlite",
            Self::Postgres => "postgres",
        }
    }
}

/// What `cargo xtask dataset sample` was asked to do.
pub(crate) struct Request {
    pub(crate) seed: Option<u64>,
    pub(crate) database: DatabaseKind,
    pub(crate) out: Option<PathBuf>,
    pub(crate) check: bool,
}

/// Makes the sample, or checks it; the result says whether everything went well.
pub(crate) fn run(repo: &Repo, request: &Request) -> anyhow::Result<bool> {
    let Some(program) = build_program(repo.root())? else {
        return Ok(false);
    };
    if request.check {
        return check(repo, &program, request.seed);
    }
    let out = match (&request.out, request.database) {
        (Some(out), _) => std::path::absolute(out)
            .with_context(|| format!("{} is not a usable folder", out.display()))?,
        (None, DatabaseKind::Sqlite) => repo.root().join(SQLITE_OUT),
        (None, DatabaseKind::Postgres) => repo.root().join(POSTGRES_OUT),
    };
    let make = |url: Option<&str>| {
        let mut command = program_command(
            &program,
            repo.root(),
            &Arguments {
                seed: request.seed,
                start: now()?,
                database: request.database,
                out: &out,
            },
            url,
        );
        let status = command
            .status()
            .with_context(|| format!("could not start {}", program.display()))?;
        Ok::<bool, anyhow::Error>(status.success())
    };
    match request.database {
        DatabaseKind::Sqlite => make(None),
        DatabaseKind::Postgres => db::with_postgres(repo, |postgres| {
            make(Some(&postgres.empty_database("bbsample")?))
        }),
    }
}

/// Makes the sample on SQLite for `cargo xtask dev --sample`, new every time, and returns its data
/// folder. `None` when the program did not build or did not finish.
pub(crate) fn make_for_dev(root: &Path) -> anyhow::Result<Option<PathBuf>> {
    let Some(program) = build_program(root)? else {
        return Ok(None);
    };
    let out = root.join(DEV_OUT);
    let status = program_command(
        &program,
        root,
        &Arguments {
            seed: None,
            start: now()?,
            database: DatabaseKind::Sqlite,
            out: &out,
        },
        None,
    )
    .status()
    .with_context(|| format!("could not start {}", program.display()))?;
    Ok(status.success().then(|| out.join(DATA_FOLDER)))
}

/// Builds the program and returns where it is; `None` when the build failed (cargo said why).
fn build_program(root: &Path) -> anyhow::Result<Option<PathBuf>> {
    let built = steps::run(root, &[steps::sample_build(Profile::Debug)])?;
    Ok(built.executable.filter(|_| built.passed))
}

/// The moment the sample ends: now, in seconds since 1970.
fn now() -> anyhow::Result<i64> {
    let since = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("the clock of this computer is before 1970")?;
    i64::try_from(since.as_secs()).context("the clock of this computer is too far ahead")
}

/// What the program is told.
struct Arguments<'a> {
    seed: Option<u64>,
    start: i64,
    database: DatabaseKind,
    out: &'a Path,
}

/// The command that runs the program. The address of a PostgreSQL database goes in the
/// environment, never on the command line, where every program on the computer can read it.
fn program_command(
    program: &Path,
    root: &Path,
    arguments: &Arguments<'_>,
    url: Option<&str>,
) -> Command {
    let mut command = Command::new(program);
    command.current_dir(root);
    if let Some(seed) = arguments.seed {
        command.arg("--seed").arg(seed.to_string());
    }
    command
        .arg("--start")
        .arg(arguments.start.to_string())
        .arg("--database")
        .arg(arguments.database.name())
        .arg("--out")
        .arg(arguments.out);
    if let Some(url) = url {
        command.env(DATABASE_URL_VARIABLE, url);
    }
    command
}

/// `--check`: makes the sample twice on each database with one seed, and holds the four dumps
/// against each other and against the rules of TEST-SAMPLE-DATA.
fn check(repo: &Repo, program: &Path, seed: Option<u64>) -> anyhow::Result<bool> {
    let root = repo.root();
    let folder = root.join(CHECK_OUT);
    if folder.exists() {
        fs::remove_dir_all(&folder)
            .with_context(|| format!("could not remove the old {}", folder.display()))?;
    }
    let names = fs::read_to_string(root.join(NAMES_FILE))
        .with_context(|| format!("could not read {NAMES_FILE}"))?;
    let names =
        NameList::parse(&names).map_err(|problem| anyhow::anyhow!("{NAMES_FILE}: {problem}"))?;
    let runs = db::with_postgres(repo, |postgres| {
        let mut runs = Vec::new();
        for (label, database) in CHECK_RUNS {
            let out = folder.join(label);
            let database_name = format!("bbsample_{}", label.replace('-', "_"));
            let url = match database {
                DatabaseKind::Sqlite => None,
                DatabaseKind::Postgres => Some(postgres.empty_database(&database_name)?),
            };
            let made = program_command(
                program,
                root,
                &Arguments {
                    seed,
                    start: CHECK_START,
                    database,
                    out: &out,
                },
                url.as_deref(),
            )
            .stdin(Stdio::null())
            .output()
            .with_context(|| format!("could not start {}", program.display()))?;
            let said = format!(
                "{}{}",
                String::from_utf8_lossy(&made.stdout),
                String::from_utf8_lossy(&made.stderr)
            );
            if !made.status.success() {
                bail!("The sample program failed for the run {label}:\n{said}");
            }
            fs::write(out.join("output.txt"), said)
                .with_context(|| format!("could not write the output of {label}"))?;
            if database == DatabaseKind::Postgres {
                // The tables are in the cluster, not in the folder: they are read like a file,
                // so a secret in them is found too.
                postgres.dump_data(&database_name, &out.join(POSTGRES_DUMP_FILE))?;
            }
            runs.push(Run::read(label, &out)?);
            if database == DatabaseKind::Postgres {
                postgres.drop_database(&database_name)?;
            }
        }
        Ok(runs)
    })?;
    let problems = test_sample_data::problems(&names, &runs);
    for problem in &problems {
        output::info(&format!("TEST-SAMPLE-DATA: {problem}"));
    }
    if problems.is_empty() {
        output::info(&format!(
            "TEST-SAMPLE-DATA: the sample organization is the same in all {} runs, on SQLite and on PostgreSQL, and fictional through and through.",
            runs.len()
        ));
    }
    Ok(problems.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_program_reads_the_same_variable_and_database_names() {
        let library = include_str!("../../crates/bb-sample/src/lib.rs");
        assert!(
            library.contains(&format!(
                "pub const DATABASE_URL_VARIABLE: &str = \"{DATABASE_URL_VARIABLE}\";"
            )),
            "the variable has another name in bb-sample"
        );
        // The program pins the names of its databases; these are the words xtask passes.
        let program = include_str!("../../crates/bb-sample/src/main.rs");
        for kind in [DatabaseKind::Sqlite, DatabaseKind::Postgres] {
            assert!(
                program.contains(&format!("#[value(name = \"{}\")]", kind.name())),
                "bb-sample does not pin the database name {}",
                kind.name()
            );
        }
    }

    #[test]
    fn the_data_folder_has_the_name_the_program_gives_it() {
        let folder = include_str!("../../crates/bb-sample/src/folder.rs");
        assert!(folder.contains(&format!("pub const DATA_FOLDER: &str = \"{DATA_FOLDER}\";")));
    }

    #[test]
    fn the_browser_tests_read_where_the_sample_is_made() {
        let tests = include_str!("../../web/app/e2e/support/sample.ts");
        let folder = include_str!("../../crates/bb-sample/src/folder.rs");
        assert!(tests.contains(&format!("const SAMPLE_FOLDER = \"{SQLITE_OUT}\";")));
        for (name, constant) in [("DATA_FOLDER", "data"), ("SIGN_IN_FILE", "sign-in.txt")] {
            assert!(
                tests.contains(&format!("const {name} = \"{constant}\";")),
                "{name}"
            );
            assert!(
                folder.contains(&format!("pub const {name}: &str = \"{constant}\";")),
                "{name}"
            );
        }
    }

    #[test]
    fn the_address_of_a_database_goes_in_the_environment_and_not_on_the_command_line() {
        let out = Path::new("out");
        let command = program_command(
            Path::new("bb-sample"),
            Path::new("."),
            &Arguments {
                seed: Some(7),
                start: 1_800_000_000,
                database: DatabaseKind::Postgres,
                out,
            },
            Some("postgres://bugaboo:secret@127.0.0.1:1/x"),
        );
        let arguments: Vec<String> = command
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            arguments,
            [
                "--seed",
                "7",
                "--start",
                "1800000000",
                "--database",
                "postgres",
                "--out",
                "out"
            ]
        );
        assert!(!arguments.iter().any(|argument| argument.contains("secret")));
        let environment: Vec<_> = command.get_envs().collect();
        assert_eq!(environment.len(), 1);
        assert_eq!(environment[0].0, DATABASE_URL_VARIABLE);
    }

    #[test]
    fn without_a_seed_the_program_uses_its_own_default() {
        let command = program_command(
            Path::new("bb-sample"),
            Path::new("."),
            &Arguments {
                seed: None,
                start: 1,
                database: DatabaseKind::Sqlite,
                out: Path::new("out"),
            },
            None,
        );
        assert!(command.get_args().all(|argument| argument != "--seed"));
    }

    #[test]
    fn the_check_runs_twice_on_each_database() {
        let sqlite = CHECK_RUNS
            .iter()
            .filter(|(_, database)| *database == DatabaseKind::Sqlite)
            .count();
        assert_eq!((sqlite, CHECK_RUNS.len() - sqlite), (2, 2));
    }
}
