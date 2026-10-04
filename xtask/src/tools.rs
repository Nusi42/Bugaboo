//! The external tools (`tools/dev/tools.toml`): where they live and how checks run them.
//!
//! Tools are installed by `cargo xtask setup` into `.tools/bin` (ADR-0014, ADR-0024). Checks find
//! them only there (or in `node_modules/.bin` for npm tools) and never fall back to the PATH, so
//! every machine runs the pinned versions.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Context as _;
use serde::Deserialize;

use crate::checks::Finding;
use crate::repo::Repo;

/// The tool list, relative to the repository root.
pub(crate) const TOOLS_PATH: &str = "tools/dev/tools.toml";

/// Where setup puts tools, relative to the repository root.
pub(crate) const TOOLS_DIR: &str = ".tools";

#[derive(Deserialize)]
struct File {
    tool: Vec<Tool>,
}

/// One pinned tool.
#[derive(Deserialize, Clone, Debug)]
pub(crate) struct Tool {
    pub(crate) name: String,
    pub(crate) kind: String,
    #[serde(rename = "crate")]
    pub(crate) crate_name: Option<String>,
    pub(crate) version: String,
    #[serde(rename = "windows-x64")]
    pub(crate) windows_x64: Option<Download>,
    #[serde(rename = "linux-x64")]
    pub(crate) linux_x64: Option<Download>,
}

/// A prebuilt archive and the checksum it must have.
#[derive(Deserialize, Clone, Debug)]
pub(crate) struct Download {
    pub(crate) url: String,
    pub(crate) sha256: String,
}

impl Tool {
    /// The archive for the platform xtask runs on.
    pub(crate) fn download(&self) -> anyhow::Result<&Download> {
        let (platform, entry) = if cfg!(windows) {
            ("windows-x64", &self.windows_x64)
        } else {
            ("linux-x64", &self.linux_x64)
        };
        entry.as_ref().with_context(|| {
            format!(
                "{} has no download for {platform} in {TOOLS_PATH}. Add one with its sha256.",
                self.name
            )
        })
    }
}

/// Reads the pinned tool list.
pub(crate) fn load(repo: &Repo) -> anyhow::Result<Vec<Tool>> {
    let path = repo.path(TOOLS_PATH);
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("could not read {}", path.display()))?;
    parse(&text).with_context(|| format!("{TOOLS_PATH} is not valid"))
}

pub(crate) fn parse(text: &str) -> anyhow::Result<Vec<Tool>> {
    let file: File = toml::from_str(text)?;
    Ok(file.tool)
}

/// The repository that owns this xtask and its `.tools` folder.
///
/// Checks that run on a copy of the repository (the examples in the tests) still use these tools.
fn home() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

/// The folder setup installs binaries into.
pub(crate) fn bin_dir(root: &Path) -> PathBuf {
    root.join(TOOLS_DIR).join("bin")
}

fn exe(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    }
}

/// Finds an installed tool, or says how to install it.
pub(crate) fn find(name: &str) -> Result<PathBuf, Finding> {
    let root = home();
    let npm_name = if cfg!(windows) {
        format!("{name}.cmd")
    } else {
        name.to_owned()
    };
    [
        bin_dir(&root).join(exe(name)),
        root.join("node_modules").join(".bin").join(npm_name),
    ]
    .into_iter()
    .find(|candidate| candidate.is_file())
    .ok_or_else(|| {
        Finding::general(format!(
            "The tool `{name}` is not installed. Run `cargo xtask setup` first."
        ))
    })
}

/// What a finished tool run printed.
pub(crate) struct Output {
    pub(crate) success: bool,
    /// Standard output and standard error together, trimmed.
    pub(crate) text: String,
}

/// Runs a program in `cwd` and collects its output.
pub(crate) fn run(program: &Path, args: &[&str], cwd: &Path) -> anyhow::Result<Output> {
    let out = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .with_context(|| format!("could not start {}", program.display()))?;
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    Ok(Output {
        success: out.status.success(),
        text: text.trim().to_owned(),
    })
}

/// Runs an installed tool in `cwd`; a missing tool is a finding, never a silent skip.
pub(crate) fn run_installed(
    name: &str,
    args: &[&str],
    cwd: &Path,
) -> anyhow::Result<Result<Output, Finding>> {
    match find(name) {
        Ok(program) => Ok(Ok(run(&program, args, cwd)?)),
        Err(finding) => Ok(Err(finding)),
    }
}

/// Turns a failed tool run into findings: one finding with the tool's own words.
pub(crate) fn findings_from(run: Result<Output, Finding>, fix: &str) -> Vec<Finding> {
    match run {
        Err(missing) => vec![missing],
        Ok(output) if output.success => Vec::new(),
        Ok(output) => {
            let mut message = String::from(fix);
            message.push('\n');
            message.push_str(&shorten(&output.text, 60));
            vec![Finding::general(message)]
        }
    }
}

/// Keeps at most `lines` lines of tool output so a failure stays readable.
fn shorten(text: &str, lines: usize) -> String {
    let all: Vec<&str> = text.lines().collect();
    let mut out = String::new();
    for line in all.iter().take(lines) {
        let _ = writeln!(out, "        {line}");
    }
    if all.len() > lines {
        let _ = writeln!(out, "        … and {} more lines.", all.len() - lines);
    }
    out.trim_end().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_tool_list_parses_and_has_downloads_for_both_platforms() {
        let text = include_str!("../../tools/dev/tools.toml");
        let tools = parse(text).unwrap();
        assert!(tools.iter().any(|tool| tool.name == "taplo"));
        for tool in tools.iter().filter(|tool| tool.kind == "download") {
            assert!(
                tool.windows_x64.is_some(),
                "{} lacks windows-x64",
                tool.name
            );
            assert!(tool.linux_x64.is_some(), "{} lacks linux-x64", tool.name);
        }
    }

    #[test]
    fn missing_tool_names_the_fix() {
        let finding = find("no-such-tool-anywhere").err().unwrap();
        assert!(finding.message.contains("cargo xtask setup"));
    }

    #[test]
    fn shorten_cuts_long_output() {
        let long = (0..100)
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(shorten(&long, 3).contains("97 more lines"));
    }
}
