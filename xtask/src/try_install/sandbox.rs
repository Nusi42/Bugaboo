//! The try-out in Windows Sandbox: a clean Windows that is thrown away when it closes.
//!
//! With `wsb`, the command-line tool of Windows Sandbox (Windows 11 24H2 and later, found on the
//! search path), the sandbox is started with the release folder and the previous program mapped
//! read only and the results folder mapped for writing, the script runs inside as the sandbox's
//! administrator (`SYSTEM`; a sandbox started this way has no signed-in person), and the sandbox
//! is stopped at the end, also after a failure. Where `wsb` is missing, a `.wsb` configuration
//! file with the same folders starts the script when the sandbox signs in, and the script shuts
//! the sandbox down when it is done.

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{Context as _, bail};

use super::Run;
use super::steps::RESULTS_FILE;
use crate::output;

/// The command-line tool of Windows Sandbox, found on the search path.
const WSB: &str = "wsb";
/// The program that opens a `.wsb` configuration file, found on the search path.
const SANDBOX_PROGRAM: &str = "WindowsSandbox.exe";
/// The configuration file of the fallback, in the folder of the run.
const CONFIG_FILE: &str = "try-install.wsb";
/// How long the fallback waits for the script to finish.
const FALLBACK_WAIT: Duration = Duration::from_hours(1);
const FALLBACK_POLL: Duration = Duration::from_secs(10);

/// What the sandbox is given: memory enough for the server, and no network, which the run does
/// not need.
const MEMORY_IN_MB: u32 = 4096;

/// Text for an XML element.
fn xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// The command that starts the script inside the sandbox. A mapped folder lands on the desktop of
/// the sandbox's account, so the script is looked for there, in every profile the system lists,
/// instead of at a path written down here. `close` shuts the sandbox down at the end (the
/// fallback, which has no `wsb stop`).
pub(super) fn guest_command(results: &str, script: &str, close: bool) -> String {
    let close = if close { " -CloseWhenDone" } else { "" };
    format!(
        "powershell.exe -NoProfile -ExecutionPolicy Bypass -Command \"$s = Get-CimInstance Win32_UserProfile | ForEach-Object {{ Join-Path $_.LocalPath 'Desktop\\{results}\\{script}' }} | Where-Object {{ Test-Path $_ }} | Select-Object -First 1; & $s{close}\""
    )
}

/// The configuration of the sandbox: the release and the previous program read only, the results
/// folder writable, and for the fallback the command that runs when it signs in.
pub(super) fn configuration(run: &Run, logon: Option<&str>) -> String {
    let folder = |path: &Path, read_only: bool| {
        format!(
            "<MappedFolder><HostFolder>{}</HostFolder><ReadOnly>{read_only}</ReadOnly></MappedFolder>",
            xml(&path.to_string_lossy())
        )
    };
    let logon = logon.map_or_else(String::new, |command| {
        format!(
            "<LogonCommand><Command>{}</Command></LogonCommand>",
            xml(command)
        )
    });
    format!(
        "<Configuration><Networking>Disable</Networking><MemoryInMB>{MEMORY_IN_MB}</MemoryInMB><MappedFolders>{}{}{}</MappedFolders>{logon}</Configuration>",
        folder(&run.release, true),
        folder(&run.previous, true),
        folder(&run.results, false),
    )
}

/// The names the guest finds the script by: the results folder and the script in it.
fn names(run: &Run) -> anyhow::Result<(String, String)> {
    let name = |path: &Path| {
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .with_context(|| format!("{} has no name", path.display()))
    };
    Ok((name(&run.results)?, name(&run.script)?))
}

/// Runs the script in a sandbox, with `wsb` where it is there.
pub(super) fn run(run: &Run) -> anyhow::Result<()> {
    let wsb_works = Command::new(WSB)
        .arg("--version")
        .output()
        .is_ok_and(|answer| answer.status.success());
    if wsb_works {
        with_wsb(run)
    } else {
        output::info(&format!(
            "{WSB} was not found: the sandbox starts from {CONFIG_FILE}"
        ));
        with_file(run)
    }
}

/// A sandbox started by `wsb`, stopped when this goes out of scope.
struct Sandbox(String);

impl Drop for Sandbox {
    fn drop(&mut self) {
        let stopped = Command::new(WSB)
            .args(["stop", "--id", &self.0])
            .status()
            .is_ok_and(|status| status.success());
        if !stopped {
            output::note(&format!(
                "the sandbox {} may still run: stop it with `wsb stop --id {}`",
                self.0, self.0
            ));
        }
    }
}

/// The ID in the answer of `wsb start --raw`.
fn sandbox_id(answer: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(answer)
        .ok()?
        .get("Id")?
        .as_str()
        .map(str::to_owned)
}

fn with_wsb(run: &Run) -> anyhow::Result<()> {
    let started = Command::new(WSB)
        .args(["start", "--raw", "--config", &configuration(run, None)])
        .output()
        .context("could not start wsb")?;
    let answer = String::from_utf8_lossy(&started.stdout);
    let Some(id) = sandbox_id(&answer) else {
        bail!("Windows Sandbox did not start: {}", answer.trim());
    };
    let sandbox = Sandbox(id);
    let (results, script) = names(run)?;
    output::info("try-install: the sandbox runs the steps ...");
    let ran = Command::new(WSB)
        .args(["exec", "--raw", "--id", &sandbox.0, "--run-as", "System"])
        .args(["--command", &guest_command(&results, &script, false)])
        .output()
        .context("could not run the script in the sandbox")?;
    if !ran.status.success() {
        output::note(&format!(
            "wsb exec: {}",
            String::from_utf8_lossy(&ran.stdout).trim()
        ));
    }
    drop(sandbox);
    Ok(())
}

fn with_file(run: &Run) -> anyhow::Result<()> {
    let (results, script) = names(run)?;
    let file = run.work.join(CONFIG_FILE);
    let command = guest_command(&results, &script, true);
    std::fs::write(&file, configuration(run, Some(&command)))
        .with_context(|| format!("could not write {}", file.display()))?;
    Command::new(SANDBOX_PROGRAM)
        .arg(&file)
        .spawn()
        .with_context(|| format!("could not start {SANDBOX_PROGRAM}: is Windows Sandbox on?"))?;
    let started = Instant::now();
    let done = || {
        std::fs::read_to_string(run.results.join(RESULTS_FILE))
            .ok()
            .and_then(|text| text.lines().last().map(str::to_owned))
            .is_some_and(|last| last == super::PASSED || last == "failed")
    };
    while !done() {
        if started.elapsed() > FALLBACK_WAIT {
            bail!(
                "the sandbox did not finish within {} minutes; close it by hand",
                FALLBACK_WAIT.as_secs() / 60
            );
        }
        std::thread::sleep(FALLBACK_POLL);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_in(folder: &Path) -> Run {
        Run {
            release: folder.join("0.6.0-test"),
            previous: folder.join("previous"),
            results: folder.join("results"),
            script: folder.join("results").join("run.ps1"),
            work: folder.to_path_buf(),
        }
    }

    #[test]
    fn ops_04_the_sandbox_reads_the_release_and_writes_only_its_results() {
        let folder = Path::new("work & more");
        let text = configuration(&run_in(folder), None);
        let release = xml(&folder.join("0.6.0-test").to_string_lossy());
        let results = xml(&folder.join("results").to_string_lossy());
        assert!(
            text.contains(&format!(
                "<HostFolder>{release}</HostFolder><ReadOnly>true</ReadOnly>"
            )),
            "{text}"
        );
        assert!(
            text.contains(&format!(
                "<HostFolder>{results}</HostFolder><ReadOnly>false</ReadOnly>"
            )),
            "{text}"
        );
        assert!(text.contains("&amp; more"), "{text}");
        assert!(!text.contains("LogonCommand"));
        // The fallback starts the script when the sandbox signs in, and closes it at the end.
        let command = guest_command("results", "run.ps1", true);
        let text = configuration(&run_in(folder), Some(&command));
        assert!(text.contains("<LogonCommand><Command>"), "{text}");
        assert!(command.contains("-CloseWhenDone"));
        assert!(!guest_command("results", "run.ps1", false).contains("-CloseWhenDone"));
    }

    #[test]
    fn ops_04_the_id_of_the_sandbox_comes_from_the_answer_of_wsb() {
        assert_eq!(
            sandbox_id("{\n  \"Id\": \"42c947e6-e45b\"\n}\n").as_deref(),
            Some("42c947e6-e45b")
        );
        assert_eq!(sandbox_id("Fehler"), None);
    }
}
