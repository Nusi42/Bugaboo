//! `cargo xtask dev`: the Vite dev server and the Bugaboo server, side by side (ADR-0023).
//!
//! This file is the only place in xtask that reads `tools/dev/ports.json`.

use std::path::Path;
use std::process::{Child, Command};
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Context as _, bail};
use serde::Deserialize;

use crate::output;
use crate::steps::{self, Step, WEB_APP};

/// Where the ports live; `BB_PORTS_FILE` overrides it for a second checkout (ADR-0023).
const PORTS_FILE: &str = "tools/dev/ports.json";
const PORTS_FILE_VARIABLE: &str = "BB_PORTS_FILE";
const POLL: Duration = Duration::from_millis(250);

/// The ports `dev` needs. Other keys in the file belong to other tools.
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub(crate) struct Ports {
    pub(crate) server: u16,
    pub(crate) web: u16,
}

impl Ports {
    pub(crate) fn parse(json: &str) -> anyhow::Result<Self> {
        serde_json::from_str(json).with_context(|| format!("{PORTS_FILE} is not valid"))
    }

    pub(crate) fn load(root: &Path) -> anyhow::Result<Self> {
        let path = std::env::var_os(PORTS_FILE_VARIABLE)
            .map_or_else(|| root.join(PORTS_FILE), std::path::PathBuf::from);
        let json = std::fs::read_to_string(&path)
            .with_context(|| format!("could not read {}", path.display()))?;
        Self::parse(&json)
    }
}

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

/// Stops the program and everything it started (`npm` starts `node`).
fn stop(child: &mut Child) {
    if cfg!(windows) {
        // `Child::kill` would leave the grandchildren running.
        let _ = Command::new("taskkill")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .output();
    } else {
        let _ = child.kill();
    }
    let _ = child.wait();
}

pub(crate) fn run(root: &Path) -> anyhow::Result<bool> {
    let ports = Ports::load(root)?;
    let server_build = Step::new(
        "build the bugaboo binary",
        "cargo",
        &["build", "--package", "bb-server"],
    );
    if !steps::run(root, &[server_build])? {
        return Ok(false);
    }

    let mut running = Vec::new();
    let server_port = ports.server.to_string();
    running.push(Running {
        name: "server",
        child: Command::new(steps::server_binary(root, "debug"))
            .args(["serve", "--port", &server_port])
            .current_dir(root)
            .spawn()
            .context("could not start the bugaboo server")?,
    });
    output::line(&format!("Server:  http://127.0.0.1:{}", ports.server));

    if steps::web_app_exists(root) {
        let web_port = ports.web.to_string();
        running.push(Running {
            name: "browser app",
            child: Command::new(steps::npm())
                .args([
                    "run",
                    "dev",
                    "-w",
                    WEB_APP,
                    "--",
                    "--port",
                    &web_port,
                    "--strictPort",
                ])
                .current_dir(root)
                .spawn()
                .context("could not start the Vite dev server")?,
        });
        output::line(&format!("Browser: http://localhost:{}", ports.web));
    } else {
        output::line("The browser app does not exist yet (web/app); running the server alone.");
    }
    output::line("Press Ctrl+C to stop.");
    wait_for_exit(&mut running)
}

/// Waits until one program ends; the others are stopped when `running` is dropped.
fn wait_for_exit(running: &mut [Running]) -> anyhow::Result<bool> {
    loop {
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
    fn dev_ports_are_read_from_the_json_and_other_keys_are_ignored() {
        let ports = Ports::parse(r#"{"$comment":"x","server":47200,"web":47201,"vitestUi":1}"#);
        assert_eq!(
            ports.unwrap(),
            Ports {
                server: 47200,
                web: 47201
            }
        );
    }

    #[test]
    fn dev_ports_missing_key_is_an_error_naming_the_file() {
        let error = Ports::parse(r#"{"server":47200}"#).unwrap_err();
        assert!(format!("{error:#}").contains("tools/dev/ports.json"));
    }

    #[test]
    fn dev_ports_the_real_file_is_valid_and_in_the_reserved_block() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let json = std::fs::read_to_string(root.join(PORTS_FILE)).unwrap();
        let ports = Ports::parse(&json).unwrap();
        for port in [ports.server, ports.web] {
            assert!((47200..=47299).contains(&port));
        }
    }
}
