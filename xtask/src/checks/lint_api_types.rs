//! LINT-API-TYPES: no hand-written API types in the browser app (ADR-0008).
//!
//! The types of everything the server sends and receives come from the OpenAPI document
//! (`@bugaboo/api-client`). In `web/app/` a `type` or `interface` that has the name of a schema of
//! the document, or a name that says it is a server answer (`…Response`, `…Dto`, `…Payload`), is a
//! copy that will drift.

use std::collections::BTreeSet;

use anyhow::Context as _;

use super::web_source::{WebSource, code_of, read_web_sources};
use super::{Check, Context, Finding};
use crate::generate::api::SPEC_PATH;

/// The app; the client package itself is where the types live.
const APP: &str = "web/app/";
/// Endings of names that say "this is what the server sends".
const SUFFIXES: [&str; 3] = ["Response", "Dto", "Payload"];

pub(crate) struct LintApiTypes;

impl Check for LintApiTypes {
    fn id(&self) -> &'static str {
        "LINT-API-TYPES"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let spec = std::fs::read_to_string(context.repo.path(SPEC_PATH))
            .with_context(|| format!("could not read {SPEC_PATH}; run `cargo xtask gen api`"))?;
        let schemas = schema_names(&spec)?;
        Ok(evaluate(&read_web_sources(context.repo)?, &schemas))
    }
}

/// The names under `components.schemas`.
fn schema_names(spec: &str) -> anyhow::Result<BTreeSet<String>> {
    let document: serde_json::Value =
        serde_json::from_str(spec).with_context(|| format!("{SPEC_PATH} is not valid JSON"))?;
    Ok(document
        .pointer("/components/schemas")
        .and_then(serde_json::Value::as_object)
        .map(|schemas| schemas.keys().cloned().collect())
        .unwrap_or_default())
}

fn evaluate(sources: &[WebSource], schemas: &BTreeSet<String>) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (path, text) in sources.iter().filter(|(path, _)| path.starts_with(APP)) {
        for (index, line) in text.lines().enumerate() {
            let Some(name) = declared_type(code_of(line)) else {
                continue;
            };
            let why = if schemas.contains(name) {
                Some(format!("`{name}` is a schema of the API"))
            } else {
                SUFFIXES
                    .iter()
                    .find(|suffix| name.ends_with(**suffix) && name.len() > suffix.len())
                    .map(|suffix| format!("`{name}` is named like a server answer (`…{suffix}`)"))
            };
            if let Some(why) = why {
                findings.push(Finding::at(
                    format!("{path}:{}", index + 1),
                    format!(
                        "{why}. Use the generated type: `components[\"schemas\"][\"…\"]` from `@bugaboo/api-client`."
                    ),
                ));
            }
        }
    }
    findings
}

/// The name declared by `type X`, `interface X`, with or without `export` and `declare`.
fn declared_type(code: &str) -> Option<&str> {
    let mut words = code.split_whitespace().peekable();
    while matches!(words.peek(), Some(&"export" | &"declare")) {
        words.next();
    }
    if !matches!(words.next()?, "type" | "interface") {
        return None;
    }
    let name = words.next()?;
    let end = name
        .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
        .unwrap_or(name.len());
    (end > 0).then(|| &name[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(path: &str, text: &str) -> usize {
        let schemas = BTreeSet::from(["Problem".to_owned(), "ServerStatus".to_owned()]);
        evaluate(&[(path.to_owned(), text.to_owned())], &schemas).len()
    }

    #[test]
    fn bad_example() {
        for bad in [
            "export interface Problem { code: string }",
            "type ServerStatus = { status: string };",
            "export type IssueResponse = { id: string };",
            "interface UserDto {\n  name: string;\n}",
            "export declare type Problem = unknown;",
        ] {
            assert_eq!(run("web/app/src/a.ts", bad), 1, "{bad}");
        }
    }

    #[test]
    fn good_example() {
        for good in [
            "import type { components } from \"@bugaboo/api-client\";",
            "type Status = components[\"schemas\"][\"ServerStatus\"];",
            "interface ButtonProps { label: string }",
            "type Response = string;",
            "// interface Problem { }",
        ] {
            assert_eq!(run("web/app/src/a.ts", good), 0, "{good}");
        }
    }

    #[test]
    fn only_the_app_is_checked_not_the_client_package() {
        assert_eq!(
            run(
                "web/packages/api-client/src/x.ts",
                "export type Problem = 1;"
            ),
            0
        );
    }

    #[test]
    fn schema_names_are_read_from_the_document() {
        let spec = r#"{"components":{"schemas":{"A":{},"B":{}}}}"#;
        let names: Vec<String> = schema_names(spec).unwrap().into_iter().collect();
        assert_eq!(names, ["A", "B"]);
        assert!(schema_names("{}").unwrap().is_empty());
    }
}
