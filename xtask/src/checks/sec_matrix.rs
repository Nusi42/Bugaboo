//! SEC-MATRIX: the generated permission matrix covers every route of the API description for every
//! kind of actor (ADR-0004, ADR-0025, SEC-01, SEC-02, SEC-03).
//!
//! The matrix is a test (`crates/bb-server/src/tests/matrix`): it runs the example request of every
//! route as every actor against the real server on both databases and writes down what it covered
//! in `tools/checks/sec-matrix-record.txt`; the test fails when the file differs from what ran.
//! This check holds the file and the table of examples against the API description, so a route
//! that is added without its row, its example or its run fails before the tests do:
//!
//! - the record names the fixed list of actors (nobody; a person without a role; each starting
//!   role; a role on the parent project; a role on a sibling project only; *No access* on the
//!   project; the person who holds exactly the permission the route declares) and, as soon as the API description has access keys, the two actors with a key (*See
//!   only*; a key for another project);
//! - every route of the description has a line in the record with the permission it declares, and
//!   was run by every actor of the record;
//! - every route has a row in the table of example requests;
//! - no line names a route that does not exist, and the answers are well formed: nobody signed in is
//!   `not-signed-in` for every route that is not public.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Context as _;
use serde_json::Value;

use super::{Check, Context, Finding};
use crate::generate::api::SPEC_PATH;

/// The record of what the matrix covered.
const RECORD_PATH: &str = "tools/checks/sec-matrix-record.txt";
/// The table of example requests.
const EXAMPLES_PATH: &str = "crates/bb-server/src/tests/matrix/examples.rs";
/// How a row of the table starts.
const ROW_START: &str = "Example::route(\"";
/// The actors every record has.
const ACTORS: [&str; 12] = [
    "nobody",
    "no-role",
    "role-administrator",
    "role-project-lead",
    "role-team-member",
    "role-contributor",
    "role-viewer",
    "role-guest",
    "lead-on-parent",
    "lead-on-sibling-only",
    "no-access-on-target",
    "exactly-the-permission",
];
/// The actors with an access key, wanted once the description has a route that makes a key.
const KEY_ACTORS: [&str; 2] = ["key-see-only", "key-for-another-project"];
/// The route that makes an access key.
const KEY_ROUTE: &str = "makeAccessKey";
/// What an answer is called in the record.
const ANSWERS: [&str; 4] = ["allowed", "not-signed-in", "forbidden", "not-found"];

pub(crate) struct SecMatrix;

impl Check for SecMatrix {
    fn id(&self) -> &'static str {
        "SEC-MATRIX"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let read = |path: &str| {
            std::fs::read_to_string(context.repo.path(path))
                .map(|text| text.replace("\r\n", "\n"))
                .with_context(|| format!("could not read {path}"))
        };
        let spec: Value = serde_json::from_str(&read(SPEC_PATH)?)
            .with_context(|| format!("{SPEC_PATH} is not valid JSON"))?;
        let record = read(RECORD_PATH).ok();
        let examples = read(EXAMPLES_PATH)?;
        Ok(evaluate(&spec, record.as_deref(), &examples))
    }
}

/// One route of the API description.
struct Route {
    operation: String,
    method: String,
    path: String,
    declared: String,
}

fn routes(spec: &Value) -> Vec<Route> {
    let mut routes = Vec::new();
    for (path, item) in spec
        .get("paths")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
    {
        for (method, operation) in item.as_object().into_iter().flatten() {
            let text = |key: &str| {
                operation
                    .get(key)
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            };
            if let Some(operation_id) = text("operationId") {
                routes.push(Route {
                    operation: operation_id,
                    method: method.to_uppercase(),
                    path: path.clone(),
                    declared: text("x-permission").unwrap_or_default(),
                });
            }
        }
    }
    routes
}

/// A line of the record: the route as it was run and what each actor was answered.
struct Line {
    method: String,
    path: String,
    declared: String,
    answers: BTreeMap<String, String>,
}

/// The actors and the routes of a record.
fn parse(record: &str) -> (Vec<String>, BTreeMap<String, Line>) {
    let mut actors = Vec::new();
    let mut lines = BTreeMap::new();
    for line in record.lines().filter(|line| !line.starts_with('#')) {
        if let Some(rest) = line.strip_prefix("actors ") {
            actors = rest.split_whitespace().map(str::to_owned).collect();
        } else if let Some(rest) = line.strip_prefix("route ")
            && let Some((head, answers)) = rest.split_once(" : ")
        {
            let mut head = head.split_whitespace();
            let (Some(operation), Some(method), Some(path), Some(declared)) =
                (head.next(), head.next(), head.next(), head.next())
            else {
                continue;
            };
            lines.insert(
                operation.to_owned(),
                Line {
                    method: method.to_owned(),
                    path: path.to_owned(),
                    declared: declared.to_owned(),
                    answers: answers
                        .split_whitespace()
                        .filter_map(|pair| pair.split_once('='))
                        .map(|(actor, answer)| (actor.to_owned(), answer.to_owned()))
                        .collect(),
                },
            );
        }
    }
    (actors, lines)
}

/// The operations that have a row in the table of examples.
fn rows(examples: &str) -> BTreeSet<&str> {
    examples
        .split(ROW_START)
        .skip(1)
        .filter_map(|rest| rest.split_once('"').map(|(operation, _)| operation))
        .collect()
}

fn evaluate(spec: &Value, record: Option<&str>, examples: &str) -> Vec<Finding> {
    let routes = routes(spec);
    if routes.is_empty() {
        return vec![Finding::at(SPEC_PATH, "The document has no routes.")];
    }
    let Some(record) = record else {
        return vec![Finding::at(
            RECORD_PATH,
            "The record of the matrix is missing. Run the matrix and write it: \
             `BB_SEC_MATRIX_RECORD=write cargo xtask db run -- cargo test -p bb-server sec_01_every_route`.",
        )];
    };
    let (actors, lines) = parse(record);
    let mut findings = Vec::new();
    let has_keys = routes.iter().any(|route| route.operation == KEY_ROUTE);
    let wanted: Vec<&str> = if has_keys {
        ACTORS.iter().chain(&KEY_ACTORS).copied().collect()
    } else {
        ACTORS.to_vec()
    };
    for actor in wanted {
        if !actors.iter().any(|known| known == actor) {
            let why = if KEY_ACTORS.contains(&actor) {
                " The API description has access keys, so a key is an actor of the matrix \
                 (M2-F27): a *See only* key and a key for another project."
            } else {
                ""
            };
            findings.push(Finding::at(
                RECORD_PATH,
                format!("The matrix does not run the actor `{actor}`.{why}"),
            ));
        }
    }
    let rows = rows(examples);
    for route in &routes {
        let place = format!("{} {} ({})", route.method, route.path, route.operation);
        if !rows.contains(route.operation.as_str()) {
            findings.push(Finding::at(
                EXAMPLES_PATH,
                format!("{place} has no example request: add its row to the table."),
            ));
        }
        let Some(line) = lines.get(&route.operation) else {
            findings.push(Finding::at(
                RECORD_PATH,
                format!("{place} is missing from the record of the matrix: run the matrix."),
            ));
            continue;
        };
        if line.method != route.method || line.path != route.path || line.declared != route.declared
        {
            findings.push(Finding::at(
                RECORD_PATH,
                format!(
                    "{place} was run as `{} {}` declaring `{}`, but the description says \
                     `{}`: run the matrix again.",
                    line.method, line.path, line.declared, route.declared
                ),
            ));
        }
        for actor in &actors {
            match line.answers.get(actor) {
                None => findings.push(Finding::at(
                    RECORD_PATH,
                    format!("{place} was not run as `{actor}`."),
                )),
                Some(answer) if !ANSWERS.contains(&answer.as_str()) => findings.push(Finding::at(
                    RECORD_PATH,
                    format!("{place} as `{actor}`: `{answer}` is no answer the matrix knows."),
                )),
                Some(_) => {}
            }
        }
        let nobody = line.answers.get("nobody").map(String::as_str);
        let expected = if route.declared == "public" {
            "allowed"
        } else {
            "not-signed-in"
        };
        if nobody.is_some_and(|answer| answer != expected) {
            findings.push(Finding::at(
                RECORD_PATH,
                format!(
                    "{place} declares `{}`, so nobody signed in must be `{expected}`, not `{}`.",
                    route.declared,
                    nobody.unwrap_or_default()
                ),
            ));
        }
    }
    let described: BTreeSet<&str> = routes
        .iter()
        .map(|route| route.operation.as_str())
        .collect();
    for operation in lines.keys() {
        if !described.contains(operation.as_str()) {
            findings.push(Finding::at(
                RECORD_PATH,
                format!("The record has a route `{operation}` that the API description does not: run the matrix again."),
            ));
        }
    }
    for operation in rows.difference(&described) {
        findings.push(Finding::at(
            EXAMPLES_PATH,
            format!(
                "The table has a row for `{operation}`, which is no route of the API description."
            ),
        ));
    }
    findings
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn spec() -> Value {
        json!({"paths": {
            "/api/v1/status": {"get": {"operationId": "getStatus", "x-permission": "public"}},
            "/api/v1/projects": {"get": {"operationId": "listProjects", "x-permission": "see-projects"}}
        }})
    }

    fn actors_line(extra: &[&str]) -> String {
        let mut all: Vec<&str> = ACTORS.to_vec();
        all.extend(extra);
        format!("actors {}\n", all.join(" "))
    }

    fn route_line(
        operation: &str,
        method: &str,
        path: &str,
        declared: &str,
        extra: &[&str],
    ) -> String {
        let nobody = if declared == "public" {
            "allowed"
        } else {
            "not-signed-in"
        };
        let mut all: Vec<&str> = ACTORS.to_vec();
        all.extend(extra);
        let answers: Vec<String> = all
            .iter()
            .map(|actor| {
                format!(
                    "{actor}={}",
                    if *actor == "nobody" {
                        nobody
                    } else {
                        "allowed"
                    }
                )
            })
            .collect();
        format!(
            "route {operation} {method} {path} {declared} : {}\n",
            answers.join(" ")
        )
    }

    fn record(extra: &[&str]) -> String {
        format!(
            "# written by the matrix\n{}{}{}",
            actors_line(extra),
            route_line("getStatus", "GET", "/api/v1/status", "public", extra),
            route_line(
                "listProjects",
                "GET",
                "/api/v1/projects",
                "see-projects",
                extra
            ),
        )
    }

    const EXAMPLES: &str = "Example::route(\"getStatus\"),\nExample::route(\"listProjects\").filters_what_is_seen(),\n";

    fn messages(spec: &Value, record: Option<&str>, examples: &str) -> Vec<String> {
        evaluate(spec, record, examples)
            .into_iter()
            .map(|finding| finding.message)
            .collect()
    }

    /// The small description, record and table of examples in `tools/checks/fixtures/SEC-MATRIX`.
    fn fixture(kind: &str, name: &str) -> String {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tools/checks/fixtures/SEC-MATRIX")
            .join(kind)
            .join(name);
        std::fs::read_to_string(path).unwrap().replace("\r\n", "\n")
    }

    #[test]
    fn good_example_a_record_that_covers_every_route_for_every_actor() {
        let found = messages(&spec(), Some(&record(&[])), EXAMPLES);
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn good_example_fixture() {
        let spec: Value = serde_json::from_str(&fixture("good", "api.json")).unwrap();
        let found = messages(
            &spec,
            Some(&fixture("good", "record.txt")),
            &fixture("good", "examples.txt"),
        );
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn bad_example_a_record_that_misses_a_route() {
        let spec: Value = serde_json::from_str(&fixture("bad", "api.json")).unwrap();
        let found = messages(
            &spec,
            Some(&fixture("bad", "record-misses-a-route.txt")),
            &fixture("good", "examples.txt"),
        );
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("missing from the record"), "{found:?}");
    }

    #[test]
    fn bad_example_a_route_without_an_example_request() {
        let found = messages(
            &spec(),
            Some(&record(&[])),
            "Example::route(\"getStatus\"),\n",
        );
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("no example request"), "{found:?}");
    }

    #[test]
    fn bad_example_a_missing_record_an_actor_that_did_not_run_and_a_stale_line() {
        assert_eq!(messages(&spec(), None, EXAMPLES).len(), 1);
        // One actor missing from the list of actors.
        let fewer = record(&[]).replace(" no-access-on-target", "");
        let found = messages(&spec(), Some(&fewer), EXAMPLES);
        assert!(
            found
                .iter()
                .any(|m| m.contains("does not run the actor `no-access-on-target`")),
            "{found:?}"
        );
        // A route that is gone.
        let stale = format!(
            "{}{}",
            record(&[]),
            route_line("gone", "GET", "/gone", "public", &[])
        );
        let found = messages(&spec(), Some(&stale), EXAMPLES);
        assert!(found.iter().any(|m| m.contains("`gone`")), "{found:?}");
        // A row for a route that is gone.
        let found = messages(
            &spec(),
            Some(&record(&[])),
            &format!("{EXAMPLES}Example::route(\"gone\"),\n"),
        );
        assert!(
            found.iter().any(|m| m.contains("row for `gone`")),
            "{found:?}"
        );
    }

    #[test]
    fn bad_example_a_route_that_changed_its_declaration_and_an_answer_that_nobody_signed_in_cannot_have()
     {
        let changed =
            record(&[]).replace("/api/v1/projects see-projects", "/api/v1/projects public");
        let found = messages(&spec(), Some(&changed), EXAMPLES);
        assert!(
            found.iter().any(|m| m.contains("run the matrix again")),
            "{found:?}"
        );
        let lax = record(&[]).replace("nobody=not-signed-in", "nobody=allowed");
        let found = messages(&spec(), Some(&lax), EXAMPLES);
        assert!(
            found.iter().any(|m| m.contains("nobody signed in must be")),
            "{found:?}"
        );
    }

    #[test]
    fn good_example_the_actors_with_a_key_are_wanted_once_the_description_has_keys() {
        let mut with_keys = spec();
        with_keys["paths"]["/api/v1/access-keys"] =
            json!({"post": {"operationId": KEY_ROUTE, "x-permission": "own-account"}});
        let examples = format!("{EXAMPLES}Example::route(\"{KEY_ROUTE}\"),\n");
        // Without the actors the check says so ...
        let record_without = format!(
            "{}{}",
            record(&[]),
            route_line(KEY_ROUTE, "POST", "/api/v1/access-keys", "own-account", &[])
        );
        let found = messages(&with_keys, Some(&record_without), &examples);
        assert_eq!(
            found.iter().filter(|m| m.contains("access keys")).count(),
            2,
            "{found:?}"
        );
        // ... and with them it passes.
        let with = format!(
            "{}{}",
            record(&KEY_ACTORS),
            route_line(
                KEY_ROUTE,
                "POST",
                "/api/v1/access-keys",
                "own-account",
                &KEY_ACTORS
            )
        );
        let found = messages(&with_keys, Some(&with), &examples);
        assert!(found.is_empty(), "{found:?}");
    }
}
