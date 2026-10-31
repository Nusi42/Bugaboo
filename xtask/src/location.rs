//! BUILD-LOCATION: nothing is built in the master (ADR-0039).
//!
//! A checkout is the master only when it says so: `BUGABOO_CHECKOUT_ROLE=master` in the
//! environment or in the ignored file `.env.local` at its root. `cargo xtask guard-master` writes
//! it. The path of a checkout decides nothing, so a plain clone, also one in a synced folder,
//! builds, tests and commits as it is.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::bail;

/// The variable that marks a checkout, in the environment or in [`ENV_FILE`].
pub(crate) const ROLE_VARIABLE: &str = "BUGABOO_CHECKOUT_ROLE";
/// The value that marks the master.
pub(crate) const MASTER: &str = "master";
/// The ignored file of a checkout with its own settings (`KEY=value` lines).
pub(crate) const ENV_FILE: &str = ".env.local";

/// What a checkout is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Role {
    /// Holds `main` and publishes it; nothing is built there.
    Master,
    /// Any other checkout: builds, tests and commits.
    Work,
}

/// The settings of a `.env.local` text: `KEY=value` lines; blank lines and `#` lines are ignored,
/// and a value may be in quotes.
pub(crate) fn parse_env(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| {
            (
                key.trim().to_owned(),
                value.trim().trim_matches(['"', '\'']).to_owned(),
            )
        })
        .collect()
}

/// The settings of the `.env.local` of a checkout; none if the file is not there.
pub(crate) fn read_env(root: &Path) -> BTreeMap<String, String> {
    std::fs::read_to_string(root.join(ENV_FILE))
        .map(|text| parse_env(&text))
        .unwrap_or_default()
}

/// The role of the checkout at `root`: the environment `variable` first, then the file.
pub(crate) fn role_of(root: &Path, variable: impl Fn(&str) -> Option<String>) -> Role {
    let value = variable(ROLE_VARIABLE)
        .filter(|value| !value.trim().is_empty())
        .or_else(|| read_env(root).remove(ROLE_VARIABLE));
    match value {
        Some(value) if value.trim().eq_ignore_ascii_case(MASTER) => Role::Master,
        _ => Role::Work,
    }
}

/// The role of the checkout at `root`, from the real environment.
pub(crate) fn role(root: &Path) -> Role {
    role_of(root, |name| std::env::var(name).ok())
}

/// The top folder of the checkout the command runs in, when that is marked as the master.
pub(crate) fn master_here() -> Option<std::path::PathBuf> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    let top = std::path::PathBuf::from(String::from_utf8_lossy(&out.stdout).trim());
    (out.status.success()
        && read_env(&top)
            .get(ROLE_VARIABLE)
            .is_some_and(|value| value.eq_ignore_ascii_case(MASTER)))
    .then_some(top)
}

/// Stops xtask before it builds, installs or tests anything in the master. `allowed` is true for
/// the commands that build nothing (publishing checks, marking the master).
pub(crate) fn guard(root: &Path, allowed: bool) -> anyhow::Result<()> {
    if let Some(problem) = check(root, role(root)).filter(|_| !allowed) {
        bail!("{problem}");
    }
    Ok(())
}

/// The problem with building in the checkout at `root`, if there is one.
pub(crate) fn check(root: &Path, role: Role) -> Option<String> {
    (role == Role::Master).then(|| {
        format!(
            "This checkout ({}) is marked as the master ({ROLE_VARIABLE}={MASTER}). Nothing may be \
             built, installed or tested here; work in a clone under `<work root>` instead \
             (ADR-0039). If a `target` folder was created here, delete it.",
            root.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    fn none(_: &str) -> Option<String> {
        None
    }

    #[test]
    fn bad_example_a_checkout_marked_as_the_master() {
        let temp = TempDir::new("location-master");
        temp.write(ENV_FILE, "# mine\nBUGABOO_CHECKOUT_ROLE=\"master\"\n");
        let role = role_of(temp.path(), none);
        assert_eq!(role, Role::Master);
        assert!(check(temp.path(), role).is_some());
        // The environment marks it as well, and decides over the file.
        let empty = TempDir::new("location-master-env");
        let marked = |name: &str| (name == ROLE_VARIABLE).then(|| "Master".to_owned());
        assert_eq!(role_of(empty.path(), marked), Role::Master);
        temp.write(ENV_FILE, "BUGABOO_CHECKOUT_ROLE=work\n");
        assert_eq!(role_of(temp.path(), marked), Role::Master);
    }

    #[test]
    fn good_example_a_plain_clone_whatever_its_path() {
        // A synced folder, by its name only, is nothing special.
        let temp = TempDir::new("synced-folder-made-up");
        assert_eq!(role_of(temp.path(), none), Role::Work);
        assert!(check(temp.path(), Role::Work).is_none());
        temp.write(ENV_FILE, "BUGABOO_CHECKOUT_ROLE=work\nOTHER=1\n");
        assert_eq!(role_of(temp.path(), none), Role::Work);
    }

    #[test]
    fn the_master_may_run_only_the_commands_that_build_nothing() {
        let temp = TempDir::new("location-guard");
        temp.write(ENV_FILE, "BUGABOO_CHECKOUT_ROLE=master\n");
        // The guard reads the real environment; the file decides here.
        if role(temp.path()) == Role::Master {
            assert!(guard(temp.path(), false).is_err());
            assert!(guard(temp.path(), true).is_ok());
        }
    }

    #[test]
    fn the_settings_file_is_read_line_by_line() {
        let found = parse_env("# c\n\nA=1\n B = \"two words\" \nbroken\nC='3'\n");
        assert_eq!(found.get("A").map(String::as_str), Some("1"));
        assert_eq!(found.get("B").map(String::as_str), Some("two words"));
        assert_eq!(found.get("C").map(String::as_str), Some("3"));
        assert_eq!(found.len(), 3);
    }
}
