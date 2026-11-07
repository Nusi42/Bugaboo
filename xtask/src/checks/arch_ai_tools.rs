//! ARCH-AI-TOOLS: AI tools exist only in the one registry, and none reaches what is designed out
//! (ADR-0013, COMP-10, ADR-0035 decision 9).
//!
//! The registry is the table in `crates/bb-app/src/ai/tools/registry.rs`; the names of the tools
//! are the words of `Tool` in `crates/bb-core/src/ai.rs`. The rules:
//!
//! - **Every tool is in the table once**, as `reads(Tool::X, "use_case", …)` or
//!   `changes(Tool::X, "use_case", …)`: it declares whether it reads or changes (which fixes the
//!   scope it needs), and the use case it runs, which must exist in `bb-app`. The table has no
//!   tool that `Tool` does not name.
//! - **Every tool that changes has a test of its preview** in `crates/bb-app/src/tests/`, named
//!   `ai_21_preview_<tool>…`.
//! - **No file builds a tool outside the registry**: no `ToolSpec` and no tool name written as a
//!   string in product code of any other file (the words of `Tool` are the one home of the names).
//! - **No tool calls a use case that is designed out** (COMP-10): the list `DESIGNED_OUT` of the
//!   registry, every use case of the folders `working_hours/` and `roles/`, and their modules. This
//!   holds for the tools folder and for `bb-mcp` and `bb-ai`, which list and call tools only through
//!   the registry.
//!
//! Comments and code behind `#[cfg(test)]` do not count.

use std::collections::BTreeSet;

use super::required_tests::crate_sources;
use super::{Check, Context, Finding};
use crate::rust_source::{strip_comments, without_tests};
use crate::source::Source;

const REGISTRY: &str = "crates/bb-app/src/ai/tools/registry.rs";
const TOOL_WORDS: &str = "crates/bb-core/src/ai.rs";
const TOOLS_FOLDER: &str = "crates/bb-app/src/ai/tools/";
const TESTS_FOLDER: &str = "crates/bb-app/src/tests/";
const APP_FOLDER: &str = "crates/bb-app/src/";
/// The folders whose use cases a tool never runs.
const DESIGNED_OUT_FOLDERS: [&str; 2] = [
    "crates/bb-app/src/working_hours/",
    "crates/bb-app/src/roles/",
];
/// The modules a tool never imports.
const DESIGNED_OUT_PATHS: [&str; 4] = [
    "working_hours::",
    "roles::",
    "bb_app::working_hours",
    "bb_app::roles",
];
/// The code that lists and calls tools through the registry and must not reach past it.
const OUTSIDE_FOLDERS: [&str; 2] = ["crates/bb-mcp/", "crates/bb-ai/"];

pub(crate) struct ArchAiTools;

impl Check for ArchAiTools {
    fn id(&self) -> &'static str {
        "ARCH-AI-TOOLS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        // The tests are read too (the previews must have theirs): every Rust file of the crates.
        Ok(findings(&crate_sources(context.repo)?))
    }
}

/// One tool: its variant, its word, and what the table says of it.
#[derive(Debug, PartialEq, Eq)]
struct Tool {
    variant: String,
    word: String,
}

/// One entry of the table.
#[derive(Debug, PartialEq, Eq)]
struct Entry {
    changes: bool,
    variant: String,
    use_case: String,
}

fn is_ident(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

/// The tools the enum `Tool` names: `Variant => "word",` between its braces.
fn tools_named(text: &str) -> Vec<Tool> {
    let code = strip_comments(text);
    let Some(start) = code.find("pub enum Tool {") else {
        return Vec::new();
    };
    let body = &code[start..];
    let end = body.find("\n    }").unwrap_or(body.len());
    body[..end]
        .lines()
        .filter_map(|line| {
            let (variant, word) = line.split_once("=>")?;
            let word = word.trim().trim_end_matches(',').trim().trim_matches('"');
            let variant = variant.trim();
            (variant.chars().all(is_ident) && !variant.is_empty() && !word.is_empty()).then(|| {
                Tool {
                    variant: variant.to_owned(),
                    word: word.to_owned(),
                }
            })
        })
        .collect()
}

/// The entries of the table: `reads(Tool::X, "use_case", …)` and `changes(…)`.
fn entries_of(text: &str) -> Vec<Entry> {
    let code = strip_comments(text);
    let mut found = Vec::new();
    for (word, changes) in [("reads(", false), ("changes(", true)] {
        let mut from = 0;
        while let Some(at) = code[from..].find(word) {
            let start = from + at;
            from = start + word.len();
            let before = code[..start].chars().next_back();
            if before.is_some_and(is_ident) {
                continue;
            }
            let rest = code[from..].trim_start();
            let Some(rest) = rest.strip_prefix("Tool::") else {
                continue;
            };
            let variant: String = rest.chars().take_while(|c| is_ident(*c)).collect();
            let after = rest[variant.len()..].trim_start();
            let Some(after) = after.strip_prefix(',') else {
                continue;
            };
            let use_case = after
                .trim_start()
                .strip_prefix('"')
                .and_then(|quoted| quoted.split_once('"'))
                .map(|(name, _)| name.to_owned())
                .unwrap_or_default();
            found.push(Entry {
                changes,
                variant,
                use_case,
            });
        }
    }
    found
}

/// The strings of the list `DESIGNED_OUT` of the registry.
fn designed_out_of(text: &str) -> Vec<String> {
    let code = strip_comments(text);
    let Some(start) = code.find("DESIGNED_OUT") else {
        return Vec::new();
    };
    // The list follows the `=`; the type before it (`&[&str]`) has brackets too.
    let body = &code[start..];
    let body = &body[body.find('=').unwrap_or(0)..];
    let open = body.find("&[").map_or(0, |at| at + 2);
    let close = body[open..].find(']').map_or(body.len(), |at| open + at);
    body[open..close]
        .split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

/// The names of the use cases a file defines: `pub async fn name` and `pub fn name`.
fn public_functions(text: &str) -> Vec<String> {
    let code = strip_comments(&without_tests(text));
    code.lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let rest = line
                .strip_prefix("pub async fn ")
                .or_else(|| line.strip_prefix("pub fn "))?;
            let name: String = rest.chars().take_while(|c| is_ident(*c)).collect();
            (!name.is_empty()).then_some(name)
        })
        .collect()
}

/// Whether code calls `name`: `name(` or `.name(` or `name::<`, as a whole word.
fn calls(code: &str, name: &str) -> bool {
    let mut from = 0;
    while let Some(at) = code[from..].find(name) {
        let start = from + at;
        let end = start + name.len();
        from = end;
        if code[..start].chars().next_back().is_some_and(is_ident) {
            continue;
        }
        let after = code[end..].trim_start();
        if after.starts_with('(') || after.starts_with("::<") {
            return true;
        }
    }
    false
}

fn is_test_file(path: &str) -> bool {
    path.starts_with(TESTS_FOLDER) || path.contains("/tests/") || path.ends_with("/tests.rs")
}

fn findings(sources: &[Source]) -> Vec<Finding> {
    let text_of = |path: &str| {
        sources
            .iter()
            .find(|(each, _)| each == path)
            .map(|(_, text)| text.as_str())
    };
    let (Some(registry), Some(words)) = (text_of(REGISTRY), text_of(TOOL_WORDS)) else {
        return vec![Finding::general(format!(
            "The registry `{REGISTRY}` and the words of the tools `{TOOL_WORDS}` must both exist (ADR-0013)."
        ))];
    };
    let tools = tools_named(words);
    let entries = entries_of(registry);
    let mut found = Vec::new();
    found.extend(table_findings(sources, &tools, &entries));
    found.extend(preview_findings(sources, &tools, &entries));
    found.extend(second_registry_findings(sources, &tools));
    found.extend(designed_out_findings(sources, registry, &entries));
    found
}

/// Every tool once, every entry a tool, a use case that exists.
fn table_findings(sources: &[Source], tools: &[Tool], entries: &[Entry]) -> Vec<Finding> {
    let mut found = Vec::new();
    let defined: BTreeSet<String> = sources
        .iter()
        .filter(|(path, _)| path.starts_with(APP_FOLDER) && !is_test_file(path))
        .flat_map(|(_, text)| public_functions(text))
        .collect();
    for tool in tools {
        let count = entries
            .iter()
            .filter(|entry| entry.variant == tool.variant)
            .count();
        if count != 1 {
            found.push(Finding::at(
                REGISTRY,
                format!(
                    "The tool `{}` has {count} entries in the registry; it needs exactly one that says `reads` or `changes`, the use case and the schema of its input.",
                    tool.word
                ),
            ));
        }
    }
    for entry in entries {
        if !tools.iter().any(|tool| tool.variant == entry.variant) {
            found.push(Finding::at(
                REGISTRY,
                format!(
                    "The registry has an entry for `Tool::{}`, which `Tool` in {TOOL_WORDS} does not name.",
                    entry.variant
                ),
            ));
        }
        if !defined.contains(&entry.use_case) {
            found.push(Finding::at(
                REGISTRY,
                format!(
                    "The tool `Tool::{}` names the use case `{}`, which is no public function of `bb-app`.",
                    entry.variant, entry.use_case
                ),
            ));
        }
    }
    found
}

/// A test of the preview for each tool that changes.
fn preview_findings(sources: &[Source], tools: &[Tool], entries: &[Entry]) -> Vec<Finding> {
    let tests: String = sources
        .iter()
        .filter(|(path, _)| path.starts_with(TESTS_FOLDER))
        .map(|(_, text)| text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    tools
        .iter()
        .filter(|tool| {
            entries
                .iter()
                .any(|entry| entry.changes && entry.variant == tool.variant)
        })
        .filter(|tool| !tests.contains(&format!("fn ai_21_preview_{}", tool.word)))
        .map(|tool| {
            Finding::at(
                REGISTRY,
                format!(
                    "The tool `{}` changes something and has no test of its preview. Add a test named `ai_21_preview_{}_…` in {TESTS_FOLDER}.",
                    tool.word, tool.word
                ),
            )
        })
        .collect()
}

/// No file but the registry and the words builds a tool.
fn second_registry_findings(sources: &[Source], tools: &[Tool]) -> Vec<Finding> {
    let mut found = Vec::new();
    for (path, text) in sources {
        if path == REGISTRY || path == TOOL_WORDS || is_test_file(path) {
            continue;
        }
        let code = strip_comments(&without_tests(text));
        if code.contains("ToolSpec {") || code.contains("ToolSpec::") {
            found.push(Finding::at(
                path,
                "A tool is built here. Tools are made only in the registry (`registry.rs`); add an entry there.",
            ));
        }
        for tool in tools {
            if code.contains(&format!("\"{}\"", tool.word)) {
                found.push(Finding::at(
                    path,
                    format!(
                        "The tool name `{}` is written as a string here. Take it from `Tool` and call the tool through the registry.",
                        tool.word
                    ),
                ));
            }
        }
    }
    found
}

/// No tool, and nothing that lists or calls tools, reaches a use case that is designed out.
fn designed_out_findings(sources: &[Source], registry: &str, entries: &[Entry]) -> Vec<Finding> {
    let mut forbidden: BTreeSet<String> = designed_out_of(registry).into_iter().collect();
    for (path, text) in sources {
        if DESIGNED_OUT_FOLDERS
            .iter()
            .any(|folder| path.starts_with(folder))
        {
            forbidden.extend(public_functions(text));
        }
    }
    let mut found = Vec::new();
    for entry in entries {
        if forbidden.contains(&entry.use_case) {
            found.push(Finding::at(
                REGISTRY,
                format!(
                    "The tool `Tool::{}` runs `{}`, a use case that is designed out: tools never read working hours, other people's time or roles, and never rank or evaluate people (COMP-10).",
                    entry.variant, entry.use_case
                ),
            ));
        }
    }
    for (path, text) in sources {
        let watched = path.starts_with(TOOLS_FOLDER)
            || OUTSIDE_FOLDERS
                .iter()
                .any(|folder| path.starts_with(folder));
        if !watched || is_test_file(path) {
            continue;
        }
        let code = strip_comments(&without_tests(text));
        let mut named: Vec<&str> = forbidden
            .iter()
            .map(String::as_str)
            .filter(|name| calls(&code, name))
            .collect();
        named.extend(
            DESIGNED_OUT_PATHS
                .iter()
                .filter(|path| code.contains(**path)),
        );
        for name in named {
            found.push(Finding::at(
                path,
                format!(
                    "`{name}` is designed out of the tools (COMP-10, ADR-0035 decision 9): a tool never reads working hours, other people's time or roles, and never ranks, evaluates or picks people."
                ),
            ));
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORDS: &str = "pub enum Tool {\n    /// Search.\n    SearchIssues => \"search_issues\",\n    /// Make.\n    CreateIssue => \"create_issue\",\n    }\n";
    const REGISTRY_TEXT: &str = "pub const DESIGNED_OUT: &[&str] = &[\"check_access\"];\npub const REGISTRY: [ToolSpec; 2] = [\n    reads(Tool::SearchIssues, \"search\", a),\n    changes(\n        Tool::CreateIssue,\n        \"create_issue\",\n        b,\n    ),\n];\n";

    fn source(path: &str, text: &str) -> Source {
        (path.to_owned(), text.to_owned())
    }

    fn good() -> Vec<Source> {
        vec![
            source(REGISTRY, REGISTRY_TEXT),
            source(TOOL_WORDS, WORDS),
            source(
                "crates/bb-app/src/search/run.rs",
                "pub async fn search() {}\npub async fn create_issue() {}",
            ),
            source(
                "crates/bb-app/src/tests/ai_tools.rs",
                "async fn ai_21_preview_create_issue_says_what_it_would_record() {}",
            ),
            source(
                "crates/bb-app/src/working_hours/read.rs",
                "pub async fn work_days() {}",
            ),
        ]
    }

    fn messages(sources: &[Source]) -> Vec<String> {
        findings(sources)
            .into_iter()
            .map(|finding| finding.message)
            .collect()
    }

    #[test]
    fn good_example_a_table_with_every_tool_once_and_a_test_of_each_preview() {
        assert_eq!(messages(&good()), Vec::<String>::new());
    }

    #[test]
    fn good_example_the_table_is_read_across_lines() {
        let entries = entries_of(REGISTRY_TEXT);
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().any(|entry| entry.changes
            && entry.variant == "CreateIssue"
            && entry.use_case == "create_issue"));
    }

    #[test]
    fn bad_example_a_tool_that_is_not_in_the_table() {
        let mut sources = good();
        sources[0] = source(
            REGISTRY,
            "pub const DESIGNED_OUT: &[&str] = &[];\nreads(Tool::SearchIssues, \"search\", a),",
        );
        let found = messages(&sources);
        assert!(
            found
                .iter()
                .any(|text| text.contains("`create_issue` has 0"))
        );
    }

    #[test]
    fn bad_example_a_tool_that_changes_without_a_test_of_its_preview() {
        let mut sources = good();
        sources[3] = source("crates/bb-app/src/tests/ai_tools.rs", "fn other() {}");
        let found = messages(&sources);
        assert!(
            found
                .iter()
                .any(|text| text.contains("no test of its preview"))
        );
    }

    #[test]
    fn bad_example_a_tool_built_outside_the_registry() {
        let mut sources = good();
        sources.push(source(
            "crates/bb-mcp/src/lib.rs",
            "fn mine() { let own = ToolSpec { tool: 1 }; call(\"search_issues\"); }",
        ));
        let found = messages(&sources);
        assert!(
            found
                .iter()
                .any(|text| text.contains("A tool is built here"))
        );
        assert!(
            found
                .iter()
                .any(|text| text.contains("`search_issues` is written"))
        );
    }

    #[test]
    fn bad_example_a_tool_that_runs_a_use_case_of_working_hours() {
        let mut sources = good();
        sources[0] = source(
            REGISTRY,
            "pub const DESIGNED_OUT: &[&str] = &[];\nreads(Tool::SearchIssues, \"work_days\", a),\nchanges(Tool::CreateIssue, \"create_issue\", b),",
        );
        let found = messages(&sources);
        assert!(found.iter().any(|text| text.contains("designed out")));
    }

    #[test]
    fn bad_example_a_tool_file_that_calls_a_designed_out_use_case() {
        let mut sources = good();
        sources.push(source(
            "crates/bb-app/src/ai/tools/read.rs",
            "async fn x() { check_access(ctx, q).await; self.work_days(q); }",
        ));
        sources.push(source(
            "crates/bb-mcp/src/lib.rs",
            "use bb_app::roles::roles;",
        ));
        let found = messages(&sources);
        assert!(found.iter().any(|text| text.contains("`check_access`")));
        assert!(found.iter().any(|text| text.contains("`work_days`")));
        assert!(found.iter().any(|text| text.contains("`bb_app::roles`")));
    }

    #[test]
    fn good_example_a_comment_and_a_test_module_do_not_count() {
        let mut sources = good();
        sources.push(source(
            "crates/bb-app/src/ai/tools/read.rs",
            "// check_access(ctx)\nfn x() {}\n#[cfg(test)]\nmod tests { fn y() { work_days(1); } }",
        ));
        assert_eq!(messages(&sources), Vec::<String>::new());
    }
}
