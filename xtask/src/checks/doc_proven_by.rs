//! DOC-PROVEN-BY: a test that a "Proven by" cell cites next to a check is a test that check
//! requires (ADR-0020).
//!
//! The security chapter says, row by row, what proves a mitigation. When a cell names a check
//! that holds a list of required tests (`TEST-WORKFLOWS`, `TEST-UPLOADS`, …) and also a test by
//! name, the test must be in that check's list: otherwise the proof can be deleted unseen, which is
//! the mistake M4 made. A cell with no such check is not looked at here (a test is only a name
//! then). The lists are read from the checks that own them (`required_tests::lists`).

use super::required_tests::{Crates, Required, crate_sources, lists};
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
        let sources = crate_sources(context.repo)?;
        let crates = Crates::of(&sources);
        let browser = browser_tests(context.repo)?;
        for folder in FOLDERS {
            for name in sorted_files(context.repo, folder, |name| has_extension(name, "md"))? {
                let path = format!("{folder}/{name}");
                let text = read_text(context.repo, &path)?;
                findings.extend(evaluate(&path, &text, &lists()));
                findings.extend(unknown_tests(&path, &text, &crates, &browser));
            }
        }
        Ok(findings)
    }
}

/// Whether a backticked word is the name of a test, by its form: snake case with a feature ID in
/// front, `wf_02_…`, `sec_06_…`, `iss_01a_…`, `m3_f53_…` (a digit in one of the first two parts).
/// However short the rest, it is a name; `known_field_names` or `credential_set` are not. A name
/// with no number, `sec_the_server_sends_to_only_so_many_addresses_…`, is one too when it starts
/// with a short feature word and is long (five parts or more).
fn is_test_name(word: &str) -> bool {
    let parts: Vec<&str> = word.split('_').collect();
    word.starts_with(|c: char| c.is_ascii_lowercase())
        && word
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && parts.len() >= 2
        && parts.iter().all(|part| !part.is_empty())
        && (parts
            .iter()
            .take(2)
            .any(|part| part.chars().any(|c| c.is_ascii_digit()))
            || (parts.len() >= 5 && (2..=4).contains(&parts[0].len())))
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

/// A title as the docs cite a test: lower case, every run of other characters one underscore.
fn cited_form(title: &str) -> String {
    let mut named = String::new();
    for c in title.chars().map(|c| c.to_ascii_lowercase()) {
        if c.is_ascii_alphanumeric() {
            named.push(c);
        } else if !named.ends_with('_') {
            named.push('_');
        }
    }
    named.trim_matches('_').to_owned()
}

/// The titles of the tests a text of the browser side declares with `test("…"`, `it("…"` or `describe("…"` (not
/// `test.skip`, not a line that is a comment), in the form the docs cite them.
fn titles_in(text: &str) -> Vec<String> {
    let code: String = text
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join(
            "
",
        );
    let mut titles = Vec::new();
    for opener in ["test(", "it(", "describe("] {
        for (at, _) in code.match_indices(opener) {
            let before = code[..at].chars().last();
            if before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.') {
                continue;
            }
            let rest = code[at + opener.len()..].trim_start();
            let Some(quote) = rest
                .chars()
                .next()
                .filter(|c| matches!(c, '"' | '\'' | '`'))
            else {
                continue;
            };
            if let Some(end) = rest[1..].find(quote) {
                titles.push(cited_form(&rest[1..=end]));
            }
        }
    }
    titles
}

/// The titles of the tests of the browser side, in the form the docs cite them.
fn browser_tests(repo: &crate::repo::Repo) -> anyhow::Result<Vec<String>> {
    let mut found = Vec::new();
    for file in repo.all_files()? {
        if file.starts_with("web/") && (has_extension(&file, "ts") || has_extension(&file, "tsx")) {
            found.extend(titles_in(&read_text(repo, &file)?));
        }
    }
    Ok(found)
}

/// The "Proven by" cells of the tables of a text, with the number of their line.
fn proof_cells(text: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
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
        if let Some(cell) = cells.into_iter().nth(at) {
            found.push((index + 1, cell));
        }
    }
    found
}

/// The tests a "Proven by" cell cites by name that are no test: a name that was renamed, never
/// written, switched off or not reached by any `mod` proves nothing, whatever check the cell names
/// (the rule is the one of the lists of required tests). A test of the browser side is found by its
/// title.
fn unknown_tests(path: &str, text: &str, crates: &Crates<'_>, browser: &[String]) -> Vec<Finding> {
    let cells = proof_cells(text);
    let mut cited: Vec<&str> = Vec::new();
    for (_, cell) in &cells {
        cited.extend(backticked(cell).filter(|word| is_test_name(word)));
    }
    if cited.is_empty() {
        return Vec::new();
    }
    let gone = crates.missing(&cited);
    let mut findings = Vec::new();
    for (line, cell) in &cells {
        for name in backticked(cell).filter(|word| is_test_name(word)) {
            if gone.contains(&name) && !browser.iter().any(|title| title == name) {
                findings.push(Finding::at(
                    format!("{path}:{line}"),
                    format!("`{name}` is cited as proof, but it is no test that runs: no function of that name with a test attribute that is switched on and reached by a `mod`, and no browser test of that title. It was renamed, switched off or never written."),
                ));
            }
        }
    }
    findings
}

fn evaluate(path: &str, text: &str, lists: &[(&str, &[Required])]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (line, cell) in proof_cells(text) {
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
        for name in backticked(&cell).filter(|word| is_test_name(word)) {
            if !cited
                .iter()
                .any(|(_, required)| required.iter().any(|each| each.name == name))
            {
                let ids: Vec<&str> = cited.iter().map(|(id, _)| *id).collect();
                findings.push(Finding::at(
                    format!("{path}:{line}"),
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

    fn lib_with(code: &str) -> Vec<(String, String)> {
        vec![("crates/x/src/lib.rs".to_owned(), code.to_owned())]
    }

    #[test]
    fn bad_example_a_cited_test_that_no_function_has() {
        let sources = lib_with(
            "#[test]
fn sec_the_server_sends_to_only_so_many_addresses() {}
",
        );
        let good = table("`sec_the_server_sends_to_only_so_many_addresses`");
        assert!(unknown_tests("a.md", &good, &Crates::of(&sources), &[]).is_empty());
        // A name with no number is a name too, and a renamed test is found out.
        let bad = table("`sec_the_server_sends_only_so_many_sign_up_e_mails_in_an_hour` (WP-12)");
        let found = unknown_tests("a.md", &bad, &Crates::of(&sources), &[]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].location.as_deref(), Some("a.md:3"));
    }

    #[test]
    fn bad_example_a_cited_test_that_is_ignored_or_only_in_a_comment() {
        let sources = lib_with(
            "#[test]
#[ignore = \"later\"]
fn wf_01_an_ignored_test_proves_nothing_at_all() {}
             // fn wf_01_a_name_in_a_comment_proves_nothing_either() {}
             fn wf_01_a_helper_without_a_test_attribute_proves_nothing() {}
",
        );
        for name in [
            "wf_01_an_ignored_test_proves_nothing_at_all",
            "wf_01_a_name_in_a_comment_proves_nothing_either",
            "wf_01_a_helper_without_a_test_attribute_proves_nothing",
        ] {
            let bad = table(&format!("`{name}`"));
            assert_eq!(
                unknown_tests("a.md", &bad, &Crates::of(&sources), &[]).len(),
                1,
                "{name}"
            );
        }
    }

    #[test]
    fn a_browser_test_is_found_by_its_whole_title() {
        let titles = titles_in(
            "test(\"ntf_01 a comment raises her count, Open lands\", async () => {});
             // test(\"ntf_01 a title in a comment\", () => {});
             test.skip(\"ntf_01 a skipped title\", () => {});
",
        );
        assert_eq!(titles, ["ntf_01_a_comment_raises_her_count_open_lands"]);
        let cited = table("`ntf_01_a_comment_raises_her_count_open_lands`, `ntf_01_a_comment`");
        let found = unknown_tests("a.md", &cited, &Crates::of(&[]), &titles);
        // A part of a title is no title.
        assert_eq!(found.len(), 1);
        assert!(found[0].message.contains("`ntf_01_a_comment`"));
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
        let sources = crate_sources(&repo).unwrap();
        let browser = browser_tests(&repo).unwrap();
        let found: Vec<_> = evaluate(path, &text, &lists())
            .into_iter()
            .chain(unknown_tests(path, &text, &Crates::of(&sources), &browser))
            .map(|finding| finding.message)
            .collect();
        assert!(found.is_empty(), "{found:#?}");
    }
}
