//! `cargo xtask ci`: exactly what GitHub Actions will run later (coding-standards §7).

use std::path::Path;

use anyhow::Context as _;

use crate::steps::{self, Profile, Step};

/// The static Linux program of this machine's kind (OPS-01, DIST-01): one file for any distro.
const fn static_linux_target() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "aarch64-unknown-linux-musl"
    } else {
        "x86_64-unknown-linux-musl"
    }
}

/// Steps that only make sense on Linux: the static program is built and kept as proof that
/// the one-file build works there. The Windows runner builds its program with `cargo xtask build`.
fn linux_only(step: Step) -> Step {
    if cfg!(target_os = "linux") {
        step
    } else {
        Step {
            skip: Some("only on Linux"),
            ..step
        }
    }
}

pub(crate) fn plan(root: &Path) -> anyhow::Result<Vec<Step>> {
    let xtask = std::env::current_exe().context("could not find the running xtask program")?;
    let target = static_linux_target();
    Ok(vec![
        Step::new("all checks", xtask.to_string_lossy(), &["check", "all"]),
        // SQLite and PostgreSQL: the cluster is started, used and stopped around the tests.
        Step::new(
            "Rust tests",
            xtask.to_string_lossy(),
            &["db", "run", "--", "cargo", "test", "--workspace"],
        ),
        steps::web_build(root),
        Step::web(root, "browser unit tests", &["test"]),
        // The end-to-end tests start the program themselves (ADR-0023).
        steps::server_build(Profile::Debug),
        Step::web(root, "browser end-to-end tests", &["run", "test:e2e"]),
        linux_only(Step::new(
            "add the static Linux target",
            "rustup",
            &["target", "add", target],
        )),
        linux_only(Step::new(
            "build the static Linux program",
            xtask.to_string_lossy(),
            &["build", "--target", target],
        )),
    ])
}

pub(crate) fn run(root: &Path) -> anyhow::Result<bool> {
    Ok(steps::run(root, &plan(root)?)?.passed)
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
                "build the bugaboo program",
                "browser end-to-end tests",
                "add the static Linux target",
                "build the static Linux program"
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
        let on_linux = !cfg!(target_os = "linux");
        assert_eq!(
            skipped,
            [false, false, true, true, false, true, on_linux, on_linux]
        );
    }
}
