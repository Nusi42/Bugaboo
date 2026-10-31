//! The identity of an author or committer: only a public one is allowed (coding-standards §2).
//!
//! A public identity is a name and the noreply address of a GitHub account that has that name as
//! its login (`Name <id+Name@users.noreply.github.com>`), or the identity GitHub itself uses for
//! the merges it makes. Nothing else is accepted: a private e-mail address or a real name in the
//! author field of a commit is published with the history. The check never prints the name or the
//! address; a finding names the commit only.

use std::collections::BTreeSet;

use anyhow::Context as _;

use super::{Finding, Repo, Rule};

/// The domain of the noreply addresses of GitHub.
const NOREPLY_DOMAIN: &str = "users.noreply.github.com";
/// What GitHub writes as the committer of a merge made on its web site.
const GITHUB_OWN: (&str, &str) = ("GitHub", "noreply@github.com");
/// The two roles in a commit, and the variable of `git var` that names each.
const ROLES: [(&str, &str); 2] = [
    ("author", "GIT_AUTHOR_IDENT"),
    ("committer", "GIT_COMMITTER_IDENT"),
];

/// A name and an e-mail address.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Identity {
    pub(crate) name: String,
    pub(crate) email: String,
}

impl Identity {
    /// The identity in `Name <email> 1700000000 +0100`, the form of `git var GIT_AUTHOR_IDENT`.
    pub(crate) fn parse(ident: &str) -> Option<Self> {
        let (name, rest) = ident.split_once('<')?;
        let (email, _) = rest.split_once('>')?;
        Some(Self {
            name: name.trim().to_owned(),
            email: email.trim().to_owned(),
        })
    }

    /// Whether this identity may be published: see the top of this file.
    pub(crate) fn is_public(&self) -> bool {
        if (self.name.as_str(), self.email.as_str()) == GITHUB_OWN {
            return true;
        }
        let Some(local) = self.email.strip_suffix(&format!("@{NOREPLY_DOMAIN}")) else {
            return false;
        };
        // `12345+login` or just `login`.
        let login = local.rsplit_once('+').map_or(local, |(_, login)| login);
        !login.is_empty()
            && login.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
            && login.eq_ignore_ascii_case(&self.name)
    }
}

/// What is wrong with the identities the next commit is recorded with (the commit-msg hook), one
/// text for each problem. Git answers for the author and the committer, from the environment and
/// the configuration, the way the commit will be recorded.
pub(crate) fn recorded_problems(repo: &Repo) -> Vec<String> {
    let mut problems = Vec::new();
    for (role, variable) in ROLES {
        let identity = repo
            .git_bytes(&["var", variable])
            .ok()
            .and_then(|bytes| Identity::parse(&String::from_utf8_lossy(&bytes)));
        match identity {
            Some(identity) if identity.is_public() => {}
            Some(_) => problems.push(format!(
                "The {role} of this commit {}",
                Rule::Identity.message()
            )),
            None => problems.push(format!(
                "The {role} of this commit is not set. {}",
                Rule::Identity.message()
            )),
        }
    }
    problems
}

/// The commits on no remote with an author or a committer that is not public: one finding for
/// each commit and role, naming the commit only.
pub(super) fn unpushed_findings(repo: &Repo) -> anyhow::Result<Vec<Finding>> {
    if repo
        .git_bytes(&["rev-parse", "--verify", "--quiet", "HEAD"])
        .is_err()
    {
        return Ok(Vec::new());
    }
    let listing = repo
        .git_bytes(&[
            "log",
            "--format=%h%x1f%an%x1f%ae%x1f%cn%x1f%ce%x1e",
            "--branches",
            "--tags",
            "HEAD",
            "--not",
            "--remotes",
        ])
        .context("could not read the authors of the commits")?;
    let mut findings = Vec::new();
    let mut seen = BTreeSet::new();
    for record in super::history::records(&listing) {
        let parts: Vec<&str> = record.split('\u{1f}').collect();
        let [hash, author, author_email, committer, committer_email] = parts[..] else {
            continue;
        };
        for (role, name, email) in [
            ("author", author, author_email),
            ("committer", committer, committer_email),
        ] {
            let identity = Identity {
                name: name.to_owned(),
                email: email.to_owned(),
            };
            if !identity.is_public() && seen.insert((hash.to_owned(), role)) {
                findings.push(Finding::at(
                    format!("commit {hash}"),
                    format!("The {role} {}", Rule::Identity.message()),
                ));
            }
        }
    }
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(name: &str, email: &str) -> Identity {
        Identity {
            name: name.to_owned(),
            email: email.to_owned(),
        }
    }

    #[test]
    fn the_output_of_git_var_is_read() {
        let found =
            Identity::parse("Mira Made-Up <1+mira@users.noreply.github.com> 1793409015 +0100");
        assert_eq!(
            found,
            Some(identity("Mira Made-Up", "1+mira@users.noreply.github.com"))
        );
        assert_eq!(Identity::parse("no address"), None);
    }

    #[test]
    fn good_example_public_identities() {
        for (name, email) in [
            ("mira", "1234+mira@users.noreply.github.com"),
            ("Mira", "mira@users.noreply.github.com"),
            ("mira-42", "7+Mira-42@users.noreply.github.com"),
            ("GitHub", "noreply@github.com"),
        ] {
            assert!(identity(name, email).is_public(), "{name} {email}");
        }
    }

    #[test]
    fn bad_example_private_identities() {
        for (name, email) in [
            ("Mira Made-Up", "mira@example.test"),
            ("mira", "mira@gmail.test"),
            // The address is public, but the name is another one.
            ("Mira Made-Up", "1+mira@users.noreply.github.com"),
            ("mira", "1+other@users.noreply.github.com"),
            ("mira", "1+mira@users.noreply.github.com.example.test"),
            ("", "1+@users.noreply.github.com"),
            ("GitHub", "someone@github.com"),
        ] {
            assert!(!identity(name, email).is_public(), "{name} {email}");
        }
    }
}
