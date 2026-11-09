//! SEC-MATRIX for the assistant (M7-F52, AI-10, AI-05, AI-03): the `ask` lines of the record. Every
//! starting role has one: what the fake provider was given when that person asked, and what the
//! assistant could propose and apply for them. The record is written by the test
//! `sec_01_the_assistant_gives_and_changes_no_more_than_the_person_may`.
//!
//! The assistant must never see or change more than the person may, so the check holds each line
//! against what already stands in the record and in the tables of the starting roles:
//!
//! - what the assistant read is what the tool `get_issue` answered the same person at `/mcp`, and
//!   what it could propose is what `update_issue` answered them;
//! - the status and the author were in what the provider was given exactly as the tables of the
//!   starting roles say (the field *Status*, the kind *Authors*), and not at all when nothing was
//!   read;
//! - nothing that is *Never sent to AI* was in any request to the provider;
//! - *Apply* ran when there was a proposal, and only then.

use std::collections::BTreeMap;

use bb_api_words::sec_matrix::{self as words, RECORD_PATH};
use bb_core::access::{FieldLevel, PeopleLevel};
use bb_core::ai::Tool;
use bb_core::fields::Field;
use bb_core::people_kinds::PeopleKind;
use bb_policy::StartingRole;

use super::Finding;
use super::sec_matrix_tools::tool_answer;

/// What an `ask` line says after the colon, by name.
type Said = BTreeMap<String, String>;

/// The `ask` lines of a record, by the entry point and the person who asked, written `entry actor`.
fn ask_lines(record: &str) -> BTreeMap<String, Said> {
    record
        .lines()
        .filter_map(|line| line.strip_prefix(words::ASK_LINE))
        .filter_map(|rest| rest.split_once(" : "))
        .map(|(head, said)| {
            (
                head.split_whitespace().collect::<Vec<_>>().join(" "),
                said.split_whitespace()
                    .filter_map(|pair| pair.split_once('='))
                    .map(|(name, word)| (name.to_owned(), word.to_owned()))
                    .collect(),
            )
        })
        .collect()
}

/// What the tables of the starting roles say the provider was given of an issue the person read, or
/// nothing of it when they read none.
fn table_words(role: StartingRole, read: bool) -> [(&'static str, &'static str); 2] {
    let status_shown = read && role.field_level(Field::Status) != FieldLevel::Hidden;
    let author = if read {
        role.people_level(PeopleKind::Authors)
    } else {
        PeopleLevel::Hidden
    };
    [
        (
            "status",
            if status_shown {
                words::SHOWN
            } else {
                words::LEFT_OUT
            },
        ),
        ("author", author.code()),
    ]
}

/// The `ask` lines of the record: one for every starting role, held against the `tool` lines and the
/// tables of the starting roles.
pub(super) fn assistant_findings(record: &str) -> Vec<Finding> {
    let lines = ask_lines(record);
    let mut found = Vec::new();
    for role in StartingRole::ALL {
        let actor = words::role_actor(role.code());
        let place = format!(
            "What `{}` gave and did for `{actor}`",
            words::ENTRY_ASSISTANT
        );
        let Some(said) = lines.get(&format!("{} {actor}", words::ENTRY_ASSISTANT)) else {
            found.push(Finding::at(
                RECORD_PATH,
                format!("{place} is missing from the record: run the matrix."),
            ));
            continue;
        };
        let word = |name: &str| said.get(name).map_or("nothing", String::as_str);
        let mut wrong = |message: String| found.push(Finding::at(RECORD_PATH, message));
        for (name, tool) in [("read", Tool::GetIssue), ("propose", Tool::UpdateIssue)] {
            let tool_says = tool_answer(record, tool, &actor).unwrap_or_default();
            if word(name) != tool_says {
                wrong(format!(
                    "{place}: `{name}={}` must be what the tool `{}` answered them at `/mcp`, \
                     `{tool_says}`: the assistant does no more than the person.",
                    word(name),
                    tool.code(),
                ));
            }
        }
        for (name, wanted) in table_words(*role, word("read") == words::ALLOWED) {
            if word(name) != wanted {
                wrong(format!(
                    "{place} must have `{name}={wanted}` as the tables of the starting roles say, \
                     not `{}`.",
                    word(name)
                ));
            }
        }
        if word("kept") != words::HELD_BACK {
            wrong(format!(
                "{place}: something that is *Never sent to AI* reached the provider (`kept={}`).",
                word("kept")
            ));
        }
        let applied = if word("propose") == words::ALLOWED {
            words::ALLOWED
        } else {
            words::NO_PROPOSAL
        };
        if word("apply") != applied {
            wrong(format!(
                "{place} must have `apply={applied}` after `propose={}`, not `{}`.",
                word("propose"),
                word("apply")
            ));
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use bb_policy::Permission;

    use super::*;

    /// A good record: an `ask` line for every role and the `tool` lines it is held against. A role
    /// that sees the status reads the issue and may change it; the others read nothing.
    fn record() -> String {
        let mut text = String::new();
        let mut answers = Vec::new();
        for role in StartingRole::ALL {
            let actor = words::role_actor(role.code());
            let sees = role.permissions().contains(&Permission::SeeIssues);
            let (read, propose, apply) = if sees {
                (words::ALLOWED, words::ALLOWED, words::ALLOWED)
            } else {
                (words::NOT_FOUND, words::NOT_FOUND, words::NO_PROPOSAL)
            };
            let [(_, status), (_, author)] = table_words(*role, sees);
            let _ = writeln!(
                text,
                "{}{} {actor} : read={read} status={status} author={author} kept={} \
                 propose={propose} apply={apply}",
                words::ASK_LINE,
                words::ENTRY_ASSISTANT,
                words::HELD_BACK
            );
            answers.push((actor, read, propose));
        }
        for (tool, at) in [(Tool::GetIssue, 1), (Tool::UpdateIssue, 2)] {
            let said: Vec<String> = answers
                .iter()
                .map(|each| format!("{}={}", each.0, if at == 1 { each.1 } else { each.2 }))
                .collect();
            let _ = writeln!(
                text,
                "tool {} mcp changes everyone : {}",
                tool.code(),
                said.join(" ")
            );
        }
        text
    }

    fn messages(record: &str) -> Vec<String> {
        assistant_findings(record)
            .into_iter()
            .map(|finding| finding.message)
            .collect()
    }

    fn says(record: &str, part: &str) -> bool {
        messages(record).iter().any(|each| each.contains(part))
    }

    #[test]
    fn good_example_a_line_for_every_role_that_does_no_more_than_the_person() {
        assert_eq!(messages(&record()), Vec::<String>::new());
    }

    #[test]
    fn bad_example_a_role_without_a_line() {
        let without: String = record()
            .lines()
            .filter(|line| !line.starts_with("ask assistant role-hr"))
            .fold(String::new(), |mut all, line| {
                let _ = writeln!(all, "{line}");
                all
            });
        assert!(says(&without, "role-hr"));
    }

    #[test]
    fn bad_example_what_is_never_sent_to_ai_reaches_the_provider() {
        assert!(says(
            &record().replace("kept=held-back", "kept=sent"),
            "Never sent to AI"
        ));
    }

    #[test]
    fn bad_example_the_assistant_shows_or_changes_more_than_the_person_may() {
        let text = record();
        let shown = text.replace("status=hidden", "status=shown");
        assert!(says(&shown, "tables of the starting roles"));
        let (before, update) = text.split_once("tool update_issue").unwrap_or_default();
        let proposes = format!(
            "{before}tool update_issue{}",
            update.replace("role-viewer=allowed", "role-viewer=forbidden")
        );
        assert!(says(&proposes, "does no more than the person"));
        let applies = text.replace(
            "propose=not-found apply=none",
            "propose=not-found apply=allowed",
        );
        assert!(says(&applies, "apply=none"));
    }
}
