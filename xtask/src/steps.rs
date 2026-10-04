//! Running a list of commands one after the other (used by `build` and `ci`), and the commands
//! of the browser app. The browser app (`web/app`) may not exist yet; its steps are then skipped
//! with a clear note instead of failing.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use anyhow::Context as _;

use crate::output;

/// The workspace folder of the browser app, relative to the repository root.
pub(crate) const WEB_APP: &str = "web/app";

/// One command of a plan.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Step {
    pub(crate) name: &'static str,
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    /// Why this step is skipped, if it is.
    pub(crate) skip: Option<&'static str>,
}

impl Step {
    pub(crate) fn new(name: &'static str, program: impl Into<String>, args: &[&str]) -> Self {
        Self {
            name,
            program: program.into(),
            args: args.iter().map(|arg| (*arg).to_owned()).collect(),
            skip: None,
        }
    }

    /// A step of the browser app: skipped while `web/app` does not exist.
    pub(crate) fn web(root: &Path, name: &'static str, args: &[&str]) -> Self {
        let mut step = Self::new(name, npm(), args);
        if !web_app_exists(root) {
            step.skip = Some("web/app does not exist yet");
        }
        step
    }
}

/// `npm` is a `.cmd` file on Windows, which `Command` only finds under that name.
pub(crate) fn npm() -> &'static str {
    if cfg!(windows) { "npm.cmd" } else { "npm" }
}

pub(crate) fn web_app_exists(root: &Path) -> bool {
    root.join(WEB_APP).join("package.json").is_file()
}

/// The `bugaboo` binary built by cargo for `profile` (`debug` or `release`).
pub(crate) fn server_binary(root: &Path, profile: &str) -> PathBuf {
    let target =
        std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from);
    target.join(profile).join(if cfg!(windows) {
        "bugaboo.exe"
    } else {
        "bugaboo"
    })
}

/// Runs the steps in order and stops at the first failure. Returns whether all passed.
pub(crate) fn run(root: &Path, steps: &[Step]) -> anyhow::Result<bool> {
    let mut failed = 0_usize;
    let mut skipped = 0_usize;
    for (index, step) in steps.iter().enumerate() {
        if let Some(reason) = step.skip {
            output::step_skipped(step.name, reason);
            skipped += 1;
            continue;
        }
        output::step_start(index + 1, steps.len(), step.name);
        let started = Instant::now();
        let status = Command::new(&step.program)
            .args(&step.args)
            .current_dir(root)
            .status()
            .with_context(|| format!("could not start `{}` for \"{}\"", step.program, step.name))?;
        output::step_end(step.name, status.success(), started.elapsed());
        if !status.success() {
            failed += 1;
            break;
        }
    }
    output::steps_summary(steps.len(), skipped, failed);
    Ok(failed == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_steps_are_skipped_without_the_app_and_run_with_it() {
        let root = tempfile::tempdir().unwrap();
        let step = Step::web(root.path(), "web test", &["test"]);
        assert!(step.skip.is_some());

        std::fs::create_dir_all(root.path().join(WEB_APP)).unwrap();
        std::fs::write(root.path().join(WEB_APP).join("package.json"), "{}").unwrap();
        assert_eq!(Step::web(root.path(), "web test", &["test"]).skip, None);
    }

    #[test]
    fn steps_run_in_order_and_stop_at_the_first_failure() {
        let root = tempfile::tempdir().unwrap();
        let cargo = std::env::var("CARGO").unwrap();
        let ok = Step::new("ok", &cargo, &["--version"]);
        let bad = Step::new("bad", &cargo, &["no-such-subcommand-for-test"]);
        assert!(run(root.path(), &[ok]).unwrap());
        assert!(!run(root.path(), &[bad]).unwrap());
    }

    #[test]
    fn a_missing_program_is_an_error_naming_the_step() {
        let root = tempfile::tempdir().unwrap();
        let step = Step::new("missing", "bugaboo-no-such-program", &[]);
        let error = run(root.path(), &[step]).unwrap_err();
        assert!(format!("{error:#}").contains("missing"));
    }
}
