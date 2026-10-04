//! Everything the `bugaboo` command tells the person at the terminal.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "command-line output; this module is the only place that prints (coding-standards §3)"
)]

use bb_core::message;

pub(crate) fn line(message: &str) {
    println!("{message}");
}

pub(crate) fn warning(text: &str) {
    eprintln!("{}", message::server_warning(text));
}

pub(crate) fn error(text: &str) {
    eprintln!("{}", message::server_error(text));
}
