//! `cargo xtask guard-master [<master>]`: marks a checkout as the master (ADR-0039).
//!
//! The command runs in a work folder, because the master builds nothing. It writes, into the
//! master, only files that Git ignores or keeps inside `.git`:
//!
//! - `.env.local` with `BUGABOO_CHECKOUT_ROLE=master`, which every xtask command reads;
//! - the hook `pre-push` in `.git/bugaboo-hooks/`, which runs `check push` with the xtask program
//!   of the work folder that ran this command, and the setting `core.hooksPath` that makes Git
//!   use that folder (so the hook runs also when another hook folder is configured);
//! - a program for Cargo's `rustc-wrapper` in the same folder and `.cargo/config.local.toml`
//!   pointing to it, so even a plain `cargo build` in the master stops at once.
//!
//! Running it again changes nothing that is right already.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, bail};

use crate::location::{ENV_FILE, MASTER, ROLE_VARIABLE};
use crate::repo::Repo;

/// The folder below `.git` that holds the hook and the build-refusing program.
const HOOKS_FOLDER: &str = "bugaboo-hooks";
/// The first line of a Cargo setting file this command wrote.
const WRITTEN_MARK: &str = "# Written by `cargo xtask guard-master`; Git ignores this file.";
const CARGO_CONFIG: &str = ".cargo/config.local.toml";

/// The text of `.env.local` with `key` set to `value`: the line is replaced if the key is there,
/// else added; the other lines stay.
pub(crate) fn with_setting(text: &str, key: &str, value: &str) -> String {
    let mut done = false;
    let mut lines: Vec<String> = text
        .lines()
        .map(|line| {
            let is_key = line
                .split_once('=')
                .is_some_and(|(found, _)| found.trim() == key);
            if is_key && !line.trim_start().starts_with('#') {
                done = true;
                format!("{key}={value}")
            } else {
                line.to_owned()
            }
        })
        .collect();
    if !done {
        lines.push(format!("{key}={value}"));
    }
    lines.join("\n") + "\n"
}

/// A path as the scripts and Cargo write it: with forward slashes.
fn slashed(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// The hook of the master.
fn hook_text(xtask: &Path) -> anyhow::Result<String> {
    let exe = slashed(xtask);
    if exe.contains('\'') {
        bail!("The path of the xtask program has a quote in it; move the work folder.");
    }
    Ok(format!(
        "#!/bin/sh\n\
         # Written by `cargo xtask guard-master`. The master builds nothing: this runs the checks of\n\
         # a push with the xtask program that a work folder built.\n\
         xtask='{exe}'\n\
         [ -x \"$xtask\" ] || {{ echo \"The program $xtask is gone. Run cargo xtask guard-master in a work folder again.\" >&2; exit 1; }}\n\
         exec \"$xtask\" check push \"$@\"\n"
    ))
}

/// The program Cargo runs instead of the compiler, and its file name.
fn refuse_program() -> (&'static str, &'static str) {
    if cfg!(windows) {
        (
            "refuse-build.cmd",
            "@echo off\r\necho This checkout is the master: nothing is built here. Work in a work folder (ADR-0039). 1>&2\r\nexit /b 1\r\n",
        )
    } else {
        (
            "refuse-build.sh",
            "#!/bin/sh\necho 'This checkout is the master: nothing is built here. Work in a work folder (ADR-0039).' >&2\nexit 1\n",
        )
    }
}

fn write_executable(path: &Path, text: &str) -> anyhow::Result<()> {
    fs::write(path, text).with_context(|| format!("could not write {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

/// Marks the checkout at `master` as the master, with the hook running `xtask`. Returns what it
/// did, one line each.
pub(crate) fn mark(master: &Path, xtask: &Path) -> anyhow::Result<Vec<String>> {
    let git_folder = master.join(".git");
    if !git_folder.is_dir() {
        bail!(
            "{} is not a Git checkout with its own .git folder.",
            master.display()
        );
    }
    let config_path = master.join(CARGO_CONFIG);
    let existing = fs::read_to_string(&config_path).unwrap_or_default();
    if !existing.is_empty() && !existing.starts_with(WRITTEN_MARK) {
        bail!(
            "{CARGO_CONFIG} exists and was not written by this command; add \n             `rustc-wrapper = \"{}\"` under `[build]` by hand.",
            slashed(&git_folder.join(HOOKS_FOLDER).join(refuse_program().0))
        );
    }
    let mut done = Vec::new();

    let env_path = master.join(ENV_FILE);
    let before = fs::read_to_string(&env_path).unwrap_or_default();
    fs::write(&env_path, with_setting(&before, ROLE_VARIABLE, MASTER))?;
    done.push(format!("{ENV_FILE}: {ROLE_VARIABLE}={MASTER}"));

    let hooks = git_folder.join(HOOKS_FOLDER);
    fs::create_dir_all(&hooks)?;
    write_executable(&hooks.join("pre-push"), &hook_text(xtask)?)?;
    let hooks_path = slashed(&hooks);
    let set = Command::new("git")
        .args(["config", "--local", "core.hooksPath", &hooks_path])
        .current_dir(master)
        .status()
        .context("could not run git")?;
    if !set.success() {
        bail!("could not set core.hooksPath in the master.");
    }
    done.push(
        "core.hooksPath: the pre-push hook of the master runs the checks that build nothing"
            .to_owned(),
    );

    let (name, text) = refuse_program();
    let program = hooks.join(name);
    write_executable(&program, text)?;
    fs::create_dir_all(master.join(".cargo"))?;
    fs::write(
        &config_path,
        format!(
            "{WRITTEN_MARK}\n[build]\nrustc-wrapper = \"{}\"\n",
            slashed(&program)
        ),
    )?;
    done.push(format!("{CARGO_CONFIG}: Cargo refuses to compile"));
    Ok(done)
}

/// `cargo xtask guard-master [<master>]`: the master is the folder named, or the folder that the
/// `origin` remote of this checkout points to.
pub(crate) fn run(repo: &Repo, master: Option<PathBuf>) -> anyhow::Result<Vec<String>> {
    let master = if let Some(path) = master {
        path
    } else {
        let url = repo
            .git_bytes(&["config", "--get", "remote.origin.url"])
            .context("this checkout has no `origin` remote; name the master folder")?;
        PathBuf::from(String::from_utf8_lossy(&url).trim())
    };
    let master = std::path::absolute(&master).unwrap_or(master);
    if !master.is_dir() {
        bail!(
            "The master {} is not a folder on this computer; name it as the argument.",
            master.display()
        );
    }
    let xtask = std::env::current_exe().context("could not find the running xtask program")?;
    mark(&master, &xtask)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::location::{self, Role};
    use crate::testutil::TempDir;

    #[test]
    fn a_setting_is_replaced_or_added_and_the_rest_stays() {
        assert_eq!(with_setting("", "A", "1"), "A=1\n");
        assert_eq!(with_setting("# c\nA=0\nB=2\n", "A", "1"), "# c\nA=1\nB=2\n");
        assert_eq!(with_setting("B=2\n", "A", "1"), "B=2\nA=1\n");
        // A comment that names the key is not the key.
        assert_eq!(with_setting("# A=0\n", "A", "1"), "# A=0\nA=1\n");
    }

    #[test]
    fn the_master_is_marked_its_hook_is_wired_and_a_second_run_changes_nothing() {
        let master = TempDir::new("guard-master");
        assert!(master.git(&["init", "--quiet"]));
        let tool = master.path().join("made-up-xtask");
        let first = mark(master.path(), &tool).unwrap();
        assert_eq!(first.len(), 3);
        // The role is read back the way every command reads it.
        assert_eq!(location::role_of(master.path(), |_| None), Role::Master);
        // Git runs the hook folder: the setting is there and the hook calls the push checks.
        let hooks = master.path().join(".git").join(HOOKS_FOLDER);
        let out = Command::new("git")
            .args(["config", "--local", "--get", "core.hooksPath"])
            .current_dir(master.path())
            .output()
            .unwrap();
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), slashed(&hooks));
        let hook = fs::read_to_string(hooks.join("pre-push")).unwrap();
        assert!(hook.contains("check push \"$@\"") && hook.contains("made-up-xtask"));
        let config = fs::read_to_string(master.path().join(CARGO_CONFIG)).unwrap();
        assert!(config.contains("rustc-wrapper") && config.contains("refuse-build"));
        // Again: the same files.
        let env = fs::read_to_string(master.path().join(ENV_FILE)).unwrap();
        mark(master.path(), &tool).unwrap();
        assert_eq!(
            fs::read_to_string(master.path().join(ENV_FILE)).unwrap(),
            env
        );
        assert_eq!(
            fs::read_to_string(master.path().join(CARGO_CONFIG)).unwrap(),
            config
        );
    }

    #[test]
    fn other_settings_of_the_master_are_kept_or_refused_not_overwritten() {
        let master = TempDir::new("guard-master-keep");
        assert!(master.git(&["init", "--quiet"]));
        master.write(ENV_FILE, "OTHER=1\n");
        master.write(CARGO_CONFIG, "[build]\njobs = 2\n");
        let tool = master.path().join("made-up-xtask");
        // A Cargo setting file that is not ours stops the command before it writes it.
        assert!(mark(master.path(), &tool).is_err());
        assert_eq!(
            fs::read_to_string(master.path().join(CARGO_CONFIG)).unwrap(),
            "[build]\njobs = 2\n"
        );
        fs::remove_file(master.path().join(CARGO_CONFIG)).unwrap();
        mark(master.path(), &tool).unwrap();
        let env = fs::read_to_string(master.path().join(ENV_FILE)).unwrap();
        assert!(env.contains("OTHER=1") && env.contains("BUGABOO_CHECKOUT_ROLE=master"));
    }

    #[test]
    fn a_folder_without_git_is_refused() {
        let folder = TempDir::new("guard-master-none");
        assert!(mark(folder.path(), Path::new("x")).is_err());
        assert!(!folder.path().join(ENV_FILE).exists());
    }
}
