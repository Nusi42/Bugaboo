//! The Git hooks of a clone (ADR-0014): `cargo xtask setup` installs the hooks of `lefthook.yml`
//! with the pinned lefthook. A hooks folder that the global Git configuration names for every
//! repository is left as it is; the clone is told to use its own folder instead.

use std::path::Path;
use std::process::Command;

use anyhow::{Context as _, bail};

use crate::output;
use crate::tools;

/// The Git setting that names the folder of the hooks.
const HOOKS_PATH: &str = "core.hooksPath";

/// Makes Git run the checks on commit and push (`lefthook.yml`), using the pinned lefthook.
/// Running it again only rewrites the same hook files.
pub(crate) fn install(root: &Path) -> anyhow::Result<()> {
    if !root.join(".git").exists() || !root.join("lefthook.yml").is_file() {
        return Ok(());
    }
    let git = || {
        let mut command = tools::command("git");
        command.current_dir(root);
        command
    };
    let arguments = lefthook_arguments(&git)?;
    let lefthook = tools::bin_dir(root).join(tools::exe("lefthook"));
    let status = tools::command(&lefthook)
        .args(arguments)
        .current_dir(root)
        .status()
        .with_context(|| format!("could not run {}", lefthook.display()))?;
    if !status.success() {
        bail!("lefthook install failed (see the messages above).");
    }
    output::info("ok       Git hooks (lefthook)");
    Ok(())
}

/// The arguments of lefthook that install the hooks of this clone; `git` makes a Git command that
/// runs in it. When the global (or system) Git configuration names a hooks folder for every
/// repository, lefthook refuses to install. This clone is then told to use its own folder again,
/// with a setting of this clone only (the global one stays as it is), and lefthook installs there.
/// A hooks folder that this clone names for itself is never replaced: the message names the two
/// ways on.
fn lefthook_arguments(git: &dyn Fn() -> Command) -> anyhow::Result<&'static [&'static str]> {
    let setting = |scope: &[&str]| -> Option<String> {
        let out = git()
            .arg("config")
            .args(scope)
            .args(["--get", HOOKS_PATH])
            .output()
            .ok()?;
        let value = String::from_utf8_lossy(&out.stdout).trim().to_owned();
        (out.status.success() && !value.is_empty()).then_some(value)
    };
    if setting(&[]).is_none() {
        return Ok(&["install"]);
    }
    let out = git()
        .args(["rev-parse", "--git-common-dir"])
        .output()
        .context("could not run git")?;
    let git_folder = String::from_utf8_lossy(&out.stdout)
        .trim()
        .replace('\\', "/");
    if !out.status.success() || git_folder.is_empty() {
        bail!("`git rev-parse --git-common-dir` failed in this clone.");
    }
    let own = format!("{git_folder}/hooks");
    match setting(&["--local"]) {
        Some(local) if local.replace('\\', "/") == own => {}
        Some(local) => bail!(
            "This clone names its own hooks folder ({HOOKS_PATH} = {local}). Choose one:\n  \
             git config --local --unset {HOOKS_PATH}    (then run `cargo xtask setup` again)\n  \
             {}/bin/lefthook install --force    (puts the hooks into {local})",
            tools::TOOLS_DIR
        ),
        None => {
            let status = git()
                .args(["config", "--local", HOOKS_PATH, &own])
                .status()
                .context("could not run git")?;
            if !status.success() {
                bail!("could not set {HOOKS_PATH} of this clone.");
            }
            output::info(&format!(
                "note     your Git configuration names a hooks folder for every repository; this \
                 clone now uses its own ({HOOKS_PATH} = {own}, set for this clone only)"
            ));
        }
    }
    Ok(&["install", "--force"])
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::testutil::TempDir;

    /// A clone in a throw-away folder whose Git reads `global` as the global configuration and no
    /// system configuration: the real global configuration is never read or changed.
    struct ScratchRepo {
        folder: TempDir,
        global: std::path::PathBuf,
    }

    impl ScratchRepo {
        fn new(name: &str, global: &str) -> Self {
            let folder = TempDir::new(name);
            let global = folder.write("global.gitconfig", global);
            let clone = Self { folder, global };
            fs::create_dir_all(clone.repo()).unwrap();
            let status = clone.git().args(["init", "--quiet"]).status().unwrap();
            assert!(status.success());
            clone
        }

        fn repo(&self) -> std::path::PathBuf {
            self.folder.path().join("repo")
        }

        fn git(&self) -> Command {
            let mut command = tools::command("git");
            crate::repo::isolate_git(&mut command);
            command
                .env("GIT_CONFIG_GLOBAL", &self.global)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .current_dir(self.repo());
            command
        }

        fn local_hooks_path(&self) -> Option<String> {
            let out = self
                .git()
                .args(["config", "--local", "--get", HOOKS_PATH])
                .output()
                .unwrap();
            out.status
                .success()
                .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
        }
    }

    #[test]
    fn good_example_without_a_hooks_folder_lefthook_installs_as_usual() {
        let clone = ScratchRepo::new("setup-hooks-plain", "");
        assert_eq!(lefthook_arguments(&|| clone.git()).unwrap(), ["install"]);
        assert_eq!(clone.local_hooks_path(), None);
    }

    #[test]
    fn a_global_hooks_folder_is_left_alone_and_this_clone_uses_its_own() {
        let global = "[core]\n\thooksPath = .githooks\n";
        let clone = ScratchRepo::new("setup-hooks-global", global);
        assert_eq!(
            lefthook_arguments(&|| clone.git()).unwrap(),
            ["install", "--force"]
        );
        assert_eq!(clone.local_hooks_path().as_deref(), Some(".git/hooks"));
        assert_eq!(fs::read_to_string(&clone.global).unwrap(), global);
        // A second run finds the setting of the clone and changes nothing.
        assert_eq!(
            lefthook_arguments(&|| clone.git()).unwrap(),
            ["install", "--force"]
        );
        assert_eq!(clone.local_hooks_path().as_deref(), Some(".git/hooks"));
    }

    #[test]
    fn bad_example_a_hooks_folder_of_the_clone_itself_stops_with_the_two_ways_on() {
        let clone = ScratchRepo::new("setup-hooks-local", "");
        let status = clone
            .git()
            .args(["config", "--local", HOOKS_PATH, "my-hooks"])
            .status()
            .unwrap();
        assert!(status.success());
        let error = lefthook_arguments(&|| clone.git()).unwrap_err().to_string();
        assert!(
            error.contains("git config --local --unset core.hooksPath"),
            "{error}"
        );
        assert!(error.contains("lefthook install --force"), "{error}");
        assert_eq!(clone.local_hooks_path().as_deref(), Some("my-hooks"));
    }
}
