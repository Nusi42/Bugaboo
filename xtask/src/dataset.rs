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
use serde::Deserialize;

use crate::checks::test_sample_data::{self, NAMES_FILE, NameList, Run};
use crate::db;
use crate::output;
use crate::repo::Repo;
use crate::steps::{self, Profile};

/// Where the folders and file names of the sample are written down once; the program `bb-sample`
/// and the end-to-end tests read the same file.
const LAYOUT_FILE: &str = "tools/dev/sample.json";
/// The moment every run of `--check` ends at: 2027-01-15 08:00:00 UTC, an arbitrary fixed one.
const CHECK_START: i64 = 1_800_000_000;
/// What `--check` makes of the tables of a PostgreSQL run, so the scan for secrets covers them.
const POSTGRES_DUMP_FILE: &str = "postgres.sql";
/// The runs of `--check`: twice on each database.
const CHECK_RUNS: [(&str, DatabaseKind); 4] = [
    ("sqlite-1", DatabaseKind::Sqlite),
    ("sqlite-2", DatabaseKind::Sqlite),
    ("postgres-1", DatabaseKind::Postgres),
    ("postgres-2", DatabaseKind::Postgres),
];

/// `tools/dev/sample.json`: where the sample is made and what its files are called.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Layout {
    /// Where the sample is made on SQLite when `--out` is not given.
    out: String,
    /// The same on PostgreSQL.
    out_postgres: String,
    /// Where `cargo xtask dev --sample` keeps its sample.
    dev_out: String,
    /// Where `--check` makes its runs, one folder each.
    check_out: String,
    /// The data folder inside an output folder.
    data_folder: String,
    /// The file with the passwords: the only file that may hold a secret.
    pub(crate) sign_in_file: String,
    /// The canonical dump.
    pub(crate) dump_file: String,
    /// The environment variable that holds the address of the PostgreSQL database of the program.
    database_url_variable: String,
}

impl Layout {
    /// Reads the layout of the repository at `root`.
    pub(crate) fn load(root: &Path) -> anyhow::Result<Self> {
        let path = root.join(LAYOUT_FILE);
        let text = fs::read_to_string(&path)
            .with_context(|| format!("could not read {}", path.display()))?;
        serde_json::from_str(&text).with_context(|| format!("{LAYOUT_FILE} is not valid"))
    }
}

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
    let layout = Layout::load(repo.root())?;
    let out = match (&request.out, request.database) {
        (Some(out), _) => std::path::absolute(out)
            .with_context(|| format!("{} is not a usable folder", out.display()))?,
        (None, DatabaseKind::Sqlite) => repo.root().join(&layout.out),
        (None, DatabaseKind::Postgres) => repo.root().join(&layout.out_postgres),
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
            (&layout, url),
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
    let layout = Layout::load(root)?;
    let out = root.join(&layout.dev_out);
    let status = program_command(
        &program,
        root,
        &Arguments {
            seed: None,
            start: now()?,
            database: DatabaseKind::Sqlite,
            out: &out,
        },
        (&layout, None),
    )
    .status()
    .with_context(|| format!("could not start {}", program.display()))?;
    Ok(status.success().then(|| out.join(&layout.data_folder)))
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
    (layout, url): (&Layout, Option<&str>),
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
        command.env(&layout.database_url_variable, url);
    }
    command
}

/// `--check`: makes the sample twice on each database with one seed, and holds the four dumps
/// against each other and against the rules of TEST-SAMPLE-DATA.
fn check(repo: &Repo, program: &Path, seed: Option<u64>) -> anyhow::Result<bool> {
    let root = repo.root();
    let layout = Layout::load(root)?;
    let folder = root.join(&layout.check_out);
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
                (&layout, url.as_deref()),
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
            runs.push(Run::read(label, &out, &layout)?);
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

    fn real_layout() -> Layout {
        Layout::load(Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()).unwrap()
    }

    #[test]
    fn the_program_pins_the_names_of_the_databases_xtask_passes() {
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
    fn the_real_layout_file_is_usable() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let layout = Layout::load(root).unwrap();
        for folder in [
            &layout.out,
            &layout.out_postgres,
            &layout.dev_out,
            &layout.check_out,
        ] {
            // Inside the work folder, written with / as separator.
            assert!(folder.starts_with("target/"), "{folder}");
            assert!(!folder.contains('\\') && !folder.contains(".."), "{folder}");
        }
        let distinct: std::collections::BTreeSet<&String> = [
            &layout.out,
            &layout.out_postgres,
            &layout.dev_out,
            &layout.check_out,
        ]
        .into_iter()
        .collect();
        assert_eq!(distinct.len(), 4, "every run has a folder of its own");
        for name in [
            &layout.data_folder,
            &layout.sign_in_file,
            &layout.dump_file,
            &layout.database_url_variable,
        ] {
            assert!(!name.trim().is_empty());
        }
    }

    #[test]
    fn the_address_of_a_database_goes_in_the_environment_and_not_on_the_command_line() {
        let out = Path::new("out");
        let layout = real_layout();
        let command = program_command(
            Path::new("bb-sample"),
            Path::new("."),
            &Arguments {
                seed: Some(7),
                start: 1_800_000_000,
                database: DatabaseKind::Postgres,
                out,
            },
            (&layout, Some("postgres://bugaboo:secret@127.0.0.1:1/x")),
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
        assert_eq!(environment[0].0, layout.database_url_variable.as_str());
    }

    #[test]
    fn without_a_seed_the_program_uses_its_own_default() {
        let layout = real_layout();
        let command = program_command(
            Path::new("bb-sample"),
            Path::new("."),
            &Arguments {
                seed: None,
                start: 1,
                database: DatabaseKind::Sqlite,
                out: Path::new("out"),
            },
            (&layout, None),
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
