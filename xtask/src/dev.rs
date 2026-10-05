//! `cargo xtask dev`: the Vite dev server and the Bugaboo server, side by side (ADR-0023).
//!
//! This file is the only place in xtask that reads `tools/dev/ports.json`.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::{Child, Command};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep;
use std::time::{Duration, Instant};

use anyhow::{Context as _, bail};
use serde::Deserialize;

use crate::output;
use crate::steps::{self, Profile};

/// Where the ports live (ADR-0023).
const PORTS_FILE: &str = "tools/dev/ports.json";
/// Names the block of ports of a folder that is not called `bugaboo.winN`.
const BLOCK_VARIABLE: &str = "BUGABOO_PORT_BLOCK";
/// The work folders are called `bugaboo.winN` (ADR-0024); the N is the block.
const WORK_FOLDER: &str = "bugaboo.win";
const POLL: Duration = Duration::from_millis(250);
/// How long a program gets to stop after being asked to, before it is killed.
const STOP_GRACE: Duration = Duration::from_secs(5);

/// The layout of the ports file: blocks of `stride` ports from `first`, and the offset of each
/// port inside a block. The block of a work folder is `bugaboo.winN` -> N; any other folder uses
/// block 1.
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub(crate) struct Layout {
    first: u32,
    stride: u32,
    blocks: u32,
    offsets: BTreeMap<String, u32>,
}

/// The ports `dev` needs.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Ports {
    pub(crate) server: u16,
    pub(crate) web: u16,
}

/// The block of ports for the folder `folder_name`: the one `override_value` (the value of
/// `BUGABOO_PORT_BLOCK`) names, else the N of `bugaboo.winN`, else block 1 (CI, a fresh clone).
pub(crate) fn block_of(
    folder_name: &str,
    override_value: Option<&str>,
    blocks: u32,
) -> anyhow::Result<u32> {
    let wrong = |source: String| {
        anyhow::anyhow!(
            "{source} has no block of ports: {PORTS_FILE} gives blocks 1 to {blocks}. Set              {BLOCK_VARIABLE} to one of them."
        )
    };
    let valid = |number: Option<u32>, source: String| match number {
        Some(number) if (1..=blocks).contains(&number) => Ok(number),
        _ => Err(wrong(source)),
    };
    if let Some(value) = override_value.filter(|value| !value.is_empty()) {
        return valid(value.parse().ok(), format!("{BLOCK_VARIABLE}={value}"));
    }
    let number = folder_name
        .strip_prefix(WORK_FOLDER)
        .filter(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()));
    match number {
        None => Ok(1),
        Some(rest) => valid(rest.parse().ok(), format!("The folder {folder_name}")),
    }
}

impl Layout {
    pub(crate) fn parse(json: &str) -> anyhow::Result<Self> {
        serde_json::from_str(json).with_context(|| format!("{PORTS_FILE} is not valid"))
    }

    /// The port called `key` in block `block` (counted from 1).
    pub(crate) fn port(&self, block: u32, key: &str) -> anyhow::Result<u16> {
        let offset = self
            .offsets
            .get(key)
            .with_context(|| format!("{PORTS_FILE} has no port `{key}`"))?;
        let number = self.first + (block - 1) * self.stride + offset;
        u16::try_from(number).with_context(|| format!("{PORTS_FILE}: {number} is no port"))
    }

    /// The layout in the repository and the block of its folder.
    fn load(root: &Path) -> anyhow::Result<(Self, u32)> {
        let path = root.join(PORTS_FILE);
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("could not read {}", path.display()))?;
        let layout = Self::parse(&text)?;
        let name = root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let block = block_of(
            &name,
            std::env::var(BLOCK_VARIABLE).ok().as_deref(),
            layout.blocks,
        )?;
        Ok((layout, block))
    }
}

impl Ports {
    pub(crate) fn load(root: &Path) -> anyhow::Result<Self> {
        let (layout, block) = Layout::load(root)?;
        Ok(Self {
            server: layout.port(block, "server")?,
            web: layout.port(block, "web")?,
        })
    }

    /// One port by its key in the file (for tools other than `dev`, such as `db`).
    pub(crate) fn load_port(root: &Path, key: &str) -> anyhow::Result<u16> {
        let (layout, block) = Layout::load(root)?;
        layout.port(block, key)
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

pub(crate) fn run(root: &Path) -> anyhow::Result<bool> {
    let ports = Ports::load(root)?;
    let built = steps::run(root, &[steps::server_build(Profile::Debug)])?;
    let Some(server) = built.executable.filter(|_| built.passed) else {
        return Ok(false);
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
                .args(["--tls", "off", "--data-dir", "target/dev-data"])
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

    const LAYOUT: &str = r#"{"$comment":"x","first":47200,"stride":100,"blocks":9,
        "offsets":{"server":0,"web":1,"postgres":40}}"#;

    fn layout() -> Layout {
        Layout::parse(LAYOUT).unwrap()
    }

    #[test]
    fn dev_ports_every_work_folder_has_a_block_of_its_own() {
        let layout = layout();
        let block = |name: &str| block_of(name, None, layout.blocks).unwrap();
        assert_eq!(layout.port(block("bugaboo.win1"), "server").unwrap(), 47200);
        assert_eq!(layout.port(block("bugaboo.win2"), "server").unwrap(), 47300);
        assert_eq!(layout.port(block("bugaboo.win3"), "server").unwrap(), 47400);
        assert_eq!(layout.port(block("bugaboo.win3"), "web").unwrap(), 47401);
        assert_eq!(
            layout.port(block("bugaboo.win3"), "postgres").unwrap(),
            47440
        );
        assert_eq!(
            layout.port(block("bugaboo.win9"), "postgres").unwrap(),
            48040
        );
    }

    #[test]
    fn dev_ports_any_other_folder_uses_the_first_block() {
        for name in [
            "bugaboo",
            "checkout",
            "bugaboo.win",
            "bugaboo.winx",
            "my-bugaboo.win3",
        ] {
            assert_eq!(block_of(name, None, 9).unwrap(), 1, "{name}");
        }
    }

    #[test]
    fn dev_ports_the_variable_names_another_block() {
        assert_eq!(block_of("checkout", Some("5"), 9).unwrap(), 5);
        // It wins over the name of the folder; an empty value is no value.
        assert_eq!(block_of("bugaboo.win2", Some("7"), 9).unwrap(), 7);
        assert_eq!(block_of("bugaboo.win2", Some(""), 9).unwrap(), 2);
        // A block that does not exist is an error that names the variable.
        for wrong in ["0", "10", "two", "-1"] {
            let error = block_of("checkout", Some(wrong), 9).unwrap_err();
            assert!(format!("{error:#}").contains(BLOCK_VARIABLE), "{wrong}");
        }
        let error = block_of("bugaboo.win10", None, 9).unwrap_err();
        assert!(format!("{error:#}").contains("bugaboo.win10"));
        assert!(block_of("bugaboo.win0", None, 9).is_err());
    }

    #[test]
    fn dev_ports_missing_key_is_an_error_naming_the_file() {
        let error = layout().port(1, "nothing").unwrap_err();
        assert!(format!("{error:#}").contains("tools/dev/ports.json"));
        let error = Layout::parse(r#"{"first":47200}"#).unwrap_err();
        assert!(format!("{error:#}").contains("tools/dev/ports.json"));
    }

    #[test]
    fn dev_ports_the_real_file_is_valid_and_every_block_is_in_the_reserved_range() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let json = std::fs::read_to_string(root.join(PORTS_FILE)).unwrap();
        let layout = Layout::parse(&json).unwrap();
        for block in 1..=layout.blocks {
            for key in layout.offsets.keys() {
                let port = layout.port(block, key).unwrap();
                assert!(
                    (47200..=48099).contains(&port),
                    "block {block}, {key}: {port}"
                );
            }
        }
    }

    #[test]
    fn dev_waiting_ends_at_once_after_ctrl_c() {
        let interrupted = AtomicBool::new(true);
        assert!(wait_for_exit(&mut [], &interrupted).unwrap());
    }
}
