//! Running a list of commands one after the other (used by `dev`, `build` and `ci`), and the
//! commands of the browser app and the server that more than one of them needs. The browser app
//! (`web/app`) may not exist yet; its steps are then skipped with a clear note instead of failing.

use std::path::{Path, PathBuf};
use std::process::{ExitCode, Stdio};
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
/// The name of the program that judges a load run (`[[bin]]` of bb-load-check).
const LOAD_CHECK_BINARY: &str = "bb-load-check";

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
    /// Starts when its turn comes and runs beside the steps after it; its output goes to a log
    /// file that is shown, and its result counts, when the step that joins it is reached (or at
    /// the end of the plan).
    pub(crate) background: bool,
    /// Waits for the steps that run in the background before it starts.
    pub(crate) joins_background: bool,
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
            background: false,
            joins_background: false,
        }
    }

    /// Runs beside the steps after it (see `background`).
    pub(crate) fn in_background(self) -> Self {
        Self {
            background: true,
            ..self
        }
    }

    /// Starts only after every step in the background has ended (see `joins_background`).
    pub(crate) fn after_background(self) -> Self {
        Self {
            joins_background: true,
            ..self
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

/// Builds the `bb-load-check` program, which judges a run of `cargo xtask load`.
pub(crate) fn load_check_build(profile: Profile) -> Step {
    program_build(
        profile,
        "build the load check program",
        "bb-load-check",
        LOAD_CHECK_BINARY,
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

/// A step that runs beside the others.
struct Running {
    name: &'static str,
    child: std::process::Child,
    log: PathBuf,
    started: Instant,
}

/// The log file of a step in the background, inside the build folder of the repository.
fn log_path(root: &Path, name: &str) -> PathBuf {
    let slug: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    root.join("target").join(format!("step-{slug}.log"))
}

/// Starts a step in the background with its output in its log file, in a process group of its own
/// so that it can be ended with everything it started.
fn start_background(root: &Path, step: &Step) -> anyhow::Result<Running> {
    let log = log_path(root, step.name);
    if let Some(folder) = log.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let file = std::fs::File::create(&log)
        .with_context(|| format!("could not write {}", log.display()))?;
    let mut command = crate::tools::command(&step.program);
    command
        .args(&step.args)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(file.try_clone()?)
        .stderr(file);
    let child =
        crate::tools::spawn_in_group(&mut command, step.name).with_context(|| start_error(step))?;
    output::line(&format!(
        "start {} in the background (log: {})",
        step.name,
        log.display()
    ));
    Ok(Running {
        name: step.name,
        child,
        log,
        started: Instant::now(),
    })
}

/// Waits for the steps in the background, shows their logs and tells how each one went; true when
/// all of them passed. With `stop`, they are ended with everything they started instead of waited
/// for (an earlier step failed, or the run is leaving on an error). It never fails itself.
fn finish_background(running: &mut Vec<Running>, stop: bool) -> bool {
    let mut all_passed = true;
    for mut step in running.drain(..) {
        let ended_well = if stop {
            crate::tools::stop_tree(&mut step.child);
            false
        } else {
            step.child.wait().is_ok_and(|status| status.success())
        };
        output::line(&format!("--- output of {} ---", step.name));
        match std::fs::read(&step.log) {
            Ok(bytes) => output::line(String::from_utf8_lossy(&bytes).trim_end()),
            Err(error) => output::line(&format!(
                "The log could not be read ({error}); it is {}.",
                step.log.display()
            )),
        }
        output::line("---");
        if stop {
            output::line(&format!(
                "{} was stopped because an earlier step failed.",
                step.name
            ));
        }
        output::step_end(step.name, ended_well, step.started.elapsed());
        all_passed &= ended_well;
    }
    all_passed
}

/// Runs the steps in order and stops at the first failure. A step in the background runs beside
/// the steps after it; its result is told, and fails the run, where the plan joins it. On every
/// way out, an error included, the steps still in the background are ended and reported.
pub(crate) fn run(root: &Path, steps: &[Step]) -> anyhow::Result<Outcome> {
    let mut running: Vec<Running> = Vec::new();
    let mut skipped = 0_usize;
    let result = run_in_order(root, steps, &mut running, &mut skipped);
    // What is still in the background is waited for after a good run and ended after a failure.
    let stop = result.as_ref().map_or(true, |outcome| !outcome.passed);
    let background_passed = finish_background(&mut running, stop);
    let mut outcome = result?;
    outcome.passed &= background_passed;
    output::steps_summary(steps.len(), skipped, usize::from(!outcome.passed));
    Ok(outcome)
}

fn run_in_order(
    root: &Path,
    steps: &[Step],
    running: &mut Vec<Running>,
    skipped: &mut usize,
) -> anyhow::Result<Outcome> {
    let mut outcome = Outcome {
        passed: true,
        executable: None,
    };
    for (index, step) in steps.iter().enumerate() {
        if let Some(reason) = step.skip {
            output::step_skipped(step.name, reason);
            *skipped += 1;
            continue;
        }
        if step.joins_background && !finish_background(running, false) {
            outcome.passed = false;
            break;
        }
        output::step_start(index + 1, steps.len(), step.name);
        if step.background {
            running.push(start_background(root, step)?);
            continue;
        }
        let started = Instant::now();
        let mut command = crate::tools::command(&step.program);
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
    fn the_load_check_program_is_built_the_same_way() {
        let step = load_check_build(Profile::Debug);
        assert!(step.args.contains(&"bb-load-check".to_owned()));
        assert_eq!(step.executable, Some("bb-load-check"));
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

    fn cargo_step(name: &'static str, args: &[&str]) -> Step {
        Step::new(name, std::env::var("CARGO").unwrap(), args)
    }

    #[test]
    fn a_step_in_the_background_runs_and_its_log_is_kept() {
        let root = tempfile::tempdir().unwrap();
        let steps = [
            cargo_step("version in the background", &["--version"]).in_background(),
            cargo_step("version", &["--version"]),
            cargo_step("version after it", &["--version"]).after_background(),
        ];
        assert!(run(root.path(), &steps).unwrap().passed);
        let log =
            std::fs::read_to_string(log_path(root.path(), "version in the background")).unwrap();
        assert!(log.starts_with("cargo "), "{log}");
    }

    #[test]
    fn bad_example_a_failing_step_in_the_background_fails_the_run() {
        let root = tempfile::tempdir().unwrap();
        let steps = [
            cargo_step("fails", &["no-such-subcommand-for-test"]).in_background(),
            cargo_step("version", &["--version"]),
        ];
        assert!(!run(root.path(), &steps).unwrap().passed);
    }

    #[test]
    fn bad_example_a_step_that_joins_does_not_start_after_a_failure_in_the_background() {
        let root = tempfile::tempdir().unwrap();
        let steps = [
            cargo_step("fails", &["no-such-subcommand-for-test"]).in_background(),
            Step::new("must not run", "bugaboo-no-such-program", &[]).after_background(),
        ];
        // The missing program would be an error if the step started.
        assert!(!run(root.path(), &steps).unwrap().passed);
    }

    /// A step that runs for half a minute unless it is ended.
    fn long_step(name: &'static str) -> Step {
        if cfg!(windows) {
            Step::new(name, "ping", &["-n", "30", "127.0.0.1"])
        } else {
            Step::new(name, "sleep", &["30"])
        }
    }

    /// The log of a step that was ended can be deleted: on Windows a file cannot be deleted while
    /// a program that writes to it is running.
    fn assert_ended(root: &Path, name: &str, started: Instant) {
        assert!(started.elapsed() < std::time::Duration::from_secs(20));
        std::fs::remove_file(log_path(root, name)).unwrap();
    }

    #[test]
    fn a_failing_step_ends_the_one_in_the_background_and_fails_the_run() {
        let root = tempfile::tempdir().unwrap();
        let started = Instant::now();
        let steps = [
            long_step("long").in_background(),
            cargo_step("fails", &["no-such-subcommand-for-test"]),
        ];
        assert!(!run(root.path(), &steps).unwrap().passed);
        assert_ended(root.path(), "long", started);
    }

    #[test]
    fn a_step_that_cannot_start_ends_the_one_in_the_background_before_the_error_leaves() {
        let root = tempfile::tempdir().unwrap();
        let started = Instant::now();
        let steps = [
            long_step("long").in_background(),
            Step::new("missing", "bugaboo-no-such-program", &[]),
        ];
        assert!(run(root.path(), &steps).is_err());
        assert_ended(root.path(), "long", started);
    }

    #[test]
    fn a_background_step_that_cannot_start_ends_the_earlier_one() {
        let root = tempfile::tempdir().unwrap();
        let started = Instant::now();
        let steps = [
            long_step("long").in_background(),
            Step::new("missing", "bugaboo-no-such-program", &[]).in_background(),
        ];
        assert!(run(root.path(), &steps).is_err());
        assert_ended(root.path(), "long", started);
    }
}
