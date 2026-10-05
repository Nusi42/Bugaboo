//! Running a list of commands one after the other (used by `dev`, `build` and `ci`), and the
//! commands of the browser app and the server that more than one of them needs. The browser app
//! (`web/app`) may not exist yet; its steps are then skipped with a clear note instead of failing.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::Instant;

use anyhow::Context as _;
use serde_json::Value;

use crate::output;

/// The workspace folder of the browser app, relative to the repository root.
pub(crate) const WEB_APP: &str = "web/app";
/// The name of the server program that `cargo build` makes (`[[bin]]` of bb-server).
const SERVER_BINARY: &str = "bugaboo";
/// The name of the sample program that `cargo build` makes (`[[bin]]` of bb-sample).
const SAMPLE_BINARY: &str = "bb-sample";

/// Which cargo profile to build.
#[derive(Clone, Copy)]
pub(crate) enum Profile {
    Debug,
    Release,
}

/// One command of a plan.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Step {
    pub(crate) name: &'static str,
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    /// Why this step is skipped, if it is.
    pub(crate) skip: Option<&'static str>,
    /// For cargo builds: the program whose path to read from cargo's JSON messages.
    pub(crate) executable: Option<&'static str>,
}

/// What running steps produced.
pub(crate) struct Outcome {
    pub(crate) passed: bool,
    /// The path cargo reported for the step's `executable`, if it asked for one.
    pub(crate) executable: Option<PathBuf>,
}

impl Step {
    pub(crate) fn new(name: &'static str, program: impl Into<String>, args: &[&str]) -> Self {
        Self {
            name,
            program: program.into(),
            args: args.iter().map(|arg| (*arg).to_owned()).collect(),
            skip: None,
            executable: None,
        }
    }

    /// A step of the browser app: skipped while `web/app` does not exist.
    pub(crate) fn web(root: &Path, name: &'static str, args: &[&str]) -> Self {
        let mut step = Self::new(name, npm(), args);
        if !web_app_exists(root) {
            step.skip = Some("web/app does not exist yet");
        }
        step
    }
}

/// Builds the browser app (`web/app/dist`).
pub(crate) fn web_build(root: &Path) -> Step {
    Step::web(root, "build the browser app", &["run", "build"])
}

/// Builds the `bugaboo` program. Cargo reports where it put it, so `build.target-dir` in
/// `.cargo/config.local.toml` or `CARGO_TARGET_DIR` (even relative) need no guessing.
pub(crate) fn server_build(profile: Profile) -> Step {
    program_build(
        profile,
        "build the bugaboo program",
        "bb-server",
        SERVER_BINARY,
    )
}

/// Builds the `bb-sample` program, which makes the sample organization (`cargo xtask dataset`).
pub(crate) fn sample_build(profile: Profile) -> Step {
    program_build(
        profile,
        "build the sample program",
        "bb-sample",
        SAMPLE_BINARY,
    )
}

fn program_build(
    profile: Profile,
    name: &'static str,
    package: &str,
    binary: &'static str,
) -> Step {
    let mut args = vec![
        "build",
        "--message-format=json-render-diagnostics",
        "--package",
        package,
    ];
    if matches!(profile, Profile::Release) {
        args.push("--release");
    }
    let mut step = Step::new(name, "cargo", &args);
    step.executable = Some(binary);
    step
}

/// `npm` is a `.cmd` file on Windows, which `Command` only finds under that name.
pub(crate) fn npm() -> &'static str {
    if cfg!(windows) { "npm.cmd" } else { "npm" }
}

pub(crate) fn web_app_exists(root: &Path) -> bool {
    root.join(WEB_APP).join("package.json").is_file()
}

pub(crate) fn exit_code(passed: bool) -> ExitCode {
    if passed {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// The path of the program `name` in cargo's `--message-format=json` output.
pub(crate) fn executable_from(cargo_output: &str, name: &str) -> Option<PathBuf> {
    cargo_output
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|message| message["reason"] == "compiler-artifact")
        .filter(|message| message["target"]["name"] == name)
        .filter_map(|message| message["executable"].as_str().map(PathBuf::from))
        .next_back()
}

/// Runs the steps in order and stops at the first failure.
pub(crate) fn run(root: &Path, steps: &[Step]) -> anyhow::Result<Outcome> {
    let mut outcome = Outcome {
        passed: true,
        executable: None,
    };
    let mut skipped = 0_usize;
    for (index, step) in steps.iter().enumerate() {
        if let Some(reason) = step.skip {
            output::step_skipped(step.name, reason);
            skipped += 1;
            continue;
        }
        output::step_start(index + 1, steps.len(), step.name);
        let started = Instant::now();
        let mut command = Command::new(&step.program);
        command.args(&step.args).current_dir(root);
        let passed = if let Some(name) = step.executable {
            // Cargo's messages are data for us; its readable diagnostics stay on stderr.
            let out = command
                .stderr(Stdio::inherit())
                .output()
                .with_context(|| start_error(step))?;
            outcome.executable = executable_from(&String::from_utf8_lossy(&out.stdout), name);
            out.status.success()
        } else {
            command
                .status()
                .with_context(|| start_error(step))?
                .success()
        };
        output::step_end(step.name, passed, started.elapsed());
        if !passed {
            outcome.passed = false;
            break;
        }
    }
    output::steps_summary(steps.len(), skipped, usize::from(!outcome.passed));
    Ok(outcome)
}

fn start_error(step: &Step) -> String {
    format!("could not start `{}` for \"{}\"", step.program, step.name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_steps_are_skipped_without_the_app_and_run_with_it() {
        let root = tempfile::tempdir().unwrap();
        assert!(web_build(root.path()).skip.is_some());

        std::fs::create_dir_all(root.path().join(WEB_APP)).unwrap();
        std::fs::write(root.path().join(WEB_APP).join("package.json"), "{}").unwrap();
        assert_eq!(web_build(root.path()).skip, None);
    }

    #[test]
    fn the_server_build_asks_cargo_where_the_program_is() {
        let debug = server_build(Profile::Debug);
        assert!(
            debug
                .args
                .contains(&"--message-format=json-render-diagnostics".to_owned())
        );
        assert!(!debug.args.contains(&"--release".to_owned()));
        assert_eq!(debug.executable, Some("bugaboo"));
        assert!(
            server_build(Profile::Release)
                .args
                .contains(&"--release".to_owned())
        );
    }

    #[test]
    fn the_sample_program_is_built_the_same_way() {
        let step = sample_build(Profile::Debug);
        assert!(step.args.contains(&"bb-sample".to_owned()));
        assert_eq!(step.executable, Some("bb-sample"));
        assert!(
            sample_build(Profile::Release)
                .args
                .contains(&"--release".to_owned())
        );
    }

    #[test]
    fn the_executable_comes_from_the_compiler_artifact_message() {
        let output = [
            r#"{"reason":"compiler-artifact","target":{"name":"bb-api","kind":["lib"]},"executable":null}"#,
            r#"{"reason":"compiler-artifact","target":{"name":"bugaboo","kind":["bin"]},"executable":"D:/custom target/debug/bugaboo.exe"}"#,
            r#"{"reason":"build-finished","success":true}"#,
            "not json at all",
        ]
        .join("\n");
        assert_eq!(
            executable_from(&output, "bugaboo"),
            Some(PathBuf::from("D:/custom target/debug/bugaboo.exe"))
        );
        assert_eq!(executable_from(&output, "other"), None);
        assert_eq!(executable_from("", "bugaboo"), None);
    }

    #[test]
    fn steps_run_in_order_and_stop_at_the_first_failure() {
        let root = tempfile::tempdir().unwrap();
        let cargo = std::env::var("CARGO").unwrap();
        let ok = Step::new("ok", &cargo, &["--version"]);
        let bad = Step::new("bad", &cargo, &["no-such-subcommand-for-test"]);
        assert!(run(root.path(), &[ok]).unwrap().passed);
        assert!(!run(root.path(), &[bad]).unwrap().passed);
    }

    #[test]
    fn a_missing_program_is_an_error_naming_the_step() {
        let root = tempfile::tempdir().unwrap();
        let step = Step::new("missing", "bugaboo-no-such-program", &[]);
        let error = run(root.path(), &[step]).err().unwrap();
        assert!(format!("{error:#}").contains("missing"));
    }
}
