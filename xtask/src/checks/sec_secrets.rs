//! SEC-SECRETS: no keys, tokens or passwords committed (SEC-16, `.gitleaks.toml`).
//!
//! Staged runs scan only what is about to be committed; every other run scans the whole history.

use std::path::Path;

use super::{Check, Context, Finding};
use crate::tools;

pub(crate) struct SecSecrets;

impl Check for SecSecrets {
    fn id(&self) -> &'static str {
        "SEC-SECRETS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let what = if context.files.is_some() {
            Scan::Staged
        } else {
            Scan::History
        };
        scan(context.repo.real_root(), what)
    }
}

/// What gitleaks reads.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scan {
    /// The files being committed.
    Staged,
    /// Everything the repository ever held.
    History,
}

fn scan(root: &Path, what: Scan) -> anyhow::Result<Vec<Finding>> {
    let mut args = vec![
        "git",
        "--no-banner",
        "--no-color",
        "--log-level",
        "warn",
        "--redact",
        "--config",
        ".gitleaks.toml",
    ];
    if what == Scan::Staged {
        args.extend(["--staged", "--pre-commit"]);
    }
    args.push(".");
    let output = tools::run_installed("gitleaks", &args, root)?;
    Ok(tools::findings_from(
        output,
        "A secret was found (the value is hidden). Remove it, change the real key or password, \
         and never commit it again; keep secrets in files that Git ignores.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    fn repo_with(temp: &TempDir, content: &str) {
        temp.write(".gitleaks.toml", "[extend]\nuseDefault = true\n");
        temp.write("notes.txt", content);
        assert!(temp.git(&["init", "--quiet"]));
        assert!(temp.git(&["add", "."]));
        assert!(temp.git(&["commit", "--quiet", "-m", "chore: test"]));
    }

    #[test]
    fn bad_example() {
        let temp = TempDir::new("secrets-bad");
        // Built at run time so this source file does not contain a token itself.
        let token = format!("ghp_{}{}", "aB3dE5gH7jK9mN1pQ3rS", "5tU7vW9xY1zA3bC5dE7f");
        repo_with(&temp, &format!("token = \"{token}\"\n"));
        crate::testutil::assert_real_findings(&scan(temp.path(), Scan::History).unwrap());
    }

    /// A repository that holds the file of the fixture and the rules of Bugaboo's own
    /// `.gitleaks.toml` only: the default rules of gitleaks are switched off here, so a finding is
    /// one of Bugaboo's rules (access keys, minisign secret keys) and nothing else. The marker that
    /// lets the fixture stand in this repository is cut off, or the scan would skip the line it is
    /// about.
    fn repo_with_the_rule_of_bugaboo(temp: &TempDir, fixture: &str) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let config = std::fs::read_to_string(root.join(".gitleaks.toml")).unwrap();
        assert!(config.contains("useDefault = true"));
        temp.write(
            ".gitleaks.toml",
            config.replace("useDefault = true", "useDefault = false"),
        );
        let text =
            std::fs::read_to_string(root.join("tools/checks/fixtures/SEC-SECRETS").join(fixture))
                .unwrap();
        let without_marker: String = text
            .lines()
            .map(|line| line.split("  # ").next().unwrap_or(line))
            .collect::<Vec<_>>()
            .join(
                "
",
            );
        temp.write("notes.txt", without_marker);
        assert!(temp.git(&["init", "--quiet"]));
        assert!(temp.git(&["add", "."]));
        assert!(temp.git(&["commit", "--quiet", "-m", "chore: test"]));
    }

    #[test]
    fn bad_example_an_access_key_is_found_by_the_rule_of_bugaboo() {
        let temp = TempDir::new("secrets-key-bad");
        repo_with_the_rule_of_bugaboo(&temp, "bad/access-key.txt");
        crate::testutil::assert_real_findings(&scan(temp.path(), Scan::History).unwrap());
    }

    #[test]
    fn good_example_things_that_only_look_like_access_keys_are_not_found() {
        let temp = TempDir::new("secrets-key-good");
        repo_with_the_rule_of_bugaboo(&temp, "good/not-a-key.txt");
        assert!(scan(temp.path(), Scan::History).unwrap().is_empty());
    }

    #[test]
    fn bad_example_the_secrets_of_ai_apps_are_found_by_the_rule_of_bugaboo() {
        let temp = TempDir::new("secrets-oauth-bad");
        repo_with_the_rule_of_bugaboo(&temp, "bad/oauth-secrets.txt");
        let findings = scan(temp.path(), Scan::History).unwrap();
        crate::testutil::assert_real_findings(&findings);
        // The code, the access token and the renewing token.
        assert!(
            findings[0].message.contains("leaks found: 3"),
            "{}",
            findings[0].message
        );
    }

    #[test]
    fn good_example_things_that_only_look_like_secrets_of_ai_apps_are_not_found() {
        let temp = TempDir::new("secrets-oauth-good");
        repo_with_the_rule_of_bugaboo(&temp, "good/not-an-oauth-secret.txt");
        assert!(scan(temp.path(), Scan::History).unwrap().is_empty());
    }

    #[test]
    fn bad_example_a_minisign_secret_key_is_found_by_the_rules_of_bugaboo() {
        let temp = TempDir::new("secrets-minisign-bad");
        repo_with_the_rule_of_bugaboo(&temp, "bad/minisign-secret-key.txt");
        let findings = scan(temp.path(), Scan::History).unwrap();
        crate::testutil::assert_real_findings(&findings);
        // Both first lines and both key lines, encrypted and not.
        assert!(
            findings[0].message.contains("leaks found: 4"),
            "{}",
            findings[0].message
        );
    }

    #[test]
    fn good_example_a_public_key_and_a_signature_of_minisign_are_not_found() {
        let temp = TempDir::new("secrets-minisign-good");
        repo_with_the_rule_of_bugaboo(&temp, "good/minisign-not-a-secret-key.txt");
        assert!(scan(temp.path(), Scan::History).unwrap().is_empty());
    }

    #[test]
    fn good_example() {
        let temp = TempDir::new("secrets-good");
        repo_with(&temp, "Nothing secret here.\n");
        assert!(scan(temp.path(), Scan::History).unwrap().is_empty());
    }
}
