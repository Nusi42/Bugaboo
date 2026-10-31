//! `cargo xtask`: the single runner for every check, generator and developer task.
//!
//! Git hooks, Claude Code hooks and CI only ever call this program (ADR-0014, ADR-0020).
//! Every check is listed once in `tools/checks/catalog.toml`.

#![forbid(unsafe_code)]

mod bench;
mod bench_budgets;
mod build;
mod bundle;
mod catalog;
mod checks;
mod ci;
mod dataset;
mod dataset_large;
mod db;
mod dev;
mod docs;
mod fakecc;
#[path = "gen/mod.rs"]
mod generate;
mod git_identity;
mod hook;
mod load;
mod location;
mod master;
mod metadata;
mod nightly;
mod output;
mod ports;
mod publish;
mod repo;
mod runner;
mod rust_registry;
mod rust_source;
mod search_index;
mod setup;
mod source;
mod steps;
#[cfg(test)]
mod testutil;
mod tools;
mod wasm;
mod web_code;

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
    /// Build the WebAssembly modules of the browser (`bb-wasm` and `bb-wasm-search`, with `wasm-bindgen` and `wasm-opt`).
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
    /// Ten people work on the same issues through the API at speed, and the invariants of
    /// architecture §11 are checked afterwards (TEST-CONCURRENT). Both databases unless one is named.
    Load(load::Arguments),
    /// Run the scenarios of the benchmarks on the large dataset and hold the numbers against
    /// their budgets (PERF-BENCH). Both databases unless one is named.
    Bench {
        /// Only this database.
        #[arg(long, value_parser = parse_database)]
        database: Option<DatabaseKind>,
    },
    /// The nightly run: `ci`, the large dataset if it is older than the last migration, and the
    /// benchmarks. It waits while another full run is on this computer.
    Nightly {
        /// Instead: register the daily task (03:00, in this folder) with the scheduler of Windows.
        #[arg(long)]
        install: bool,
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
    /// Store the API description as the frozen copy that ARCH-API-FREEZE compares with. The lead
    /// does it at `m4-done`; after that a change of /api/v1 may only add (ADR-0008).
    Freeze,
    /// Install the pinned external tools into `.tools/` (tools/dev/tools.toml).
    Setup,
    /// Mark the master repository (ADR-0039): write `.env.local` with the role, make its pre-push
    /// hook run the checks that build nothing, and make Cargo refuse to compile there. Run it in
    /// a work folder; the master is its `origin` unless a folder is named.
    GuardMaster {
        /// The master folder (default: the folder the `origin` remote points to).
        master: Option<PathBuf>,
    },
    /// The index of the text search (ADR-0016).
    SearchIndex {
        #[command(subcommand)]
        action: SearchIndexAction,
    },
    /// Write files that are generated from a single source.
    Gen {
        /// What to generate.
        #[arg(value_parser = generate::possible_values())]
        target: String,
    },
}

#[derive(Subcommand)]
enum SearchIndexAction {
    /// Build the index again from the stored texts: for a database that has none yet, and after
    /// the rule that prepares texts changed.
    Rebuild {
        /// The data folder (default: the one `cargo xtask dev` uses).
        #[arg(long)]
        data_dir: Option<PathBuf>,
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
        /// How many issues to make (default: the full sample of tools/dev/sample.json; the
        /// comments, relations and the rest are scaled to it).
        #[arg(long, conflicts_with = "check")]
        issues: Option<u32>,
        /// The folder for the data folder and the sign-in details (default: the folder that
        /// tools/dev/sample.json names).
        #[arg(long, conflicts_with = "check")]
        out: Option<PathBuf>,
        /// Instead: make it twice on each database and check that the dumps are the same and
        /// fictional (TEST-SAMPLE-DATA). It always uses both databases and its own folders.
        #[arg(long)]
        check: bool,
    },
    /// The large dataset of PERF-01: 500 people, 500 projects and a million issues, written in
    /// big transactions (architecture §11, "Large dataset"). On both databases unless one is named.
    Large {
        /// The number the dataset is made from (default 42).
        #[arg(long)]
        seed: Option<u64>,
        /// Only this database (default: both).
        #[arg(long, conflicts_with = "check", value_parser = parse_database)]
        database: Option<DatabaseKind>,
        /// How many issues (default: the number of tools/dev/sample.json).
        #[arg(long, conflicts_with = "check")]
        issues: Option<u32>,
        /// Instead: make a small one twice on each database and check that it holds the rules
        /// of the large dataset and is the same both times.
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
    /// Push-time checks (Git pre-push hook): `check push <remote> <url>`. A work folder pushing
    /// to the master runs the checks of a push; the master pushing to the public remote runs the
    /// checks that build nothing; every other push is refused (ADR-0039).
    Push {
        /// The name of the remote (given by Git).
        remote: Option<String>,
        /// The URL of the remote (given by Git).
        url: Option<String>,
    },
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
    location::guard(repo.root(), builds_nothing(&cli.command))?;
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
        Command::Freeze => {
            checks::arch_api_freeze::write_frozen(&repo)?;
            output::wrote(checks::arch_api_freeze::FROZEN_PATH);
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
        Command::GuardMaster { master } => {
            for line in master::run(&repo, master)? {
                output::info(&line);
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Gen { target } => {
            output::generated(&generate::write_named(&repo, &catalog, &target)?);
            Ok(ExitCode::SUCCESS)
        }
        Command::SearchIndex {
            action: SearchIndexAction::Rebuild { data_dir },
        } => Ok(steps::exit_code(search_index::rebuild(
            repo.root(),
            data_dir.as_deref(),
        )?)),
        Command::Dev { sample } => Ok(steps::exit_code(dev::run(repo.root(), sample)?)),
        Command::Dataset { kind } => Ok(steps::exit_code(dataset::run(&repo, &request_of(kind))?)),
        Command::Bench { database } => Ok(steps::exit_code(bench::run(
            &repo,
            &database.into_iter().collect::<Vec<_>>(),
        )?)),
        Command::Nightly { install } => Ok(steps::exit_code(nightly::run(&repo, install)?)),
        Command::Load(arguments) => Ok(steps::exit_code(load::run(&repo, &arguments.request())?)),
        Command::Build { target } => Ok(steps::exit_code(build::run(
            repo.root(),
            target.as_deref(),
        )?)),
        Command::Wasm => Ok(steps::exit_code(wasm::run(repo.root())?)),
        Command::Ci => {
            let _lock = nightly::FullRun::take(&repo)?;
            Ok(steps::exit_code(ci::run(repo.root())?))
        }
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
        Command::Check { scope } => run_check(&repo, &catalog, &registry, scope),
    }
}

/// `cargo xtask check …`: picks the checks of the scope and runs them.
fn run_check(
    repo: &repo::Repo,
    catalog: &catalog::Catalog,
    registry: &[Box<dyn checks::Check>],
    scope: CheckScope,
) -> anyhow::Result<ExitCode> {
    let scope = match scope {
        CheckScope::Edit { files } => runner::Scope::Edit(repo.relative_paths(&files)),
        CheckScope::Staged => runner::Scope::Staged(repo.staged_files()?),
        CheckScope::Push { url, .. } => {
            match push_scope(location::role(repo.root()), url.as_deref()) {
                Ok(scope) => scope,
                Err(why) => {
                    output::error(&why);
                    return Ok(ExitCode::FAILURE);
                }
            }
        }
        CheckScope::All => runner::Scope::All,
        CheckScope::Only { ids } => runner::Scope::Only(ids),
        CheckScope::CommitMsg { file } => return commit_msg(repo, &file),
    };
    let passed = runner::run(repo, catalog, registry, &scope)?;
    Ok(if passed {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// Whether the command builds, installs and tests nothing, so the master may run it: marking the
/// master, listing the checks and the checks of a push.
fn builds_nothing(command: &Command) -> bool {
    matches!(
        command,
        Command::GuardMaster { .. }
            | Command::List
            | Command::Check {
                scope: CheckScope::Push { .. }
            }
    )
}

/// The checks a push runs, or why it is refused (ADR-0039): the one decision is `publish::plan`.
fn push_scope(role: location::Role, url: Option<&str>) -> Result<runner::Scope, String> {
    match publish::plan(role, url) {
        publish::Plan::Full => Ok(runner::Scope::Push),
        publish::Plan::NoBuild => Ok(runner::Scope::Only(
            publish::NO_BUILD_CHECKS.map(str::to_owned).to_vec(),
        )),
        publish::Plan::Refuse(why) => Err(why),
    }
}

/// What `cargo xtask dataset …` was asked to do.
fn request_of(kind: DatasetKind) -> dataset::Request {
    match kind {
        DatasetKind::Sample {
            seed,
            database,
            issues,
            out,
            check,
        } => dataset::Request {
            seed,
            issues,
            database: database.unwrap_or(DatabaseKind::Sqlite),
            out,
            check,
            large: false,
            databases: Vec::new(),
        },
        DatasetKind::Large {
            seed,
            database,
            issues,
            check,
        } => dataset::Request {
            seed,
            issues,
            database: database.unwrap_or(DatabaseKind::Sqlite),
            out: None,
            check,
            large: true,
            databases: database.into_iter().collect(),
        },
    }
}

/// `check commit-msg <file>`: the Git commit-msg hook (GIT-COMMIT-MSG and SEC-PRIVATE).
fn commit_msg(repo: &repo::Repo, file: &std::path::Path) -> anyhow::Result<ExitCode> {
    let problems = commit_message_problems(repo, file)?;
    for problem in &problems {
        output::info(problem);
    }
    Ok(if problems.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// What is wrong with a commit about to be recorded: the form of the message, what is private in
/// it, and the author and committer it will have.
fn commit_message_problems(
    repo: &repo::Repo,
    file: &std::path::Path,
) -> anyhow::Result<Vec<String>> {
    let mut problems = checks::git_commit_msg::check_file(file)?;
    problems.extend(checks::sec_private::message_problems(file)?);
    problems.extend(checks::sec_private::recorded_identity_problems(repo));
    Ok(problems)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_commit_message_hook_reads_the_form_and_what_is_private() {
        let temp = testutil::TempDir::new("commit-message-hook");
        assert!(temp.git(&["init", "--quiet"]));
        let identity = |name: &str, email: &str| {
            assert!(temp.git(&["config", "user.name", name]));
            assert!(temp.git(&["config", "user.email", email]));
        };
        identity("mira", "1+mira@users.noreply.github.com");
        let repo = repo::Repo::at(temp.path().to_path_buf());
        let problems =
            |text: &str| commit_message_problems(&repo, &temp.write("MSG", text)).unwrap();
        let trailer = "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>";
        assert!(problems(&format!("fix(web): add x\n\n{trailer}\n")).is_empty());
        assert_eq!(problems("wip\n").len(), 2);
        // A session line is refused by the message rules and by the private-data rules.
        assert_eq!(problems("fix: add x\n\nClaude-Session: made-up\n").len(), 2);
        // The author and the committer about to be recorded: a private address is refused.
        identity("Mira Made-Up", "mira@example.test");
        assert_eq!(problems("fix(web): x\n").len(), 2);
    }

    #[test]
    fn the_pre_push_hook_decides_by_the_role_and_the_remote() {
        // The arguments Git gives the hook: the name and the URL of the remote.
        let parsed =
            Cli::try_parse_from(["xtask", "check", "push", "origin", "https://example.test/x"])
                .unwrap();
        let Command::Check {
            scope: CheckScope::Push { url, .. },
        } = &parsed.command
        else {
            panic!("not a push");
        };
        assert!(builds_nothing(&parsed.command));
        let network = url.as_deref();
        assert!(push_scope(location::Role::Work, network).is_err());
        let Ok(runner::Scope::Only(ids)) = push_scope(location::Role::Master, network) else {
            panic!("the master runs the checks that build nothing");
        };
        assert_eq!(ids, ["SEC-PRIVATE", "DEP-LICENSES"]);
        assert!(matches!(
            push_scope(location::Role::Work, Some("../master")),
            Ok(runner::Scope::Push)
        ));
        assert!(push_scope(location::Role::Master, Some("../master")).is_err());
        // Building is not allowed in the master.
        let build = Cli::try_parse_from(["xtask", "build"]).unwrap();
        assert!(!builds_nothing(&build.command));
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
