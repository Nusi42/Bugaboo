//! ARCH-ERRORS: every error code has a message in the catalog, and the API does not make up its
//! own error texts (architecture §7, UX §9).
//!
//! The codes are the enum `ErrorCode` in `crates/bb-core/src/error_code.rs`: a line per code with
//! its text and its HTTP status; its message is `error.<area>.<name>` (the text with `-` for `_`),
//! made from the text. This check reads the enum itself and the catalog: codes and messages are
//! unique, shaped right, and the message exists and has no placeholder (an error may never carry
//! a name or a number that the person is not allowed to see). It also finds
//! error answers built from a text instead of an `ErrorCode`, in the code of the HTTP crates: a
//! tuple `(StatusCode::X, …)` (also across lines) with a text, `format!`, `to_string()` or JSON
//! written in the code, or a bare error status turned into an answer. Last, it reads the generated
//! API description (`web/packages/api-client/openapi.json`): the `ErrorCode` schema lists exactly
//! the codes of the table, and every route documents the standard problem+json answers. Developer
//! texts (the `Display` of error types) never reach a person: an answer can only be made from an
//! `ErrorCode`, and this check finds the ways around that (M1-F16).

use std::collections::{BTreeMap, BTreeSet};

use bb_core::ErrorCode;
use serde_json::Value;

use super::api_spec;
use super::{Check, Context, Finding};
use crate::generate::api::SPEC_PATH;
use crate::generate::icu::placeholders;
use crate::generate::messages::{SOURCE_LOCALE_PATH, source_pairs};
use crate::rust_source::without_tests;
use crate::source::{Kind, Source};

/// Where the codes are declared; only shown in messages.
const TABLE_PATH: &str = "crates/bb-core/src/error_code.rs";

/// The code that speaks HTTP; error answers there must come from an `ErrorCode`.
const HTTP_CODE: [&str; 2] = ["crates/bb-api/", "crates/bb-server/"];

/// Every route documents these answers: going over a limit and failing inside.
const STANDARD_STATUSES: [&str; 3] = ["429", "500", "503"];

/// The handlers; their extractors must be the `Api*` wrappers.
const EXTRACTOR_CODE: &str = "crates/bb-api/";
/// Defines the wrappers, so it may use the extractors they wrap.
const WRAPPERS_PATH: &str = "crates/bb-api/src/errors.rs";
/// Extractors whose own rejections are plain text, with the wrapper to use instead.
const EXTRACTORS: [(&str, &str); 4] = [
    ("Json", "ApiJson"),
    ("Query", "ApiQuery"),
    ("Path", "ApiPath"),
    ("Form", "ApiForm"),
];

pub(crate) struct ArchErrors;

impl Check for ArchErrors {
    fn id(&self) -> &'static str {
        "ARCH-ERRORS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let sources = context.sources(self.id(), Kind::Rust)?;
        let messages = source_pairs(context.repo)?;
        let spec = api_spec::load(context.repo)?;
        Ok(evaluate(&table(), &sources, &messages, &spec))
    }
}

/// One code of the enum.
struct Entry {
    variant: String,
    code: String,
    status: u16,
    /// The key of its message.
    key: String,
}

/// The codes, as the enum declares them.
fn table() -> Vec<Entry> {
    ErrorCode::ALL
        .iter()
        .map(|code| Entry {
            variant: format!("{code:?}"),
            code: code.code().to_owned(),
            status: code.status(),
            key: code.message().as_str().to_owned(),
        })
        .collect()
}

fn evaluate(
    table: &[Entry],
    sources: &[Source],
    messages: &[(String, String)],
    spec: &Value,
) -> Vec<Finding> {
    let mut findings = check_table(table, messages);
    findings.extend(check_spec(spec, table));
    for (path, text) in sources {
        if HTTP_CODE.iter().any(|folder| path.starts_with(folder)) {
            findings.extend(check_answers(path, &without_tests(text)));
        }
        if path.starts_with(EXTRACTOR_CODE) && path != WRAPPERS_PATH {
            findings.extend(check_extractors(path, &without_tests(text)));
        }
    }
    findings
}

fn check_table(table: &[Entry], messages: &[(String, String)]) -> Vec<Finding> {
    let mut findings = Vec::new();
    if table.is_empty() {
        findings.push(Finding::at(TABLE_PATH, "No error codes found."));
    }
    let by_key: BTreeMap<&str, &str> = messages
        .iter()
        .map(|(key, text)| (key.as_str(), text.as_str()))
        .collect();
    let mut codes = BTreeSet::new();
    let mut keys = BTreeSet::new();
    for entry in table {
        let place = format!("{TABLE_PATH}: {}", entry.variant);
        if !codes.insert(entry.code.as_str()) {
            findings.push(Finding::at(
                &place,
                format!("The code `{}` is used twice.", entry.code),
            ));
        }
        if !is_code(&entry.code) {
            findings.push(Finding::at(
                &place,
                format!(
                    "`{}` is not a code; use `area.name` in lower case with underscores.",
                    entry.code
                ),
            ));
        }
        if !(400..600).contains(&entry.status) {
            findings.push(Finding::at(
                &place,
                format!("{} is not an error status; use 400 to 599.", entry.status),
            ));
        }
        if !keys.insert(entry.key.as_str()) {
            findings.push(Finding::at(
                &place,
                format!(
                    "`{}` is the message of two codes; each code needs its own.",
                    entry.key
                ),
            ));
        }
        match by_key.get(entry.key.as_str()) {
            None => findings.push(Finding::at(
                &place,
                format!(
                    "The message `{}` of the code `{}` is not in {SOURCE_LOCALE_PATH}; add it and run `cargo xtask gen messages`.",
                    entry.key, entry.code
                ),
            )),
            Some(text) => {
                if placeholders(text).is_ok_and(|found| !found.is_empty()) {
                    findings.push(Finding::at(
                        &place,
                        format!(
                            "The message `{}` has a placeholder; an error message must not carry a value that may be hidden.",
                            entry.key
                        ),
                    ));
                }
            }
        }
    }
    findings
}

fn is_code(code: &str) -> bool {
    let word = |part: &str| {
        !part.is_empty()
            && part.starts_with(|c: char| c.is_ascii_lowercase())
            && part.chars().all(|c| c.is_ascii_lowercase() || c == '-')
    };
    code.split_once('.')
        .is_some_and(|(area, name)| word(area) && word(name))
}

/// The status constants that are not errors.
fn is_error_status(name: &str) -> bool {
    const NOT_ERRORS: [&str; 10] = [
        "OK",
        "CREATED",
        "ACCEPTED",
        "NO_CONTENT",
        "NOT_MODIFIED",
        "MOVED_PERMANENTLY",
        "FOUND",
        "SEE_OTHER",
        "TEMPORARY_REDIRECT",
        "PERMANENT_REDIRECT",
    ];
    !NOT_ERRORS.contains(&name) && !name.starts_with("SWITCHING") && !name.starts_with("CONTINUE")
}

/// What an element of an answer looks like when its text is written in the code.
fn is_written_text(element: &str) -> bool {
    let element = element.trim();
    element.starts_with('"')
        || element.starts_with("format!")
        || element.starts_with("String::from")
        || element.starts_with("Json(")
        || element.starts_with("axum::Json(")
        || element.starts_with("json!")
        || element.contains(".to_string()")
}

/// The top-level parts of what follows an opening parenthesis, up to the closing one. Strings are
/// skipped whole, so a comma or a bracket inside text does not count.
fn tuple_elements(after_open: &str) -> Vec<&str> {
    let bytes = after_open.as_bytes();
    let mut elements = Vec::new();
    let (mut depth, mut start, mut index) = (0_usize, 0_usize, 0_usize);
    while let Some(&byte) = bytes.get(index) {
        match byte {
            b'"' => {
                index += 1;
                while let Some(&inner) = bytes.get(index) {
                    match inner {
                        b'\\' => index += 1,
                        b'"' => break,
                        _ => {}
                    }
                    index += 1;
                }
            }
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' if depth == 0 => {
                elements.push(&after_open[start..index]);
                return elements;
            }
            b')' | b']' | b'}' => depth -= 1,
            b',' if depth == 0 => {
                elements.push(&after_open[start..index]);
                start = index + 1;
            }
            _ => {}
        }
        index += 1;
    }
    elements
}

/// Finds error answers with a text written in the code: `(StatusCode::NOT_FOUND, "text")`, also
/// over several lines and with more elements (`(status, [headers], format!(…))`), and an error
/// status turned into an answer on its own (`StatusCode::NOT_FOUND.into_response()`), which
/// would have no problem body.
fn check_answers(path: &str, code: &str) -> Vec<Finding> {
    const STATUS: &str = "StatusCode::";
    let mut findings = Vec::new();
    for (at, _) in code.match_indices(STATUS) {
        let after = &code[at + STATUS.len()..];
        let name_end = after
            .find(|c: char| !(c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_'))
            .unwrap_or(after.len());
        let name = &after[..name_end];
        if name.is_empty() || !is_error_status(name) {
            continue;
        }
        let line = code[..at].matches('\n').count() + 1;
        let place = format!("{path}:{line}");
        let rest = after[name_end..].trim_start();
        if rest.starts_with(".into_response()") {
            findings.push(Finding::at(
                place,
                "An error status turned into an answer without a body. Answer with a `Problem` made from an `ErrorCode`.",
            ));
            continue;
        }
        let is_tuple_start = code[..at].trim_end().ends_with('(') && rest.starts_with(',');
        if is_tuple_start
            && tuple_elements(&rest[1..])
                .iter()
                .any(|element| is_written_text(element))
        {
            findings.push(Finding::at(
                place,
                "An error answer with a text written in the code. Answer with a `Problem` made from an `ErrorCode`; its text comes from the message catalog.",
            ));
        }
    }
    findings
}

/// The parameters of every function: the text between its parentheses, up to the arrow.
fn parameter_lists(code: &str) -> Vec<(usize, &str)> {
    let mut lists = Vec::new();
    for (at, _) in code.match_indices("fn ") {
        let before_ok = code[..at]
            .chars()
            .last()
            .is_none_or(|c| !c.is_alphanumeric() && c != '_');
        let Some(open) = code[at..].find('(').map(|offset| at + offset) else {
            continue;
        };
        if !before_ok || code[at..open].contains(['{', ';']) {
            continue;
        }
        let elements = tuple_elements(&code[open + 1..]);
        let length: usize = elements.iter().map(|e| e.len() + 1).sum();
        let end = (open + length).min(code.len());
        lists.push((at, &code[open + 1..end]));
    }
    lists
}

/// Finds handler parameters that use an extractor with plain-text rejections instead of the
/// wrapper that answers as problem+json. Using `Json` to build an answer is fine; only
/// the parameters are looked at.
fn check_extractors(path: &str, code: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (at, parameters) in parameter_lists(code) {
        for (extractor, wrapper) in EXTRACTORS {
            let used = parameters.match_indices(extractor).any(|(position, _)| {
                let before = parameters[..position].chars().last();
                let after = parameters[position + extractor.len()..].chars().next();
                before.is_none_or(|c| !c.is_alphanumeric() && c != '_')
                    && matches!(after, Some('<' | '('))
            });
            if used {
                let line = code[..at].matches('\n').count() + 1;
                findings.push(Finding::at(
                    format!("{path}:{line}"),
                    format!("A handler takes `{extractor}` as an extractor, whose rejections are plain text. Use `{wrapper}` (bb-api errors), which answers as problem+json."),
                ));
            }
        }
    }
    findings
}

/// The generated API description agrees with the table and documents the standard answers.
fn check_spec(spec: &Value, table: &[Entry]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let listed: BTreeSet<&str> = spec
        .pointer("/components/schemas/ErrorCode/enum")
        .and_then(Value::as_array)
        .map(|codes| codes.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let known: BTreeSet<&str> = table.iter().map(|entry| entry.code.as_str()).collect();
    if listed != known {
        findings.push(Finding::at(
            SPEC_PATH,
            "The `ErrorCode` schema of the API description does not list exactly the codes of the table; run `cargo xtask gen api`.",
        ));
    }
    if spec.pointer("/components/schemas/Problem").is_none() {
        findings.push(Finding::at(
            SPEC_PATH,
            "The API description has no `Problem` schema.",
        ));
    }
    for operation in api_spec::operations(spec).into_iter().flatten() {
        for status in STANDARD_STATUSES {
            let content = operation
                .body
                .pointer(&format!("/responses/{status}/content"));
            if content
                .and_then(|content| content.get("application/problem+json"))
                .is_none()
            {
                findings.push(Finding::at(
                    operation.place(),
                    format!("The route does not document the {status} answer as application/problem+json."),
                ));
            }
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(variant: &str, code: &str, status: u16, key: &str) -> Entry {
        Entry {
            variant: variant.to_owned(),
            code: code.to_owned(),
            status,
            key: key.to_owned(),
        }
    }

    fn good_table() -> Vec<Entry> {
        vec![
            entry(
                "WrongCredentials",
                "auth.wrong-credentials",
                401,
                "error.auth.wrong",
            ),
            entry(
                "NotFound",
                "resource.not-found",
                404,
                "error.resource.not-found",
            ),
        ]
    }

    fn messages(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(k, t)| ((*k).to_owned(), (*t).to_owned()))
            .collect()
    }

    /// A description that agrees with `good_table`.
    const GOOD_SPEC: &str = r#"{"components":{"schemas":{"Problem":{},"ErrorCode":{"enum":["auth.wrong-credentials","resource.not-found"]}}},
        "paths":{"/api/v1/x":{"get":{"responses":{
            "429":{"content":{"application/problem+json":{}}},
            "500":{"content":{"application/problem+json":{}}},
            "503":{"content":{"application/problem+json":{}}}}}}}}"#;

    fn run_with_spec(
        table: &[Entry],
        pairs: &[(&str, &str)],
        api: &str,
        spec: &str,
    ) -> Vec<String> {
        let sources = vec![("crates/bb-api/src/router.rs".to_owned(), api.to_owned())];
        let spec = api_spec::parse(spec).unwrap();
        evaluate(table, &sources, &messages(pairs), &spec)
            .into_iter()
            .map(|f| f.message)
            .collect()
    }

    fn run(table: &[Entry], pairs: &[(&str, &str)], api: &str) -> Vec<String> {
        run_with_spec(table, pairs, api, GOOD_SPEC)
    }

    const GOOD_MESSAGES: [(&str, &str); 2] = [
        ("error.auth.wrong", "No."),
        ("error.resource.not-found", "Not here."),
    ];

    #[test]
    fn good_example() {
        assert!(
            run(
                &good_table(),
                &GOOD_MESSAGES,
                "let a = (StatusCode::OK, \"fine\");\nlet b = problem(ErrorCode::NotFound);\n"
            )
            .is_empty()
        );
    }

    #[test]
    fn bad_example() {
        let table = vec![
            entry("A", "auth.x", 401, "error.a"),
            entry("B", "auth.x", 200, "error.a"),
            entry("C", "Auth", 404, "error.missing"),
            entry("D", "x.y", 404, "error.d"),
        ];
        let found = run(
            &table,
            &[("error.a", "No."), ("error.d", "Hello {name}")],
            "return (StatusCode::NOT_FOUND, \"Not found\");\n",
        )
        .join("\n");
        assert!(found.contains("used twice"), "{found}");
        assert!(found.contains("not an error status"), "{found}");
        assert!(found.contains("is not a code"), "{found}");
        assert!(found.contains("message of two codes"), "{found}");
        assert!(found.contains("`error.missing`"), "{found}");
        assert!(found.contains("has a placeholder"), "{found}");
        assert!(found.contains("text written in the code"), "{found}");
    }

    #[test]
    fn answers_are_found_across_lines_and_in_longer_tuples() {
        for bad in [
            "(\n    StatusCode::NOT_FOUND,\n    \"Not found\",\n)",
            "(StatusCode::BAD_REQUEST, [(CONTENT_TYPE, \"text/plain\")], \"no\")",
            "(StatusCode::BAD_REQUEST, headers, format!(\"bad {x}\"))",
            "(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())",
            "(StatusCode::BAD_REQUEST, Json(json!({\"error\": x})))",
            "StatusCode::NOT_FOUND.into_response()",
        ] {
            let found = run(&good_table(), &GOOD_MESSAGES, bad);
            assert_eq!(found.len(), 1, "{bad}: {found:?}");
        }
        for good in [
            "(StatusCode::NOT_FOUND, body).into_response()",
            "json(StatusCode::SERVICE_UNAVAILABLE, HEALTH_NOT_READY_BODY)",
            "(StatusCode::OK, \"fine, with a comma\")",
            "(StatusCode::NO_CONTENT, format!(\"{x}\"))",
            "(StatusCode::FOUND, [(LOCATION, \"/\")])",
        ] {
            let found = run(&good_table(), &GOOD_MESSAGES, good);
            assert!(found.is_empty(), "{good}: {found:?}");
        }
    }

    #[test]
    fn handlers_must_use_the_wrapped_extractors() {
        let run = |code: &str| {
            let sources = vec![("crates/bb-api/src/api.rs".to_owned(), code.to_owned())];
            let spec = api_spec::parse(GOOD_SPEC).unwrap();
            evaluate(&good_table(), &sources, &messages(&GOOD_MESSAGES), &spec).len()
        };
        for bad in [
            "async fn a(Json(body): Json<Input>) {}",
            "async fn a(axum::Json(body): axum::Json<Input>) {}",
            "async fn a(state: State<S>, Query(q): Query<Q>) {}",
            "async fn a(axum::extract::Path(id): axum::extract::Path<u32>) {}",
            "async fn a(Form(f): Form<F>) {}",
        ] {
            assert!(run(bad) >= 1, "{bad}");
        }
        for good in [
            "async fn a(ApiJson(body): ApiJson<Input>) {}",
            "async fn a(ApiQuery(q): ApiQuery<Q>, ApiPath(id): ApiPath<u32>) {}",
            "async fn a() -> axum::Json<Status> { axum::Json(Status) }",
            "async fn a(path: PathBuf) {}",
        ] {
            assert_eq!(run(good), 0, "{good}");
        }
    }

    #[test]
    fn the_api_description_must_agree_with_the_table() {
        let missing_answer = GOOD_SPEC.replace(
            "\"503\":{\"content\":{\"application/problem+json\":{}}}",
            "\"503\":{}",
        );
        let found = run_with_spec(&good_table(), &GOOD_MESSAGES, "", &missing_answer).join("\n");
        assert!(
            found.contains("does not document the 503 answer"),
            "{found}"
        );

        let other_codes = GOOD_SPEC.replace("auth.wrong-credentials", "auth.other");
        let found = run_with_spec(&good_table(), &GOOD_MESSAGES, "", &other_codes).join("\n");
        assert!(found.contains("does not list exactly the codes"), "{found}");
    }

    #[test]
    fn a_code_whose_message_is_not_in_the_catalog_is_found() {
        let found = run(&good_table(), &GOOD_MESSAGES[..1], "").join("\n");
        assert!(
            found.contains("The message `error.resource.not-found`"),
            "{found}"
        );
    }

    #[test]
    fn the_error_codes_of_this_repository_pass() {
        let repo = crate::repo::Repo::discover().unwrap();
        let sources = crate::source::read_sources(&repo, None, Kind::Rust).unwrap();
        let spec = api_spec::load(&repo).unwrap();
        let found = evaluate(&table(), &sources, &source_pairs(&repo).unwrap(), &spec);
        assert!(
            found.is_empty(),
            "{:?}",
            found.iter().map(|f| &f.message).collect::<Vec<_>>()
        );
    }
}
