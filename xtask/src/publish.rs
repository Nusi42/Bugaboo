//! Where a push may go and which checks it runs (ADR-0039).
//!
//! A work folder pushes only to a remote that is a path (the master); only the master pushes to a
//! remote on the network (the public one). The pre-push hook (`cargo xtask check push <remote>
//! <url>`) asks [`plan`], the one place that decides.

use crate::location::Role;

/// The checks that build nothing, which the master runs before it publishes: the private-data
/// guard on the commits to push and on every tracked file, and the licences of the dependencies.
/// `main` only ever moves to a commit that passed the full run in a work folder.
pub(crate) const NO_BUILD_CHECKS: [&str; 2] = ["SEC-PRIVATE", "DEP-LICENSES"];

/// What a push does.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Plan {
    /// A work folder pushing to the master (or a push without a known remote): the checks of a
    /// push.
    Full,
    /// The master pushing to the public remote: only the checks of [`NO_BUILD_CHECKS`].
    NoBuild,
    /// Not allowed; the text says what to do instead.
    Refuse(String),
}

/// Whether the URL of a remote is a path on this computer (`C:\x`, `/x`, `../x`, `\\host\share`,
/// `file://…`, `~/x`, a relative path) and not an address on the network (`https://…`,
/// `ssh://…`, `git@host:path`).
pub(crate) fn is_local_remote(url: &str) -> bool {
    let url = url.trim();
    if url.starts_with("file://") {
        return true;
    }
    let bytes = url.as_bytes();
    let drive = bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/');
    if drive || url.starts_with(['/', '\\', '.', '~']) {
        return true;
    }
    // `scheme://…` and `host:path` have a colon; a relative path has none.
    !url.contains(':')
}

/// What a push from a checkout with this `role` to `url` does. Without a URL (the command run by
/// hand) a work folder runs the checks of a push and the master the checks that build nothing.
pub(crate) fn plan(role: Role, url: Option<&str>) -> Plan {
    let local = url.map(is_local_remote);
    match (role, local) {
        (Role::Work, None | Some(true)) => Plan::Full,
        (Role::Master, None | Some(false)) => Plan::NoBuild,
        (Role::Work, Some(false)) => Plan::Refuse(
            "A work folder pushes only to the master. The public remote is pushed from the \
             master: `git -C <master> push github main`."
                .to_owned(),
        ),
        (Role::Master, Some(true)) => Plan::Refuse(
            "The master pushes only to the public remote: `git -C <master> push github main`. \
             Work reaches the master from a work folder."
                .to_owned(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_remotes_are_paths_and_the_rest_are_addresses() {
        for local in [
            r"D:\code\made-up",
            "c:/made-up/bugaboo",
            "/srv/made-up",
            "../made-up",
            "./made-up",
            r"\\host\share\made-up",
            "file:///srv/made-up",
            "~/made-up",
            "made-up/bugaboo",
        ] {
            assert!(is_local_remote(local), "{local}");
        }
        for network in [
            "https://example.test/made-up/bugaboo.git",
            "ssh://git@example.test/made-up/bugaboo.git",
            "git@example.test:made-up/bugaboo.git",
            "example.test:made-up/bugaboo.git",
            "git://example.test/made-up",
        ] {
            assert!(!is_local_remote(network), "{network}");
        }
    }

    #[test]
    fn every_combination_of_role_and_remote_is_decided() {
        let path = Some(r"C:\made-up\bugaboo");
        let network = Some("https://example.test/made-up/bugaboo.git");
        assert_eq!(plan(Role::Work, path), Plan::Full);
        assert_eq!(plan(Role::Work, None), Plan::Full);
        assert_eq!(plan(Role::Master, network), Plan::NoBuild);
        assert_eq!(plan(Role::Master, None), Plan::NoBuild);
        for refused in [plan(Role::Work, network), plan(Role::Master, path)] {
            let Plan::Refuse(text) = refused else {
                panic!("should be refused");
            };
            assert!(text.contains("git -C <master> push github main"));
        }
    }
}
