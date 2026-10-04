//! Everything the `bugaboo` command tells the person at the terminal.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "command-line output; this module is the only place that prints (coding-standards §3)"
)]

use crate::text::t;

pub(crate) fn line(message: &str) {
    println!("{message}");
}

pub(crate) fn warning(message: &str) {
    eprintln!("{}", t("server.warning", &[("message", message)]));
}

pub(crate) fn error(message: &str) {
    eprintln!("{}", t("server.error", &[("message", message)]));
}
