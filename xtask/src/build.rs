//! `cargo xtask build`: the browser app, then the release binary (DIST-01).

use std::path::Path;

use crate::steps::{self, Step, WEB_APP};

/// Release builds embed `web/app/dist`, so the web build comes first.
pub(crate) fn plan(root: &Path) -> Vec<Step> {
    vec![
        Step::web(
            root,
            "build the browser app",
            &["run", "build", "-w", WEB_APP],
        ),
        Step::new(
            "build the bugaboo binary",
            "cargo",
            &["build", "--release", "--package", "bb-server"],
        ),
    ]
}

pub(crate) fn run(root: &Path) -> anyhow::Result<bool> {
    steps::run(root, &plan(root))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_web_build_comes_before_the_release_binary() {
        let root = tempfile::tempdir().unwrap();
        let plan = plan(root.path());
        assert_eq!(plan[0].name, "build the browser app");
        assert!(plan[0].skip.is_some(), "no web/app in the empty folder");
        assert!(plan[1].args.contains(&"--release".to_owned()));
        assert_eq!(plan[1].skip, None);
    }
}
