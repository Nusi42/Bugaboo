//! `cargo xtask build`: the browser app, then the release binary (DIST-01).

use std::path::Path;

use crate::steps::{self, Profile, Step};

/// Release builds embed `web/app/dist`, so the web build comes first.
pub(crate) fn plan(root: &Path) -> Vec<Step> {
    vec![
        steps::web_build(root),
        steps::server_build(Profile::Release),
    ]
}

pub(crate) fn run(root: &Path) -> anyhow::Result<bool> {
    Ok(steps::run(root, &plan(root))?.passed)
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
