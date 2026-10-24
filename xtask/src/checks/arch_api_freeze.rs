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
//! The comparison is made on *facts*: one line of text for each name, type, answer, header, media
//! type and rule that a client can depend on. An answer member stays always there, a request member
//! stays optional, and a request is never tightened: no new required member, parameter or body, no
//! lower limit, no new `pattern`, `enum`, `format` or `additionalProperties: false` on what was
//! free. The members of `oneOf`, `anyOf` and `allOf` count as the schema that holds them, so a new
//! variant is an addition and a property that moves between variants is not seen.
//!
//! The frozen file itself is held too, and the check fails closed: `run` compares it with the copy
//! at the tag `m4-done` once the tag exists, and before that with the copy at the merge base with
//! `main` when `main` has one. A tag whose copy cannot be read, or a git that fails, is a finding,
//! never a skip. `cargo xtask freeze` makes the same comparison before it writes.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Context as _;
use serde_json::Value;

use super::api_spec::{self, resolve};
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

/// A limit, with the fact it belongs to (a limit on something new is no tightening).
type Limits = BTreeMap<String, (f64, String)>;

/// What a client can depend on in the description.
///
/// `names` holds one line of text for each fact that must stay (a frozen fact that the new
/// description lacks is a break). `upper` and `lower` hold the limits of what a request may hold: a
/// limit may be loosened and never tightened. `required` holds what a request must have: a thing
/// that is required now and was not in the frozen copy is new input, a break when its owner existed.
#[derive(Default)]
struct Facts {
    names: BTreeSet<String>,
    upper: Limits,
    lower: Limits,
    /// What is required of a request, with the fact it belongs to.
    required: BTreeMap<String, String>,
}

/// The upper and lower limits of validation.
const UPPER: [&str; 3] = ["maximum", "maxLength", "maxItems"];
const LOWER: [&str; 3] = ["minimum", "minLength", "minItems"];

/// Which side of the API reaches a schema: what a client sends, and what it reads.
#[derive(Clone, Copy, Default)]
struct Side {
    request: bool,
    answer: bool,
    /// Below a `oneOf` or `anyOf`: a client sends one variant, so what one variant requires is no
    /// input of the others.
    variant: bool,
}

/// The name of the schema a `$ref` points to.
fn ref_name(reference: &str) -> &str {
    reference.rsplit('/').next().unwrap_or(reference)
}

/// What a schema is made of, as words: the schemas it refers to, the types it has (`null`
/// included), its `format`, `oneOf`, `anyOf` and `allOf` seen through, an array as `[]` and what its
/// items are.
fn tokens(schema: &Value) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        found.insert(ref_name(reference).to_owned());
    }
    match schema.get("type") {
        Some(Value::String(kind)) => {
            found.insert(kind.clone());
        }
        Some(Value::Array(kinds)) => {
            found.extend(kinds.iter().filter_map(Value::as_str).map(str::to_owned));
        }
        _ => {}
    }
    if let Some(format) = schema.get("format").and_then(Value::as_str) {
        found.insert(format!("format:{format}"));
    }
    for key in ["oneOf", "anyOf", "allOf"] {
        for member in schema
            .get(key)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            found.extend(tokens(member));
        }
    }
    if let Some(items) = schema.get("items") {
        found.extend(tokens(items).into_iter().map(|token| format!("[]{token}")));
    }
    found
}

/// The names of the schemas that `roots` reach, `$ref` after `$ref`.
fn reach(spec: &Value, roots: &[&Value]) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut pending: Vec<&Value> = roots.to_vec();
    while let Some(value) = pending.pop() {
        match value {
            Value::Object(members) => {
                if let Some(reference) = members.get("$ref").and_then(Value::as_str)
                    && let Some(pointer) = reference.strip_prefix('#')
                    && seen.insert(ref_name(reference).to_owned())
                    && let Some(target) = spec.pointer(pointer)
                {
                    pending.push(target);
                }
                pending.extend(members.values());
            }
            Value::Array(items) => pending.extend(items.iter()),
            _ => {}
        }
    }
    seen
}

/// The schemas of the bodies of a request, and of its parameters.
fn request_roots<'a>(spec: &'a Value, operation: &'a Value) -> Vec<&'a Value> {
    let mut roots: Vec<&Value> = operation
        .get("parameters")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|parameter| resolve(spec, parameter))
        .collect();
    if let Some(body) = operation.get("requestBody") {
        roots.push(resolve(spec, body));
    }
    roots
}

/// The answers of an operation, `$ref` followed.
fn answers<'a>(spec: &'a Value, operation: &'a Value) -> Vec<(&'a str, &'a Value)> {
    operation
        .get("responses")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .map(|(status, answer)| (status.as_str(), resolve(spec, answer)))
        .collect()
}

fn facts(spec: &Value) -> Facts {
    let mut found = Facts::default();
    let mut requested = Vec::new();
    let mut answered = Vec::new();
    for operation in api_spec::operations(spec).into_iter().flatten() {
        requested.extend(request_roots(spec, operation.body));
        answered.extend(answers(spec, operation.body).into_iter().map(|(_, a)| a));
    }
    path_facts(spec, &mut found);
    let requests = reach(spec, &requested);
    let replies = reach(spec, &answered);
    for (name, schema) in api_spec::schemas(spec) {
        found.names.insert(format!("schema {name}"));
        let side = Side {
            request: requests.contains(name),
            answer: replies.contains(name),
            variant: false,
        };
        schema_facts(schema, name, &format!("schema {name}"), side, &mut found);
    }
    found
}

fn limits(schema: &Value, at: &str, owner: &str, found: &mut Facts) {
    for key in UPPER {
        if let Some(value) = schema.get(key).and_then(Value::as_f64) {
            found
                .upper
                .insert(format!("{at} {key}"), (value, owner.to_owned()));
        }
    }
    for key in LOWER {
        if let Some(value) = schema.get(key).and_then(Value::as_f64) {
            found
                .lower
                .insert(format!("{at} {key}"), (value, owner.to_owned()));
        }
    }
}

/// Whether a request text has no rule of its own: no word list, no pattern, no format.
fn is_free_text(schema: &Value) -> bool {
    let is_text = tokens(schema).contains("string");
    is_text
        && ["enum", "const", "pattern", "format"]
            .iter()
            .all(|key| schema.get(key).is_none())
}

/// The rules that only tighten what a request may be: the pattern and the default are facts that
/// must stay as they are, a free text is a fact that vanishes when a rule is put on it, and an
/// object that takes more than it names is a fact that vanishes when it is closed.
fn request_rules(schema: &Value, at: &str, found: &mut Facts) {
    if let Some(pattern) = schema.get("pattern").and_then(Value::as_str) {
        found.names.insert(format!("pattern {at} {pattern}"));
    }
    if let Some(default) = schema.get("default") {
        found.names.insert(format!("default {at} {default}"));
    }
    if is_free_text(schema) {
        found.names.insert(format!("free-text {at}"));
    }
    if schema.get("type").and_then(Value::as_str) == Some("object")
        && schema.get("additionalProperties") != Some(&Value::Bool(false))
    {
        found.names.insert(format!("open-object {at}"));
    }
}

/// The parameters of an operation: their types, values, rules and whether they are required.
fn parameter_facts(spec: &Value, operation: &Value, place: &str, owner: &str, found: &mut Facts) {
    for parameter in operation
        .get("parameters")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let parameter = resolve(spec, parameter);
        let kind = parameter
            .get("in")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let name = parameter
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let at = format!("{place} {kind} {name}");
        found.names.insert(format!("parameter {at}"));
        if parameter.get("required").and_then(Value::as_bool) == Some(true) {
            found
                .required
                .insert(format!("required-parameter {at}"), owner.to_owned());
        } else {
            found.names.insert(format!("optional-parameter {at}"));
        }
        if let Some(schema) = parameter.get("schema") {
            for token in tokens(schema) {
                found.names.insert(format!("parameter-type {at} {token}"));
            }
            for value in schema
                .get("enum")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
            {
                found.names.insert(format!("parameter-value {at} {value}"));
            }
            request_rules(schema, &format!("parameter {at}"), found);
            limits(schema, &format!("parameter {at}"), owner, found);
        }
    }
}

fn path_facts(spec: &Value, found: &mut Facts) {
    let Some(paths) = spec.get("paths").and_then(Value::as_object) else {
        return;
    };
    for path in paths.keys() {
        found.names.insert(format!("path {path}"));
    }
    for operation in api_spec::operations(spec).into_iter().flatten() {
        let place = operation.place();
        let owner = format!(
            "operation {place} {}",
            operation.text("operationId").unwrap_or_default()
        );
        found.names.insert(owner.clone());
        if let Some(body) = operation.body.get("requestBody") {
            found.names.insert(format!("body {place}"));
            let body = resolve(spec, body);
            if body.get("required").and_then(Value::as_bool) == Some(true) {
                found
                    .required
                    .insert(format!("required-body {place}"), owner.clone());
            }
            for (media, content) in body
                .get("content")
                .and_then(Value::as_object)
                .into_iter()
                .flatten()
            {
                found.names.insert(format!("request-media {place} {media}"));
                if let Some(schema) = content.get("schema") {
                    for token in tokens(schema) {
                        found
                            .names
                            .insert(format!("request {place} {media} {token}"));
                    }
                }
            }
        }
        parameter_facts(spec, operation.body, &place, &owner, found);
        for (status, answer) in answers(spec, operation.body) {
            found.names.insert(format!("answer {place} {status}"));
            for header in answer
                .get("headers")
                .and_then(Value::as_object)
                .into_iter()
                .flat_map(serde_json::Map::keys)
            {
                found
                    .names
                    .insert(format!("header {place} {status} {header}"));
            }
            for (media, content) in answer
                .get("content")
                .and_then(Value::as_object)
                .into_iter()
                .flatten()
            {
                found
                    .names
                    .insert(format!("media {place} {status} {media}"));
                if let Some(schema) = content.get("schema") {
                    for token in tokens(schema) {
                        found
                            .names
                            .insert(format!("answer-schema {place} {status} {media} {token}"));
                    }
                }
            }
        }
    }
}

/// The properties, types, requiredness, limits and enum values of a schema, below the name `at`;
/// `owner` is the fact of the schema they belong to.
fn schema_facts(schema: &Value, at: &str, owner: &str, side: Side, found: &mut Facts) {
    let required: BTreeSet<&str> = schema
        .get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    if side.request {
        request_rules(schema, at, found);
    }
    if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
        for (name, property) in properties {
            let here = format!("{at}.{name}");
            found.names.insert(format!("property {here}"));
            let kinds = tokens(property);
            for kind in &kinds {
                found.names.insert(format!("type {here} {kind}"));
            }
            // What a reader can count on is always there; what a writer may leave out stays optional.
            if side.answer && required.contains(name.as_str()) && !kinds.contains("null") {
                found.names.insert(format!("always {here}"));
            }
            if side.request {
                if required.contains(name.as_str()) {
                    if !side.variant {
                        found
                            .required
                            .insert(format!("required-member {here}"), owner.to_owned());
                    }
                } else {
                    found.names.insert(format!("optional {here}"));
                }
            }
            schema_facts(property, &here, owner, side, found);
        }
    }
    if side.request {
        limits(schema, at, owner, found);
    }
    for key in ["enum", "const"] {
        let values: Vec<&Value> = match schema.get(key) {
            Some(Value::Array(values)) => values.iter().collect(),
            Some(value) => vec![value],
            None => Vec::new(),
        };
        for value in values.into_iter().filter_map(Value::as_str) {
            found.names.insert(format!("value {at} {value}"));
        }
    }
    if let Some(items) = schema.get("items") {
        schema_facts(items, &format!("{at}[]"), owner, side, found);
    }
    if let Some(extra) = schema.get("additionalProperties") {
        schema_facts(extra, &format!("{at}{{}}"), owner, side, found);
    }
    for key in ["oneOf", "anyOf", "allOf"] {
        let side = Side {
            variant: side.variant || key != "allOf",
            ..side
        };
        for member in schema
            .get(key)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            schema_facts(member, at, owner, side, found);
        }
    }
}

/// What the frozen description has and the current one lacks or tightened.
fn broken(frozen: &Value, current: &Value) -> Vec<String> {
    let (old, now) = (facts(frozen), facts(current));
    let mut gone: Vec<String> = old
        .names
        .iter()
        .filter(|fact| !now.names.contains(*fact))
        .cloned()
        .collect();
    // A limit that is new, or lower (upper) or higher (lower), on something that was there.
    for (key, (is, owner)) in &now.upper {
        match old.upper.get(key) {
            Some((was, _)) if is < was => {
                gone.push(format!("limit {key} {was} (tightened to {is})"));
            }
            None if old.names.contains(owner) => {
                gone.push(format!(
                    "limit {key} (a new limit of {is} on what was free)"
                ));
            }
            _ => {}
        }
    }
    for (key, (is, owner)) in &now.lower {
        match old.lower.get(key) {
            Some((was, _)) if is > was => {
                gone.push(format!("limit {key} {was} (tightened to {is})"));
            }
            None if old.names.contains(owner) => {
                gone.push(format!(
                    "limit {key} (a new limit of {is} on what was free)"
                ));
            }
            _ => {}
        }
    }
    // New input a request must have, on a route or a schema that was there.
    for (what, owner) in &now.required {
        if !old.required.contains_key(what) && old.names.contains(owner) {
            gone.push(format!(
                "{what} (required now, and a client of the frozen API does not send it)"
            ));
        }
    }
    gone
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
                    "`{fact}` is in the frozen API and is gone or tightened. Renaming, removing or tightening anything of /api/v1 needs a decision record and a new version; adding is free (ADR-0008)."
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
