//! ARCH-API-FREEZE: nothing in `/api/v1` is renamed, removed or tightened after the API freeze
//! (ADR-0008, amendment M4).
//!
//! The frozen copy (`tools/checks/api-frozen.json`, the generated API description in compact
//! form) is stored by `cargo xtask freeze`; the lead does it at `m4-done`. From then on every
//! description the code writes is compared with it. A path, an operation, a parameter, an answer, a
//! schema, a property, an enum value (an error code is one) of the frozen copy that the new
//! description does not have fails; what the new description adds passes. A change that
//! must stand needs a decision record and a new version, and then a new frozen copy.
//!
//! The comparison is one recursive walk over the frozen and the new description (`compare.rs`):
//! every operation, parameter, request body and answer, and through each `$ref` every property,
//! item, map value and `oneOf`, `anyOf` and `allOf` branch. Each node is named by its path and
//! compared by its side: a request may only loosen (nothing becomes required, nothing is added to
//! or tightened in a type, format, `enum`, `pattern`, `default`, limit or `additionalProperties`,
//! no variant goes), an answer may only grow (no member, variant, value, header or media type goes,
//! nothing stops being always there, no `null` or wider type, no other format). What is new has no
//! frozen node, so a new optional member brings its own rules.
//!
//! The frozen file itself is held too, and the check fails closed: `run` compares it with the copy
//! at the tag `m4-done` once the tag exists, and before that with the copy at the merge base with
//! `main` when `main` has one. A tag whose copy cannot be read, or a git that fails, is a finding,
//! never a skip. `cargo xtask freeze` makes the same comparison before it writes.

use anyhow::Context as _;
use serde_json::Value;

use self::compare::broken;
use super::api_spec;
use super::{Check, Context, Finding};
use crate::generate::api::SPEC_PATH;
use crate::repo::Repo;
use crate::source::read_text;

/// The frozen copy of the API description, relative to the repository root.
pub(crate) const FROZEN_PATH: &str = "tools/checks/api-frozen.json";

/// The tag of the milestone whose copy every later copy must hold.
const FREEZE_TAG: &str = "m4-done";

/// The most findings written out, so a rename of a whole family reads as one problem.
const MOST_FINDINGS: usize = 40;

pub(crate) struct ArchApiFreeze;

impl Check for ArchApiFreeze {
    fn id(&self) -> &'static str {
        "ARCH-API-FREEZE"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let current = api_spec::load(context.repo)?;
        let Ok(text) = read_text(context.repo, FROZEN_PATH) else {
            return Ok(vec![Finding::at(
                FROZEN_PATH,
                "There is no frozen copy of the API description. Store it with `cargo xtask freeze` (ADR-0008).",
            )]);
        };
        let frozen = api_spec::parse(&text)?;
        let mut findings = evaluate(&frozen, &current);
        // The frozen copy must hold everything of the one it replaces.
        match copy_to_keep(context.repo) {
            Ok(Some((origin, kept))) => {
                findings.extend(lost_in_the_copy(&origin, &kept, &frozen));
            }
            Ok(None) => {}
            Err(error) => findings.push(Finding::at(
                FROZEN_PATH,
                format!("The copy the frozen copy must hold cannot be read: {error:#}"),
            )),
        }
        Ok(findings)
    }
}

/// The findings for a frozen copy that lacks a fact of the copy it has to keep.
fn lost_in_the_copy(origin: &str, kept: &Value, frozen: &Value) -> Vec<Finding> {
    let gone = broken(kept, frozen);
    if gone.is_empty() {
        return Vec::new();
    }
    vec![Finding::at(
        FROZEN_PATH,
        format!(
            "The frozen copy lacks {} facts of the copy {origin}, for example: {}",
            gone.len(),
            gone.iter().take(5).cloned().collect::<Vec<_>>().join("; ")
        ),
    )]
}

/// Runs `git`; the answer is its text, or an error when git does not run or fails.
fn git(repo: &Repo, arguments: &[&str]) -> anyhow::Result<String> {
    let output = std::process::Command::new("git")
        .args(arguments)
        .current_dir(repo.root())
        .output()
        .with_context(|| format!("git {} did not start", arguments.join(" ")))?;
    anyhow::ensure!(
        output.status.success(),
        "git {} failed: {}",
        arguments.join(" "),
        String::from_utf8_lossy(&output.stderr).trim()
    );
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// Whether a name is a reference of the repository.
fn is_reference(repo: &Repo, name: &str) -> anyhow::Result<bool> {
    Ok(!git(repo, &["for-each-ref", "--format=%(refname)", name])?.is_empty())
}

/// The frozen copy as it is stored at a commit or tag: nothing when the commit has none.
fn copy_at(repo: &Repo, reference: &str) -> anyhow::Result<Option<Value>> {
    let listed = git(repo, &["ls-tree", reference, "--", FROZEN_PATH])?;
    if listed.is_empty() {
        return Ok(None);
    }
    let text = git(repo, &["show", &format!("{reference}:{FROZEN_PATH}")])?;
    let copy = serde_json::from_str(&text)
        .with_context(|| format!("the frozen copy at {reference} is not JSON"))?;
    Ok(Some(copy))
}

/// The copy that the frozen copy must hold, with where it is from: the one at the tag `m4-done` when
/// the tag exists, else the one at the merge base with `main` when `main` has one. `None` when there
/// is none yet. A tag that is there and has no readable copy, and a git that fails, are errors.
fn copy_to_keep(repo: &Repo) -> anyhow::Result<Option<(String, Value)>> {
    if is_reference(repo, &format!("refs/tags/{FREEZE_TAG}"))? {
        let copy = copy_at(repo, &format!("refs/tags/{FREEZE_TAG}"))?;
        let copy = copy.with_context(|| format!("the tag {FREEZE_TAG} has no frozen copy"))?;
        return Ok(Some((format!("at the tag {FREEZE_TAG}"), copy)));
    }
    for main in ["refs/remotes/origin/main", "refs/heads/main"] {
        if !is_reference(repo, main)? {
            continue;
        }
        let base = git(repo, &["merge-base", "HEAD", main])?;
        return Ok(
            copy_at(repo, &base)?.map(|copy| (format!("at the merge base with {main}"), copy))
        );
    }
    Ok(None)
}

/// Stores the API description as it is now as the frozen copy. It refuses a copy that lacks a fact of
/// the one it must keep.
pub(crate) fn write_frozen(repo: &Repo) -> anyhow::Result<()> {
    let spec = api_spec::load(repo)?;
    if let Some((origin, kept)) = copy_to_keep(repo)? {
        let gone = broken(&kept, &spec);
        anyhow::ensure!(
            gone.is_empty(),
            "a new frozen copy must hold everything of the copy {origin}; {} facts are gone, for example: {}",
            gone.len(),
            gone.iter().take(5).cloned().collect::<Vec<_>>().join("; ")
        );
    }
    let compact = serde_json::to_string(&spec).context("the API description is not JSON")?;
    if let Some(folder) = repo.path(FROZEN_PATH).parent() {
        std::fs::create_dir_all(folder)
            .with_context(|| format!("could not make {}", folder.display()))?;
    }
    std::fs::write(repo.path(FROZEN_PATH), format!("{compact}\n"))
        .with_context(|| format!("could not write {FROZEN_PATH}"))
}

fn evaluate(frozen: &Value, current: &Value) -> Vec<Finding> {
    let gone = broken(frozen, current);
    let mut findings: Vec<Finding> = gone
        .iter()
        .take(MOST_FINDINGS)
        .map(|fact| {
            Finding::at(
                SPEC_PATH,
                format!(
                    "{fact}. The frozen API has it otherwise: renaming, removing or tightening anything of /api/v1 needs a decision record and a new version; adding is free (ADR-0008)."
                ),
            )
        })
        .collect();
    if gone.len() > MOST_FINDINGS {
        findings.push(Finding::at(
            SPEC_PATH,
            format!("And {} more.", gone.len() - MOST_FINDINGS),
        ));
    }
    findings
}

mod compare;

#[cfg(test)]
mod copy_tests;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod examples {
    use serde_json::json;

    use super::*;

    fn frozen() -> Value {
        json!({"components": {"schemas": {"Thing": {"type": "object", "properties": {"id": {"type": "string"}}}}}})
    }

    #[test]
    fn bad_example_a_property_that_is_gone() {
        let now =
            json!({"components": {"schemas": {"Thing": {"type": "object", "properties": {}}}}});
        assert!(!evaluate(&frozen(), &now).is_empty());
    }

    #[test]
    fn good_example_a_property_that_is_added() {
        let mut now = frozen();
        now["components"]["schemas"]["Thing"]["properties"]["title"] = json!({"type": "string"});
        assert!(evaluate(&frozen(), &now).is_empty());
    }
}
