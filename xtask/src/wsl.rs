//! The WSL distribution that builds Bugaboo's Linux programs (ADR-0037).
//!
//! Without a setting, every WSL call goes to the computer's default distribution. A checkout may
//! name another one in its own ignored `.env.local` (`BUGABOO_WSL_DISTRIBUTION=<name>`), for a
//! computer whose default distribution belongs to something else. The name is local to the
//! computer and never written into the repository. It is read here and nowhere else, and every
//! WSL call of a build passes it on as `wsl.exe -d <name>`.

use std::path::Path;

use crate::location::read_env;

/// The setting of `.env.local` that names the distribution.
pub(crate) const DISTRIBUTION_VARIABLE: &str = "BUGABOO_WSL_DISTRIBUTION";

/// The program that starts commands in WSL, found on the search path.
pub(crate) const PROGRAM: &str = "wsl.exe";

/// The distribution named in the `.env.local` of the checkout at `root`; `None` for the default.
pub(crate) fn build_distribution(root: &Path) -> Option<String> {
    read_env(root)
        .remove(DISTRIBUTION_VARIABLE)
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty())
}

/// The arguments of `wsl.exe`: `-d <distribution>` when one is named, then `rest`.
pub(crate) fn arguments(distribution: Option<&str>, rest: &[&str]) -> Vec<String> {
    distribution
        .map(|name| vec!["-d".to_owned(), name.to_owned()])
        .unwrap_or_default()
        .into_iter()
        .chain(rest.iter().map(|argument| (*argument).to_owned()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::location::ENV_FILE;
    use crate::testutil::TempDir;

    #[test]
    fn sec_18_the_build_distribution_comes_from_the_local_settings_or_is_the_default() {
        let temp = TempDir::new("wsl-distribution");
        assert_eq!(build_distribution(temp.path()), None);
        temp.write(ENV_FILE, "BUGABOO_CHECKOUT_ROLE=work\n");
        assert_eq!(build_distribution(temp.path()), None);
        temp.write(ENV_FILE, "BUGABOO_WSL_DISTRIBUTION=\n");
        assert_eq!(
            build_distribution(temp.path()),
            None,
            "an empty value is no name"
        );
        temp.write(
            ENV_FILE,
            "# for builds\nBUGABOO_WSL_DISTRIBUTION=\"builder\"\n",
        );
        assert_eq!(build_distribution(temp.path()).as_deref(), Some("builder"));
    }

    #[test]
    fn sec_18_a_named_distribution_goes_before_every_other_argument() {
        assert_eq!(arguments(None, &["wslpath", "-a"]), ["wslpath", "-a"]);
        assert_eq!(
            arguments(Some("builder"), &["wslpath", "-a"]),
            ["-d", "builder", "wslpath", "-a"]
        );
    }
}
