//! DOC-PROVEN-BY: a test that a "Proven by" cell cites next to a check is a test that check
//! requires (ADR-0020).
//!
//! The security chapter says, row by row, what proves a mitigation. When a cell names a check
//! that holds a list of required tests (`TEST-WORKFLOWS`, `TEST-UPLOADS`, …) and also a test by
//! name, the test must be in that check's list: otherwise the proof can be deleted unseen, which is
//! the mistake M4 made. A cell with no such check is not looked at here (a test is only a name
//! then). The lists are read from the checks that own them (`required_tests::lists`).

use super::required_tests::{Required, lists};
use super::{Check, Context, Finding};
use crate::docs::source::sorted_files;
use crate::docs::text::has_extension;
use crate::source::read_text;

/// The folders whose tables are read.
const FOLDERS: [&str; 2] = ["docs/architecture", "docs/adr"];

/// The title of the column that holds the proof.
const COLUMN: &str = "Proven by";

pub(crate) struct DocProvenBy;

impl Check for DocProvenBy {
    fn id(&self) -> &'static str {
        "DOC-PROVEN-BY"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let mut findings = Vec::new();
        for folder in FOLDERS {
            for name in sorted_files(context.repo, folder, |name| has_extension(name, "md"))? {
                let path = format!("{folder}/{name}");
                let text = read_text(context.repo, &path)?;
                findings.extend(evaluate(&path, &text, &lists()));
            }
        }
        Ok(findings)
    }
}

/// Whether a backticked word is the name of a test, by its form: snake case with a feature ID in
/// front, `wf_02_…`, `sec_06_…`, `iss_01a_…`, `m3_f53_…` (a digit in one of the first two parts).
/// However short the rest, it is a name; `known_field_names` or `credential_set` are not.
fn is_test_name(word: &str) -> bool {
    let parts: Vec<&str> = word.split('_').collect();
    word.starts_with(|c: char| c.is_ascii_lowercase())
        && word
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && parts.len() >= 2
        && parts.iter().all(|part| !part.is_empty())
        && parts
            .iter()
            .take(2)
            .any(|part| part.chars().any(|c| c.is_ascii_digit()))
}

/// Adds a character to the cell that is being read.
fn push(cells: &mut [String], c: char) {
    if let Some(cell) = cells.last_mut() {
        cell.push(c);
    }
}

/// The cells of a table row: split at `|`, but not at `\|`, which is a pipe in the text.
fn cells_of(line: &str) -> Vec<String> {
    let mut cells = vec![String::new()];
    let mut chars = line.trim().chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'|') => {
                chars.next();
                push(&mut cells, '|');
            }
            '|' => cells.push(String::new()),
            other => push(&mut cells, other),
        }
    }
    // The row starts and ends with a pipe, so the first and the last piece are empty.
    if cells.first().is_some_and(|cell| cell.trim().is_empty()) {
        cells.remove(0);
    }
    if cells.last().is_some_and(|cell| cell.trim().is_empty()) {
        cells.pop();
    }
    cells
}

/// The words between backticks.
fn backticked(cell: &str) -> impl Iterator<Item = &str> {
    cell.split('`').skip(1).step_by(2)
}

fn evaluate(path: &str, text: &str, lists: &[(&str, &[Required])]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut column = None;
    for (index, line) in text.lines().enumerate() {
        if !line.trim_start().starts_with('|') {
            column = None;
            continue;
        }
        let cells = cells_of(line);
        let Some(at) = column else {
            column = cells.iter().position(|cell| cell.trim() == COLUMN);
            continue;
        };
        let Some(cell) = cells.get(at).map(String::as_str) else {
            continue;
        };
        let cited: Vec<&(&str, &[Required])> = lists
            .iter()
            .filter(|(id, _)| {
                cell.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
                    .any(|word| word == *id)
            })
            .collect();
        if cited.is_empty() {
            continue;
        }
        for name in backticked(cell).filter(|word| is_test_name(word)) {
            if !cited
                .iter()
                .any(|(_, required)| required.iter().any(|each| each.name == name))
            {
                let ids: Vec<&str> = cited.iter().map(|(id, _)| *id).collect();
                findings.push(Finding::at(
                    format!("{path}:{}", index + 1),
                    format!(
                        "`{name}` is cited as proof next to {}, but that check does not require it. Add it to the list of the check, or stop citing the check in this cell.",
                        ids.join(" and ")
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

    const LIST: [Required; 1] = [Required::anywhere(
        "wf_01_a_thing_that_is_required_and_long",
    )];

    fn table(cell: &str) -> String {
        format!("| STRIDE | Threat | Proven by |\n|---|---|---|\n| T | x | {cell} |\n")
    }

    #[test]
    fn good_example_a_cited_test_that_the_check_requires() {
        let text = table("TEST-X, `wf_01_a_thing_that_is_required_and_long` (WP-04)");
        assert!(evaluate("a.md", &text, &[("TEST-X", &LIST)]).is_empty());
    }

    #[test]
    fn bad_example_a_cited_test_that_the_check_does_not_require() {
        let text = table("TEST-X, `wf_01_another_thing_nobody_requires_here`");
        let found = evaluate("a.md", &text, &[("TEST-X", &LIST)]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].location.as_deref(), Some("a.md:3"));
    }

    #[test]
    fn a_cell_without_such_a_check_and_short_words_are_left_alone() {
        let text =
            table("`wf_01_another_thing_nobody_requires_here`, `LockClass`, `known_field_names`");
        assert!(evaluate("a.md", &text, &[("TEST-X", &LIST)]).is_empty());
        let other = table("TEST-X, `known_field_names`, `getMailServer`");
        assert!(evaluate("a.md", &other, &[("TEST-X", &LIST)]).is_empty());
    }

    #[test]
    fn bad_example_a_short_name_is_a_name_too() {
        let text = table("TEST-X, `wf_01_short`");
        assert_eq!(evaluate("a.md", &text, &[("TEST-X", &LIST)]).len(), 1);
        let good = table("TEST-X, `wf_01_a_thing_that_is_required_and_long`, `m3_f53_x`");
        assert_eq!(evaluate("a.md", &good, &[("TEST-X", &LIST)]).len(), 1);
    }

    #[test]
    fn an_escaped_pipe_does_not_split_a_cell() {
        assert_eq!(cells_of("| a \\| b | c |"), [" a | b ", " c "]);
        let text = table("`a \\| b`, TEST-X, `wf_01_another_thing_nobody_requires_here`");
        assert_eq!(evaluate("a.md", &text, &[("TEST-X", &LIST)]).len(), 1);
    }

    #[test]
    fn the_security_chapter_of_this_repository_cites_only_required_tests() {
        let repo = crate::repo::Repo::discover().unwrap();
        let path = "docs/architecture/12-security.md";
        let text = read_text(&repo, path).unwrap();
        let found: Vec<_> = evaluate(path, &text, &lists())
            .into_iter()
            .map(|finding| finding.message)
            .collect();
        assert!(found.is_empty(), "{found:#?}");
    }
}
