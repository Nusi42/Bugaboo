//! `cargo xtask build`: the browser app, then the release binary (DIST-01).

use std::path::Path;

use crate::steps::{self, Profile, Step};

/// Release builds embed `web/app/dist`, so the web build comes first.
pub(crate) fn plan(root: &Path, target: Option<&str>) -> Vec<Step> {
    let mut server = steps::server_build(Profile::Release);
    if let Some(target) = target {
        server
            .args
            .extend(["--target".to_owned(), target.to_owned()]);
    }
    vec![steps::web_build(root), server]
}

pub(crate) fn run(root: &Path, target: Option<&str>) -> anyhow::Result<bool> {
    Ok(steps::run(root, &plan(root, target))?.passed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_web_build_comes_before_the_release_binary() {
        let root = tempfile::tempdir().unwrap();
        let plan = plan(root.path(), None);
        assert_eq!(plan[0].name, "build the browser app");
        assert!(plan[0].skip.is_some(), "no web/app in the empty folder");
        assert!(plan[1].args.contains(&"--release".to_owned()));
        assert_eq!(plan[1].skip, None);
    }

    #[test]
    fn another_system_is_built_with_target() {
        let root = tempfile::tempdir().unwrap();
        let plan = plan(root.path(), Some("x86_64-unknown-linux-musl"));
        let args = &plan[1].args;
        let at = args.iter().position(|arg| arg == "--target").unwrap();
        assert_eq!(args[at + 1], "x86_64-unknown-linux-musl");
    }
}
