//! The `bugaboo` binary: command line, setup wizard, service install and wiring of all adapters.

#![forbid(unsafe_code)]

mod cli;
mod config;
mod data_folder;
mod log;
mod output;
mod probe;
mod wiring;

use std::process::ExitCode;

/// The allocator of the program: the one of the system, wrapped so that the child process of a
/// preview can limit its own memory (ADR-0032 §4). Without a limit it only counts.
#[global_allocator]
static ALLOCATOR: bb_files::preview::ChildAllocator = bb_files::preview::child_allocator();

#[tokio::main]
async fn main() -> ExitCode {
    cli::run().await
}

#[cfg(test)]
#[path = "tests/sign_in/mod.rs"]
mod sign_in_tests;
