//! `cargo xtask hook post-edit`: the Claude Code `PostToolUse` hook.
//!
//! Claude Code passes the tool call as JSON on stdin (`tool_input.file_path`, or `notebook_path` for notebooks). We run the edit
//! checks for that file; on failure the short report goes to stderr with exit code 2, which
//! Claude Code feeds back to the agent.

use std::path::Path;
use std::process::{Command, ExitCode};

use anyhow::Context as _;
use serde::Deserialize;

use crate::checks;
use crate::output;

/// Exit code that makes Claude Code show stderr to the agent.
const FEEDBACK_EXIT_CODE: u8 = 2;
/// Keeps the agent's context small.
const MAX_REPORT_LINES: usize = 20;

#[derive(Deserialize)]
struct HookInput {
    tool_input: Option<ToolInput>,
}

#[derive(Deserialize)]
struct ToolInput {
    file_path: Option<String>,
    /// `NotebookEdit` names its file this way.
    notebook_path: Option<String>,
}

/// The file the agent just edited, if the hook input names one.
pub(crate) fn edited_file(stdin: &str) -> anyhow::Result<Option<String>> {
    let input: HookInput = serde_json::from_str(stdin).context("the hook input is not JSON")?;
    Ok(input
        .tool_input
        .and_then(|tool| tool.file_path.or(tool.notebook_path)))
}

/// Keeps what went wrong: drops passing checks and cuts the rest short.
pub(crate) fn short_report(stdout: &str, stderr: &str) -> String {
    let lines: Vec<&str> = stdout
        .lines()
        .chain(stderr.lines())
        .filter(|line| !line.trim().is_empty() && !line.starts_with("ok "))
        .collect();
    let mut report = lines
        .iter()
        .take(MAX_REPORT_LINES)
        .copied()
        .collect::<Vec<_>>()
        .join("\n");
    if lines.len() > MAX_REPORT_LINES {
        report.push_str("\n...");
    }
    report
}

pub(crate) fn post_edit(root: &Path, stdin: &str) -> anyhow::Result<ExitCode> {
    let exe = std::env::current_exe().context("could not find the running xtask program")?;
    post_edit_with(root, stdin, &exe)
}

/// The hook with the program that runs the edit checks given: the tests give one that does
/// nothing of the kind.
fn post_edit_with(root: &Path, stdin: &str, exe: &Path) -> anyhow::Result<ExitCode> {
    let Some(file) = edited_file(stdin)? else {
        return Ok(ExitCode::SUCCESS);
    };
    // Claude Code adds a session line to every memory note it saves; it must not reach Git. A
    // note that cannot be cleaned is reported, and the edit checks still run.
    if let Err(error) = checks::sec_private::strip_generated_keys(&root.join(&file)) {
        output::error(&format!("{error:#}"));
    }
    // The checks run in a child process: it prints for people, and we turn that into feedback.
    let out = Command::new(exe)
        .args(["check", "edit", &file])
        .current_dir(root)
        .output()
        .context("could not run the edit checks")?;
    if out.status.success() {
        return Ok(ExitCode::SUCCESS);
    }
    let report = short_report(
        &String::from_utf8_lossy(&out.stdout),
        &String::from_utf8_lossy(&out.stderr),
    );
    output::hook_feedback(&report);
    Ok(ExitCode::from(FEEDBACK_EXIT_CODE))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EDIT_INPUT: &str = r#"{"session_id":"s","hook_event_name":"PostToolUse","tool_name":"Edit",
        "tool_input":{"file_path":"C:\\work\\a.rs","old_string":"a","new_string":"b"},"tool_response":{}}"#;

    #[test]
    fn the_file_path_comes_from_tool_input() {
        assert_eq!(
            edited_file(EDIT_INPUT).unwrap().as_deref(),
            Some("C:\\work\\a.rs")
        );
    }

    #[test]
    fn notebook_edits_name_their_file_as_notebook_path() {
        let input = r#"{"tool_input":{"notebook_path":"/w/n.ipynb"}}"#;
        assert_eq!(edited_file(input).unwrap().as_deref(), Some("/w/n.ipynb"));
    }

    #[test]
    fn input_without_a_file_path_is_not_an_error() {
        assert_eq!(edited_file(r#"{"tool_input":{}}"#).unwrap(), None);
        assert_eq!(edited_file(r"{}").unwrap(), None);
    }

    #[test]
    fn input_that_is_not_json_is_an_error() {
        assert!(edited_file("not json").is_err());
    }

    #[test]
    fn the_report_drops_passing_checks_and_is_cut_short() {
        let stdout = "ok    A   fine\nFAIL  B   broken\n        x.rs: fix it\n";
        assert_eq!(
            short_report(stdout, ""),
            "FAIL  B   broken\n        x.rs: fix it"
        );

        let many = "FAIL line\n".repeat(50);
        let report = short_report(&many, "");
        assert_eq!(report.lines().count(), MAX_REPORT_LINES + 1);
        assert!(report.ends_with("..."));
    }

    /// The hook input for an edit of `file`.
    fn input_for(file: &Path) -> String {
        serde_json::json!({ "tool_input": { "file_path": file } }).to_string()
    }

    #[test]
    fn after_an_edit_of_a_memory_note_its_session_line_is_gone() {
        let temp = crate::testutil::TempDir::new("hook-memory");
        let key = checks::sec_private::MEMORY_SESSION_KEY;
        let note = temp.write(
            ".claude/memory/note.md",
            format!("---\nname: n\n{key}: 0123456789abcdef\n---\nBody\n"),
        );
        // `git` stands in for the edit checks: what it answers does not matter here.
        post_edit_with(temp.path(), &input_for(&note), Path::new("git")).unwrap();
        assert_eq!(
            std::fs::read_to_string(&note).unwrap(),
            "---\nname: n\n---\nBody\n"
        );
    }

    #[test]
    fn a_note_that_cannot_be_cleaned_does_not_stop_the_edit_checks() {
        let temp = crate::testutil::TempDir::new("hook-memory-broken");
        // Not text: reading it fails.
        let note = temp.write(".claude/memory/note.md", b"\xFF\xFE\xFD");
        assert!(post_edit_with(temp.path(), &input_for(&note), Path::new("git")).is_ok());
    }
}
