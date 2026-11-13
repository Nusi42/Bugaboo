//! The Git hooks of a clone (ADR-0014): `cargo xtask setup` installs the hooks of `lefthook.yml`
//! with the pinned lefthook. A hooks folder that the global Git configuration names for every
//! repository is left as it is; the clone is told to use its own folder instead.

use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context as _, bail};

use crate::output;
use crate::tools;

/// The Git setting that names the folder of the hooks.
const HOOKS_PATH: &str = "core.hooksPath";

/// The command that undoes the setting of the clone.
const UNSET: &str = "git config --local --unset core.hooksPath";

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
    let plan = plan(root, &git)?;
    if let Some(note) = &plan.note {
        output::note(note);
    }
    let lefthook = tools::bin_dir(root).join(tools::exe("lefthook"));
    let status = tools::command(&lefthook)
        .args(plan.arguments)
        .current_dir(root)
        .status()
        .with_context(|| format!("could not run {}", lefthook.display()))?;
    if !status.success() {
        bail!("lefthook install failed (see the messages above).");
    }
    output::info("ok       Git hooks (lefthook)");
    Ok(())
}

/// How lefthook installs the hooks of a clone.
#[derive(Debug)]
struct Plan {
    /// The arguments of lefthook.
    arguments: &'static [&'static str],
    /// What the person should know about a change to the Git settings of the clone.
    note: Option<String>,
}

/// How to install the hooks of the clone at `root`; `git` makes a Git command that runs in it.
///
/// When the global (or system) Git configuration names a hooks folder for every repository,
/// lefthook refuses to install. The clone is then told to use its own folder again, with a setting
/// of the clone only (the global one stays as it is), and lefthook installs there with `--force`.
/// The note says that the hooks of the global folder no longer run in this clone and how to undo
/// it; setup does not ask, because agents run it unattended. `--force` would overwrite a kept copy
/// (`<hook>.old`) of a hook of the person's own, so setup stops first and names it. A hooks folder
/// that the clone names for itself is never replaced: the message names the two ways on.
fn plan(root: &Path, git: &dyn Fn() -> Command) -> anyhow::Result<Plan> {
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
        return Ok(Plan {
            arguments: &["install"],
            note: None,
        });
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
    let local = setting(&["--local"]);
    if let Some(local) = local
        .as_ref()
        .filter(|local| local.replace('\\', "/") != own)
    {
        bail!(
            "This clone names its own hooks folder ({HOOKS_PATH} = {local}). Choose one:\n  \
             {UNSET}    (then run the command again)\n  \
             {}/bin/lefthook install --force    (puts the hooks into {local})",
            tools::TOOLS_DIR
        );
    }
    refuse_kept_copies(&root.join(&own))?;
    let note = match local {
        Some(_) => None,
        None => {
            let status = git()
                .args(["config", "--local", HOOKS_PATH, &own])
                .status()
                .context("could not run git")?;
            if !status.success() {
                bail!("could not set {HOOKS_PATH} of this clone.");
            }
            let everywhere = setting(&["--global"]).map_or_else(
                || "The hooks folder that your system Git configuration names".to_owned(),
                |folder| {
                    format!(
                        "`{folder}`, the hooks folder that your global Git configuration names \
                         for every repository,"
                    )
                },
            );
            Some(format!(
                "{everywhere} no longer runs its hooks in this clone. So that the checks of \
                 Bugaboo run, this clone now uses its own folder ({HOOKS_PATH} = {own}, set for \
                 this clone only; your global setting is unchanged). To undo it: {UNSET}"
            ))
        }
    };
    Ok(Plan {
        arguments: &["install", "--force"],
        note,
    })
}

/// What every hook the pinned lefthook writes holds (its `LEFTHOOK` variables); a wrapper that only
/// runs `lefthook` does not.
const LEFTHOOK_MARK: &[u8] = b"LEFTHOOK";

/// Stops when `folder` holds a kept copy (`<hook>.old`) next to a hook that is not lefthook's:
/// lefthook keeps a hook it replaces under that name, and `--force` would overwrite the older copy.
/// When the hook is lefthook's own, it is replaced alone and the copy stays. A hook that cannot be
/// read counts as foreign.
fn refuse_kept_copies(folder: &Path) -> anyhow::Result<()> {
    let Ok(entries) = fs::read_dir(folder) else {
        return Ok(());
    };
    for path in entries.flatten().map(|entry| entry.path()) {
        if path.extension().is_none_or(|extension| extension != "old") {
            continue;
        }
        let hook = path.with_extension("");
        let own = fs::read(&hook).is_ok_and(|bytes| {
            bytes
                .windows(LEFTHOOK_MARK.len())
                .any(|part| part == LEFTHOOK_MARK)
        });
        if !own {
            bail!(
                "{} is a kept copy of an older hook, and installing the hooks would overwrite it \
                 with {}. Move or delete one of the two, then run the command again.",
                path.display(),
                hook.display()
            );
        }
    }
    Ok(())
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
        let plan = plan(&clone.repo(), &|| clone.git()).unwrap();
        assert_eq!(plan.arguments, ["install"]);
        assert_eq!(plan.note, None);
        assert_eq!(clone.local_hooks_path(), None);
    }

    #[test]
    fn a_global_hooks_folder_is_left_alone_and_this_clone_uses_its_own() {
        let global = "[core]\n\thooksPath = .githooks\n";
        let clone = ScratchRepo::new("setup-hooks-global", global);
        let first = plan(&clone.repo(), &|| clone.git()).unwrap();
        assert_eq!(first.arguments, ["install", "--force"]);
        let note = first.note.unwrap();
        assert!(note.contains(".githooks"), "{note}");
        assert!(
            note.contains("no longer runs its hooks in this clone"),
            "{note}"
        );
        assert!(
            note.contains("git config --local --unset core.hooksPath"),
            "{note}"
        );
        assert_eq!(clone.local_hooks_path().as_deref(), Some(".git/hooks"));
        assert_eq!(fs::read_to_string(&clone.global).unwrap(), global);
        // A second run finds the setting of the clone and changes nothing.
        let second = plan(&clone.repo(), &|| clone.git()).unwrap();
        assert_eq!(second.arguments, ["install", "--force"]);
        assert_eq!(second.note, None);
        assert_eq!(clone.local_hooks_path().as_deref(), Some(".git/hooks"));
    }

    #[test]
    fn bad_example_a_kept_copy_of_a_hook_of_its_own_stops_before_it_is_overwritten() {
        let global = "[core]\n\thooksPath = .githooks\n";
        let clone = ScratchRepo::new("setup-hooks-old", global);
        let hooks = clone.repo().join(".git").join("hooks");
        fs::write(hooks.join("pre-commit"), "#!/bin/sh\necho mine\n").unwrap();
        fs::write(hooks.join("pre-commit.old"), "#!/bin/sh\necho older\n").unwrap();
        let error = plan(&clone.repo(), &|| clone.git())
            .unwrap_err()
            .to_string();
        assert!(error.contains("pre-commit.old"), "{error}");
        assert_eq!(
            fs::read_to_string(hooks.join("pre-commit.old")).unwrap(),
            "#!/bin/sh\necho older\n"
        );
        // A wrapper that only names lefthook is not lefthook's hook: lefthook would keep it as `.old`.
        fs::write(
            hooks.join("pre-commit"),
            "#!/bin/sh\nnpx lefthook run pre-commit\n",
        )
        .unwrap();
        let error = plan(&clone.repo(), &|| clone.git())
            .unwrap_err()
            .to_string();
        assert!(error.contains("pre-commit.old"), "{error}");
        // A hook that is not text is foreign too.
        fs::write(hooks.join("pre-commit"), [0xff, 0xfe, b'L', 0x00]).unwrap();
        assert!(plan(&clone.repo(), &|| clone.git()).is_err());
        // Once lefthook's own hook is in place, its kept copy is safe: the hook is replaced alone.
        fs::write(
            hooks.join("pre-commit"),
            "#!/bin/sh\nif [ \"$LEFTHOOK\" = \"0\" ]; then exit 0; fi\n",
        )
        .unwrap();
        assert_eq!(
            plan(&clone.repo(), &|| clone.git()).unwrap().arguments,
            ["install", "--force"]
        );
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
        let error = plan(&clone.repo(), &|| clone.git())
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("git config --local --unset core.hooksPath"),
            "{error}"
        );
        assert!(error.contains("lefthook install --force"), "{error}");
        assert_eq!(clone.local_hooks_path().as_deref(), Some("my-hooks"));
    }
}
