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

/// Not async: the child process of a preview runs with no runtime at all (its memory limit would
/// count the stacks of every worker), so the command line is read first and the runtime is built
/// only for the commands that need one.
fn main() -> ExitCode {
    cli::start()
}

#[cfg(test)]
#[path = "tests/sign_in/mod.rs"]
mod sign_in_tests;
