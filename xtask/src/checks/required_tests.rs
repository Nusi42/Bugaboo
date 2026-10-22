//! The one implementation of "these tests must exist" (TEST-ISSUE-IDS, TEST-WORKFLOWS, TEST-UPLOADS,
//! TEST-EDIT-LOCKS, PERF-N1, ARCH-LOCK-ORDER): a list of test names, found in the Rust files of the
//! product crates. A name counts only when the test can run:
//!
//! - it has a test attribute (`#[test]`, `#[tokio::test]`),
//! - no `#[ignore]` in any form (`#[ignore = "…"]`, `#[cfg_attr(…, ignore)]`) and no `cfg(…)` on the
//!   function or on a block around it (`#[cfg(test)]` alone is fine: a test only runs under it),
//! - its file is part of the crate: a chain of `mod name;` (or `#[path = "…"]`) leads from a crate
//!   root (`lib.rs`, `main.rs`, a file in `tests/`) to it, without a `cfg` on the way,
//! - no inner `#![cfg(…)]` at the top of its file or of a module around it switches it off,
//! - and, when the list asks for it, the test runs on both databases (ARCH-DB-PARITY).
//!
//! A list that names a folder (`within`) also wants every Rust file in that folder reached by a
//! `mod`: a test file nobody declares never runs, whether or not a required test is in it.
//!
//! The lists and the words of the findings are the checks' own.

use std::collections::{HashMap, HashSet};

use super::Finding;
use crate::docs::text::has_extension;
use crate::repo::Repo;
use crate::rust_source::{strip, strip_comments};
use crate::source::{Source, read_text};

/// A test that has to exist.
pub(super) struct Required {
    pub(super) name: &'static str,
    pub(super) both_databases: bool,
    /// Only a test in a file below this path counts (empty: anywhere in the crates).
    pub(super) within: &'static str,
}

impl Required {
    /// A test anywhere in the crates.
    pub(super) const fn anywhere(name: &'static str) -> Self {
        Self {
            name,
            both_databases: false,
            within: "",
        }
    }

    /// A test anywhere in the crates that runs on both databases.
    pub(super) const fn both(name: &'static str) -> Self {
        Self {
            name,
            both_databases: true,
            within: "",
        }
    }

    /// The same test, but only in a file below `path` (a file or a folder).
    pub(super) const fn within(self, path: &'static str) -> Self {
        Self {
            name: self.name,
            both_databases: self.both_databases,
            within: path,
        }
    }
}

/// Every check that holds a list of required tests, by ID: DOC-PROVEN-BY reads the lists to see that
/// a test the docs cite as proof is required by the check they cite.
pub(super) fn lists() -> [(&'static str, &'static [Required]); 6] {
    use super::Check as _;
    [
        (
            super::test_issue_ids::TestIssueIds.id(),
            &super::test_issue_ids::REQUIRED,
        ),
        (
            super::test_workflows::TestWorkflows.id(),
            &super::test_workflows::REQUIRED,
        ),
        (
            super::test_uploads::TestUploads.id(),
            &super::test_uploads::REQUIRED,
        ),
        (
            super::test_edit_locks::TestEditLocks.id(),
            &super::test_edit_locks::REQUIRED,
        ),
        (super::perf_n1::PerfN1.id(), &super::perf_n1::REQUIRED),
        (
            super::arch_lock_order::ArchLockOrder.id(),
            &super::arch_lock_order::REQUIRED,
        ),
    ]
}

/// What runs a test on both databases.
const BOTH_DATABASES: &str = "each_database(";

/// Every Rust file of the crates, the folders of tests too: the tests are there. Whatever changed,
/// all of them are read.
pub(super) fn crate_sources(repo: &Repo) -> anyhow::Result<Vec<Source>> {
    let mut sources = Vec::new();
    for file in repo.all_files()? {
        if file.starts_with("crates/") && has_extension(&file, "rs") {
            let text = read_text(repo, &file)?;
            sources.push((file, text));
        }
    }
    Ok(sources)
}

/// A module a file declares with `mod name;`.
struct Declared {
    name: String,
    /// The file it names by `#[path = "…"]`, if it does.
    path: Option<String>,
    /// The inline modules (`mod a { mod b; }`) around the declaration, outermost first: the files
    /// of a module declared inside them are looked for in folders of these names.
    inline: Vec<String>,
}

/// The modules a file declares with `mod name;` and no `cfg` that switches them off. `blank` is the
/// code with comments and strings blanked (a `mod x;` in a string is no module), `kept` the same
/// code with the strings kept, from which the `#[path]` is read; both have the same positions.
fn declarations(blank: &str, kept: &str) -> Vec<Declared> {
    let mut modules = Vec::new();
    for (at, _) in blank.match_indices("mod ") {
        let rest = &blank[at + 4..];
        let Some(end) = rest.find([';', '{']) else {
            continue;
        };
        if !rest[end..].starts_with(';') || rest[..end].trim().contains(char::is_whitespace) {
            continue;
        }
        let attributes = attributes_before(&blank[..at]);
        // A `cfg` on the declaration, on a block around it, or an inner `#![cfg(…)]` at the top of
        // the file or of that block switches the module off.
        if has_cfg(attributes) || inside_switched_off_block(blank, at) {
            continue;
        }
        let kept_attributes = &kept[at - attributes.len()..at];
        modules.push(Declared {
            name: rest[..end].trim().to_owned(),
            path: path_attribute(kept_attributes),
            inline: inline_modules(blank, at),
        });
    }
    modules
}

/// The names of the inline modules that the position `at` is inside, outermost first.
fn inline_modules(code: &str, at: usize) -> Vec<String> {
    let mut open: Vec<Option<String>> = Vec::new();
    let mut header_start = 0;
    for (index, character) in code[..at].char_indices() {
        match character {
            '{' => {
                let header = attributes_before(&code[header_start..index]);
                let words: Vec<&str> = header.split_whitespace().collect();
                let named = words
                    .iter()
                    .position(|word| *word == "mod")
                    .and_then(|place| words.get(place + 1))
                    .map(|name| (*name).to_owned());
                open.push(named);
                header_start = index + 1;
            }
            '}' => {
                open.pop();
                header_start = index + 1;
            }
            ';' => header_start = index + 1,
            _ => {}
        }
    }
    open.into_iter().flatten().collect()
}

/// The text of `#[path = "…"]` among the attributes.
fn path_attribute(attributes: &str) -> Option<String> {
    let after = &attributes[attributes.find("path")? + 4..];
    let after = after.trim_start().strip_prefix('=')?.trim_start();
    let after = after.strip_prefix('"')?;
    Some(after[..after.find('"')?].to_owned())
}

/// The attributes of the item that starts right after `before`: everything since the end of the
/// item before it.
fn attributes_before(before: &str) -> &str {
    let start = before.rfind(['}', ';', '{']).map_or(0, |end| end + 1);
    &before[start..]
}

/// Whether attributes can switch an item off: a `cfg(…)` among them. `#[cfg(test)]` alone does not,
/// because a test only runs under it (a module of tests that clippy lets unwrap is written so).
fn has_cfg(attributes: &str) -> bool {
    attributes
        .replace("#[cfg(test)]", "")
        .replace("#![cfg(test)]", "")
        .contains("cfg(")
}

/// Whether a `#![cfg(…)]` is among the inner attributes that start `body` (the top of a file, or
/// the first lines inside `mod m {`): it switches off everything in that scope.
fn starts_with_inner_cfg(body: &str) -> bool {
    let mut rest = body.trim_start();
    while let Some(after) = rest.strip_prefix("#![") {
        let Some(end) = after.find(']') else {
            return false;
        };
        if has_cfg(&rest[..end + 4]) {
            return true;
        }
        rest = after[end + 1..].trim_start();
    }
    false
}

/// Whether the file, or a block that encloses the position `at`, is switched off: `cfg(` in the
/// attributes of a block (a `mod` around the tests), or an inner `#![cfg(…)]` at the top of the
/// file or of such a block.
fn inside_switched_off_block(code: &str, at: usize) -> bool {
    if starts_with_inner_cfg(code) {
        return true;
    }
    let mut open: Vec<bool> = Vec::new();
    let mut header_start = 0;
    for (index, character) in code[..at].char_indices() {
        match character {
            '{' => {
                open.push(
                    has_cfg(&code[header_start..index])
                        || starts_with_inner_cfg(&code[index + 1..]),
                );
                header_start = index + 1;
            }
            '}' => {
                open.pop();
                header_start = index + 1;
            }
            ';' => header_start = index + 1,
            _ => {}
        }
    }
    open.contains(&true)
}

/// Whether the attributes hold a test attribute: `#[test]`, `#[tokio::test]`, also with arguments
/// (`#[tokio::test(flavor = "multi_thread")]`). `cfg(test)` is a condition, not a test attribute.
fn has_test_attribute(attributes: &str) -> bool {
    attributes.match_indices("test").any(|(at, word)| {
        let before = attributes[..at].chars().next_back();
        let after = attributes[at + word.len()..].chars().next();
        matches!(before, Some('[' | ':')) && matches!(after, Some(']' | '('))
    })
}

/// Whether the function whose `fn` stands at `at` is a test that is not switched off. The
/// attributes are everything between the end of the item before it (`}` or `;`) and the function;
/// comments are already blank in `code`, so a doc comment between the attributes does not hide one,
/// and an attribute over several lines is read whole. `ignore` anywhere in them counts as off, and
/// so does `cfg(` in them or in the attributes of a block around the function.
fn is_live_test(code: &str, at: usize) -> bool {
    let attributes = attributes_before(&code[..at]);
    has_test_attribute(attributes)
        && !attributes.contains("ignore")
        && !has_cfg(attributes)
        && !inside_switched_off_block(code, at)
}

/// The text of the function that starts at `at`: up to the next item of the same level (or the
/// end of the block around it).
fn body_at(code: &str, at: usize) -> &str {
    let line_start = code[..at].rfind('\n').map_or(0, |start| start + 1);
    let line = &code[line_start..];
    let indent = line.len() - line.trim_start().len();
    let mut offset = at;
    for (index, line) in code[at..].split_inclusive('\n').enumerate() {
        let start = offset;
        offset += line.len();
        if index == 0 {
            continue;
        }
        let trimmed = line.trim_start();
        let depth = line.len() - trimmed.len();
        let starts_item = ["#[", "fn ", "async fn ", "pub ", "mod ", "use ", "const "]
            .iter()
            .any(|word| trimmed.starts_with(word));
        if !trimmed.trim().is_empty() && ((depth <= indent && starts_item) || depth < indent) {
            return &code[at..start];
        }
    }
    &code[at..]
}

/// Where every file of the crates is declared as a module, so that "does a crate root reach this
/// file" is a walk up.
struct Tree {
    /// For a file, the files that declare it (without a `cfg`).
    declared_by: HashMap<String, Vec<String>>,
}

impl Tree {
    fn of(code: &[(&str, String, String)]) -> Self {
        let mut declared_by: HashMap<String, Vec<String>> = HashMap::new();
        for (path, text, blank) in code {
            let (folder, file) = path.rsplit_once('/').unwrap_or(("", path));
            // The modules of a root or of a `mod.rs` are beside it, those of `a.rs` in `a/`.
            let children = if is_root(path) || file == "mod.rs" {
                folder.to_owned()
            } else {
                format!("{folder}/{}", file.trim_end_matches(".rs"))
            };
            for declared in declarations(blank, text) {
                let Declared {
                    name,
                    path: explicit,
                    inline,
                } = declared;
                // The files of a module are in a folder per inline module around its declaration.
                let base = join(&children, &inline.join("/"));
                let targets = explicit.map_or_else(
                    || vec![format!("{base}/{name}.rs"), format!("{base}/{name}/mod.rs")],
                    |relative| {
                        let from = if inline.is_empty() { folder } else { &base };
                        vec![join(from, &relative)]
                    },
                );
                for target in targets {
                    declared_by
                        .entry(target)
                        .or_default()
                        .push((*path).to_owned());
                }
            }
        }
        Self { declared_by }
    }

    /// Whether a chain of declarations leads from a crate root to the file.
    fn reaches(&self, path: &str) -> bool {
        self.walk(path, &mut HashSet::new())
    }

    fn walk(&self, path: &str, seen: &mut HashSet<String>) -> bool {
        if is_root(path) {
            return true;
        }
        if !seen.insert(path.to_owned()) {
            return false;
        }
        self.declared_by
            .get(path)
            .is_some_and(|declarers| declarers.iter().any(|from| self.walk(from, seen)))
    }
}

/// `folder/relative`, with `..` resolved.
fn join(folder: &str, relative: &str) -> String {
    let mut parts: Vec<&str> = folder.split('/').filter(|part| !part.is_empty()).collect();
    for part in relative.split('/') {
        match part {
            ".." => {
                parts.pop();
            }
            "." | "" => {}
            other => parts.push(other),
        }
    }
    parts.join("/")
}

/// Whether Cargo builds the file by itself: `crates/<crate>/src/{lib,main}.rs`, `build.rs`, a file in
/// `src/bin/`, or one directly in `tests/`, `benches/` or `examples/`.
fn is_root(path: &str) -> bool {
    let parts: Vec<&str> = path.split('/').collect();
    matches!(
        parts.as_slice(),
        ["crates", _, "src", "lib.rs" | "main.rs"]
            | ["crates", _, "build.rs"]
            | ["crates", _, "src", "bin", _]
            | ["crates", _, "tests" | "benches" | "examples", _]
    )
}

/// Whether `path` is `within` or below it, on a folder boundary (`…/editing` does not hold
/// `…/editing_old.rs`). Empty `within`: anywhere.
fn is_within(path: &str, within: &str) -> bool {
    let folder = within.trim_end_matches('/');
    folder.is_empty()
        || path == folder
        || path
            .strip_prefix(folder)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// What is wrong with one required test, or nothing.
enum Verdict<'a> {
    Fine,
    Missing,
    /// The test exists but cannot run: where, and why.
    SwitchedOff(&'a str, &'static str),
    /// It runs on one database only: where.
    OneDatabase(&'a str),
}

const NO_ATTRIBUTE: &str = "it has no test attribute, or an `#[ignore]` or a `cfg` switches it off";
const NO_MODULE: &str = "no `mod` leads to its file from the root of the crate (or only behind a \
                         `cfg`), so it never runs";

/// What is wrong with the test `each`, looking at every function of that name.
fn judge<'a>(code: &'a [(&'a str, String, String)], tree: &Tree, each: &Required) -> Verdict<'a> {
    let needle = format!("fn {}(", each.name);
    let mut off = None;
    let mut one_database = None;
    for (path, _, text) in code {
        if !is_within(path, each.within) {
            continue;
        }
        for (at, _) in text.match_indices(&needle) {
            if !is_live_test(text, at) {
                off = Some((*path, NO_ATTRIBUTE));
            } else if !tree.reaches(path) {
                off = off.or(Some((*path, NO_MODULE)));
            } else if each.both_databases && !body_at(text, at).contains(BOTH_DATABASES) {
                one_database = Some(*path);
            } else {
                return Verdict::Fine;
            }
        }
    }
    match (one_database, off) {
        (Some(path), _) => Verdict::OneDatabase(path),
        (None, Some((path, why))) => Verdict::SwitchedOff(path, why),
        (None, None) => Verdict::Missing,
    }
}

/// The Rust files in the folders that `required` names that no `mod` reaches from a crate root (or
/// only behind a `cfg`): tests in them never run, and nobody sees it.
fn unreached(code: &[(&str, String, String)], tree: &Tree, required: &[Required]) -> Vec<Finding> {
    let mut folders: Vec<&str> = required
        .iter()
        .map(|each| each.within)
        .filter(|within| !within.is_empty() && !has_extension(within, "rs"))
        .collect();
    folders.sort_unstable();
    folders.dedup();
    code.iter()
        .filter(|(path, _, _)| {
            folders.iter().any(|folder| is_within(path, folder)) && !tree.reaches(path)
        })
        .map(|(path, _, _)| {
            Finding::at(
                *path,
                format!(
                    "This file is in a folder of required tests, but {NO_MODULE}. Declare it with \
                     `mod` or delete it."
                ),
            )
        })
        .collect()
}

/// The findings for the tests of `required` that are missing (`why_needed` finishes the sentence
/// "The test … is missing. …"), switched off, or run on one database only.
pub(super) fn evaluate(
    sources: &[Source],
    required: &[Required],
    why_needed: &str,
) -> Vec<Finding> {
    let code: Vec<(&str, String, String)> = sources
        .iter()
        .map(|(path, text)| (path.as_str(), strip_comments(text), strip(text)))
        .collect();
    let tree = Tree::of(&code);
    let unreached = unreached(&code, &tree, required);
    required
        .iter()
        .filter_map(|each| match judge(&code, &tree, each) {
            Verdict::Fine => None,
            Verdict::Missing => Some(Finding::general(format!(
                "The test `{}` is missing. {why_needed} write it again or, if the rule changed, \
                 change this list.",
                each.name
            ))),
            Verdict::SwitchedOff(path, why) => Some(Finding::at(
                path,
                format!("The test `{}` cannot run: {why}. {why_needed}", each.name),
            )),
            Verdict::OneDatabase(path) => Some(Finding::at(
                path,
                format!(
                    "The test `{}` has to run on both databases: wrap it in `each_database` \
                     (ARCH-DB-PARITY).",
                    each.name
                ),
            )),
        })
        .chain(unreached)
        .collect()
}

#[cfg(test)]
mod tests;

/// What the example tests of a check prove, once for every check that has a list of tests.
#[cfg(test)]
pub(super) mod examples {
    use std::fmt::Write as _;

    use super::{Required, crate_sources, evaluate};
    use crate::repo::Repo;
    use crate::source::Source;

    /// A crate whose root declares `tests`, with every required test in it, each with `wrapper`
    /// as its body.
    fn all_tests(required: &[Required], wrapper: &str) -> Vec<Source> {
        let mut text = String::new();
        for each in required {
            let _ = write!(
                text,
                "#[tokio::test]
async fn {}() {{
    {wrapper}
}}

",
                each.name
            );
        }
        vec![
            ("crates/x/src/lib.rs".to_owned(), "mod tests;\n".to_owned()),
            ("crates/x/src/tests.rs".to_owned(), text),
        ]
    }

    /// The list of a check that names where its tests are is met by this repository.
    pub(in super::super) fn this_repository_has_every_test(required: &[Required]) {
        let found: Vec<_> = evaluate(
            &crate_sources(&Repo::discover().unwrap()).unwrap(),
            required,
            "",
        )
        .into_iter()
        .map(|finding| finding.message)
        .collect();
        assert!(found.is_empty(), "{found:?}");
    }

    /// Nothing at all: every test of the list is found missing.
    pub(in super::super) fn without_the_sources_every_test_is_missing(required: &[Required]) {
        assert_eq!(evaluate(&[], required, "").len(), required.len());
    }

    /// Every test exists and runs on both databases: no finding.
    pub(in super::super) fn every_test_is_there(required: &[Required]) {
        let sources = all_tests(required, "each_database(\"x\", body).await");
        assert!(evaluate(&sources, required, "").is_empty());
    }

    /// A missing test, and a test on one database only, are found.
    pub(in super::super) fn a_missing_or_single_test_is_found(required: &[Required]) {
        let missing = evaluate(
            &[("crates/x/src/tests.rs".to_owned(), String::new())],
            required,
            "",
        );
        assert_eq!(missing.len(), required.len());
        let single = evaluate(&all_tests(required, "body().await"), required, "");
        assert_eq!(
            single.len(),
            required.iter().filter(|each| each.both_databases).count()
        );
    }

    /// A name in a comment is not a test.
    pub(in super::super) fn a_name_in_a_comment_is_no_test(required: &[Required], name: &str) {
        let mut sources = all_tests(required, "each_database(\"x\", body).await");
        for (_, text) in &mut sources {
            *text = text.replace(&format!("async fn {name}"), &format!("// async fn {name}"));
        }
        assert_eq!(evaluate(&sources, required, "").len(), 1);
    }
}
