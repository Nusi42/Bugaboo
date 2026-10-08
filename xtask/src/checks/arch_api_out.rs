//! ARCH-API-OUT: what leaves the API is an `Authorized<T>` (ADR-0004, ADR-0025 decision 10,
//! architecture §7 *Enforcement points*).
//!
//! The type system carries most of it: `answer_authorized` and `Page::of_authorized` take an
//! `Authorized<T>`, which only `bb-policy` can make. This check finds the ways around them that a
//! text can show, in `bb-api` and `bb-mcp`:
//!
//! - **any function** that answers through the plain wrapper `answer(…)` or the sign-in answer
//!   `signed_in(…)`, which take any value, a handler or a helper that a handler calls: a handler
//!   (a function with `#[utoipa::path]`) must have its operation in
//!   `tools/checks/api-out-exemptions.toml` with a reason (the steps of signing in, which are
//!   about the caller and hold nothing of anybody else), a helper its name and file. The wrappers
//!   themselves are named that way too. `answer(Page::of_authorized(…))` is a page of authorized
//!   items and needs no entry;
//! - **`axum::Json`** used to answer (`Json(…)`, `Json<…>`) anywhere outside the functions named in
//!   the same file with a reason (the wrapper itself, the health answer, the description of the
//!   API): a handler that answers a plain struct this way never asked the engine.
//!
//! An entry that no longer matches a use fails, so the file shrinks as routes move to
//! `Authorized<T>`. Comments, string and character literals do not count, and neither does code
//! behind `#[cfg(test)]`.

use std::collections::BTreeSet;

use anyhow::Context as _;
use serde::Deserialize;

use super::{Check, Context, Finding};
use crate::rust_source::{strip, without_tests};
use crate::source::{Kind, Source, is_word_char, name_uses};

/// The crates whose answers leave the program.
const FOLDERS: [&str; 2] = ["crates/bb-api/src/", "crates/bb-mcp/src/"];
/// Where the exceptions are written down, each with its reason.
const EXEMPTIONS_PATH: &str = "tools/checks/api-out-exemptions.toml";
/// The wrappers that take any value.
const PLAIN: [&str; 2] = ["answer", "signed_in"];
/// What a page of authorized items starts with: its argument is made from `Authorized<T>`.
const AUTHORIZED_PAGE: &str = "Page::of_authorized(";
/// The shortest reason that can say anything.
const REASON_MIN: usize = 20;

pub(crate) struct ArchApiOut;

impl Check for ArchApiOut {
    fn id(&self) -> &'static str {
        "ARCH-API-OUT"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let text = std::fs::read_to_string(context.repo.path(EXEMPTIONS_PATH))
            .with_context(|| format!("could not read {EXEMPTIONS_PATH}"))?;
        let exemptions: Exemptions =
            toml::from_str(&text).with_context(|| format!("{EXEMPTIONS_PATH} is not valid"))?;
        Ok(evaluate(
            &context.sources(self.id(), Kind::Rust)?,
            &exemptions.exempt,
        ))
    }
}

/// The file of exceptions.
#[derive(Deserialize)]
struct Exemptions {
    #[serde(default)]
    exempt: Vec<Exemption>,
}

/// One exception: a handler by its operation ID (may use the plain wrappers), or a function by its
/// name and file (may build JSON itself), and why.
#[derive(Deserialize)]
struct Exemption {
    operation: Option<String>,
    function: Option<String>,
    file: Option<String>,
    reason: String,
}

/// A function found in a source: where it stands and, for a handler, its operation ID.
struct Function {
    name: String,
    /// The first and the last line of the function, signature and body (from 1).
    lines: (usize, usize),
    /// The code of the function (signature and body), comments and texts blanked.
    code: String,
    operation: Option<String>,
}

/// The text up to the bracket that closes the first `open` in `text`, brackets included.
fn balanced(text: &str, open: char, close: char) -> Option<&str> {
    let mut depth = 0_usize;
    let mut started = false;
    for (at, character) in text.char_indices() {
        if character == open {
            depth += 1;
            started = true;
        } else if character == close && started {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(&text[..at + close.len_utf8()]);
            }
        }
    }
    None
}

/// The operation ID in the attribute that stands above the function that starts on `line` (from
/// 1), read from the original text: the attribute, its lines and the doc comments above it, up to
/// the first line that is none of them.
fn operation_above(original: &[&str], line: usize) -> Option<String> {
    let mut above = Vec::new();
    for text in original[..line - 1].iter().rev() {
        let belongs = text.starts_with(char::is_whitespace)
            || text.starts_with('#')
            || text.starts_with(')')
            || text.starts_with("//");
        if !belongs {
            break;
        }
        above.push(*text);
    }
    above.reverse();
    let block = above.join("\n");
    let rest = block.split_once("operation_id")?.1;
    let rest = rest.trim_start().strip_prefix('=')?.trim_start();
    let rest = rest.strip_prefix('"')?;
    Some(rest.split_once('"')?.0.to_owned())
}

/// Every function of the source, with its code and (for a handler) its operation ID.
fn functions(original: &str, code: &str) -> Vec<Function> {
    let original_lines: Vec<&str> = original.lines().collect();
    let mut found = Vec::new();
    for (at, _) in code.match_indices("fn ") {
        if code[..at].chars().next_back().is_some_and(is_word_char) {
            continue;
        }
        let rest = &code[at + 3..];
        let name: String = rest.chars().take_while(|c| is_word_char(*c)).collect();
        if name.is_empty() {
            continue;
        }
        // The signature runs to the first `{` or `;` outside the parentheses.
        let mut depth = 0_i32;
        let mut body_at = None;
        for (offset, character) in rest.char_indices() {
            match character {
                '(' => depth += 1,
                ')' => depth -= 1,
                '{' if depth == 0 => {
                    body_at = Some(offset);
                    break;
                }
                ';' if depth == 0 => break,
                _ => {}
            }
        }
        let Some(body_at) = body_at else {
            continue;
        };
        let Some(body) = balanced(&rest[body_at..], '{', '}') else {
            continue;
        };
        let end = at + 3 + body_at + body.len();
        let first = code[..at].matches('\n').count() + 1;
        let last = code[..end].matches('\n').count() + 1;
        found.push(Function {
            name,
            lines: (first, last),
            code: code[at..end].to_owned(),
            operation: operation_above(&original_lines, first),
        });
    }
    found
}

/// The places in `code` where `word` is called or named as a whole name, not as the method of a
/// value (after a dot), not as a definition (after `fn`) and not as part of a longer name.
fn calls<'a>(code: &'a str, word: &'a str) -> impl Iterator<Item = usize> + 'a {
    name_uses(code, word).filter(move |at| {
        code[at + word.len()..]
            .chars()
            .next()
            .is_some_and(|c| matches!(c, '(' | '<' | ':'))
    })
}

/// What the argument of the call that starts at `at` (the word, then `(`) begins with.
fn argument_at(code: &str, at: usize, word: &str) -> String {
    let after = &code[at + word.len()..];
    after
        .strip_prefix('(')
        .map(|inner| inner.trim_start().chars().take(40).collect())
        .unwrap_or_default()
}

fn evaluate(sources: &[Source], exemptions: &[Exemption]) -> Vec<Finding> {
    let mut findings = Vec::new();
    // What each exemption was needed for.
    let mut used: BTreeSet<usize> = BTreeSet::new();
    for (path, text) in sources {
        if !FOLDERS.iter().any(|folder| path.starts_with(folder)) {
            continue;
        }
        let code = strip(&without_tests(text));
        for function in functions(text, &code) {
            let place = |line: usize| format!("{path}:{line}");
            let allowed_operation = |used: &mut BTreeSet<usize>| {
                let index = exemptions.iter().position(|exemption| {
                    exemption.operation.is_some() && exemption.operation == function.operation
                })?;
                used.insert(index);
                Some(index)
            };
            let allowed_function = |used: &mut BTreeSet<usize>| {
                let index = exemptions.iter().position(|exemption| {
                    exemption.function.as_deref() == Some(function.name.as_str())
                        && exemption.file.as_deref() == Some(path.as_str())
                })?;
                used.insert(index);
                Some(index)
            };
            // Raw JSON: `Json(…)`, `Json<…>`, `Json::…`, the type or the constructor of the answer.
            if let Some(at) = calls(&function.code, "Json").next() {
                let line = function.lines.0 + function.code[..at].matches('\n').count();
                if allowed_function(&mut used).is_none() && allowed_operation(&mut used).is_none() {
                    findings.push(Finding::at(
                        place(line),
                        format!(
                            "`{}` answers with `axum::Json`, which takes any value. Answer with \
                             `answer_authorized(&evaluation.authorize(view))` or a page made by \
                             `Page::of_authorized`, so only an `Authorized<T>` leaves.",
                            function.name
                        ),
                    ));
                }
            }
            // Any function, a handler or a helper, that answers through the wrappers that take any
            // value: a handler by its operation, a helper (and the wrappers themselves) by name.
            for word in PLAIN {
                for at in calls(&function.code, word) {
                    if argument_at(&function.code, at, word).starts_with(AUTHORIZED_PAGE) {
                        continue;
                    }
                    if allowed_operation(&mut used).is_some()
                        || allowed_function(&mut used).is_some()
                    {
                        continue;
                    }
                    let line = function.lines.0 + function.code[..at].matches('\n').count();
                    let (what, how) = if function.operation.is_some() {
                        ("handler", "name the operation")
                    } else {
                        ("function", "name the function and its file")
                    };
                    findings.push(Finding::at(
                        place(line),
                        format!(
                            "The {what} `{}` answers with `{word}(…)`, which takes any value. \
                             Answer with `answer_authorized` and a value made by the engine, or \
                             {how} in {EXEMPTIONS_PATH} with the reason it holds nothing of \
                             anybody but the caller.",
                            function.name
                        ),
                    ));
                }
            }
        }
    }
    findings.extend(exemption_findings(exemptions, &used));
    findings
}

/// The rules of the file: each entry names one thing, has a reason, and is still needed.
fn exemption_findings(exemptions: &[Exemption], used: &BTreeSet<usize>) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (index, exemption) in exemptions.iter().enumerate() {
        let name = exemption
            .operation
            .as_deref()
            .or(exemption.function.as_deref())
            .unwrap_or("?");
        let well_formed = matches!(
            (&exemption.operation, &exemption.function, &exemption.file),
            (Some(_), None, None) | (None, Some(_), Some(_))
        );
        if !well_formed {
            findings.push(Finding::at(
                EXEMPTIONS_PATH,
                format!(
                    "The entry `{name}` names either an `operation`, or a `function` with its \
                     `file`."
                ),
            ));
        } else if !used.contains(&index) {
            findings.push(Finding::at(
                EXEMPTIONS_PATH,
                format!("The entry `{name}` matches nothing that needs it any more. Remove it."),
            ));
        }
        if exemption.reason.trim().len() < REASON_MIN {
            findings.push(Finding::at(
                EXEMPTIONS_PATH,
                format!("The entry `{name}` needs a reason that says why it may."),
            ));
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(path: &str, text: &str) -> Vec<Source> {
        vec![(path.to_owned(), text.to_owned())]
    }

    fn operation(name: &str) -> Exemption {
        Exemption {
            operation: Some(name.to_owned()),
            function: None,
            file: None,
            reason: "A step of signing in: it is about the caller.".to_owned(),
        }
    }

    fn function(name: &str, file: &str) -> Exemption {
        Exemption {
            operation: None,
            function: Some(name.to_owned()),
            file: Some(file.to_owned()),
            reason: "The wrapper itself: every other answer comes through it.".to_owned(),
        }
    }

    const HANDLER: &str = "/// Lists people.\n#[utoipa::path(\n    get,\n    path = \"/api/v1/x\",\n    operation_id = \"listX\",\n    responses((status = 200, body = XBody))\n)]\nasync fn list_x(State(auth): State<AuthHandle>) -> Result<Response, ApiError> {\n";
    const FILE: &str = "crates/bb-api/src/api/x.rs";

    fn handler(body: &str) -> String {
        format!("{HANDLER}{body}\n}}\n")
    }

    #[test]
    fn good_example_a_handler_answers_with_an_authorized_value_or_a_page_of_them() {
        let text = handler(
            "    let page = Page::of_authorized(LIST, &list, &fields)?;\n    answer_authorized(&detail.map(Body::from))?;\n    Ok(answer(Page::of_authorized(LIST, &list, &fields)?))",
        );
        let found = evaluate(&source(FILE, &text), &[]);
        assert!(
            found.is_empty(),
            "{:?}",
            found.iter().map(|f| &f.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn bad_example_a_handler_answers_a_plain_struct_through_the_wrapper() {
        for call in [
            "    Ok(answer(SessionsBody { sessions }))",
            "    Ok(answer(\n        SessionsBody { sessions },\n    ))",
            "    signed_in(&started)",
        ] {
            let found = evaluate(&source(FILE, &handler(call)), &[]);
            assert_eq!(found.len(), 1, "{call}");
            assert!(found[0].message.contains("takes any value"), "{call}");
        }
    }

    #[test]
    fn bad_example_a_handler_answers_with_axum_json() {
        for text in [
            "async fn get_x() -> axum::Json<XBody> {\n    axum::Json(XBody { x: 1 })\n}\n",
            "fn helper() -> Response {\n    Json(plain).into_response()\n}\n",
            "fn helper() -> Response {\n    axum::Json::<Plain>::from(plain).into_response()\n}\n",
        ] {
            let found = evaluate(&source(FILE, text), &[]);
            assert_eq!(found.len(), 1, "{text}");
            assert!(found[0].message.contains("axum::Json"), "{text}");
        }
    }

    #[test]
    fn good_example_the_exempt_operation_and_the_exempt_function_may() {
        let sign_in = handler("    Ok(answer(SignedInBody { person }))");
        let exemptions = [operation("listX")];
        assert!(evaluate(&source(FILE, &sign_in), &exemptions).is_empty());
        let wrapper = "pub(crate) fn answer<T: Serialize>(body: T) -> Response {\n    axum::Json(body).into_response()\n}\n";
        let exemptions = [function("answer", FILE)];
        assert!(evaluate(&source(FILE, wrapper), &exemptions).is_empty());
        // A function of the same name in another file is not exempt.
        assert_eq!(
            evaluate(&source("crates/bb-api/src/other.rs", wrapper), &exemptions).len(),
            2,
            "the use and the entry that nothing needs"
        );
    }

    #[test]
    fn good_example_definitions_methods_extractors_comments_texts_and_tests_are_not_answers() {
        let text = "pub(crate) fn answer<T: Serialize>(body: T) -> u8 { 1 }\n\
                    async fn h(ApiJson(body): ApiJson<Body>) { why.answer::<S>(&e, c); let a = other_answer(1); }\n\
                    // axum::Json(plain) answer(plain)\n\
                    const T: &str = \"axum::Json(plain)\";\n\
                    #[cfg(test)]\nmod tests {\n    fn t() { axum::Json(plain); }\n}\n";
        assert!(evaluate(&source(FILE, text), &[]).is_empty());
        // Code outside the two crates is not looked at.
        let elsewhere = evaluate(
            &source(
                "crates/bb-server/src/x.rs",
                "fn f() { axum::Json(plain); }\n",
            ),
            &[],
        );
        assert!(elsewhere.is_empty());
    }

    #[test]
    fn bad_example_an_entry_that_nothing_needs_or_is_badly_made_or_has_no_reason() {
        let clean = handler("    answer_authorized(&x)");
        let mut short = operation("listX");
        short.reason = "ok".to_owned();
        let both = Exemption {
            operation: Some("a".to_owned()),
            function: Some("b".to_owned()),
            file: None,
            reason: "Both an operation and a function: not one thing.".to_owned(),
        };
        let found = evaluate(&source(FILE, &clean), &[operation("listX"), short, both]);
        let messages: Vec<&str> = found.iter().map(|f| f.message.as_str()).collect();
        assert!(
            messages.iter().any(|m| m.contains("matches nothing")),
            "{messages:?}"
        );
        assert!(
            messages.iter().any(|m| m.contains("needs a reason")),
            "{messages:?}"
        );
        assert!(
            messages.iter().any(|m| m.contains("either an `operation`")),
            "{messages:?}"
        );
    }

    #[test]
    fn the_line_of_a_finding_is_the_line_of_the_call() {
        let text = handler("    let a = 1;\n    Ok(answer(Body {}))");
        let found = evaluate(&source(FILE, &text), &[]);
        let line = found[0].location.as_deref().unwrap_or_default();
        assert!(line.ends_with(":10"), "{line}");
    }

    #[test]
    fn bad_example_a_helper_answers_a_plain_struct_for_a_handler() {
        // The handler calls a helper that answers: the helper is a function like any other.
        let helper = "fn sessions_body(list: Vec<Session>) -> Response {\n    answer(SessionsBody { list })\n}\n\
                      fn other(started: &Started) -> Result<Response, ApiError> {\n    signed_in(started)\n}\n";
        let found = evaluate(&source(FILE, helper), &[]);
        assert_eq!(
            found.len(),
            2,
            "{:?}",
            found.iter().map(|f| &f.message).collect::<Vec<_>>()
        );
        assert!(found.iter().all(|f| f.message.contains("The function")));
        // It may when it is named with its file, and the entry is then needed.
        let exemptions = [function("sessions_body", FILE), function("other", FILE)];
        assert!(evaluate(&source(FILE, helper), &exemptions).is_empty());
        // A helper that answers a page of authorized items needs no entry.
        let page = "fn page_body() -> Response {\n    answer(Page::of_authorized(LIST, &list, &fields)?)\n}\n";
        assert!(evaluate(&source(FILE, page), &[]).is_empty());
    }
}
