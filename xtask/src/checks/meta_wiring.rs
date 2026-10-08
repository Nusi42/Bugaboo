//! META-WIRING: the git hooks, the Claude Code hooks and CI call only `cargo xtask …`, and call
//! exactly the commands they should (ADR-0014).
//!
//! The files do not have to exist yet: a missing file passes.

use super::{Check, Context, Finding};
use crate::docs::source::sorted_files;
use crate::docs::text::has_extension;
use crate::source::read_text;

const LEFTHOOK_FILES: [&str; 2] = ["lefthook.yml", "lefthook.yaml"];
const CLAUDE_SETTINGS: &str = ".claude/settings.json";
const WORKFLOWS: &str = ".github/workflows";

/// Which command each Git hook runs.
const GIT_HOOKS: [(&str, &str); 3] = [
    ("pre-commit", "cargo xtask check staged"),
    ("pre-push", "cargo xtask check push"),
    ("commit-msg", "cargo xtask check commit-msg {1}"),
];

/// The only command the Claude Code `PostToolUse` hook runs. The session (and every agent it
/// starts) may run in the master repository, so the command decides from the EDITED FILE, not
/// from `$CLAUDE_PROJECT_DIR`: it finds the nearest folder above the file that has `.git` and
/// `xtask/Cargo.toml`, skips files in OneDrive and files outside any work folder, and runs that
/// work folder's own `cargo xtask hook post-edit` (ADR-0024). It needs no build before the
/// OneDrive test, so nothing is ever compiled in the master.
const POST_EDIT: &str = r#"in=$(cat); f=$(printf '%s' "$in" | sed -n 's/.*"\(file_path\|notebook_path\)" *: *"\([^"]*\)".*/\2/p' | head -n 1 | tr '\134' '/' | sed 's#//*#/#g'); [ -n "$f" ] || exit 0; d=$(dirname "$f"); while [ ! -f "$d/xtask/Cargo.toml" ] || [ ! -e "$d/.git" ]; do p=$(dirname "$d"); [ "$p" = "$d" ] && exit 0; d=$p; done; case "$d" in *OneDrive*) exit 0;; esac; cd "$d" && printf '%s' "$in" | cargo xtask hook post-edit"#;

/// The tools whose use triggers the `PostToolUse` hook.
const POST_EDIT_MATCHER: &str = "Edit|Write|MultiEdit|NotebookEdit";

/// Any other Claude Code hook may run an xtask hook command.
const HOOK_PREFIX: &str = "cargo xtask hook ";

/// The only commands CI runs.
const CI_COMMANDS: [&str; 2] = ["cargo xtask setup", "cargo xtask ci"];

/// Length of a full Git commit SHA, which actions have to be pinned to.
const SHA_LENGTH: usize = 40;

pub(crate) struct MetaWiring;

impl Check for MetaWiring {
    fn id(&self) -> &'static str {
        "META-WIRING"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let repo = context.repo;
        let mut findings = Vec::new();
        for file in LEFTHOOK_FILES {
            if repo.path(file).is_file() {
                findings.extend(check_lefthook(file, &read_text(repo, file)?));
            }
        }
        if repo.path(CLAUDE_SETTINGS).is_file() {
            findings.extend(check_settings(
                CLAUDE_SETTINGS,
                &read_text(repo, CLAUDE_SETTINGS)?,
            ));
        }
        if repo.path(WORKFLOWS).is_dir() {
            let is_workflow =
                |name: &str| has_extension(name, "yml") || has_extension(name, "yaml");
            for name in sorted_files(repo, WORKFLOWS, is_workflow)? {
                let path = format!("{WORKFLOWS}/{name}");
                findings.extend(check_workflow(&path, &read_text(repo, &path)?));
            }
        }
        Ok(findings)
    }
}

/// Splits a shell command at `&&`, `||`, `|`, `&` and `;` into its commands.
fn segments(command: &str) -> Vec<String> {
    command
        .replace("&&", "\n")
        .replace("||", "\n")
        .replace(['|', '&', ';'], "\n")
        .lines()
        .map(|segment| segment.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|segment| !segment.is_empty() && !segment.starts_with('#'))
        .collect()
}

fn wrong(path: &str, place: &str, command: &str, expected: &str) -> Finding {
    Finding::at(
        format!("{path}{place}"),
        format!(
            "`{command}` is called here, but only `{expected}` may be. Hooks and CI call xtask and nothing else; put the work into xtask and the catalogue."
        ),
    )
}

/// Every command in `command` must be one of `allowed` (exact match).
fn check_exact(path: &str, place: &str, command: &str, allowed: &[&str]) -> Vec<Finding> {
    segments(command)
        .into_iter()
        .filter(|segment| !allowed.contains(&segment.as_str()))
        .map(|segment| wrong(path, place, &segment, &allowed.join("` or `")))
        .collect()
}

/// A `run:` value of a YAML file, with its 1-based line number and the top-level key it is under.
struct Run {
    line: usize,
    section: String,
    command: String,
}

/// The `run:` values of a YAML file, including block scalars (`run: |`), and the other lines of
/// interest as `(line, key, value)`.
fn yaml_runs(text: &str, key: &str) -> Vec<Run> {
    let lines: Vec<&str> = text.lines().collect();
    let mut runs = Vec::new();
    let mut section = String::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        index += 1;
        if indent == 0
            && let Some(name) = line.trim_end().strip_suffix(':')
        {
            name.clone_into(&mut section);
        }
        let body = trimmed.trim_start_matches("- ");
        let Some(value) = body.strip_prefix(key).and_then(|v| v.strip_prefix(':')) else {
            continue;
        };
        let (value, number) = (value.trim(), index);
        if value.starts_with('|') || value.starts_with('>') {
            let mut block = Vec::new();
            while let Some(next) = lines.get(index) {
                let next_indent = next.len() - next.trim_start().len();
                if !next.trim().is_empty() && next_indent <= indent {
                    break;
                }
                block.push(*next);
                index += 1;
            }
            runs.push(Run {
                line: number,
                section: section.clone(),
                command: block.join("\n"),
            });
        } else {
            runs.push(Run {
                line: number,
                section: section.clone(),
                command: value.trim_matches(['"', '\'']).to_owned(),
            });
        }
    }
    runs
}

/// lefthook: each Git hook runs exactly its command, and runs no script files.
fn check_lefthook(path: &str, text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    for run in yaml_runs(text, "run") {
        let place = format!(":{}", run.line);
        match GIT_HOOKS.iter().find(|(hook, _)| *hook == run.section) {
            Some((_, command)) => {
                findings.extend(check_exact(path, &place, &run.command, &[command]));
            }
            None => findings.push(Finding::at(
                format!("{path}{place}"),
                format!("The hook `{}` is not one of the wired hooks.", run.section),
            )),
        }
    }
    for script in yaml_runs(text, "script") {
        findings.push(Finding::at(
            format!("{path}:{}", script.line),
            "A script file is run here. Hooks may only run `cargo xtask …` commands.",
        ));
    }
    if text.lines().any(|line| line.trim() == "scripts:") {
        findings.push(Finding::at(
            path,
            "`scripts:` runs script files. Hooks may only run `cargo xtask …` commands.",
        ));
    }
    for (hook, command) in GIT_HOOKS {
        if !yaml_runs(text, "run").iter().any(|run| run.section == hook) {
            findings.push(Finding::at(
                path,
                format!("The hook `{hook}` is missing. It must run `{command}`."),
            ));
        }
    }
    findings
}

/// Workflows: `run:` is only `cargo xtask setup` and `cargo xtask ci`; `uses:` only for actions
/// pinned to a full commit SHA.
fn check_workflow(path: &str, text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    for run in yaml_runs(text, "run") {
        findings.extend(check_exact(
            path,
            &format!(":{}", run.line),
            &run.command,
            &CI_COMMANDS,
        ));
    }
    for used in yaml_runs(text, "uses") {
        let action = used.command.split('#').next().unwrap_or("").trim();
        let pinned = action.rsplit_once('@').is_some_and(|(_, version)| {
            version.len() == SHA_LENGTH && version.bytes().all(|b| b.is_ascii_hexdigit())
        });
        if !pinned {
            findings.push(Finding::at(
                format!("{path}:{}", used.line),
                format!("`{action}` is not pinned to a full {SHA_LENGTH}-character commit SHA. Use `owner/action@<sha>`."),
            ));
        }
    }
    findings
}

/// Claude Code settings: hook commands are xtask hooks, and `PostToolUse` runs exactly
/// `cargo xtask hook post-edit`.
fn check_settings(path: &str, text: &str) -> Vec<Finding> {
    let Ok(settings) = serde_json::from_str::<serde_json::Value>(text) else {
        return vec![Finding::at(path, "This file is not valid JSON.")];
    };
    let mut findings = Vec::new();
    let Some(hooks) = settings.get("hooks").and_then(serde_json::Value::as_object) else {
        return findings;
    };
    for (event, entries) in hooks {
        let place = format!(" ({event})");
        if event == "PostToolUse" {
            for entry in entries.as_array().into_iter().flatten() {
                let matcher = entry.get("matcher").and_then(serde_json::Value::as_str);
                if matcher != Some(POST_EDIT_MATCHER) {
                    findings.push(Finding::at(
                        format!("{path}{place}"),
                        format!("The matcher must be `{POST_EDIT_MATCHER}`."),
                    ));
                }
                let mut commands = Vec::new();
                collect_commands(entry, &mut commands);
                for command in commands {
                    if one_line(&command) != one_line(POST_EDIT) {
                        findings.push(wrong(path, &place, &command, POST_EDIT));
                    }
                }
            }
            continue;
        }
        let mut commands = Vec::new();
        collect_commands(entries, &mut commands);
        for command in commands {
            findings.extend(
                segments(&command)
                    .into_iter()
                    .filter(|segment| !segment.starts_with(HOOK_PREFIX))
                    .map(|segment| wrong(path, &place, &segment, &format!("{HOOK_PREFIX}…"))),
            );
        }
    }
    findings
}

/// A command with its white space collapsed.
fn one_line(command: &str) -> String {
    command.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn collect_commands(value: &serde_json::Value, found: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, inner) in map {
                match (key.as_str(), inner.as_str()) {
                    ("command", Some(command)) => found.push(command.to_owned()),
                    _ => collect_commands(inner, found),
                }
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_commands(item, found);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA: &str = "11bd71901bbe5b1630ceea73d27597364c9af683";

    const LEFTHOOK: &str = "\
pre-commit:
  commands:
    check:
      run: cargo xtask check staged
pre-push:
  commands:
    check:
      run: cargo xtask check push
commit-msg:
  commands:
    check:
      run: cargo xtask check commit-msg {1}
";

    fn settings(matcher: &str, command: &str) -> String {
        serde_json::json!({"hooks": {"PostToolUse": [{"matcher": matcher, "hooks": [{"type": "command", "command": command}]}]}}).to_string()
    }

    #[test]
    fn good_example() {
        let workflow = format!(
            "jobs:\n  ci:\n    steps:\n      - uses: actions/checkout@{SHA} # v4\n      - run: |\n          cargo xtask setup\n          cargo xtask ci\n"
        );
        let settings = settings(POST_EDIT_MATCHER, POST_EDIT);
        assert!(check_lefthook("lefthook.yml", LEFTHOOK).is_empty());
        assert!(check_workflow("ci.yml", &workflow).is_empty());
        assert!(check_settings("settings.json", &settings).is_empty());
    }

    #[test]
    fn bad_example() {
        let lefthook = LEFTHOOK
            .replace("check staged", "check push")
            .replace(
                "cargo xtask check commit-msg",
                "cargo fmt --check && cargo xtask check commit-msg",
            )
            .replace("pre-push:", "post-merge:");
        let found = check_lefthook("lefthook.yml", &lefthook);
        let text: Vec<&str> = found.iter().map(|f| f.message.as_str()).collect();
        let has = |needle: &str| text.iter().any(|m| m.contains(needle));
        assert!(has(
            "`cargo xtask check push` is called here, but only `cargo xtask check staged`"
        ));
        assert!(has("`cargo fmt --check` is called here"));
        assert!(has("`post-merge` is not one of the wired hooks"));
        assert!(has("The hook `pre-push` is missing"));
    }

    #[test]
    fn scripts_are_not_allowed_in_lefthook() {
        let found = check_lefthook(
            "lefthook.yml",
            &format!(
                "{LEFTHOOK}pre-commit-extra:\n  scripts:\n    \"x.sh\":\n      runner: bash\n"
            ),
        );
        assert!(
            found
                .iter()
                .any(|f| f.message.contains("`scripts:` runs script files"))
        );
    }

    #[test]
    fn workflows_run_only_setup_and_ci_and_pin_actions() {
        let workflow = "steps:\n  - uses: actions/checkout@v4\n  - run: |\n      cargo xtask ci\n      npm test\n  - run: cargo xtask setup | tee log & rustup update\n";
        let found = check_workflow("ci.yml", workflow);
        assert_eq!(found.len(), 4);
        assert!(found[0].message.contains("`npm test`"));
        assert!(found[1].message.contains("`tee log`"));
        assert!(found[2].message.contains("`rustup update`"));
        assert!(found[3].message.contains("not pinned"));
    }

    #[test]
    fn claude_hooks_are_exact() {
        let wrong_command = settings(POST_EDIT_MATCHER, "cargo xtask hook post-edit");
        let found = check_settings("settings.json", &wrong_command);
        assert_eq!(found.len(), 1);
        assert!(found[0].message.contains("only `in=$(cat)"));
        let wrong_matcher = settings("Edit", POST_EDIT);
        let found = check_settings("settings.json", &wrong_matcher);
        assert_eq!(found.len(), 1);
        assert!(found[0].message.contains("The matcher must be"));
        let other =
            r#"{"hooks":{"SessionStart":[{"hooks":[{"command":"cargo xtask hook start; ls"}]}]}}"#;
        let found = check_settings("settings.json", other);
        assert_eq!(found.len(), 1);
        assert!(found[0].message.contains("`ls`"));
    }
}
