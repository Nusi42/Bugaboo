//! `cargo xtask dev`: the Vite dev server and the Bugaboo server, side by side (ADR-0023).
//!
//! The ports come from [`crate::ports`].

use std::path::Path;
use std::process::{Child, Command};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep;
use std::time::{Duration, Instant};

use anyhow::{Context as _, bail};

use crate::dataset;
use crate::output;
use crate::ports::{PORTS_FILE, Ports};
use crate::steps::{self, Profile};

const POLL: Duration = Duration::from_millis(250);
/// How long a program gets to stop after being asked to, before it is killed.
const STOP_GRACE: Duration = Duration::from_secs(5);

/// A started program that is stopped again when this value goes away.
struct Running {
    name: &'static str,
    child: Child,
}

impl Drop for Running {
    fn drop(&mut self) {
        stop(&mut self.child);
    }
}

/// Starts the program in its own process group (Unix), so it and what it starts can be
/// stopped together. On Windows `taskkill /T` stops the whole tree instead.
fn spawn(command: &mut Command, what: &str) -> anyhow::Result<Child> {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;
        command.process_group(0);
    }
    command
        .spawn()
        .with_context(|| format!("could not start the {what}"))
}

/// Stops the program and everything it started (`npm` starts `node`).
fn stop(child: &mut Child) {
    if cfg!(windows) {
        let _ = Command::new("taskkill")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .output();
    } else {
        // A negative id addresses the whole process group.
        let _ = Command::new("kill")
            .args(["-TERM", "--", &format!("-{}", child.id())])
            .output();
        let deadline = Instant::now() + STOP_GRACE;
        while Instant::now() < deadline && matches!(child.try_wait(), Ok(None)) {
            sleep(Duration::from_millis(50));
        }
        let _ = child.kill();
    }
    let _ = child.wait();
}

/// The data folder of the server in development (inside the work folder's build folder).
const DEV_DATA: &str = "target/dev-data";

pub(crate) fn run(root: &Path, sample: bool) -> anyhow::Result<bool> {
    let ports = Ports::load(root)?;
    let built = steps::run(root, &[steps::server_build(Profile::Debug)])?;
    let Some(server) = built.executable.filter(|_| built.passed) else {
        return Ok(false);
    };
    // A fresh sample organization every time; its program says where the passwords are.
    let data_dir = if sample {
        let Some(folder) = dataset::make_for_dev(root)? else {
            return Ok(false);
        };
        folder
    } else {
        root.join(DEV_DATA)
    };

    let interrupted = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&interrupted);
    ctrlc::set_handler(move || flag.store(true, Ordering::SeqCst))
        .context("could not listen for Ctrl+C")?;

    let mut running = Vec::new();
    running.push(Running {
        name: "server",
        child: spawn(
            Command::new(server)
                .args(["serve", "--port", &ports.server.to_string()])
                // Development runs on this computer only, so plain HTTP is allowed (SEC-10); its
                // data stays inside the work folder's build folder.
                .args(["--tls", "off", "--data-dir"])
                .arg(&data_dir)
                .current_dir(root),
            "bugaboo server",
        )?,
    });
    output::line(&format!("Server:  http://127.0.0.1:{}", ports.server));

    if steps::web_app_exists(root) {
        running.push(Running {
            name: "browser app",
            child: spawn(
                Command::new(steps::npm())
                    .args(["run", "dev", "--"])
                    .args(["--port", &ports.web.to_string(), "--strictPort"])
                    .current_dir(root),
                "Vite dev server",
            )?,
        });
        output::line(&format!("Browser: http://localhost:{}", ports.web));
    } else {
        output::line("The browser app does not exist yet (web/app); running the server alone.");
    }
    output::line("Press Ctrl+C to stop.");
    wait_for_exit(&mut running, &interrupted)
}

/// Waits until Ctrl+C or until one program ends; `running` stops the rest when dropped.
fn wait_for_exit(running: &mut [Running], interrupted: &AtomicBool) -> anyhow::Result<bool> {
    loop {
        if interrupted.load(Ordering::SeqCst) {
            output::line("Stopping.");
            return Ok(true);
        }
        for program in running.iter_mut() {
            if let Some(status) = program
                .child
                .try_wait()
                .context("could not check a program")?
            {
                if status.success() {
                    return Ok(true);
                }
                bail!(
                    "The {} stopped ({status}). Is its port busy? Ports are set in {PORTS_FILE}.",
                    program.name
                );
            }
        }
        sleep(POLL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_waiting_ends_at_once_after_ctrl_c() {
        let interrupted = AtomicBool::new(true);
        assert!(wait_for_exit(&mut [], &interrupted).unwrap());
    }
}
