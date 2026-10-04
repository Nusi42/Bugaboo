//! Picks the checks for a moment (edit, commit, push, CI) and runs them.

use std::time::Instant;

use anyhow::bail;

use crate::catalog::{Catalog, Entry};
use crate::checks::{Check, Context};
use crate::output;
use crate::repo::Repo;

/// Which checks to run.
pub(crate) enum Scope {
    /// Files an agent just edited: checks with `edit` whose files match.
    Edit(Vec<String>),
    /// Staged files: checks with `commit` whose files match.
    Staged(Vec<String>),
    /// Checks with `push`, on the whole repository.
    Push,
    /// Checks with `ci`, on the whole repository.
    All,
    /// Exactly these check IDs, on the whole repository.
    Only(Vec<String>),
}

/// Runs the selected checks; returns whether all passed.
pub(crate) fn run(
    repo: &Repo,
    catalog: &Catalog,
    registry: &[Box<dyn Check>],
    scope: &Scope,
) -> anyhow::Result<bool> {
    let selected = select(catalog, scope)?;
    let files = match scope {
        Scope::Edit(files) | Scope::Staged(files) => Some(files.as_slice()),
        Scope::Push | Scope::All | Scope::Only(_) => None,
    };
    // Commit-time checks look at what will be committed, not at later edits (M0-F09).
    let _staged = match scope {
        Scope::Staged(files) => Some(repo.staged_view(files)?),
        _ => None,
    };
    let context = Context {
        repo,
        catalog,
        files,
    };
    let mut failed = 0_usize;
    for entry in &selected {
        let Some(check) = registry.iter().find(|check| check.id() == entry.id) else {
            bail!(
                "{} is active in the catalogue but not implemented (META-CATALOG)",
                entry.id
            );
        };
        let started = Instant::now();
        let findings = check.run(&context)?;
        output::result(entry, &findings, started.elapsed());
        if !findings.is_empty() {
            failed += 1;
        }
    }
    output::summary(selected.len(), failed, context.files.map(<[String]>::len));
    Ok(failed == 0)
}

fn select<'a>(catalog: &'a Catalog, scope: &Scope) -> anyhow::Result<Vec<&'a Entry>> {
    let mut selected = Vec::new();
    if let Scope::Only(ids) = scope {
        for id in ids {
            match catalog.get(id) {
                Some(entry) => selected.push(entry),
                None => bail!("There is no check `{id}` in the catalogue."),
            }
        }
        return Ok(selected);
    }
    for entry in catalog.entries().iter().filter(|entry| entry.is_active()) {
        let wanted = match scope {
            Scope::Edit(files) => entry.runs_at("edit") && entry.matches_any(files)?,
            Scope::Staged(files) => entry.runs_at("commit") && entry.matches_any(files)?,
            Scope::Push => entry.runs_at("push"),
            Scope::All => entry.runs_at("ci"),
            Scope::Only(_) => false,
        };
        if wanted {
            selected.push(entry);
        }
    }
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CATALOG: &str = r#"
[[check]]
id = "RS"
title = "t"
rule = "r"
tool = "x"
when = ["edit", "commit", "ci"]
files = ["**/*.rs"]
status = "active"

[[check]]
id = "PLANNED"
title = "t"
rule = "r"
tool = "x"
when = ["commit", "ci"]
files = ["*"]
status = "planned"

[[check]]
id = "PUSH"
title = "t"
rule = "r"
tool = "x"
when = ["push"]
files = ["*"]
status = "active"
"#;

    fn ids(scope: &Scope) -> Vec<String> {
        let catalog = Catalog::parse(CATALOG).unwrap();
        select(&catalog, scope)
            .unwrap()
            .iter()
            .map(|e| e.id.clone())
            .collect()
    }

    #[test]
    fn edit_runs_only_matching_active_checks() {
        assert_eq!(ids(&Scope::Edit(vec!["xtask/src/main.rs".into()])), ["RS"]);
        assert!(ids(&Scope::Edit(vec!["README.md".into()])).is_empty());
    }

    #[test]
    fn planned_checks_never_run_by_stage() {
        assert_eq!(ids(&Scope::All), ["RS"]);
        assert_eq!(ids(&Scope::Push), ["PUSH"]);
    }

    #[test]
    fn only_runs_exactly_what_was_asked() {
        assert_eq!(ids(&Scope::Only(vec!["PUSH".into()])), ["PUSH"]);
        let catalog = Catalog::parse(CATALOG).unwrap();
        assert!(select(&catalog, &Scope::Only(vec!["NOPE".into()])).is_err());
    }
}
