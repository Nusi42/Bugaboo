//! `cargo xtask visual update`: takes the reference pictures of the views again where they changed
//! (architecture §11, *Visual tests*).
//!
//! It makes the sample that ends at a fixed moment, builds the browser app and the program, and
//! runs the Playwright project of the pictures with `--update-snapshots=changed`: a picture that
//! differs from its reference by more than the allowed share, or has none, is written new; the
//! others stay as they are. The new pictures are looked at and reviewed like code, with the before
//! and after named in the report. The pictures are compared on Windows only, where they are
//! taken (the fonts of another system differ), so elsewhere this command refuses.

use std::path::Path;

use anyhow::Context as _;

use crate::steps::{self, Profile, Step};

/// The Playwright project of the pictures (`web/tooling/playwright.config.ts`).
pub(crate) const PROJECT: &str = "visual";
/// Why the pictures are not compared on this system.
pub(crate) const ONLY_ON_WINDOWS: &str =
    "the pictures are taken and compared on Windows only (the fonts of other systems differ)";

/// The step that makes the sample of the pictures: on Windows only, where they are compared.
pub(crate) fn fixed_sample(xtask: &str) -> Step {
    let step = Step::new(
        "create the sample at a fixed moment",
        xtask,
        &["dataset", "sample", "--fixed"],
    );
    if cfg!(windows) {
        step
    } else {
        Step {
            skip: Some(ONLY_ON_WINDOWS),
            ..step
        }
    }
}

/// What `visual update` runs.
pub(crate) fn plan(root: &Path) -> anyhow::Result<Vec<Step>> {
    let xtask = std::env::current_exe().context("could not find the running xtask program")?;
    let project = format!("--project={PROJECT}");
    Ok(vec![
        fixed_sample(&xtask.to_string_lossy()),
        steps::web_build(root),
        steps::server_build(Profile::Debug),
        Step::web(
            root,
            "take the pictures that changed",
            &[
                "run",
                "test:e2e",
                "--",
                &project,
                "--update-snapshots=changed",
            ],
        ),
    ])
}

/// Runs `visual update`; refuses on a system other than Windows.
pub(crate) fn update(root: &Path) -> anyhow::Result<bool> {
    if !cfg!(windows) {
        anyhow::bail!("`cargo xtask visual update` runs on Windows only: {ONLY_ON_WINDOWS}.");
    }
    Ok(steps::run(root, &plan(root)?)?.passed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_makes_the_sample_builds_and_takes_only_the_changed_pictures() {
        let root = tempfile::tempdir().unwrap();
        let plan = plan(root.path()).unwrap();
        let names: Vec<_> = plan.iter().map(|step| step.name).collect();
        assert_eq!(
            names,
            [
                "create the sample at a fixed moment",
                "build the browser app",
                "build the bugaboo program",
                "take the pictures that changed"
            ]
        );
        let take = plan.last().unwrap();
        assert!(take.args.contains(&"--project=visual".to_owned()));
        assert!(take.args.contains(&"--update-snapshots=changed".to_owned()));
        assert_eq!(
            plan[0].args,
            ["dataset", "sample", "--fixed"].map(str::to_owned)
        );
    }
}
