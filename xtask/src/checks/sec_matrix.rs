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
//!   project, and under a lead; the person who holds exactly the permission the route declares)
//!   and, as soon as the API description has access keys, the two actors with a key (*See only*;
//!   a key for another project);
//! - every route of the description has a line in the record with the permission it declares, and
//!   was run by every actor of the record;
//! - every route has a row in the table of example requests;
//! - no line names a route that does not exist, and the answers are well formed: nobody signed in is
//!   `not-signed-in` for every route that is not public.

use std::collections::{BTreeMap, BTreeSet};

use bb_api_words::sec_matrix::{self as words, RECORD_PATH};
use bb_api_words::{marks, needs};
use bb_policy::StartingRole;
use serde_json::Value;

use super::api_spec;
use super::{Check, Context, Finding};
use crate::generate::api::SPEC_PATH;
use crate::source::read_text;

/// The table of example requests.
const EXAMPLES_PATH: &str = "crates/bb-server/src/tests/matrix/examples.rs";
/// How a row of the table starts: a plain route, or one with quirks.
const ROW_STARTS: [&str; 2] = ["Example::route(\"", "Example::with(\""];
/// The route that makes an access key.
const KEY_ROUTE: &str = "makeAccessKey";

/// The actors every record has: the fixed ones and one for each starting role.
fn every_actor() -> Vec<String> {
    let roles = StartingRole::ALL
        .iter()
        .map(|role| words::role_actor(role.code()));
    words::FIXED_ACTORS
        .iter()
        .map(|actor| (*actor).to_owned())
        .chain(roles)
        .collect()
}

pub(crate) struct SecMatrix;

impl Check for SecMatrix {
    fn id(&self) -> &'static str {
        "SEC-MATRIX"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let read = |path: &str| read_text(context.repo, path);
        let spec = api_spec::load(context.repo)?;
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
    api_spec::operations(spec)
        .into_iter()
        .flatten()
        .filter_map(|operation| {
            Some(Route {
                operation: operation.text("operationId")?.to_owned(),
                method: operation.method.clone(),
                path: operation.path.to_owned(),
                declared: operation
                    .text(marks::PERMISSION)
                    .unwrap_or_default()
                    .to_owned(),
            })
        })
        .collect()
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
fn rows(examples: &str) -> BTreeSet<String> {
    // Without white space, so a row that rustfmt broke over lines is read as one.
    let text: String = examples.split_whitespace().collect();
    ROW_STARTS
        .iter()
        .flat_map(|start| {
            text.split(start)
                .skip(1)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .filter_map(|rest| {
            rest.split_once('"')
                .map(|(operation, _)| operation.to_owned())
        })
        .collect()
}

/// The actors the record must name: the fixed list, and the two with a key once the API has keys.
fn actor_findings(actors: &[String], routes: &[Route]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let has_keys = routes.iter().any(|route| route.operation == KEY_ROUTE);
    let mut wanted = every_actor();
    if has_keys {
        wanted.extend(words::KEY_ACTORS.map(str::to_owned));
    }
    for actor in wanted {
        if !actors.contains(&actor) {
            let why = if words::KEY_ACTORS.contains(&actor.as_str()) {
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
    findings
}

/// What the record and the table say about one route.
fn route_findings(
    route: &Route,
    rows: &BTreeSet<String>,
    lines: &BTreeMap<String, Line>,
    actors: &[String],
) -> Vec<Finding> {
    let mut found = Vec::new();
    let place = format!("{} {} ({})", route.method, route.path, route.operation);
    if !rows.contains(route.operation.as_str()) {
        found.push(Finding::at(
            EXAMPLES_PATH,
            format!("{place} has no example request: add its row to the table."),
        ));
    }
    let Some(line) = lines.get(&route.operation) else {
        found.push(Finding::at(
            RECORD_PATH,
            format!("{place} is missing from the record of the matrix: run the matrix."),
        ));
        return found;
    };
    if line.method != route.method || line.path != route.path || line.declared != route.declared {
        found.push(Finding::at(
            RECORD_PATH,
            format!(
                "{place} was run as `{} {}` declaring `{}`, but the description says \
                 `{}`: run the matrix again.",
                line.method, line.path, line.declared, route.declared
            ),
        ));
    }
    for actor in actors {
        match line.answers.get(actor) {
            None => found.push(Finding::at(
                RECORD_PATH,
                format!("{place} was not run as `{actor}`."),
            )),
            Some(answer) if !words::ANSWERS.contains(&answer.as_str()) => found.push(Finding::at(
                RECORD_PATH,
                format!("{place} as `{actor}`: `{answer}` is no answer the matrix knows."),
            )),
            Some(_) => {}
        }
    }
    let nobody = line.answers.get(words::NOBODY).map(String::as_str);
    let expected = if route.declared == needs::PUBLIC {
        words::ALLOWED
    } else {
        words::NOT_SIGNED_IN
    };
    if nobody.is_some_and(|answer| answer != expected) {
        found.push(Finding::at(
            RECORD_PATH,
            format!(
                "{place} declares `{}`, so nobody signed in must be `{expected}`, not `{}`.",
                route.declared,
                nobody.unwrap_or_default()
            ),
        ));
    }
    found
}

fn evaluate(spec: &Value, record: Option<&str>, examples: &str) -> Vec<Finding> {
    let routes = routes(spec);
    if routes.is_empty() {
        return vec![Finding::at(SPEC_PATH, "The document has no routes.")];
    }
    let Some(record) = record else {
        return vec![Finding::at(
            RECORD_PATH,
            format!(
                "The record of the matrix is missing. Run the matrix and write it: `{}`.",
                words::write_command()
            ),
        )];
    };
    let (actors, lines) = parse(record);
    let mut findings = actor_findings(&actors, &routes);
    let rows = rows(examples);
    for route in &routes {
        findings.extend(route_findings(route, &rows, &lines, &actors));
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
    for operation in rows
        .iter()
        .filter(|operation| !described.contains(operation.as_str()))
    {
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
            "/api/v1/status": {"get": described("getStatus", needs::PUBLIC)},
            "/api/v1/projects": {"get": described("listProjects", "see-projects")}
        }})
    }

    /// An operation of the description with what it declares.
    fn described(operation: &str, declared: &str) -> Value {
        let mut fields = serde_json::Map::new();
        fields.insert("operationId".to_owned(), operation.into());
        fields.insert(marks::PERMISSION.to_owned(), declared.into());
        Value::Object(fields)
    }

    /// Every actor of a record, and these besides.
    fn actors_with<'a>(extra: &[&'a str], every: &'a [String]) -> Vec<&'a str> {
        let mut all: Vec<&str> = every.iter().map(String::as_str).collect();
        all.extend(extra);
        all
    }

    fn actors_line(extra: &[&str]) -> String {
        let every = every_actor();
        let all = actors_with(extra, &every);
        format!("actors {}\n", all.join(" "))
    }

    fn route_line(
        operation: &str,
        method: &str,
        path: &str,
        declared: &str,
        extra: &[&str],
    ) -> String {
        let nobody = if declared == needs::PUBLIC {
            words::ALLOWED
        } else {
            words::NOT_SIGNED_IN
        };
        let every = every_actor();
        let all = actors_with(extra, &every);
        let answers: Vec<String> = all
            .iter()
            .map(|actor| {
                format!(
                    "{actor}={}",
                    if *actor == words::NOBODY {
                        nobody
                    } else {
                        words::ALLOWED
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
            route_line("getStatus", "GET", "/api/v1/status", needs::PUBLIC, extra),
            route_line(
                "listProjects",
                "GET",
                "/api/v1/projects",
                "see-projects",
                extra
            ),
        )
    }

    const EXAMPLES: &str = "Example::route(\"getStatus\"),\nExample::with(\"listProjects\", &[Quirk::FiltersWhatIsSeen]),\n";

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
        crate::source::normalize(&std::fs::read_to_string(path).unwrap())
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
            route_line("gone", "GET", "/gone", needs::PUBLIC, &[])
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
            json!({"post": described(KEY_ROUTE, needs::OWN_ACCOUNT)});
        let examples = format!("{EXAMPLES}Example::route(\"{KEY_ROUTE}\"),\n");
        // Without the actors the check says so ...
        let record_without = format!(
            "{}{}",
            record(&[]),
            route_line(
                KEY_ROUTE,
                "POST",
                "/api/v1/access-keys",
                needs::OWN_ACCOUNT,
                &[]
            )
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
            record(&words::KEY_ACTORS),
            route_line(
                KEY_ROUTE,
                "POST",
                "/api/v1/access-keys",
                needs::OWN_ACCOUNT,
                &words::KEY_ACTORS
            )
        );
        let found = messages(&with_keys, Some(&with), &examples);
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn a_row_that_rustfmt_broke_over_lines_is_still_a_row() {
        let table = "Example::with(
        \"getStatus\",
        &[Quirk::EndsTheSession],
    ),
Example::route(\"x\"),
";
        assert_eq!(
            rows(table),
            BTreeSet::from(["getStatus".to_owned(), "x".to_owned()])
        );
    }
}
