//! `cargo xtask`: the single runner for every check, generator and developer task.
//!
//! Git hooks, Claude Code hooks and CI only ever call this program (ADR-0014, ADR-0020).
//! Every check is listed once in `tools/checks/catalog.toml`.

#![forbid(unsafe_code)]

mod catalog;
mod checks;
#[path = "gen/mod.rs"]
mod generate;
mod location;
mod output;
mod repo;
mod runner;

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
    /// List every check in the catalogue and whether it is implemented.
    List,
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
    /// Only the checks with these IDs.
    Only {
        /// Check IDs, for example `META-CATALOG`.
        ids: Vec<String>,
    },
}

fn main() -> ExitCode {
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
        Command::List => {
            output::list(&catalog, &registry);
            Ok(ExitCode::SUCCESS)
        }
        Command::Gen { target } => {
            let written = match target {
                GenTarget::Tokens => generate::tokens::run(&repo)?,
                GenTarget::Messages => generate::messages::run(&repo)?,
            };
            output::generated(&written);
            Ok(ExitCode::SUCCESS)
        }
        Command::Check { scope } => {
            let scope = match scope {
                CheckScope::Edit { files } => runner::Scope::Edit(repo.relative_paths(&files)),
                CheckScope::Staged => runner::Scope::Staged(repo.staged_files()?),
                CheckScope::Push => runner::Scope::Push,
                CheckScope::All => runner::Scope::All,
                CheckScope::Only { ids } => runner::Scope::Only(ids),
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
