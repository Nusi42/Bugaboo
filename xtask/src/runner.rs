//! Picks the checks for a moment (edit, commit, push, CI) and runs them.

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use anyhow::bail;

use crate::catalog::{Catalog, Entry};
use crate::checks::{Check, Context, Finding};
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
    // Commit-time checks look at an export of what will be committed, not at later edits
    // (M0-F09). The working tree is only read, never written.
    let staged = match scope {
        Scope::Staged(_) => Some(repo.export_staged()?),
        _ => None,
    };
    let context = Context {
        repo: staged.as_ref().map_or(repo, |tree| &tree.repo),
        catalog,
        files,
    };
    let mut failed = 0_usize;
    for ran in run_entries(registry, &selected, &context)? {
        output::result(ran.entry, &ran.findings, ran.took);
        if !ran.findings.is_empty() {
            failed += 1;
        }
    }
    output::summary(selected.len(), failed, context.files.map(<[String]>::len));
    Ok(failed == 0)
}

/// What one check found, and how long it took.
pub(crate) struct Ran<'e> {
    pub(crate) entry: &'e Entry,
    pub(crate) findings: Vec<Finding>,
    pub(crate) took: Duration,
}

/// What a check answered, and how long it took; kept until every check has run.
type Outcome = anyhow::Result<(Vec<Finding>, Duration)>;

/// Runs the checks side by side, as many at once as the machine has cores, and answers their
/// results in the order of `entries` (catalogue order), so the output reads the same every time.
/// Most checks are short, but several read the whole repository, so one after the other they
/// grew past the commit-time budget as the code grew (META-SPEED, architecture §10).
pub(crate) fn run_entries<'e>(
    registry: &[Box<dyn Check>],
    entries: &[&'e Entry],
    context: &Context<'_>,
) -> anyhow::Result<Vec<Ran<'e>>> {
    let mut checks = Vec::with_capacity(entries.len());
    for entry in entries {
        let Some(check) = registry.iter().find(|check| check.id() == entry.id) else {
            bail!(
                "{} is active in the catalogue but not implemented (META-CATALOG)",
                entry.id
            );
        };
        checks.push(check.as_ref());
    }
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<Option<Outcome>>> =
        Mutex::new((0..checks.len()).map(|_| None).collect());
    let workers = std::thread::available_parallelism()
        .map_or(1, std::num::NonZeroUsize::get)
        .min(checks.len());
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let at = next.fetch_add(1, Ordering::Relaxed);
                    let Some(check) = checks.get(at) else {
                        break;
                    };
                    let started = Instant::now();
                    let result = check.run(context).map(|found| (found, started.elapsed()));
                    if let Ok(mut slots) = results.lock()
                        && let Some(slot) = slots.get_mut(at)
                    {
                        *slot = Some(result);
                    }
                }
            });
        }
    });
    let slots = results
        .into_inner()
        .map_err(|_| anyhow::anyhow!("a check stopped while it held the list of results"))?;
    let mut ran = Vec::with_capacity(entries.len());
    for (entry, slot) in entries.iter().zip(slots) {
        let Some(result) = slot else {
            bail!("{} did not run to its end", entry.id);
        };
        let (findings, took) = result?;
        ran.push(Ran {
            entry,
            findings,
            took,
        });
    }
    Ok(ran)
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
