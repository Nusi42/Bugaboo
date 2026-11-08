//! SEC-MATRIX for the tools at `/mcp` (M7-F41, AI-21, AI-36): the `tool` lines of the record. Every
//! tool of the registry has one, answered for nobody, a person with each starting role, an agent
//! with no work and an agent with work. The record is written by the test
//! `sec_01_every_tool_answers_every_actor_as_the_tables_say`.

use std::collections::BTreeMap;

use bb_api_words::sec_matrix::{self as words, RECORD_PATH};
use bb_core::ai::Tool;
use bb_policy::StartingRole;

use super::Finding;

/// A `tool` line of the record: the tool as it was called at `/mcp` and what each actor was answered.
struct ToolLine {
    entry: String,
    effect: String,
    audience: String,
    answers: BTreeMap<String, String>,
}

/// The `tool` lines of a record, by the name of the tool.
fn tool_lines(record: &str) -> BTreeMap<String, ToolLine> {
    let mut lines = BTreeMap::new();
    for line in record.lines() {
        let Some(rest) = line.strip_prefix(words::TOOL_LINE) else {
            continue;
        };
        let Some((head, answers)) = rest.split_once(" : ") else {
            continue;
        };
        let mut head = head.split_whitespace();
        let (Some(name), Some(entry), Some(effect), Some(audience)) =
            (head.next(), head.next(), head.next(), head.next())
        else {
            continue;
        };
        lines.insert(
            name.to_owned(),
            ToolLine {
                entry: entry.to_owned(),
                effect: effect.to_owned(),
                audience: audience.to_owned(),
                answers: answers
                    .split_whitespace()
                    .filter_map(|pair| pair.split_once('='))
                    .map(|(actor, answer)| (actor.to_owned(), answer.to_owned()))
                    .collect(),
            },
        );
    }
    lines
}

/// Who a tool is for, in the words of the record.
fn audience_of(tool: Tool) -> &'static str {
    if tool.is_for_agents() {
        "agents"
    } else if tool.is_for_people() {
        "people"
    } else {
        "everyone"
    }
}

/// The tools of the registry at `/mcp` (M7-F41): every tool has a line, every actor was answered,
/// nobody signed in is turned away, a tool of agents is not for a person, a tool about the own time
/// of a person is not for an agent, and an agent with no work changes nothing.
pub(super) fn tool_findings(record: &str) -> Vec<Finding> {
    let lines = tool_lines(record);
    let roles: Vec<&str> = StartingRole::ALL.iter().map(|role| role.code()).collect();
    let actors = words::mcp_actors(&roles);
    let mut found = Vec::new();
    for tool in Tool::ALL {
        let place = format!("The tool `{}` at `/mcp`", tool.code());
        let Some(line) = lines.get(tool.code()) else {
            found.push(Finding::at(
                RECORD_PATH,
                format!("{place} is missing from the record of the matrix: run the matrix."),
            ));
            continue;
        };
        if line.entry != words::ENTRY_MCP {
            found.push(Finding::at(
                RECORD_PATH,
                format!("{place} was run through `{}`, not `mcp`.", line.entry),
            ));
        }
        if line.audience != audience_of(*tool) {
            found.push(Finding::at(
                RECORD_PATH,
                format!(
                    "{place} is for `{}`, but the record says `{}`: run the matrix again.",
                    audience_of(*tool),
                    line.audience
                ),
            ));
        }
        for actor in &actors {
            match line.answers.get(actor).map(String::as_str) {
                None => found.push(Finding::at(
                    RECORD_PATH,
                    format!("{place} was not called as `{actor}`."),
                )),
                Some(answer) if !words::ANSWERS.contains(&answer) => found.push(Finding::at(
                    RECORD_PATH,
                    format!("{place} as `{actor}`: `{answer}` is no answer the matrix knows."),
                )),
                Some(_) => {}
            }
        }
        let answer = |actor: &str| line.answers.get(actor).map(String::as_str);
        if answer(words::NOBODY).is_some_and(|word| word != words::NOT_SIGNED_IN) {
            found.push(Finding::at(
                RECORD_PATH,
                format!("{place} must turn away nobody signed in with `not-signed-in`."),
            ));
        }
        for actor in &actors {
            let is_agent = actor.starts_with("agent");
            let allowed = answer(actor) == Some(words::ALLOWED);
            let not_for_them = if tool.is_for_agents() {
                !is_agent
            } else {
                tool.is_for_people() && is_agent
            };
            let changes_with_no_work =
                actor == words::AGENT_WITHOUT_WORK && line.effect == "changes";
            if allowed && (not_for_them || changes_with_no_work) {
                found.push(Finding::at(
                    RECORD_PATH,
                    format!("{place} was let in for `{actor}`, which it must not be."),
                ));
            }
        }
    }
    for name in lines.keys() {
        if !Tool::ALL.iter().any(|tool| tool.code() == name) {
            found.push(Finding::at(
                RECORD_PATH,
                format!(
                    "The record has a tool `{name}` that the registry does not: run the matrix again."
                ),
            ));
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::*;

    /// The `tool` lines of a good record: every tool answered as the rules of the check say.
    fn tool_record() -> String {
        let roles: Vec<&str> = StartingRole::ALL.iter().map(|role| role.code()).collect();
        let actors = words::mcp_actors(&roles);
        let mut text = String::new();
        for tool in Tool::ALL {
            let answers: Vec<String> = actors
                .iter()
                .map(|actor| {
                    let word = if actor == words::NOBODY {
                        words::NOT_SIGNED_IN
                    } else if (tool.is_for_agents() && !actor.starts_with("agent"))
                        || (tool.is_for_people() && actor.starts_with("agent"))
                        || actor == words::AGENT_WITHOUT_WORK
                    {
                        words::FORBIDDEN
                    } else {
                        words::ALLOWED
                    };
                    format!("{actor}={word}")
                })
                .collect();
            let _ = writeln!(
                text,
                "tool {} {} changes {} : {}",
                tool.code(),
                words::ENTRY_MCP,
                audience_of(*tool),
                answers.join(" ")
            );
        }
        text
    }

    fn messages_of_tools(record: &str) -> Vec<String> {
        tool_findings(record)
            .into_iter()
            .map(|finding| finding.message)
            .collect()
    }

    #[test]
    fn good_example_a_record_with_a_line_for_every_tool_and_every_actor() {
        assert!(tool_findings(&tool_record()).is_empty());
    }

    #[test]
    fn bad_example_a_tool_without_a_line_or_let_in_where_it_must_not_be() {
        let text = tool_record();
        let without = text
            .lines()
            .filter(|line| !line.starts_with("tool take "))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            messages_of_tools(&without)
                .iter()
                .any(|each| each.contains("`take`") && each.contains("missing"))
        );
        let let_in = text.replace("role-viewer=forbidden", "role-viewer=allowed");
        assert!(
            messages_of_tools(&let_in)
                .iter()
                .any(|each| each.contains("role-viewer") && each.contains("must not"))
        );
        let signed_in = text.replace("nobody=not-signed-in", "nobody=allowed");
        assert!(!tool_findings(&signed_in).is_empty());
    }
}
