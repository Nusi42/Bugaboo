//! `cargo xtask build`: the parts list and the notices, the browser app, then the release binary
//! (DIST-01, COMP-01).

use std::path::Path;

use crate::steps::{self, Profile, Step};
use crate::{build_paths, release};

/// Release builds embed `web/app/dist`, so the web build comes first. The program names no folder
/// of this computer (SEC-BUILD-PATHS).
pub(crate) fn plan(root: &Path, target: Option<&str>) -> anyhow::Result<Vec<Step>> {
    let server = build_paths::neutral(steps::server_build(Profile::Release), root)?;
    Ok(plan_with(root, target, server))
}

/// The plan around the build of the program, `server`.
fn plan_with(root: &Path, target: Option<&str>, mut server: Step) -> Vec<Step> {
    if let Some(target) = target {
        server
            .args
            .extend(["--target".to_owned(), target.to_owned()]);
    }
    vec![steps::web_build(root), server]
}

pub(crate) fn run(root: &Path, target: Option<&str>) -> anyhow::Result<bool> {
    // The parts list and the notices come first: the program takes them in when it is compiled.
    let files = release::compose(root)?;
    release::write_embedded(&files, &root.join(release::EMBEDDED_FOLDER))?;
    Ok(steps::run(root, &plan(root, target)?)?.passed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_web_build_comes_before_the_release_binary() {
        let root = tempfile::tempdir().unwrap();
        let plan = plan_with(root.path(), None, steps::server_build(Profile::Release));
        assert_eq!(plan[0].name, "build the browser app");
        assert!(plan[0].skip.is_some(), "no web/app in the empty folder");
        assert!(plan[1].args.contains(&"--release".to_owned()));
        assert_eq!(plan[1].skip, None);
    }

    #[test]
    fn sec_18_the_release_binary_names_no_folder_of_this_computer() {
        let plan = plan(&release::repository_root(), None).unwrap();
        let args = &plan[1].args;
        let at = args.iter().position(|arg| arg == "--config").unwrap();
        assert!(
            args[at + 1].contains("--remap-path-prefix=") && args[at + 1].contains("=/cargo"),
            "the setting after --config renames no folder, or not the home of cargo"
        );
    }

    #[test]
    fn another_system_is_built_with_target() {
        let root = tempfile::tempdir().unwrap();
        let plan = plan_with(
            root.path(),
            Some("x86_64-unknown-linux-musl"),
            steps::server_build(Profile::Release),
        );
        let args = &plan[1].args;
        let at = args.iter().position(|arg| arg == "--target").unwrap();
        assert_eq!(args[at + 1], "x86_64-unknown-linux-musl");
    }
}
