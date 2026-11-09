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
//! - the status, the estimate and the person it is assigned to, in the issue the person read to
//!   show what is masked for them, were in what the provider was given exactly as the tables of
//!   the starting roles say (the fields *Status* and *Estimated time*, the kind *Assigned to*),
//!   and not at all when nothing was read;
//! - nothing that is *Never sent to AI* was in any request to the provider, and the issue of a
//!   project kept from AI was refused as unknown;
//! - *Apply* ran when there was a proposal, and only then, and a reference nobody made is turned
//!   away;
//! - there is one line for every starting role, no other and none twice.

use std::collections::BTreeSet;

use bb_api_words::sec_matrix::{self as words, RECORD_PATH};
use bb_core::access::PeopleLevel;
use bb_core::ai::Tool;
use bb_core::fields::Field;
use bb_core::people_kinds::PeopleKind;
use bb_policy::StartingRole;

use super::Finding;
use super::sec_matrix::{people_word, said_lines, shown_word};
use super::sec_matrix_tools::tool_answer;

/// The fields the lines say about: the field *Status* and the field *Estimated time*, whose levels
/// differ from role to role (a Guest sees the one and not the other).
const FIELDS: [(&str, Field); 2] = [
    ("status", Field::Status),
    ("estimate", Field::EstimatedTime),
];

/// What the tables of the starting roles say the provider was given of the issue the person read to
/// show what is masked for them: nothing of it when they read none.
fn table_words(role: StartingRole, read: bool) -> Vec<(&'static str, &'static str)> {
    let mut wanted: Vec<(&str, &str)> = FIELDS
        .iter()
        .map(|(name, field)| {
            let word = if read {
                shown_word(role, *field)
            } else {
                words::LEFT_OUT
            };
            (*name, word)
        })
        .collect();
    wanted.push((
        "assigned-to",
        if read {
            people_word(role, PeopleKind::AssignedTo)
        } else {
            PeopleLevel::Hidden.code()
        },
    ));
    wanted
}

/// The `ask` lines of the record: one for every starting role, held against the `tool` lines and the
/// tables of the starting roles.
pub(super) fn assistant_findings(record: &str) -> Vec<Finding> {
    let lines = said_lines(record, words::ASK_LINE);
    let mut found = Vec::new();
    let mut seen = BTreeSet::new();
    let roles: Vec<String> = StartingRole::ALL
        .iter()
        .map(|role| {
            format!(
                "{} {}",
                words::ENTRY_ASSISTANT,
                words::role_actor(role.code())
            )
        })
        .collect();
    for (head, _) in &lines {
        if !roles.contains(head) {
            found.push(Finding::at(
                RECORD_PATH,
                format!("The record has an `ask` line `{head}` for no starting role: run the matrix again."),
            ));
        } else if !seen.insert(head) {
            found.push(Finding::at(
                RECORD_PATH,
                format!("The record has two `ask` lines `{head}`: run the matrix again."),
            ));
        }
    }
    for role in StartingRole::ALL {
        let actor = words::role_actor(role.code());
        let place = format!(
            "What `{}` gave and did for `{actor}`",
            words::ENTRY_ASSISTANT
        );
        let head = format!("{} {actor}", words::ENTRY_ASSISTANT);
        let Some((_, said)) = lines.iter().find(|(each, _)| *each == head) else {
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
        for (name, wanted) in table_words(*role, word("mine") == words::ALLOWED) {
            if word(name) != wanted {
                wrong(format!(
                    "{place} must have `{name}={wanted}` as the tables of the starting roles say, \
                     not `{}`.",
                    word(name)
                ));
            }
        }
        if word("read") == words::ALLOWED && word("mine") != words::ALLOWED {
            wrong(format!(
                "{place}: it read the issue that every role asks about, so it must have read the \
                 one it was asked to show too (`mine={}`).",
                word("mine")
            ));
        }
        if !words::FORGED_REFUSALS.contains(&word("forged")) {
            wrong(format!(
                "{place}: *Apply* with a reference that was never made for them must be refused \
                 (`forged=proposal-invalid`, `forbidden` or `not-found`), not `{}`.",
                word("forged")
            ));
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
            let tables: Vec<String> = table_words(*role, sees)
                .iter()
                .map(|(name, word)| format!("{name}={word}"))
                .collect();
            let _ = writeln!(
                text,
                "{}{} {actor} : read={read} mine={read} {} kept={} forged={} propose={propose} apply={apply}",
                words::ASK_LINE,
                words::ENTRY_ASSISTANT,
                tables.join(" "),
                words::HELD_BACK,
                words::INVALID_PROPOSAL
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
    fn bad_example_an_unknown_or_double_line_or_a_forged_reference_let_in() {
        let text = record();
        let double = format!("{text}{}", text.lines().next().unwrap_or_default());
        assert!(says(&double, "two `ask` lines"));
        let unknown = text.replace("ask assistant role-hr", "ask assistant role-nobody");
        assert!(says(&unknown, "no starting role"));
        let forged = text.replace("forged=proposal-invalid", "forged=allowed");
        assert!(says(&forged, "never made for them"));
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
        let shown = text.replace("estimate=hidden", "estimate=shown");
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
