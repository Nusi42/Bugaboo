//! `cargo xtask nightly [--install]`: the nightly run (PERF-01, architecture §11): the full
//! `cargo xtask ci`, then `dataset large` on both databases if a dataset is older than the last
//! migration (or missing), then `bench` on both. One log of the night goes to `.work/nightly/`.
//!
//! **One full run at a time** (04-milestones, "The lead's routine"): the nightly run, `ci` and
//! `bench` take the lock of the computer first and wait while another holds it. The lock is a
//! local port nobody else uses (`fullRunLock` in `tools/dev/ports.json`): the system gives it to
//! one program at a time, writes nothing anywhere and takes it back when the program ends, even
//! when it is killed. `--install` registers the daily task with the scheduler of Windows (03:00,
//! in the folder it is run in, which is the lead's); the lead installs it after the merge.

use std::fs::{self, File};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::{Context as _, bail};

use crate::dataset::{in_folder, load_layout};
use crate::dataset_large::{dataset_folder, made_for, newest_migration};
use crate::output;
use crate::ports::Layout as Ports;
use crate::repo::Repo;
use bb_dev_words::database::DatabaseKind;

/// The block whose port numbers everybody uses for the lock (every folder has a block of its
/// own, the lock is one for all).
const LOCK_BLOCK: u32 = 1;
const LOCK_PORT: &str = "fullRunLock";
/// How long a waiting run sleeps between two tries.
const WAIT: Duration = Duration::from_secs(30);
/// Set for a program the holder of the lock started: it needs no lock of its own.
const HELD_VARIABLE: &str = "BUGABOO_FULL_RUN_LOCK_HELD";
/// The name and the time of the task.
const TASK_NAME: &str = "Bugaboo nightly";
const TASK_TIME: &str = "03:00";

/// The lock of the computer, held while this value lives.
pub(crate) struct FullRun {
    /// Only held, so the port stays taken until this value is dropped.
    _lock: Option<TcpListener>,
}

/// Takes the lock if nobody holds it.
fn try_lock(port: u16) -> Option<TcpListener> {
    TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port)).ok()
}

/// Takes the lock, calling `waiting` and then `pause` after every failed try.
fn acquire_with(port: u16, mut waiting: impl FnMut(), mut pause: impl FnMut()) -> TcpListener {
    loop {
        if let Some(lock) = try_lock(port) {
            return lock;
        }
        waiting();
        pause();
    }
}

impl FullRun {
    /// Waits for the lock and takes it, unless the program that started this one holds it.
    pub(crate) fn take(repo: &Repo) -> anyhow::Result<Self> {
        if std::env::var_os(HELD_VARIABLE).is_some() {
            return Ok(Self { _lock: None });
        }
        let ports = Ports::parse(&fs::read_to_string(
            repo.root().join(crate::ports::PORTS_FILE),
        )?)?;
        let port = ports.port(LOCK_BLOCK, LOCK_PORT)?;
        let mut told = false;
        let lock = acquire_with(
            port,
            || {
                if !told {
                    output::info("Another full run is on this computer; waiting for it to end.");
                    told = true;
                }
            },
            || std::thread::sleep(WAIT),
        );
        Ok(Self { _lock: Some(lock) })
    }
}

/// Whether a dataset made for the migration `made` (none: there is none) must be made again
/// because the newest migration is `newest`. The names are compared: file times change with a
/// checkout or a copy, and a migration that was only touched would make a new dataset for nothing.
fn stale(made: Option<&str>, newest: Option<&str>) -> bool {
    made.is_none() || made != newest
}

/// The arguments of `schtasks` that register the daily task for the folder `root`.
fn schtasks_arguments(root: &Path) -> Vec<String> {
    let command = format!("cmd /c cd /d \"{}\" && cargo xtask nightly", root.display());
    [
        "/Create", "/F", "/SC", "DAILY", "/ST", TASK_TIME, "/TN", TASK_NAME, "/TR",
    ]
    .iter()
    .map(ToString::to_string)
    .chain(std::iter::once(command))
    .collect()
}

fn install(repo: &Repo) -> anyhow::Result<bool> {
    if !cfg!(windows) {
        bail!(
            "The nightly task is registered with the scheduler of Windows; on Linux use cron to run `cargo xtask nightly` at night."
        );
    }
    let status = Command::new("schtasks")
        .args(schtasks_arguments(repo.root()))
        .status()
        .context("could not run schtasks")?;
    if status.success() {
        output::info(&format!(
            "The task \"{TASK_NAME}\" runs `cargo xtask nightly` every day at {TASK_TIME} in {}.",
            repo.root().display()
        ));
    }
    Ok(status.success())
}

/// Runs `xtask <arguments>` with its output appended to the log of the night.
fn step(root: &Path, log: &File, name: &str, arguments: &[&str]) -> anyhow::Result<bool> {
    use std::io::Write as _;
    let mut note = log.try_clone()?;
    writeln!(note, "== {name}: xtask {}", arguments.join(" "))?;
    let status = Command::new(std::env::current_exe()?)
        .args(arguments)
        .current_dir(root)
        .env(HELD_VARIABLE, "1")
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log.try_clone()?)
        .status()
        .with_context(|| format!("could not run {name}"))?;
    writeln!(
        note,
        "== {name}: {}",
        if status.success() { "passed" } else { "failed" }
    )?;
    Ok(status.success())
}

fn log_file(root: &Path, layout: &bb_dev_words::sample::Layout) -> anyhow::Result<(File, PathBuf)> {
    let folder = in_folder(root, &layout.large.nightly_out);
    fs::create_dir_all(&folder)?;
    let path = folder.join(format!("{}.log", crate::bench::today()));
    Ok((File::create(&path)?, path))
}

pub(crate) fn run(repo: &Repo, install_task: bool) -> anyhow::Result<bool> {
    if install_task {
        return install(repo);
    }
    let _lock = FullRun::take(repo)?;
    let root = repo.root();
    let layout = load_layout(root)?;
    let (log, path) = log_file(root, &layout)?;
    let mut held = step(root, &log, "all checks and tests", &["ci"])?;
    let newest = newest_migration(root);
    for database in DatabaseKind::ALL.iter().copied() {
        let made = made_for(&dataset_folder(repo, &layout, database));
        if stale(made.as_deref(), newest.as_deref()) {
            held &= step(
                root,
                &log,
                "large dataset",
                &["dataset", "large", "--database", database.code()],
            )?;
        }
    }
    held &= step(root, &log, "benchmarks", &["bench"])?;
    output::info(&format!(
        "nightly: {} The log is {}.",
        if held {
            "everything held."
        } else {
            "something failed."
        },
        path.display()
    ));
    Ok(held)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    fn free_port() -> u16 {
        TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    #[test]
    fn the_lock_is_taken_by_one_at_a_time() {
        let port = free_port();
        let first = try_lock(port).unwrap();
        assert!(try_lock(port).is_none());
        drop(first);
        assert!(try_lock(port).is_some());
    }

    #[test]
    fn the_nightly_run_waits_while_the_lock_is_taken() {
        let port = free_port();
        let held = try_lock(port).unwrap();
        let waited = Cell::new(0);
        let mut holder = Some(held);
        let _lock = acquire_with(
            port,
            || waited.set(waited.get() + 1),
            || {
                // The other run ends after three tries.
                if waited.get() == 3 {
                    holder.take();
                }
            },
        );
        assert_eq!(waited.get(), 3);
    }

    #[test]
    fn a_dataset_is_made_again_when_it_is_missing_or_made_for_another_migration() {
        assert!(stale(None, Some("m0002.rs")));
        assert!(stale(Some("m0001.rs"), Some("m0002.rs")));
        assert!(!stale(Some("m0002.rs"), Some("m0002.rs")));
        // A newer file time alone changes nothing; only the name counts.
        assert!(stale(Some("m0002.rs"), None));
    }

    #[test]
    fn the_task_is_registered_daily_at_three_in_the_given_folder() {
        let arguments = schtasks_arguments(Path::new("lead"));
        assert!(arguments.windows(2).any(|pair| pair == ["/ST", "03:00"]));
        assert!(arguments.last().unwrap().contains("cargo xtask nightly"));
        assert!(arguments.last().unwrap().contains("lead"));
    }
}
