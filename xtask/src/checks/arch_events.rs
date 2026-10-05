//! ARCH-EVENTS: every command use case writes through the change port, which appends the domain
//! event in the same transaction (ADR-0007).
//!
//! The port is the trait `ChangeStore` in `bb-app` (its method `record_change` runs the change,
//! appends the event and the outbox entry, and commits them together; `bb-store` implements it).
//! A *use case* is an `async fn` of any visibility in `bb-app` that takes the context (a
//! parameter whose type mentions `Ctx`). It is a *query* when its input type ends in `Query`;
//! every other use case is a command, and its body must call `record_change`. Comments and
//! string literals are removed before the code is read, so a mention of the call does not count.
//! Calling a command a query is a way around the rule; review looks at the names (the type of the
//! input is part of the signature, not just a parameter name). This check reads the code; the
//! generated test that runs every command against the in-memory fakes and asserts the event
//! comes with the first use cases (M1-F18). The table of that test (`crates/bb-app/src/tests/events.rs`)
//! must name every public command, as a string with its name (a private function that writes a
//! change is a part of one), so a new command cannot be left untested.

use super::{Check, Context, Finding};
use crate::rust_source::{Source, read_product_sources, strip, without_tests};

/// Where the use cases and the port live.
const USE_CASES_FOLDER: &str = "crates/bb-app/src/";
/// The port every command writes through.
const PORT: &str = "trait ChangeStore";
/// The method of the port that writes a change with its event.
const RECORD: &str = "record_change";
/// The file with the table of the test that runs every command.
const TABLE_FILE: &str = "crates/bb-app/src/tests/events.rs";
/// The ending of the input type of a use case that only reads.
const QUERY_SUFFIX: &str = "Query";

pub(crate) struct ArchEvents;

impl Check for ArchEvents {
    fn id(&self) -> &'static str {
        "ARCH-EVENTS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let table = std::fs::read_to_string(context.repo.path(TABLE_FILE))
            .ok()
            .map(|text| text.replace("\r\n", "\n"));
        Ok(evaluate(
            &read_product_sources(context.repo)?,
            table.as_deref(),
        ))
    }
}

/// The text between the bracket at the start of `text` and its partner, brackets included.
fn balanced(text: &str, open: char, close: char) -> Option<&str> {
    let mut depth = 0_usize;
    for (at, character) in text.char_indices() {
        if character == open {
            depth += 1;
        } else if character == close {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(&text[..at + close.len_utf8()]);
            }
        }
    }
    None
}

/// The types of the parameters of a parameter list `( … )`.
fn parameter_types(parameters: &str) -> Vec<String> {
    let inner = &parameters[1..parameters.len() - 1];
    let mut types = Vec::new();
    let mut depth = 0_i32;
    let mut current = String::new();
    for character in inner.chars() {
        match character {
            '(' | '<' | '[' | '{' => depth += 1,
            ')' | '>' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                types.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(character);
    }
    types.push(current);
    types
        .into_iter()
        .filter_map(|parameter| {
            parameter
                .split_once(':')
                .map(|(_, ty)| ty.trim().to_owned())
        })
        .collect()
}

/// Whether `ty` mentions the word `word`.
fn mentions(ty: &str, word: &str) -> bool {
    ty.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .any(|part| part == word)
}

/// Whether the type is a query type: the last name in it ends in `Query`.
fn is_query(ty: &str) -> bool {
    let without_generics = ty.split('<').next().unwrap_or(ty);
    let name = without_generics
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .rfind(|part| !part.is_empty() && *part != "mut" && *part != "impl");
    name.is_some_and(|name| name.ends_with(QUERY_SUFFIX))
}

fn evaluate(sources: &[Source], table: Option<&str>) -> Vec<Finding> {
    let mut findings = Vec::new();
    let in_app: Vec<&Source> = sources
        .iter()
        .filter(|(path, _)| path.starts_with(USE_CASES_FOLDER))
        .collect();
    if in_app.is_empty() {
        return findings;
    }
    let defines_port = in_app.iter().any(|(_, text)| {
        let code = strip(&without_tests(text));
        code.contains(PORT) && code.contains(&format!("fn {RECORD}"))
    });
    if !defines_port {
        findings.push(Finding::at(
            USE_CASES_FOLDER,
            format!(
                "`bb-app` does not define the port `{PORT}` with a method `{RECORD}`. Commands \
                 write through it, and `bb-store` implements it (ADR-0007)."
            ),
        ));
    }
    if table.is_none() {
        findings.push(Finding::at(
            TABLE_FILE,
            "The table of the test that runs every command is missing (ADR-0007, M1-F18).",
        ));
    }
    for (path, text) in in_app {
        let code = strip(&without_tests(text));
        for (at, _) in code.match_indices("async fn ") {
            let rest = &code[at + "async fn ".len()..];
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            let Some(open) = rest.find('(') else { continue };
            let Some(parameters) = balanced(&rest[open..], '(', ')') else {
                continue;
            };
            let types = parameter_types(parameters);
            if !types.iter().any(|ty| mentions(ty, "Ctx")) || types.iter().any(|ty| is_query(ty)) {
                continue;
            }
            let after = &rest[open + parameters.len()..];
            let Some(body_start) = after.find('{') else {
                continue;
            };
            let Some(body) = balanced(&after[body_start..], '{', '}') else {
                continue;
            };
            if body.contains(&format!("{RECORD}(")) {
                let line_start = code[..at].rfind('\n').map_or(0, |newline| newline + 1);
                let is_public = code[line_start..at].trim_start().starts_with("pub");
                if is_public && table.is_some_and(|table| !table.contains(&format!("\"{name}\""))) {
                    let line = code[..at].matches('\n').count() + 1;
                    findings.push(Finding::at(
                        format!("{path}:{line}"),
                        format!(
                            "The command `{name}` is not in the table of `{TABLE_FILE}`. Add a \
                             scenario that runs it, so its event is tested (ADR-0007, M1-F18)."
                        ),
                    ));
                }
            } else {
                let line = code[..at].matches('\n').count() + 1;
                findings.push(Finding::at(
                    format!("{path}:{line}"),
                    format!(
                        "The command `{name}` never calls `{RECORD}`. Every command writes \
                         through the `ChangeStore` port, which appends the domain event in the \
                         same transaction as the change (ADR-0007). If it only reads, its \
                         input type ends in `{QUERY_SUFFIX}`."
                    ),
                ));
            }
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    const PORT_SOURCE: &str =
        "pub trait ChangeStore {\n    fn record_change<T>(&self, e: &Event) -> T;\n}\n";

    fn found(text: &str) -> Vec<String> {
        let sources = vec![
            (
                "crates/bb-app/src/ports.rs".to_owned(),
                PORT_SOURCE.to_owned(),
            ),
            ("crates/bb-app/src/issues.rs".to_owned(), text.to_owned()),
        ];
        evaluate(&sources, Some(TABLE))
            .into_iter()
            .map(|finding| finding.message)
            .collect()
    }

    /// The table of the test, as far as these examples need it.
    const TABLE: &str = "table! { \"change_status\" => s::change_status_scenario, }";

    #[test]
    fn good_example_a_command_that_writes_through_the_port() {
        let text = "pub async fn change_status(ctx: &Ctx, cmd: ChangeStatus) -> Result<(), AppError> {\n    ctx.store().record_change(&event, async |tx| Ok(())).await?;\n    Ok(())\n}\n";
        assert!(found(text).is_empty());
    }

    #[test]
    fn good_example_queries_and_other_functions_need_no_event() {
        let text = "pub async fn show_issue(ctx: &Ctx, input: ShowIssueQuery) -> Result<View, AppError> {\n    Ok(view())\n}\nasync fn helper(number: u32) {}\npub(crate) async fn list(ctx: &impl Ctx, q: &ListQuery<'_>) {}\n";
        assert!(found(text).is_empty());
    }

    #[test]
    fn bad_example_a_command_without_the_port_whatever_it_is_called() {
        let text = "pub async fn rename_project(\n    ctx: &Ctx,\n    input: RenameProject,\n) -> Result<(), AppError> {\n    store(&input).await?;\n    Ok(())\n}\nasync fn private_command(context: &Ctx<'_>, thing: Thing) {}\npub(crate) async fn crate_command(ctx: &Ctx, x: u8) {}\n";
        let messages = found(text);
        assert_eq!(messages.len(), 3, "{messages:?}");
        assert!(messages[0].contains("`rename_project`"), "{messages:?}");
    }

    #[test]
    fn bad_example_a_command_that_the_table_of_the_test_does_not_name() {
        let text = "pub async fn archive_issue(ctx: &Ctx, cmd: Archive) -> Result<(), AppError> {\n    ctx.store().record_change(&event, async |tx| Ok(())).await?;\n    Ok(())\n}\n";
        let messages = found(text);
        assert_eq!(messages.len(), 1, "{messages:?}");
        assert!(messages[0].contains("`archive_issue`"), "{messages:?}");
        assert!(messages[0].contains("not in the table"), "{messages:?}");
    }

    #[test]
    fn good_example_a_private_part_of_a_command_needs_no_line_of_its_own() {
        let text = "async fn part(ctx: &Ctx) {\n    ctx.store().record_change(&event, async |tx| Ok(())).await?;\n}\n";
        assert!(found(text).is_empty());
    }

    #[test]
    fn bad_example_the_table_of_the_test_is_missing() {
        let sources = vec![(
            "crates/bb-app/src/ports.rs".to_owned(),
            PORT_SOURCE.to_owned(),
        )];
        let messages: Vec<String> = evaluate(&sources, None)
            .into_iter()
            .map(|f| f.message)
            .collect();
        assert!(messages[0].contains("table of the test"), "{messages:?}");
    }

    #[test]
    fn bad_example_a_mention_in_a_comment_or_string_is_not_a_call() {
        let text = "pub async fn sneaky(ctx: &Ctx, cmd: Do) {\n    // ctx.store().record_change(&event, f)\n    let _ = \"record_change(\";\n    /* record_change( */\n    let _ = r#\"record_change(\"#;\n}\n";
        assert_eq!(found(text).len(), 1);
    }

    #[test]
    fn bad_example_the_port_itself_is_missing() {
        let sources = vec![(
            "crates/bb-app/src/lib.rs".to_owned(),
            "pub fn nothing() {}\n".to_owned(),
        )];
        let messages: Vec<String> = evaluate(&sources, Some(TABLE))
            .into_iter()
            .map(|f| f.message)
            .collect();
        assert!(
            messages[0].contains("does not define the port"),
            "{messages:?}"
        );
    }

    #[test]
    fn other_crates_and_tests_are_not_use_cases() {
        let other = (
            "crates/bb-api/src/x.rs".to_owned(),
            "pub async fn go(ctx: &Ctx, cmd: u32) {}\n".to_owned(),
        );
        let in_tests = (
            "crates/bb-app/src/y.rs".to_owned(),
            format!(
                "{PORT_SOURCE}#[cfg(test)]\nmod tests {{ async fn go(ctx: &Ctx, cmd: u32) {{}} }}\n"
            ),
        );
        assert!(evaluate(&[other, in_tests], Some(TABLE)).is_empty());
    }

    #[test]
    fn the_use_cases_of_this_repository_pass() {
        let repo = crate::repo::Repo::discover().unwrap();
        let sources = read_product_sources(&repo).unwrap();
        let table = std::fs::read_to_string(repo.path(TABLE_FILE)).unwrap();
        let messages: Vec<String> = evaluate(&sources, Some(&table.replace("\r\n", "\n")))
            .into_iter()
            .map(|finding| finding.message)
            .collect();
        assert!(messages.is_empty(), "{messages:?}");
    }
}
