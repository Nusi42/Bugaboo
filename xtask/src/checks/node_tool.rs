//! Shared helpers for checks that run the pinned npm tools (oxfmt, oxlint, tsc, stylelint).
//!
//! The tools come from the root `package.json` / `package-lock.json` and live in
//! `node_modules/.bin` of the work folder (ADR-0014, ADR-0024). `npm ci` installs them.

use anyhow::anyhow;
use std::path::Path;

use super::Finding;
use crate::tools;

/// What a tool printed and whether it succeeded.
pub(crate) struct ToolRun {
    pub(crate) success: bool,
    pub(crate) output: String,
}

/// Runs an installed npm tool in `root`. The tool is found by [`tools::find`], which looks in
/// `.tools/bin` and in `node_modules/.bin`.
pub(crate) fn run(root: &Path, tool: &str, args: &[&str]) -> anyhow::Result<ToolRun> {
    let program = tools::find(tool).map_err(|finding| anyhow!(finding.message))?;
    let out = tools::run(&program, args, root)?;
    Ok(ToolRun {
        success: out.success,
        output: out.text,
    })
}

/// Turns `file:line:col: message [rule]` lines (oxlint and stylelint `unix` format) into findings.
pub(crate) fn unix_findings(run: &ToolRun) -> Vec<Finding> {
    let mut findings: Vec<Finding> = run.output.lines().filter_map(parse_unix_line).collect();
    if findings.is_empty() && !run.success {
        findings.push(Finding::general(format!(
            "The tool failed without naming a problem:\n{}",
            run.output.trim()
        )));
    }
    findings
}

fn parse_unix_line(line: &str) -> Option<Finding> {
    for (index, _) in line.match_indices(':') {
        let after = &line[index + 1..];
        let Some((row, after_row)) = take_digits(after) else {
            continue;
        };
        let Some((_, message)) = after_row
            .strip_prefix(':')
            .and_then(take_digits)
            .and_then(|(column, rest)| rest.strip_prefix(':').map(|rest| (column, rest)))
        else {
            continue;
        };
        return Some(Finding::at(
            format!("{}:{row}", &line[..index]),
            message.trim().to_owned(),
        ));
    }
    None
}

fn take_digits(text: &str) -> Option<(&str, &str)> {
    let end = text
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(text.len());
    (end > 0).then(|| text.split_at(end))
}

/// The most characters of file names one call of a tool gets: the npm tools are `.cmd` files, whose
/// command line may hold 8191 characters, and a whole repository of files does not fit in that.
const BATCH_CHARS: usize = 6_000;

/// The files split into batches that each fit on one command line.
pub(crate) fn in_batches(files: &[String]) -> Vec<&[String]> {
    let mut batches = Vec::new();
    let mut start = 0;
    let mut length = 0;
    for (index, file) in files.iter().enumerate() {
        if length + file.len() + 1 > BATCH_CHARS && index > start {
            batches.push(&files[start..index]);
            start = index;
            length = 0;
        }
        length += file.len() + 1;
    }
    if start < files.len() {
        batches.push(&files[start..]);
    }
    batches
}

/// Runs oxfmt in check mode and reports every file that is not formatted.
pub(crate) fn unformatted(root: &Path, files: &[String]) -> anyhow::Result<Vec<Finding>> {
    let mut findings = Vec::new();
    for batch in in_batches(files) {
        findings.extend(unformatted_batch(root, batch)?);
    }
    Ok(findings)
}

fn unformatted_batch(root: &Path, files: &[String]) -> anyhow::Result<Vec<Finding>> {
    let mut args = vec!["--list-different", "--config", ".oxfmtrc.json"];
    args.extend(files.iter().map(String::as_str));
    let result = run(root, "oxfmt", &args)?;
    let mut findings: Vec<Finding> = result
        .output
        .lines()
        .map(|line| line.trim().replace(std::path::MAIN_SEPARATOR, "/"))
        .filter(|line| files.contains(line))
        .map(|line| {
            Finding::at(
                line,
                "Not formatted. Run `npx oxfmt --config .oxfmtrc.json <file>` to fix it.",
            )
        })
        .collect();
    if findings.is_empty() && !result.success {
        findings.push(Finding::general(format!(
            "oxfmt failed:\n{}",
            result.output.trim()
        )));
    }
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_long_list_of_files_goes_to_the_tool_in_batches_that_keep_every_file() {
        let files: Vec<String> = (0..2000)
            .map(|number| format!("web/app/src/features/x/File{number}.tsx"))
            .collect();
        let batches = in_batches(&files);
        assert!(batches.len() > 1);
        assert!(
            batches
                .iter()
                .all(|batch| batch.iter().map(|file| file.len() + 1).sum::<usize>() <= BATCH_CHARS)
        );
        assert_eq!(batches.concat(), files);
    }

    #[test]
    fn a_short_list_is_one_batch_and_nothing_is_none() {
        let files = vec!["a.ts".to_owned(), "b.ts".to_owned()];
        assert_eq!(in_batches(&files).len(), 1);
        assert!(in_batches(&[]).is_empty());
    }
}
