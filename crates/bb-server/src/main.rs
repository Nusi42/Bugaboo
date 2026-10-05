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

#[tokio::main]
async fn main() -> ExitCode {
    cli::run().await
}

#[cfg(test)]
#[path = "tests/sign_in/mod.rs"]
mod sign_in_tests;
