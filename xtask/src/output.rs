//! Everything xtask tells the person (or agent) at the terminal: short, plain, actionable.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "xtask is a command-line tool; this module is its only terminal output (coding-standards §3)"
)]

use std::time::Duration;

use crate::catalog::{Catalog, Entry};
use crate::checks::{Check, Finding};

pub(crate) fn result(entry: &Entry, findings: &[Finding], took: Duration) {
    let millis = took.as_millis();
    if findings.is_empty() {
        println!("ok    {:<22} {}  ({millis} ms)", entry.id, entry.title);
        return;
    }
    println!("FAIL  {:<22} {}  ({millis} ms)", entry.id, entry.title);
    for finding in findings {
        match &finding.location {
            Some(location) => println!("        {location}: {}", finding.message),
            None => println!("        {}", finding.message),
        }
    }
    println!("        Rule: {}", entry.rule);
}

pub(crate) fn summary(total: usize, failed: usize, files: Option<usize>) {
    let scope = match files {
        Some(1) => " for 1 changed file".to_owned(),
        Some(count) => format!(" for {count} changed files"),
        None => String::new(),
    };
    if total == 0 {
        println!("No checks apply{scope}.");
    } else if failed == 0 {
        println!("All {total} checks passed{scope}.");
    } else {
        println!("{failed} of {total} checks failed{scope}.");
    }
}

pub(crate) fn list(catalog: &Catalog, registry: &[Box<dyn Check>]) {
    for entry in catalog.entries() {
        let implemented = registry.iter().any(|check| check.id() == entry.id);
        let state = match (entry.is_active(), implemented) {
            (true, true) => "active",
            (false, false) => "planned",
            _ => "MISMATCH",
        };
        println!("{state:<9} {:<22} {}", entry.id, entry.title);
    }
}

pub(crate) fn error(message: &str) {
    eprintln!("xtask: {message}");
}

/// A plain progress or information line.
pub(crate) fn info(message: &str) {
    println!("{message}");
}

pub(crate) fn generated(written: &[String]) {
    if written.is_empty() {
        println!("Everything is already up to date.");
    }
    for path in written {
        println!("wrote {path}");
    }
}

pub(crate) fn line(message: &str) {
    println!("{message}");
}

pub(crate) fn step_start(number: usize, total: usize, name: &str) {
    println!("[{number}/{total}] {name}");
}

pub(crate) fn step_skipped(name: &str, reason: &str) {
    println!("skip  {name}: {reason}");
}

pub(crate) fn step_end(name: &str, passed: bool, took: Duration) {
    let state = if passed { "ok  " } else { "FAIL" };
    println!("{state}  {name}  ({:.1} s)", took.as_secs_f32());
}

pub(crate) fn steps_summary(total: usize, skipped: usize, failed: usize) {
    if failed > 0 {
        println!("Failed. Fix the step marked FAIL above and run it again.");
    } else if skipped > 0 {
        println!("All {} steps passed, {skipped} skipped.", total - skipped);
    } else {
        println!("All {total} steps passed.");
    }
}

/// What the agent sees after a failed edit check (Claude Code shows stderr on exit code 2).
pub(crate) fn hook_feedback(report: &str) {
    eprintln!("{report}");
}
