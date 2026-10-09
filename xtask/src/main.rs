//! `cargo xtask`: the single runner for every check, generator and developer task.
//!
//! Git hooks, Claude Code hooks and CI only ever call this program (ADR-0014, ADR-0020).
//! Every check is listed once in `tools/checks/catalog.toml`.

#![forbid(unsafe_code)]

mod build;
mod bundle;
mod catalog;
mod checks;
mod ci;
mod dataset;
mod db;
mod dev;
mod docs;
mod fakecc;
#[path = "gen/mod.rs"]
mod generate;
mod hook;
mod location;
mod metadata;
mod output;
mod ports;
mod repo;
mod runner;
mod rust_registry;
mod rust_source;
mod setup;
mod source;
mod steps;
#[cfg(test)]
mod testutil;
mod tools;
mod wasm;

use std::io::Read as _;
use std::path::PathBuf;
use std::process::ExitCode;

use bb_dev_words::database::{DatabaseKind, parse as parse_database};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "xtask",
    about = "Bugaboo's checks, generators and developer tasks"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run checks from the catalogue.
    Check {
        #[command(subcommand)]
        scope: CheckScope,
    },
    /// Start the Vite dev server and the Bugaboo server on the ports from tools/dev/ports.json.
    Dev {
        /// Start on a fresh sample organization (50 fictional people; the program says where
        /// the passwords are).
        #[arg(long)]
        sample: bool,
    },
    /// Build the browser app, then the release binary (one file, browser app inside).
    Build {
        /// Build for another system, such as x86_64-unknown-linux-musl (DIST-01).
        #[arg(long)]
        target: Option<String>,
    },
    /// Build the WebAssembly module of the browser (`bb-wasm`, with `wasm-bindgen` and `wasm-opt`).
    Wasm,
    /// Everything CI runs: checks, tests, browser build, browser tests.
    Ci,
    /// The throwaway PostgreSQL for the repository tests (architecture §11, "Database tests").
    Db {
        #[command(subcommand)]
        action: DbAction,
    },
    /// Make or check the sample organization (architecture §11, "Sample organization").
    Dataset {
        #[command(subcommand)]
        kind: DatasetKind,
    },
    /// Entry points for Claude Code hooks.
    Hook {
        #[command(subcommand)]
        event: HookEvent,
    },
    /// Build the commentable HTML pages from the Markdown sources.
    Docs,
    /// Rewrite the counted list of suppressions (META-SUPPRESS); the change needs approval.
    Suppressions,
    /// List every check in the catalogue and whether it is implemented.
    List,
    /// Install the pinned external tools into `.tools/` (tools/dev/tools.toml).
    Setup,
    /// Write files that are generated from a single source.
    Gen {
        /// What to generate.
        #[arg(value_parser = generate::possible_values())]
        target: String,
    },
}

#[derive(Subcommand)]
enum DatasetKind {
    /// 50 fictional people, made through the real use cases, the same every time from a seed.
    Sample {
        /// The number the sample is made from (default 42).
        #[arg(long)]
        seed: Option<u64>,
        /// Which database to make it on (default sqlite).
        #[arg(long, conflicts_with = "check", value_parser = parse_database)]
        database: Option<DatabaseKind>,
        /// The folder for the data folder and the sign-in details (default: the folder that
        /// tools/dev/sample.json names).
        #[arg(long, conflicts_with = "check")]
        out: Option<PathBuf>,
        /// Instead: make it twice on each database and check that the dumps are the same and
        /// fictional (TEST-SAMPLE-DATA). It always uses both databases and its own folders.
        #[arg(long)]
        check: bool,
    },
}

#[derive(Subcommand)]
enum DbAction {
    /// Start the test PostgreSQL (creates it on first use).
    Start,
    /// Stop the test PostgreSQL.
    Stop,
    /// Throw the test PostgreSQL away and start a new one.
    Reset,
    /// Print the connection URL (it contains the random password; do not share it).
    Url,
    /// Run a command with SQLite and PostgreSQL tests switched on: `db run -- cargo test`.
    Run {
        /// The command and its arguments.
        #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
}

/// Claude Code hook events.
#[derive(Subcommand)]
enum HookEvent {
    /// After an agent edited a file: run the checks for it (reads the hook JSON on stdin).
    PostEdit,
}

#[derive(Subcommand)]
enum CheckScope {
    /// Checks for files an agent just edited (Claude Code hook).
    Edit {
        /// The edited files, relative to the repository root or absolute.
        files: Vec<PathBuf>,
    },
    /// Commit-time checks for staged files (Git pre-commit hook).
    Staged,
    /// Push-time checks on the whole repository (Git pre-push hook).
    Push,
    /// Every check that CI runs.
    All,
    /// Check one commit message file (Git commit-msg hook): `check commit-msg <file>`.
    CommitMsg {
        /// The file Git wrote the message to.
        file: PathBuf,
    },
    /// Only the checks with these IDs.
    Only {
        /// Check IDs, for example `META-CATALOG`.
        ids: Vec<String>,
    },
}

fn main() -> ExitCode {
    if fakecc::requested() {
        return fakecc::run();
    }
    match run() {
        Ok(code) => code,
        Err(error) => {
            output::error(&format!("{error:#}"));
            ExitCode::FAILURE
        }
    }
}

fn run() -> anyhow::Result<ExitCode> {
    let cli = Cli::parse();
    let repo = repo::Repo::discover()?;
    location::guard(repo.root())?;
    let catalog = catalog::Catalog::load(&repo)?;
    let registry = checks::registry();
    match cli.command {
        Command::Docs => {
            output::generated(&generate::write_named(&repo, &catalog, generate::DOCS)?);
            Ok(ExitCode::SUCCESS)
        }
        Command::Suppressions => {
            checks::meta_suppress::write_list(&repo)?;
            output::wrote(checks::meta_suppress::LIST_PATH);
            Ok(ExitCode::SUCCESS)
        }
        Command::List => {
            output::list(&catalog, &registry);
            Ok(ExitCode::SUCCESS)
        }
        Command::Setup => {
            setup::run(&repo)?;
            output::info(&format!(
                "All tools are installed in {}/.",
                tools::TOOLS_DIR
            ));
            Ok(ExitCode::SUCCESS)
        }
        Command::Gen { target } => {
            output::generated(&generate::write_named(&repo, &catalog, &target)?);
            Ok(ExitCode::SUCCESS)
        }
        Command::Dev { sample } => Ok(steps::exit_code(dev::run(repo.root(), sample)?)),
        Command::Dataset {
            kind:
                DatasetKind::Sample {
                    seed,
                    database,
                    out,
                    check,
                },
        } => {
            let request = dataset::Request {
                seed,
                database: database.unwrap_or(DatabaseKind::Sqlite),
                out,
                check,
            };
            Ok(steps::exit_code(dataset::run(&repo, &request)?))
        }
        Command::Build { target } => Ok(steps::exit_code(build::run(
            repo.root(),
            target.as_deref(),
        )?)),
        Command::Wasm => Ok(steps::exit_code(wasm::run(repo.root())?)),
        Command::Ci => Ok(steps::exit_code(ci::run(repo.root())?)),
        Command::Db { action } => {
            let action = match action {
                DbAction::Start => db::Action::Start,
                DbAction::Stop => db::Action::Stop,
                DbAction::Reset => db::Action::Reset,
                DbAction::Url => db::Action::Url,
                DbAction::Run { command } => db::Action::Run(command),
            };
            Ok(steps::exit_code(db::run(&repo, &action)?))
        }
        Command::Hook {
            event: HookEvent::PostEdit,
        } => {
            let mut stdin = String::new();
            std::io::stdin().read_to_string(&mut stdin)?;
            hook::post_edit(repo.root(), &stdin)
        }
        Command::Check { scope } => {
            let scope = match scope {
                CheckScope::Edit { files } => runner::Scope::Edit(repo.relative_paths(&files)),
                CheckScope::Staged => runner::Scope::Staged(repo.staged_files()?),
                CheckScope::Push => runner::Scope::Push,
                CheckScope::All => runner::Scope::All,
                CheckScope::Only { ids } => runner::Scope::Only(ids),
                CheckScope::CommitMsg { file } => return commit_msg(&file),
            };
            let passed = runner::run(&repo, &catalog, &registry, &scope)?;
            Ok(if passed {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            })
        }
    }
}

/// `check commit-msg <file>`: the Git commit-msg hook (GIT-COMMIT-MSG and SEC-PRIVATE).
fn commit_msg(file: &std::path::Path) -> anyhow::Result<ExitCode> {
    let problems = commit_message_problems(file)?;
    for problem in &problems {
        output::info(problem);
    }
    Ok(if problems.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// What is wrong with a commit message file: the form of the subject and what is private in it.
fn commit_message_problems(file: &std::path::Path) -> anyhow::Result<Vec<String>> {
    let mut problems = checks::git_commit_msg::check_file(file)?;
    problems.extend(checks::sec_private::message_problems(file)?);
    Ok(problems)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_commit_message_hook_reads_the_form_and_what_is_private() {
        let temp = testutil::TempDir::new("commit-message-hook");
        let problems = |text: &str| commit_message_problems(&temp.write("MSG", text)).unwrap();
        assert!(problems("fix(web): x\n\nCo-Authored-By: someone\n").is_empty());
        assert_eq!(problems("wip\n").len(), 1);
        assert_eq!(problems("fix: x\n\nClaude-Session: made-up\n").len(), 1);
    }

    fn parses(arguments: &[&str]) -> bool {
        Cli::try_parse_from(
            ["xtask", "dataset", "sample"]
                .into_iter()
                .chain(arguments.iter().copied()),
        )
        .is_ok()
    }

    #[test]
    fn the_check_makes_its_own_runs_and_refuses_a_database_or_a_folder() {
        assert!(parses(&["--check"]));
        assert!(parses(&["--check", "--seed", "7"]));
        assert!(!parses(&["--check", "--database", "postgres"]));
        assert!(!parses(&["--check", "--out", "somewhere"]));
        // Without --check both are fine.
        assert!(parses(&["--database", "postgres", "--out", "somewhere"]));
        assert!(parses(&[]));
    }
}
