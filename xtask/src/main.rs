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
mod repo;
mod runner;
mod rust_source;
mod setup;
mod steps;
#[cfg(test)]
mod testutil;
mod tools;

use std::io::Read as _;
use std::path::PathBuf;
use std::process::ExitCode;

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
    Dev,
    /// Build the browser app, then the release binary (one file, browser app inside).
    Build {
        /// Build for another system, such as x86_64-unknown-linux-musl (DIST-01).
        #[arg(long)]
        target: Option<String>,
    },
    /// Everything CI runs: checks, tests, browser build, browser tests.
    Ci,
    /// The throwaway PostgreSQL for the repository tests (architecture §11, "Database tests").
    Db {
        #[command(subcommand)]
        action: DbAction,
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
        #[command(subcommand)]
        target: GenTarget,
    },
}

#[derive(Subcommand)]
enum GenTarget {
    /// CSS variables and TypeScript names from design/tokens.json.
    Tokens,
    /// Typed message keys from messages/en.json.
    Messages,
    /// The personal-data page docs/privacy/personal-data.md from the marks in the code.
    Privacy,
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
            for path in docs::write(&repo, &catalog)? {
                output::wrote(&path);
            }
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
            output::info("All tools are installed in .tools/.");
            Ok(ExitCode::SUCCESS)
        }
        Command::Gen { target } => {
            let written = match target {
                GenTarget::Tokens => generate::tokens::run(&repo)?,
                GenTarget::Messages => generate::messages::run(&repo)?,
                GenTarget::Privacy => generate::privacy::run(&repo)?,
            };
            output::generated(&written);
            Ok(ExitCode::SUCCESS)
        }
        Command::Dev => Ok(steps::exit_code(dev::run(repo.root())?)),
        Command::Build { target } => Ok(steps::exit_code(build::run(
            repo.root(),
            target.as_deref(),
        )?)),
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

/// `check commit-msg <file>`: the Git commit-msg hook (GIT-COMMIT-MSG).
fn commit_msg(file: &std::path::Path) -> anyhow::Result<ExitCode> {
    let problems = checks::git_commit_msg::check_file(file)?;
    for problem in &problems {
        output::info(problem);
    }
    Ok(if problems.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}
