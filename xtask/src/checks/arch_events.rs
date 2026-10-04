//! ARCH-EVENTS: every command use case appends a domain event (ADR-0007).
//!
//! A use case in `bb-app` is a `pub async fn` with the context first. A *command* is one whose
//! input parameter is called `cmd` (a query's is called `query`); it changes something, so its
//! body must call `append_event`, which writes the domain event in the same transaction as the
//! change. This check reads the code; the generated test that runs every command against the
//! in-memory fakes and asserts the event comes with the first use cases (M1-F18).

use super::{Check, Context, Finding};
use crate::rust_source::{Source, before_tests, read_product_sources};

/// Where the use cases live.
const USE_CASES_FOLDER: &str = "crates/bb-app/src/";
/// The call that appends the event.
const APPEND: &str = "append_event(";

pub(crate) struct ArchEvents;

impl Check for ArchEvents {
    fn id(&self) -> &'static str {
        "ARCH-EVENTS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        Ok(evaluate(&read_product_sources(context.repo)?))
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

/// Whether a parameter list has a parameter named `cmd`.
fn has_command_parameter(parameters: &str) -> bool {
    parameters.split([',', '(', ')']).any(|parameter| {
        parameter
            .trim()
            .strip_prefix("cmd")
            .is_some_and(|rest| rest.trim_start().starts_with(':'))
    })
}

fn evaluate(sources: &[Source]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (path, text) in sources
        .iter()
        .filter(|(path, _)| path.starts_with(USE_CASES_FOLDER))
    {
        let code = before_tests(text);
        for (at, _) in code.match_indices("pub async fn ") {
            let rest = &code[at + "pub async fn ".len()..];
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            let Some(open) = rest.find('(') else { continue };
            let Some(parameters) = balanced(&rest[open..], '(', ')') else {
                continue;
            };
            if !has_command_parameter(parameters) {
                continue;
            }
            let after = &rest[open + parameters.len()..];
            let Some(body_start) = after.find('{') else {
                continue;
            };
            let Some(body) = balanced(&after[body_start..], '{', '}') else {
                continue;
            };
            if !body.contains(APPEND) {
                let line = code[..at].matches('\n').count() + 1;
                findings.push(Finding::at(
                    format!("{path}:{line}"),
                    format!(
                        "The command `{name}` never calls `{APPEND}…)`. Every command appends a \
                         domain event in the same transaction as its change (ADR-0007)."
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

    fn found(text: &str) -> Vec<String> {
        let source = ("crates/bb-app/src/issues.rs".to_owned(), text.to_owned());
        evaluate(&[source])
            .into_iter()
            .map(|finding| finding.message)
            .collect()
    }

    #[test]
    fn good_example_a_command_that_appends_its_event() {
        let text = "pub async fn change_status(ctx: &Ctx, cmd: ChangeStatus) -> Result<(), AppError> {\n    let event = build(&cmd);\n    ctx.append_event(event).await?;\n    Ok(())\n}\n";
        assert!(found(text).is_empty());
    }

    #[test]
    fn good_example_queries_and_helpers_need_no_event() {
        let text = "pub async fn show_issue(ctx: &Ctx, query: ShowIssue) -> Result<View, AppError> {\n    Ok(view())\n}\nfn helper(cmd: u32) {}\n";
        assert!(found(text).is_empty());
    }

    #[test]
    fn bad_example_a_command_without_an_event() {
        let text = "pub async fn rename_project(\n    ctx: &Ctx,\n    cmd: RenameProject,\n) -> Result<(), AppError> {\n    store(&cmd).await?;\n    Ok(())\n}\n";
        let messages = found(text);
        assert_eq!(messages.len(), 1);
        assert!(messages[0].contains("`rename_project`"), "{messages:?}");
    }

    #[test]
    fn other_crates_and_tests_are_not_use_cases() {
        let other = (
            "crates/bb-api/src/x.rs".to_owned(),
            "pub async fn go(cmd: u32) {}\n".to_owned(),
        );
        let in_tests = (
            "crates/bb-app/src/y.rs".to_owned(),
            "#[cfg(test)]\nmod tests { pub async fn go(cmd: u32) {} }\n".to_owned(),
        );
        assert!(evaluate(&[other, in_tests]).is_empty());
    }

    #[test]
    fn the_use_cases_of_this_repository_pass() {
        let repo = crate::repo::Repo::discover().unwrap();
        let sources = read_product_sources(&repo).unwrap();
        let messages: Vec<String> = evaluate(&sources)
            .into_iter()
            .map(|finding| finding.message)
            .collect();
        assert!(messages.is_empty(), "{messages:?}");
    }
}
