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

/// A question that waits for an answer on the same line: no line break, shown at once.
pub(crate) fn prompt(text: &str) {
    use std::io::Write as _;
    print!("{text}");
    // A prompt that stays in the buffer would not be seen before the answer is typed. If the
    // console is gone there is nobody to ask.
    let _ = std::io::stdout().flush();
}

/// Exactly this text, without a line break added (machine-readable output).
pub(crate) fn raw(text: &str) {
    print!("{text}");
}

pub(crate) fn warning(text: &str) {
    eprintln!("{}", message::server_warning(text));
}

pub(crate) fn error(text: &str) {
    eprintln!("{}", message::server_error(text));
}
