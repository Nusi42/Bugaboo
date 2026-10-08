//! META-SUPPRESS: every suppression has a reason that names an issue or a doc section, and the
//! total may not grow unnoticed (architecture §10, CLAUDE.md).
//!
//! The counted list lives in `tools/checks/suppressions.txt`. It is rewritten by
//! `cargo xtask suppressions`, so a new suppression shows up as a visible change in review.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;

use super::{Check, Context, Finding};
use crate::docs::text::has_extension;
use crate::repo::Repo;
use crate::source::{normalize, read_text};

/// Where the counted list lives, relative to the repository root.
pub(crate) const LIST_PATH: &str = "tools/checks/suppressions.txt";

/// The file types that are searched for suppressions. This is the one list (architecture §10).
const SCANNED_TYPES: [&str; 14] = [
    "rs", "ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs", "css", "yaml", "yml", "md", "toml",
];

/// Files that are not searched, each with the reason.
const SELF_EXEMPT: [(&str, &str); 1] = [(
    "tools/checks/catalog.toml",
    "describes the markers in the titles of the checks; it is data, not code (ADR-0020)",
)];

/// This file spells out the markers it searches for. Only the lines between the two comments
/// below are left out when it is searched; a real suppression anywhere else in it counts.
const SELF_PATH: &str = "xtask/src/checks/meta_suppress.rs";
const DEFINITIONS_BEGIN: &str = "// begin marker definitions";
const DEFINITIONS_END: &str = "// end marker definitions";

// begin marker definitions
/// Rust attributes that switch a lint off or expect it.
const RUST_MARKERS: [&str; 4] = ["#[allow(", "#![allow(", "#[expect(", "#![expect("];

/// Rust attributes that apply another attribute conditionally; a suppression if it holds one.
const RUST_CONDITIONAL: &str = "cfg_attr(";

/// Comment markers of the other tools.
const COMMENT_MARKERS: [&str; 6] = [
    "eslint-disable",
    "oxlint-disable",
    "stylelint-disable",
    "ast-grep-ignore",
    "@ts-ignore",
    "@ts-expect-error",
];

/// Markers that count in every tracked text file, whatever its type: the secret scan reads its
/// marker on the line of the secret in any file (Rust, Markdown, JSON, HTML, scripts ...).
const ANY_FILE_MARKERS: [&str; 1] = ["gitleaks:allow"];

/// Markers in Markdown files, only counted inside an HTML comment.
const MARKDOWN_MARKERS: [&str; 2] = ["rumdl-disable", "markdownlint-disable"];

/// Accepted security advisories in `deny.toml` (the "not affected" statements).
const ADVISORY_MARKER: &str = "RUSTSEC-";
// end marker definitions

const HEADER: &str = "# Suppressions counted by META-SUPPRESS. Rewritten by `cargo xtask suppressions`.\n# The total may only grow in a change that also updates this file (and the lead approves it).\n";

pub(crate) struct MetaSuppress;

impl Check for MetaSuppress {
    fn id(&self) -> &'static str {
        "META-SUPPRESS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let scan = scan_repo(context.repo)?;
        let mut findings = scan.findings;
        let on_disk = fs::read_to_string(context.repo.path(LIST_PATH))
            .ok()
            .map(|text| normalize(&text));
        findings.extend(compare_list(&render_list(&scan.counts), on_disk.as_deref()));
        Ok(findings)
    }
}

/// Rewrites `tools/checks/suppressions.txt` from the code (`cargo xtask suppressions`).
pub(crate) fn write_list(repo: &Repo) -> anyhow::Result<()> {
    let scan = scan_repo(repo)?;
    fs::write(repo.path(LIST_PATH), render_list(&scan.counts))?;
    Ok(())
}

struct Scan {
    /// Suppressions per file.
    counts: BTreeMap<String, usize>,
    /// Suppressions without a proper reason.
    findings: Vec<Finding>,
}

fn scan_repo(repo: &Repo) -> anyhow::Result<Scan> {
    let mut counts = BTreeMap::new();
    let mut findings = Vec::new();
    for file in repo.all_files()? {
        let scanned = SCANNED_TYPES.iter().any(|kind| has_extension(&file, kind));
        let exempt = SELF_EXEMPT.iter().any(|(path, _)| *path == file);
        if exempt || file.contains("/fixtures/") {
            continue;
        }
        // The other files (JSON, HTML, scripts ...) are searched for the markers of every file
        // only; a file that is no text (a picture) has none.
        let Some(mut source) = (if scanned {
            Some(read_text(repo, &file)?)
        } else {
            fs::read(repo.path(&file))
                .ok()
                .and_then(|bytes| String::from_utf8(bytes).ok())
                .map(|text| normalize(&text))
        }) else {
            continue;
        };
        if file == SELF_PATH {
            source = without_definitions(&source);
        }
        let (count, problems) = scan_source(&file, &source, scanned);
        if count > 0 {
            counts.insert(file.clone(), count);
        }
        findings.extend(problems);
    }
    Ok(Scan { counts, findings })
}

/// Blanks the marker definitions of this file, keeping the line numbers.
fn without_definitions(source: &str) -> String {
    let mut inside = false;
    source
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            if trimmed == DEFINITIONS_BEGIN {
                inside = true;
            }
            let blank = inside;
            if trimmed == DEFINITIONS_END {
                inside = false;
            }
            if blank { "" } else { line }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_list(counts: &BTreeMap<String, usize>) -> String {
    let mut out = String::from(HEADER);
    for (file, count) in counts {
        let _ = writeln!(out, "{file} {count}");
    }
    let _ = writeln!(out, "total {}", counts.values().sum::<usize>());
    out
}

fn compare_list(expected: &str, on_disk: Option<&str>) -> Option<Finding> {
    (on_disk != Some(expected)).then(|| {
        Finding::at(
            LIST_PATH,
            "The suppression list does not match the code. Run `cargo xtask suppressions` and let the lead approve the change; the total may only grow in a change that updates this file.",
        )
    })
}

/// Counts the suppressions in one file and reports those without a good reason. A file of a type
/// that is not `scanned` is searched for the markers of every file only.
fn scan_source(file: &str, source: &str, scanned: bool) -> (usize, Vec<Finding>) {
    let is_rust = scanned && has_extension(file, "rs");
    let is_markdown = scanned && has_extension(file, "md");
    let is_toml = scanned && has_extension(file, "toml");
    let mut count = 0;
    let mut findings = Vec::new();
    let mut offset = 0;
    for (index, raw_line) in source.split_inclusive('\n').enumerate() {
        let line = raw_line.trim_end_matches(['\n', '\r']);
        let line_offset = offset;
        offset += raw_line.len();
        let place = format!("{file}:{}", index + 1);
        // The text after each marker on this line that must name a source.
        let mut reasons: Vec<String> = Vec::new();
        if is_rust {
            let rest_of_file = &source[line_offset..];
            for marker in RUST_MARKERS {
                for (at, _) in line.match_indices(marker) {
                    reasons.push(attribute_reason(&rest_of_file[at..]));
                }
            }
            for (at, _) in line.match_indices(RUST_CONDITIONAL) {
                let attribute = cut_attribute(&rest_of_file[at..]);
                if attribute.contains("allow(") || attribute.contains("expect(") {
                    reasons.push(attribute_reason(&rest_of_file[at..]));
                }
            }
        } else if is_markdown {
            for marker in MARKDOWN_MARKERS {
                for (at, _) in line.match_indices(marker) {
                    if line[..at].contains("<!--") {
                        reasons.push(line[at..].to_owned());
                    }
                }
            }
        } else if scanned {
            for marker in COMMENT_MARKERS {
                reasons.extend(
                    line.match_indices(marker)
                        .map(|(at, _)| line[at..].to_owned()),
                );
            }
            if is_toml {
                reasons.extend(
                    line.match_indices(ADVISORY_MARKER)
                        .map(|(at, _)| line[at..].to_owned()),
                );
            }
        }
        for marker in ANY_FILE_MARKERS {
            reasons.extend(
                line.match_indices(marker)
                    .map(|(at, _)| line[at..].to_owned()),
            );
        }
        for reason in reasons {
            count += 1;
            if !names_source(&reason) {
                findings.push(missing_reason(&place));
            }
        }
    }
    (count, findings)
}

fn missing_reason(place: &str) -> Finding {
    Finding::at(
        place,
        "This suppression has no reason that names an issue or a doc section. Add `reason = \"…\"` (Rust) or a trailing comment naming for example `coding-standards §3` or `BUG-123`. Better: fix the cause.",
    )
}

/// Cuts an attribute (or anything starting at its first parenthesis) at the parenthesis that
/// closes it. Parentheses inside string literals do not count.
fn cut_attribute(text: &str) -> &str {
    let mut depth = 0_usize;
    let mut in_string = false;
    let mut escaped = false;
    for (at, character) in text.char_indices() {
        if in_string {
            match (escaped, character) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '"') => in_string = false,
                _ => {}
            }
            continue;
        }
        match character {
            '"' => in_string = true,
            '(' => depth += 1,
            ')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return &text[..=at];
                }
            }
            _ => {}
        }
    }
    text
}

/// The `reason = "…"` text of an attribute, or an empty string when it has none.
fn attribute_reason(text: &str) -> String {
    let attribute = cut_attribute(text);
    let Some((_, after)) = attribute.split_once("reason") else {
        return String::new();
    };
    let Some((_, quoted)) = after.split_once('"') else {
        return String::new();
    };
    quoted.split('"').next().unwrap_or("").to_owned()
}

/// Whether a reason names a doc section (`§3`), a decision record or an issue (`ADR-0014`,
/// `BUG-123`) or a numbered issue (`#12`).
fn names_source(reason: &str) -> bool {
    reason.contains('§')
        || reason
            .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '#'))
            .any(|word| is_issue_word(word) || is_numbered_issue(word))
}

fn is_issue_word(word: &str) -> bool {
    word.split_once('-').is_some_and(|(prefix, number)| {
        prefix.len() >= 2
            && prefix.bytes().all(|b| b.is_ascii_uppercase())
            && !number.is_empty()
            && number.bytes().all(|b| b.is_ascii_digit())
    })
}

fn is_numbered_issue(word: &str) -> bool {
    word.strip_prefix('#')
        .is_some_and(|number| !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counted(file: &str, source: &str) -> (usize, usize) {
        let (count, findings) = scan_source(file, source, true);
        (count, findings.len())
    }

    const ALLOW: &str = RUST_MARKERS[0];
    const INNER_ALLOW: &str = RUST_MARKERS[1];
    const EXPECT: &str = RUST_MARKERS[2];

    #[test]
    fn good_example() {
        let reasoned = format!(
            "{ALLOW}clippy::print_stdout, reason = \"CLI output, coding-standards §3\")]\nfn x() {{}}\n"
        );
        assert_eq!(counted("a.rs", &reasoned), (1, 0));
        let multi = format!(
            "{INNER_ALLOW}\n    clippy::print_stdout,\n    reason = \"terminal output (ISS-12)\"\n)]\n"
        );
        assert_eq!(counted("a.rs", &multi), (1, 0));
        let ts = format!(
            "// {}-next-line no-console -- ADR-0009\n",
            COMMENT_MARKERS[1]
        );
        assert_eq!(counted("a.ts", &ts), (1, 0));
        let secret = format!(
            "const password = \"x\"; // {} -- fake test credential, SEC-16\n",
            ANY_FILE_MARKERS[0]
        );
        assert_eq!(counted("a.ts", &secret), (1, 0));
        let md = format!("<!-- {} MD013 -- ADR-0014 -->\n", MARKDOWN_MARKERS[0]);
        assert_eq!(counted("a.md", &md), (1, 0));
        let prose = format!("Prose about {} stays free.\n", MARKDOWN_MARKERS[0]);
        assert_eq!(counted("a.md", &prose), (0, 0));
        let deny = format!("ignore = [\"{ADVISORY_MARKER}2024-1\", # BUG-12\n");
        assert_eq!(counted("deny.toml", &deny), (1, 0));
    }

    #[test]
    fn bad_example() {
        assert_eq!(
            counted("a.rs", &format!("{ALLOW}dead_code)]\nfn x() {{}}\n")),
            (1, 1)
        );
        let vague = format!("{ALLOW}dead_code, reason = \"later\")]\n");
        assert_eq!(counted("a.rs", &vague), (1, 1));
        assert_eq!(counted("a.rs", &format!("{EXPECT}dead_code)]\n")), (1, 1));
        let conditional = format!("#[{RUST_CONDITIONAL}test, allow(dead_code))]\n");
        assert_eq!(counted("a.rs", &conditional), (1, 1));
        let ts = format!("// {}-next-line no-console\n", COMMENT_MARKERS[1]);
        assert_eq!(counted("a.ts", &ts), (1, 1));
        let tsx = format!("// {}\n", COMMENT_MARKERS[5]);
        assert_eq!(counted("a.tsx", &tsx), (1, 1));
        let secret = format!("const password = \"x\"; // {}\n", ANY_FILE_MARKERS[0]);
        assert_eq!(counted("a.ts", &secret), (1, 1));
        let yml = format!("# {}-line\n", COMMENT_MARKERS[0]);
        assert_eq!(counted("a.yml", &yml), (1, 1));
        let md = format!("<!-- {} MD013 -->\n", MARKDOWN_MARKERS[1]);
        assert_eq!(counted("a.md", &md), (1, 1));
        let deny = format!("ignore = [\"{ADVISORY_MARKER}2024-1\"]\n");
        assert_eq!(counted("deny.toml", &deny), (1, 1));
    }

    #[test]
    fn the_marker_of_the_secret_scan_counts_in_every_type_of_file() {
        let marker = ANY_FILE_MARKERS[0];
        // Rust and Markdown have markers of their own and still count this one.
        let rust = format!("let password = \"x\"; // {marker} -- fake credential, SEC-16\n");
        assert_eq!(counted("a.rs", &rust), (1, 0));
        assert_eq!(counted("a.rs", &format!("let x = \"{marker}\";\n")), (1, 1));
        let md = format!("password: x {marker} -- fake credential, SEC-16\n");
        assert_eq!(counted("a.md", &md), (1, 0));
        assert_eq!(counted("a.md", &format!("password: x {marker}\n")), (1, 1));
        // A type that is not searched for anything else is searched for this one.
        let json = format!("{{ \"password\": \"x\" }} // {marker} -- fake credential, SEC-16\n");
        assert_eq!(scan_source("a.json", &json, false).0, 1);
        assert!(scan_source("a.json", &json, false).1.is_empty());
        let bare = format!("password=x # {marker}\n");
        let (count, findings) = scan_source("a.ps1", &bare, false);
        assert_eq!((count, findings.len()), (1, 1));
        // And no other marker counts there.
        let other = format!("// {} x\n", COMMENT_MARKERS[1]);
        assert_eq!(scan_source("a.json", &other, false).0, 0);
        // One line, one marker: not counted twice.
        assert_eq!(
            counted("a.ts", &format!("x // {marker} -- SEC-16\n")),
            (1, 0)
        );
    }

    #[test]
    fn a_reason_in_the_next_attribute_does_not_count() {
        let source = format!("{ALLOW}dead_code)]\n#[doc = \"ISS-12\"]\nfn x() {{}}\n");
        assert_eq!(counted("a.rs", &source), (1, 1));
        let other = format!("#[{RUST_CONDITIONAL}test, derive(Debug))]\n");
        assert_eq!(counted("a.rs", &other), (0, 0));
    }

    #[test]
    fn attributes_are_cut_at_the_closing_bracket() {
        assert_eq!(cut_attribute("(a, b(c), \")\") rest"), "(a, b(c), \")\")");
        let attribute = format!("{ALLOW}x, reason = \"ADR-1\")] ISS-12");
        assert_eq!(attribute_reason(&attribute), "ADR-1");
    }

    #[test]
    fn only_the_marker_definitions_of_this_file_are_left_out() {
        let source = format!(
            "{DEFINITIONS_BEGIN}\nconst X: &str = \"{ALLOW}\";\n{DEFINITIONS_END}\n{ALLOW}dead_code)]\n"
        );
        let blanked = without_definitions(&source);
        assert_eq!(blanked.lines().count(), 4);
        assert_eq!(counted(SELF_PATH, &blanked), (1, 1));
    }

    #[test]
    fn the_list_may_not_drift() {
        let counts = BTreeMap::from([("a.rs".to_owned(), 1)]);
        let list = render_list(&counts);
        assert!(list.ends_with("a.rs 1\ntotal 1\n"));
        assert!(compare_list(&list, Some(&list)).is_none());
        let grown = render_list(&BTreeMap::from([("a.rs".to_owned(), 2)]));
        assert!(compare_list(&grown, Some(&list)).is_some());
        assert!(compare_list(&list, None).is_some());
    }

    #[test]
    fn reasons_must_name_a_source() {
        assert!(names_source("see coding-standards §3"));
        assert!(names_source("BUG-123: temporary"));
        assert!(names_source("ADR-0014"));
        assert!(names_source("tracked in #42"));
        assert!(!names_source("because it is annoying"));
    }
}
