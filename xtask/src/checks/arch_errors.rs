//! ARCH-ERRORS: every error code has a message in the catalog, and the API does not make up its
//! own error texts (architecture §7, UX §9).
//!
//! The codes are one table in `crates/bb-core/src/error_code.rs`: a line per code with its text,
//! its HTTP status and the constant of its message. This check reads that table and the catalog:
//! codes and messages are unique, shaped right, and the message exists and has no placeholder (an
//! error may never carry a name or a number that the person is not allowed to see). It also finds
//! error answers built from a text instead of an `ErrorCode`, in the code of the HTTP crates: a
//! tuple `(StatusCode::X, …)` (also across lines) with a text, `format!`, `to_string()` or JSON
//! written in the code, or a bare error status turned into an answer. Last, it reads the generated
//! API description (`web/packages/api-client/openapi.json`): the `ErrorCode` schema lists exactly
//! the codes of the table, and every route documents the standard problem+json answers. Developer
//! texts (the `Display` of error types) never reach a person: an answer can only be made from an
//! `ErrorCode`, and this check finds the ways around that (M1-F16).

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Context as _;

use super::{Check, Context, Finding};
use crate::generate::api::SPEC_PATH;
use crate::generate::icu::placeholders;
use crate::generate::messages::{SOURCE_LOCALE_PATH, parse_pairs};
use crate::generate::messages_rust::constant_name;
use crate::rust_source::without_tests;
use crate::source::{Kind, Source};

/// The table of codes.
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

/// The messages of error codes start with this word.
const KEY_PREFIX: &str = "error.";

pub(crate) struct ArchErrors;

impl Check for ArchErrors {
    fn id(&self) -> &'static str {
        "ARCH-ERRORS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let sources = context.sources(self.id(), Kind::Rust)?;
        let catalog = std::fs::read_to_string(context.repo.path(SOURCE_LOCALE_PATH))
            .with_context(|| format!("could not read {SOURCE_LOCALE_PATH}"))?;
        let messages = parse_pairs(&catalog)?;
        let spec = std::fs::read_to_string(context.repo.path(SPEC_PATH)).ok();
        Ok(evaluate(&sources, &messages, spec.as_deref()))
    }
}

/// One line of the table.
struct Entry {
    variant: String,
    code: String,
    status: u16,
    constant: String,
}

fn evaluate(sources: &[Source], messages: &[(String, String)], spec: Option<&str>) -> Vec<Finding> {
    let mut findings = Vec::new();
    match sources.iter().find(|(path, _)| path == TABLE_PATH) {
        None => findings.push(Finding::at(
            TABLE_PATH,
            "The table of error codes is missing.",
        )),
        Some((_, text)) => match read_table(text) {
            Ok(table) => {
                findings.extend(check_table(&table, messages));
                findings.extend(check_spec(spec, &table));
            }
            Err(problem) => findings.push(Finding::at(
                TABLE_PATH,
                format!("The table of error codes {problem}."),
            )),
        },
    }
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

/// One line of the macro `error_codes! { Variant => ("code", 401, message::CONSTANT), … }`.
struct Row(Entry);

impl syn::parse::Parse for Row {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        syn::Attribute::parse_outer(input)?;
        let variant: syn::Ident = input.parse()?;
        input.parse::<syn::Token![=>]>()?;
        let inner;
        syn::parenthesized!(inner in input);
        let code: syn::LitStr = inner.parse()?;
        inner.parse::<syn::Token![,]>()?;
        let status: syn::LitInt = inner.parse()?;
        inner.parse::<syn::Token![,]>()?;
        let message: syn::Path = inner.parse()?;
        let constant = message
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .unwrap_or_default();
        Ok(Self(Entry {
            variant: variant.to_string(),
            code: code.value(),
            status: status.base10_parse()?,
            constant,
        }))
    }
}

/// The table, read as Rust. A table that cannot be read is an error, not an empty table.
fn read_table(text: &str) -> Result<Vec<Entry>, String> {
    let file = syn::parse_file(text).map_err(|error| format!("is not valid Rust: {error}"))?;
    let mut entries = Vec::new();
    let mut found = false;
    for item in &file.items {
        let syn::Item::Macro(item) = item else {
            continue;
        };
        if item
            .mac
            .path
            .segments
            .last()
            .is_none_or(|s| s.ident != "error_codes")
        {
            continue;
        }
        found = true;
        let rows = item
            .mac
            .parse_body_with(syn::punctuated::Punctuated::<Row, syn::Token![,]>::parse_terminated)
            .map_err(|error| format!("has a line that is not `Name => (\"code\", status, message::CONSTANT)`: {error}"))?;
        entries.extend(rows.into_iter().map(|row| row.0));
    }
    if found {
        Ok(entries)
    } else {
        Err("has no `error_codes! { … }` table".to_owned())
    }
}

fn check_table(table: &[Entry], messages: &[(String, String)]) -> Vec<Finding> {
    let mut findings = Vec::new();
    if table.is_empty() {
        findings.push(Finding::at(TABLE_PATH, "No error codes found; each line must look like `Name => (\"area.name\", 400, message::CONSTANT)`."));
    }
    let by_constant: BTreeMap<String, &(String, String)> = messages
        .iter()
        .map(|pair| (constant_name(&pair.0), pair))
        .collect();
    let mut codes = BTreeSet::new();
    let mut constants = BTreeSet::new();
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
        if !constants.insert(entry.constant.as_str()) {
            findings.push(Finding::at(
                &place,
                format!(
                    "`{}` is the message of two codes; each code needs its own.",
                    entry.constant
                ),
            ));
        }
        match by_constant.get(&entry.constant) {
            None => findings.push(Finding::at(
                &place,
                format!("`{}` is not a message without placeholders in {SOURCE_LOCALE_PATH}; add the message and run `cargo xtask gen messages`.", entry.constant),
            )),
            Some((key, text)) => {
                if !key.starts_with(KEY_PREFIX) {
                    findings.push(Finding::at(&place, format!("The message `{key}` must start with `{KEY_PREFIX}`.")));
                }
                if placeholders(text).is_ok_and(|found| !found.is_empty()) {
                    findings.push(Finding::at(&place, format!("The message `{key}` has a placeholder; an error message must not carry a value that may be hidden.")));
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
            && part.chars().all(|c| c.is_ascii_lowercase() || c == '_')
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
fn check_spec(spec: Option<&str>, table: &[Entry]) -> Vec<Finding> {
    let Some(text) = spec else {
        return vec![Finding::at(
            SPEC_PATH,
            "The API description is missing; run `cargo xtask gen api`.",
        )];
    };
    let Ok(document) = serde_json::from_str::<serde_json::Value>(text) else {
        return vec![Finding::at(
            SPEC_PATH,
            "The API description is not valid JSON.",
        )];
    };
    let mut findings = Vec::new();
    let listed: BTreeSet<&str> = document
        .pointer("/components/schemas/ErrorCode/enum")
        .and_then(serde_json::Value::as_array)
        .map(|codes| codes.iter().filter_map(serde_json::Value::as_str).collect())
        .unwrap_or_default();
    let known: BTreeSet<&str> = table.iter().map(|entry| entry.code.as_str()).collect();
    if listed != known {
        findings.push(Finding::at(
            SPEC_PATH,
            "The `ErrorCode` schema of the API description does not list exactly the codes of the table; run `cargo xtask gen api`.",
        ));
    }
    if document.pointer("/components/schemas/Problem").is_none() {
        findings.push(Finding::at(
            SPEC_PATH,
            "The API description has no `Problem` schema.",
        ));
    }
    let paths = document
        .get("paths")
        .and_then(serde_json::Value::as_object)
        .into_iter()
        .flatten();
    for (path, item) in paths {
        for (method, operation) in item.as_object().into_iter().flatten() {
            for status in STANDARD_STATUSES {
                let content = operation.pointer(&format!("/responses/{status}/content"));
                if content
                    .and_then(|content| content.get("application/problem+json"))
                    .is_none()
                {
                    findings.push(Finding::at(
                        format!("{} {path}", method.to_uppercase()),
                        format!("The route does not document the {status} answer as application/problem+json."),
                    ));
                }
            }
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: &str = "error_codes! {\n    /// Wrong.\n    WrongCredentials => (\"auth.wrong_credentials\", 401, message::ERROR_AUTH_WRONG),\n    /// Gone.\n    NotFound => (\"resource.not_found\", 404, message::ERROR_RESOURCE_NOT_FOUND),\n}\n";

    fn messages(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(k, t)| ((*k).to_owned(), (*t).to_owned()))
            .collect()
    }

    /// A description that agrees with `TABLE`.
    const GOOD_SPEC: &str = r#"{"components":{"schemas":{"Problem":{},"ErrorCode":{"enum":["auth.wrong_credentials","resource.not_found"]}}},
        "paths":{"/api/v1/x":{"get":{"responses":{
            "429":{"content":{"application/problem+json":{}}},
            "500":{"content":{"application/problem+json":{}}},
            "503":{"content":{"application/problem+json":{}}}}}}}}"#;

    fn run_with_spec(table: &str, pairs: &[(&str, &str)], api: &str, spec: &str) -> Vec<String> {
        let sources = vec![
            (TABLE_PATH.to_owned(), table.to_owned()),
            ("crates/bb-api/src/router.rs".to_owned(), api.to_owned()),
        ];
        evaluate(&sources, &messages(pairs), Some(spec))
            .into_iter()
            .map(|f| f.message)
            .collect()
    }

    fn run(table: &str, pairs: &[(&str, &str)], api: &str) -> Vec<String> {
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
                TABLE,
                &GOOD_MESSAGES,
                "let a = (StatusCode::OK, \"fine\");\nlet b = problem(ErrorCode::NotFound);\n"
            )
            .is_empty()
        );
    }

    #[test]
    fn bad_example() {
        let table = r#"error_codes! {
    A => ("auth.x", 401, message::ERROR_A),
    B => ("auth.x", 200, message::ERROR_A),
    C => ("Auth", 404, message::ERROR_MISSING),
    D => ("x.y", 404, message::ERROR_D),
}
"#;
        let found = run(
            table,
            &[("error.a", "No."), ("error.d", "Hello {name}")],
            "return (StatusCode::NOT_FOUND, \"Not found\");\n",
        )
        .join("\n");
        assert!(found.contains("used twice"), "{found}");
        assert!(found.contains("not an error status"), "{found}");
        assert!(found.contains("is not a code"), "{found}");
        assert!(found.contains("message of two codes"), "{found}");
        assert!(found.contains("ERROR_MISSING"), "{found}");
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
            let found = run(TABLE, &GOOD_MESSAGES, bad);
            assert_eq!(found.len(), 1, "{bad}: {found:?}");
        }
        for good in [
            "(StatusCode::NOT_FOUND, body).into_response()",
            "json(StatusCode::SERVICE_UNAVAILABLE, HEALTH_NOT_READY_BODY)",
            "(StatusCode::OK, \"fine, with a comma\")",
            "(StatusCode::NO_CONTENT, format!(\"{x}\"))",
            "(StatusCode::FOUND, [(LOCATION, \"/\")])",
        ] {
            let found = run(TABLE, &GOOD_MESSAGES, good);
            assert!(found.is_empty(), "{good}: {found:?}");
        }
    }

    #[test]
    fn handlers_must_use_the_wrapped_extractors() {
        let run = |code: &str| {
            let sources = vec![
                (TABLE_PATH.to_owned(), TABLE.to_owned()),
                ("crates/bb-api/src/api.rs".to_owned(), code.to_owned()),
            ];
            evaluate(&sources, &messages(&GOOD_MESSAGES), Some(GOOD_SPEC)).len()
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
        let found = run_with_spec(TABLE, &GOOD_MESSAGES, "", &missing_answer).join("\n");
        assert!(
            found.contains("does not document the 503 answer"),
            "{found}"
        );

        let other_codes = GOOD_SPEC.replace("auth.wrong_credentials", "auth.other");
        let found = run_with_spec(TABLE, &GOOD_MESSAGES, "", &other_codes).join("\n");
        assert!(found.contains("does not list exactly the codes"), "{found}");

        let sources = vec![(TABLE_PATH.to_owned(), TABLE.to_owned())];
        let none = evaluate(&sources, &messages(&GOOD_MESSAGES), None);
        assert!(none.iter().any(|f| f.message.contains("is missing")));
    }

    #[test]
    fn a_missing_or_unreadable_table_is_an_error_not_an_empty_table() {
        let none = run(
            "fn unrelated() {}
",
            &GOOD_MESSAGES,
            "",
        )
        .join(
            "
",
        );
        assert!(none.contains("has no `error_codes!"), "{none}");
        let broken = run(
            "error_codes! { A => (\"x.y\" 401, m::A) }
",
            &GOOD_MESSAGES,
            "",
        )
        .join(
            "
",
        );
        assert!(broken.contains("has a line that is not"), "{broken}");
    }

    #[test]
    fn the_error_table_of_this_repository_passes() {
        let repo = crate::repo::Repo::discover().unwrap();
        let sources = crate::source::read_sources(&repo, None, Kind::Rust).unwrap();
        let text = std::fs::read_to_string(repo.path(SOURCE_LOCALE_PATH)).unwrap();
        let spec = std::fs::read_to_string(repo.path(SPEC_PATH)).ok();
        let found = evaluate(&sources, &parse_pairs(&text).unwrap(), spec.as_deref());
        assert!(
            found.is_empty(),
            "{:?}",
            found.iter().map(|f| &f.message).collect::<Vec<_>>()
        );
    }
}
