//! The public identity of a work folder (ADR-0036): `cargo xtask setup` copies it into the local
//! Git configuration of the folder, so the hooks accept the commits made there.
//!
//! The identity comes from the first of: the one the folder already has (when it is public), the
//! variable `BUGABOO_PUBLIC_IDENTITY` (`Name <email>`), and the local configuration of the master
//! (the `origin` remote, when that is a folder). Nothing is written in the code: the value is
//! the maintainer's configuration.

use std::path::Path;
use std::process::Command;

use crate::checks::sec_private::Identity;
use crate::output;
use crate::repo::Repo;

/// The variable that names the public identity (`Name <email>`).
const IDENTITY_VARIABLE: &str = "BUGABOO_PUBLIC_IDENTITY";

/// What setup does about the identity.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Choice {
    /// The folder has a public identity already.
    Keep,
    /// Write this one into the local configuration.
    Set(Identity),
    /// There is none to copy.
    Missing,
}

/// The identity to use: the folder's own when it is public, else the first public one of the
/// sources, in order.
pub(crate) fn choose(local: Option<&Identity>, sources: Vec<Option<Identity>>) -> Choice {
    if local.is_some_and(Identity::is_public) {
        return Choice::Keep;
    }
    sources
        .into_iter()
        .flatten()
        .find(Identity::is_public)
        .map_or(Choice::Missing, Choice::Set)
}

/// The value of a Git setting of the repository in `folder` (its own file only), if there is one.
fn local_setting(folder: &Path, key: &str) -> Option<String> {
    let out = Command::new("git")
        .args(["config", "--local", "--get", key])
        .current_dir(folder)
        .output()
        .ok()?;
    let value = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    (out.status.success() && !value.is_empty()).then_some(value)
}

/// The identity in the local configuration of the repository in `folder`.
fn identity_in(folder: &Path) -> Option<Identity> {
    Some(Identity {
        name: local_setting(folder, "user.name")?,
        email: local_setting(folder, "user.email")?,
    })
}

/// Copies the public identity into the local configuration of the folder, and says what it did.
/// A missing identity is not an error: the commit-msg hook refuses the commits instead.
pub(crate) fn copy_into(repo: &Repo) {
    let configured = std::env::var(IDENTITY_VARIABLE)
        .ok()
        .and_then(|value| Identity::parse(&value));
    output::info(&apply(repo.root(), configured));
}

/// Does the work of [`copy_into`] for the folder, with the identity of the variable given.
fn apply(root: &Path, configured: Option<Identity>) -> String {
    // The master is the folder the `origin` remote points to.
    let master = local_setting(root, "remote.origin.url")
        .map(std::path::PathBuf::from)
        .filter(|path| path.is_dir())
        .and_then(|path| identity_in(&path));
    match choose(identity_in(root).as_ref(), vec![configured, master]) {
        Choice::Keep => "ok       public Git identity".to_owned(),
        Choice::Set(identity) => {
            let done = [
                ("user.name", &identity.name),
                ("user.email", &identity.email),
            ]
            .iter()
            .all(|(key, value)| {
                Command::new("git")
                    .args(["config", "--local", key, value])
                    .current_dir(root)
                    .status()
                    .is_ok_and(|status| status.success())
            });
            if done {
                "ok       public Git identity copied into this folder".to_owned()
            } else {
                "FAILED   could not write the Git identity of this folder".to_owned()
            }
        }
        Choice::Missing => format!(
            "MISSING  public Git identity: set user.name and user.email (the GitHub noreply              address), or the variable {IDENTITY_VARIABLE}; commits are refused without it"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    fn identity(name: &str, email: &str) -> Identity {
        Identity {
            name: name.to_owned(),
            email: email.to_owned(),
        }
    }

    fn public() -> Identity {
        identity("mira", "1+mira@users.noreply.github.com")
    }

    fn private() -> Identity {
        identity("Mira Made-Up", "mira@example.test")
    }

    #[test]
    fn a_public_identity_of_the_folder_is_kept() {
        assert_eq!(
            choose(
                Some(&public()),
                vec![Some(identity("x", "1+x@users.noreply.github.com"))]
            ),
            Choice::Keep
        );
    }

    #[test]
    fn a_private_or_missing_identity_is_replaced_by_the_first_public_source() {
        let other = identity("lee", "2+lee@users.noreply.github.com");
        for local in [None, Some(&private())] {
            // A private source is never copied.
            assert_eq!(
                choose(
                    local,
                    vec![None, Some(private()), Some(other.clone()), Some(public())]
                ),
                Choice::Set(other.clone())
            );
        }
        assert_eq!(choose(None, vec![None, Some(private())]), Choice::Missing);
    }

    #[test]
    fn the_identity_of_the_master_is_copied_into_a_work_folder() {
        let master = TempDir::new("identity-master");
        assert!(master.git(&["init", "--quiet"]));
        assert!(master.git(&["config", "user.name", "mira"]));
        assert!(master.git(&["config", "user.email", "1+mira@users.noreply.github.com"]));
        let work = TempDir::new("identity-work");
        assert!(work.git(&["init", "--quiet"]));
        assert!(work.git(&["config", "user.name", "Mira Made-Up"]));
        assert!(work.git(&["config", "user.email", "mira@example.test"]));
        let url = master.path().to_string_lossy().into_owned();
        assert!(work.git(&["remote", "add", "origin", &url]));
        assert!(apply(work.path(), None).contains("copied"));
        assert_eq!(identity_in(work.path()), Some(public()));
        // A second run changes nothing.
        assert!(apply(work.path(), None).starts_with("ok       public"));
    }

    #[test]
    fn without_a_master_the_variable_is_used_and_a_private_value_is_not() {
        let work = TempDir::new("identity-variable");
        assert!(work.git(&["init", "--quiet"]));
        assert!(apply(work.path(), Some(private())).starts_with("MISSING"));
        assert_eq!(identity_in(work.path()), None);
        assert!(apply(work.path(), Some(public())).contains("copied"));
        assert_eq!(identity_in(work.path()), Some(public()));
    }
}
