//! `cargo xtask ci`: exactly what GitHub Actions will run later (coding-standards §7).

use std::path::Path;

use anyhow::Context as _;

use crate::steps::{self, Step, WEB_APP};

pub(crate) fn plan(root: &Path) -> anyhow::Result<Vec<Step>> {
    let xtask = std::env::current_exe().context("could not find the running xtask program")?;
    Ok(vec![
        Step::new("all checks", xtask.to_string_lossy(), &["check", "all"]),
        Step::new("Rust tests", "cargo", &["test", "--workspace"]),
        Step::web(
            root,
            "build the browser app",
            &["run", "build", "-w", WEB_APP],
        ),
        Step::web(root, "browser unit tests", &["test", "-w", WEB_APP]),
        Step::web(
            root,
            "browser end-to-end tests",
            &["exec", "-w", WEB_APP, "--", "playwright", "test"],
        ),
    ])
}

pub(crate) fn run(root: &Path) -> anyhow::Result<bool> {
    steps::run(root, &plan(root)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_plan_has_every_ci_step_in_order() {
        let root = tempfile::tempdir().unwrap();
        let names: Vec<_> = plan(root.path()).unwrap().iter().map(|s| s.name).collect();
        assert_eq!(
            names,
            [
                "all checks",
                "Rust tests",
                "build the browser app",
                "browser unit tests",
                "browser end-to-end tests"
            ]
        );
    }

    #[test]
    fn only_the_browser_steps_are_skipped_without_the_app() {
        let root = tempfile::tempdir().unwrap();
        let skipped: Vec<_> = plan(root.path())
            .unwrap()
            .iter()
            .map(|s| s.skip.is_some())
            .collect();
        assert_eq!(skipped, [false, false, true, true, true]);
    }
}
