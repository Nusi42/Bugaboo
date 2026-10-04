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
        let staged = context.files.is_some();
        scan(context.repo.real_root(), staged)
    }
}

fn scan(root: &Path, staged: bool) -> anyhow::Result<Vec<Finding>> {
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
    if staged {
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
        crate::testutil::assert_real_findings(&scan(temp.path(), false).unwrap());
    }

    #[test]
    fn good_example() {
        let temp = TempDir::new("secrets-good");
        repo_with(&temp, "Nothing secret here.\n");
        assert!(scan(temp.path(), false).unwrap().is_empty());
    }
}
