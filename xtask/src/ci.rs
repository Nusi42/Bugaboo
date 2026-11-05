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

/// A step that needs the browser app (the load run serves it) is skipped while `web/app` does not exist.
fn needs_browser_app(root: &Path, step: Step) -> Step {
    if steps::web_app_exists(root) {
        step
    } else {
        Step {
            skip: Some("web/app does not exist yet"),
            ..step
        }
    }
}

pub(crate) fn plan(root: &Path) -> anyhow::Result<Vec<Step>> {
    let xtask = std::env::current_exe().context("could not find the running xtask program")?;
    let target = static_linux_target();
    Ok(vec![
        // First: PERF-BUNDLE measures the build, so the checks need it.
        steps::web_build(root),
        Step::new("all checks", xtask.to_string_lossy(), &["check", "all"]),
        // One core, about five minutes: it runs beside the Rust tests and the steps up to the
        // browser tests, which wait for it (its log is shown where they wait).
        Step::new(
            "create the sample organization",
            xtask.to_string_lossy(),
            &["dataset", "sample"],
        )
        .in_background(),
        // SQLite and PostgreSQL: the cluster is started, used and stopped around the tests.
        Step::new(
            "Rust tests",
            xtask.to_string_lossy(),
            &["db", "run", "--", "cargo", "nextest", "run", "--workspace"],
        ),
        // The runner does not run the examples in the documentation comments; cargo does. They need
        // no database.
        Step::new("Rust doc tests", "cargo", &["test", "--workspace", "--doc"]),
        // The sample organization is made twice on each database and compared (TEST-SAMPLE-DATA).
        Step::new(
            "sample organization",
            xtask.to_string_lossy(),
            &["dataset", "sample", "--check"],
        ),
        // A small large dataset is made twice on each database, holds the rules of the large dataset and
        // is the same both times (PERF-BENCH, architecture section 11).
        Step::new(
            "large dataset",
            xtask.to_string_lossy(),
            &["dataset", "large", "--check"],
        ),
        Step::web(root, "browser unit tests", &["test"]),
        // The end-to-end tests start the program themselves (ADR-0023), some of them on a copy of
        // a fresh sample organization.
        steps::server_build(Profile::Debug),
        Step::web(root, "browser end-to-end tests", &["run", "test:e2e"]).after_background(),
        // Ten people work on the same issues at speed for two minutes, on each database
        // (TEST-CONCURRENT); the long run is `cargo xtask load --nightly`.
        needs_browser_app(
            root,
            Step::new("ten people at once", xtask.to_string_lossy(), &["load"]),
        ),
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
                "build the browser app",
                "all checks",
                "create the sample organization",
                "Rust tests",
                "Rust doc tests",
                "sample organization",
                "large dataset",
                "browser unit tests",
                "build the bugaboo program",
                "browser end-to-end tests",
                "ten people at once",
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
            [
                true, false, false, false, false, false, false, true, false, true, true, on_linux,
                on_linux
            ]
        );
    }

    #[test]
    fn the_sample_is_made_in_the_background_and_the_browser_tests_wait_for_it() {
        let root = tempfile::tempdir().unwrap();
        let steps = plan(root.path()).unwrap();
        let find = |name: &str| steps.iter().position(|step| step.name == name).unwrap();
        let sample = &steps[find("create the sample organization")];
        assert!(sample.background);
        // It starts before the Rust tests and ends before the browser tests start.
        assert!(find("create the sample organization") < find("Rust tests"));
        assert!(steps[find("browser end-to-end tests")].joins_background);
        assert!(find("create the sample organization") < find("browser end-to-end tests"));
        // No other step runs in the background.
        assert_eq!(steps.iter().filter(|step| step.background).count(), 1);
    }
}
