//! The part of TEST-A11Y that reads the EN 301 549 report (COMP-15): every clause of the list kept as
//! test data has its row in the report, with a result and its evidence or reason, and every test a
//! row cites by name exists.
//!
//! A row of the report is a line of a Markdown table: `| <clause> | <title> | <result> | <evidence> |`.
//! A test is cited in backticks by its title (`comp_15 every target is at least 24 px`,
//! `m3_f61 …`, `a11y …`) or by its file (`a11y-measures.spec.ts`); the titles are looked for in the
//! browser tests and in the checks of xtask. Only a clause the list marks `screen_reader = true` may
//! say *Not tested by a person*.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::Finding;
use super::test_a11y::SUITE_FOLDER;
use crate::docs::source::sorted_files;
use crate::docs::text::has_extension;
use crate::source::read_text;

/// The report, one row per clause.
const REPORT_FILE: &str = "docs/accessibility/en-301-549.md";
/// The clauses the report answers: the list of the standard, kept as test data.
const CLAUSES_FILE: &str = "tools/testdata/en-301-549.toml";

/// The result of a clause that only a person with a screen reader can tell, while nobody has.
const NOT_TESTED_BY_A_PERSON: &str = "Not tested by a person";

/// The results a row may give, as the report writes them.
const RESULTS: [&str; 5] = [
    "Supports",
    "Partly supports",
    "Does not support",
    "Not applicable",
    NOT_TESTED_BY_A_PERSON,
];

#[derive(Deserialize)]
struct Clauses {
    clause: Vec<Clause>,
}

#[derive(Deserialize)]
struct Clause {
    id: String,
    /// Whether only a person with a screen reader can tell that the clause is met.
    #[serde(default)]
    screen_reader: bool,
}

/// The clauses of the list, in its order.
fn clauses_of(list: &str) -> anyhow::Result<Vec<Clause>> {
    let clauses: Clauses = toml::from_str(list)?;
    Ok(clauses.clause)
}

/// A row of the report: its result and its evidence, by clause.
struct Row {
    result: String,
    evidence: String,
}

/// The rows of the report's tables whose first cell is a clause of the list.
fn rows<'a>(report: &str, clauses: &'a [Clause]) -> BTreeMap<&'a str, Vec<Row>> {
    let mut found: BTreeMap<&str, Vec<Row>> = BTreeMap::new();
    for line in report.lines().map(str::trim) {
        let Some(inner) = line
            .strip_prefix('|')
            .and_then(|rest| rest.strip_suffix('|'))
        else {
            continue;
        };
        let cells: Vec<&str> = inner.split('|').map(str::trim).collect();
        let [clause, _title, result, evidence] = cells.as_slice() else {
            continue;
        };
        if let Some(listed) = clauses.iter().find(|listed| listed.id == *clause) {
            found.entry(listed.id.as_str()).or_default().push(Row {
                result: (*result).to_owned(),
                evidence: (*evidence).to_owned(),
            });
        }
    }
    found
}

/// How the title of a test of accessibility starts that names no feature (`a11y the window …`).
const A11Y_TITLE: &str = "a11y ";

/// The texts in backticks of a cell that name a test: a title that starts with a feature
/// (`comp_15 …`, `ux_04_…`, `m3_f61 …`) or with `a11y `, or a file of the browser tests.
fn cited_tests(evidence: &str) -> Vec<&str> {
    evidence
        .split('`')
        .skip(1)
        .step_by(2)
        .filter(|cited| {
            cited.ends_with(".spec.ts") || cited.starts_with(A11Y_TITLE) || names_a_feature(cited)
        })
        .collect()
}

/// Whether a text starts as the title of a test does: a feature (`[a-z][a-z0-9]*`), `_`, and a
/// number, or a word of lower case letters and digits with a digit in it (`comp_15`, `m3_f61`).
fn names_a_feature(text: &str) -> bool {
    let Some((feature, rest)) = text.split_once('_') else {
        return false;
    };
    let number: String = rest
        .chars()
        .take_while(char::is_ascii_alphanumeric)
        .collect();
    feature.starts_with(|c: char| c.is_ascii_lowercase())
        && feature
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && number
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && number.chars().any(|c| c.is_ascii_digit())
}

/// Where the tests a row of the report cites are looked for: the checks of xtask, by their names.
const CHECKS_FOLDER: &str = "xtask/src/checks";

/// What is wrong with the EN 301 549 report: a clause without its row, result or evidence, or a test cited that
/// does not exist.
pub(crate) fn report_of(repo: &crate::repo::Repo) -> anyhow::Result<Vec<Finding>> {
    let clauses = clauses_of(&read_text(repo, CLAUSES_FILE)?)?;
    let report = read_text(repo, REPORT_FILE)?;
    let spec_files = sorted_files(repo, SUITE_FOLDER, |name| name.ends_with(".spec.ts"))?;
    let mut tests = String::new();
    for name in &spec_files {
        tests.push_str(&read_text(repo, &format!("{SUITE_FOLDER}/{name}"))?);
    }
    for name in sorted_files(repo, CHECKS_FOLDER, |name| has_extension(name, "rs"))? {
        tests.push_str(&read_text(repo, &format!("{CHECKS_FOLDER}/{name}"))?);
    }
    Ok(report_findings(&report, &clauses, &tests, &spec_files))
}

/// What is wrong with the report against the list of clauses, given the text of the tests it may
/// cite (`tests`) and the names of the browser test files (`spec_files`).
fn report_findings(
    report: &str,
    clauses: &[Clause],
    tests: &str,
    spec_files: &[String],
) -> Vec<Finding> {
    let found = rows(report, clauses);
    let mut findings = Vec::new();
    for clause in clauses {
        let id = &clause.id;
        let at = |message: String| Finding::at(REPORT_FILE, message);
        match found.get(id.as_str()).map(Vec::as_slice) {
            None | Some([]) => findings.push(at(format!(
                "Clause {id} of {CLAUSES_FILE} has no row. Add `| {id} | <title> | <result> | <evidence or reason> |`."
            ))),
            Some([row]) => {
                if !RESULTS.contains(&row.result.as_str()) {
                    findings.push(at(format!(
                        "Clause {id} gives the result `{}`. Write one of: {}.",
                        row.result,
                        RESULTS.join(", ")
                    )));
                }
                if row.result == NOT_TESTED_BY_A_PERSON && !clause.screen_reader {
                    findings.push(at(format!(
                        "Clause {id} says `{NOT_TESTED_BY_A_PERSON}`, but a machine or a reading can tell it. \
                         Give its result, or mark it `screen_reader = true` in {CLAUSES_FILE} if only a \
                         person with a screen reader can."
                    )));
                }
                if row.evidence.is_empty() {
                    findings.push(at(format!(
                        "Clause {id} has no evidence. Name the test that shows it, or the reason."
                    )));
                }
                for cited in cited_tests(&row.evidence) {
                    let exists = if cited.ends_with(".spec.ts") {
                        spec_files.iter().any(|file| file == cited)
                    } else {
                        tests.contains(cited)
                    };
                    if !exists {
                        findings.push(at(format!(
                            "Clause {id} cites the test `{cited}`, which does not exist."
                        )));
                    }
                }
            }
            Some(_) => findings.push(at(format!("Clause {id} has more than one row."))),
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clauses() -> Vec<Clause> {
        vec![
            Clause {
                id: "9.1.1.1".to_owned(),
                screen_reader: true,
            },
            Clause {
                id: "9.2.1.2".to_owned(),
                screen_reader: false,
            },
        ]
    }

    const TESTS: &str = "test(\"comp_15 every target is at least 24 px\", …)\n\
        test(\"m3_f61 the choices of a field\", …)\ntest(\"a11y the window, with the keyboard only\", …)";

    fn spec_files() -> Vec<String> {
        vec!["a11y-measures.spec.ts".to_owned()]
    }

    fn messages(report: &str) -> Vec<String> {
        report_findings(report, &clauses(), TESTS, &spec_files())
            .into_iter()
            .map(|finding| finding.message)
            .collect()
    }

    #[test]
    fn good_example() {
        let report = "| Clause | Title | Result | Evidence |\n|---|---|---|---|\n\
            | 9.1.1.1 | Non-text content | Not tested by a person | `comp_15 every target is at least 24 px`, `m3_f61 the choices of a field` |\n\
            | 9.2.1.2 | No keyboard trap | Partly supports | `a11y-measures.spec.ts`, `a11y the window, with the keyboard only` |\n";
        assert!(messages(report).is_empty(), "{:?}", messages(report));
    }

    #[test]
    fn bad_example() {
        // A clause without its row, a result that is none, no evidence, and a test that is not there.
        let report = "| 9.1.1.1 | Non-text content | Fine |  |\n";
        let found = messages(report);
        assert_eq!(found.len(), 3, "{found:?}");
        assert!(found[0].contains("`Fine`"));
        assert!(found[1].contains("no evidence"));
        assert!(found[2].contains("9.2.1.2"));
        let cited = messages(
            "| 9.1.1.1 | a | Supports | `comp_15 nothing like it` and `gone.spec.ts` |\n\
             | 9.2.1.2 | b | Not applicable | no keyboard |\n",
        );
        assert_eq!(cited.len(), 2, "{cited:?}");
        assert!(
            cited
                .iter()
                .all(|message| message.contains("does not exist"))
        );
        // A title of a feature with letters in its number, and one of accessibility, are checked too.
        let titled = messages(
            "| 9.1.1.1 | a | Supports | `m3_f61 nothing like it` |\n\
             | 9.2.1.2 | b | Supports | `a11y nothing like it` |\n",
        );
        assert_eq!(titled.len(), 2, "{titled:?}");
        assert!(titled[0].contains("`m3_f61 nothing like it`"));
        assert!(titled[1].contains("`a11y nothing like it`"));
        // *Not tested by a person* only for a clause that only a screen reader can tell.
        let untested = messages(
            "| 9.1.1.1 | a | Not tested by a person | heard |\n\
             | 9.2.1.2 | b | Not tested by a person | heard |\n",
        );
        assert_eq!(untested.len(), 1, "{untested:?}");
        assert!(untested[0].starts_with("Clause 9.2.1.2 says `Not tested by a person`"));
    }

    #[test]
    fn a_clause_with_two_rows_is_a_finding() {
        let report = "| 9.1.1.1 | a | Supports | why |\n| 9.1.1.1 | a | Supports | why |\n\
            | 9.2.1.2 | b | Supports | why |\n";
        assert_eq!(
            messages(report),
            vec!["Clause 9.1.1.1 has more than one row."]
        );
    }

    #[test]
    fn comp_15_every_criterion_of_the_report_has_evidence_or_a_reason() {
        let repo = crate::repo::Repo::discover().unwrap();
        let findings = report_of(&repo).unwrap();
        let messages: Vec<&str> = findings.iter().map(|f| f.message.as_str()).collect();
        assert!(messages.is_empty(), "{messages:#?}");
    }

    #[test]
    fn only_a_title_of_a_feature_or_a_spec_file_is_a_cited_test() {
        assert_eq!(
            cited_tests(
                "`ux_04_x`, `comp_15 y`, `m3_f61 z`, `a11y w`, `paths.home`, `keep_step`, \
                 `x.spec.ts`, `Tab`, `A11Y_TITLE`"
            ),
            vec!["ux_04_x", "comp_15 y", "m3_f61 z", "a11y w", "x.spec.ts"]
        );
    }
}
