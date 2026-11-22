//! `cargo xtask try-install windows-sandbox` and `cargo xtask try-install wsl` (OPS-04, follow-up
//! M1-F57): the service is installed, started, upgraded and removed for real, on a clean Windows
//! in Windows Sandbox and in a fresh WSL distribution with systemd.
//!
//! Both run the one list of steps (`steps.rs`) with the program of the release folder of this
//! version (`cargo xtask release`) and the server program of the previous milestone, which the
//! upgrade test keeps (`upgrade::kept_program`): the previous release makes its data, the release
//! takes its place and is installed as the service, whose first start stops at the upgrade while
//! the backup cannot be written, is not started again, and upgrades once the folder is back. The
//! results go to `.work/try-install/<system>/results/`, which is kept; the sandbox and the
//! distribution are removed at the end, also after a failure.

mod distribution;
mod helpers;
mod sandbox;
mod steps;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, bail};
use bb_core::backup::name::{BackupName, Purpose};
use bb_core::release::VERSION;

use crate::dataset::in_folder;
use crate::output;
use crate::release::assemble::this_build;
use crate::release::folder::{self, LINUX, PLATFORMS, Platform};
use crate::repo::Repo;
use crate::upgrade::{self, OldProgram};
use steps::{Facts, Guest, PASSED, RESULTS_FILE, UPGRADED_BACKUPS, UPGRADED_VERSION};

/// Where the runs keep their folders; the results stay for the record.
const WORK: &str = ".work/try-install";
/// The folders of one run, inside its system's folder: the results (written by the guest) and the
/// program of the previous release (read only).
const RESULTS: &str = "results";
const PREVIOUS: &str = "previous";
/// The port of the server of the try-out in `tools/dev/ports.json`.
const PORT: &str = "tryInstallServer";
/// What the host adds to the results after its own check of the upgrade.
const HOST: &str = "host:";

/// The clean system to try the install on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum System {
    /// A clean Windows in Windows Sandbox.
    WindowsSandbox,
    /// A fresh WSL distribution with systemd, imported from the published Ubuntu image.
    Wsl,
}

impl System {
    fn word(self) -> &'static str {
        match self {
            Self::WindowsSandbox => "windows-sandbox",
            Self::Wsl => "wsl",
        }
    }

    fn platform(self) -> Platform {
        match self {
            Self::WindowsSandbox => PLATFORMS[0],
            Self::Wsl => LINUX,
        }
    }

    fn old_program(self) -> OldProgram {
        match self {
            Self::WindowsSandbox => OldProgram::Server,
            Self::Wsl => OldProgram::LinuxServer,
        }
    }

    fn guest(self) -> Guest {
        match self {
            Self::WindowsSandbox => Guest::Windows,
            Self::Wsl => Guest::Linux,
        }
    }
}

/// The folders and programs of one run, on the host.
pub(crate) struct Run {
    /// The release folder of this version.
    pub(crate) release: PathBuf,
    /// The folder with the program of the previous release.
    pub(crate) previous: PathBuf,
    /// The results folder, which holds the script.
    pub(crate) results: PathBuf,
    /// The script, in the results folder.
    pub(crate) script: PathBuf,
    /// The folder of the system's own files (the image, the distribution, the `.wsb` file).
    pub(crate) work: PathBuf,
}

/// `cargo xtask try-install <system>`: the run, and whether every step went through.
pub(crate) fn run(repo: &Repo, system: System) -> anyhow::Result<bool> {
    let root = repo.root();
    let name = this_build(repo)?.release_name(VERSION);
    let release = folder::release_root(root)?.join(&name);
    let next_program = folder::program_name(&name, system.platform());
    if !release.join(&next_program).is_file() {
        bail!(
            "{} has no {next_program}: run `cargo xtask release` first",
            release.display()
        );
    }
    let tag = upgrade::previous_tag(root)?;
    let previous_built = upgrade::kept_program(root, &tag, system.old_program())?;
    let previous_program = previous_built
        .file_name()
        .context("the previous program has no name")?
        .to_string_lossy()
        .into_owned();

    let work = in_folder(root, WORK).join(system.word());
    let results = work.join(RESULTS);
    let previous = work.join(PREVIOUS);
    for folder in [&results, &previous] {
        if folder.exists() {
            fs::remove_dir_all(folder)
                .with_context(|| format!("could not clear {}", folder.display()))?;
        }
        fs::create_dir_all(folder)
            .with_context(|| format!("could not make {}", folder.display()))?;
    }
    fs::copy(&previous_built, previous.join(&previous_program))
        .with_context(|| format!("could not copy {}", previous_built.display()))?;
    let installed_name = format!("{}{}", crate::steps::SERVER_BINARY, system.platform().end);
    let port = crate::ports::Ports::load_port(root, PORT)?;
    let script = results.join(match system {
        System::WindowsSandbox => "run.ps1",
        System::Wsl => "run.sh",
    });
    let run = Run {
        release,
        previous,
        results,
        script,
        work,
    };
    output::info(&format!(
        "try-install {}: release {name}, previous {tag}, results in {}",
        system.word(),
        run.results.display()
    ));
    let script = |release: &str, previous: &str| {
        steps::script(
            system.guest(),
            &Facts {
                port,
                release,
                next_program: &next_program,
                previous,
                previous_program: &previous_program,
                installed_name: &installed_name,
            },
        )
    };
    match system {
        System::WindowsSandbox => {
            // The folders are mapped beside the results folder under their own names.
            let release_name = folder_name(&run.release)?;
            write_script(&run.script, &script(&release_name, PREVIOUS))?;
            sandbox::run(&run)?;
        }
        System::Wsl => distribution::run(&run, script)?,
    }
    judge(&run.results)
}

/// The last part of a folder's path.
fn folder_name(folder: &Path) -> anyhow::Result<String> {
    Ok(folder
        .file_name()
        .with_context(|| format!("{} has no name", folder.display()))?
        .to_string_lossy()
        .into_owned())
}

/// Writes a script with Unix line ends, which both shells read.
pub(crate) fn write_script(path: &Path, text: &str) -> anyhow::Result<()> {
    fs::write(path, text).with_context(|| format!("could not write {}", path.display()))
}

/// Reads the results the guest wrote, checks the upgrade on the host and adds its verdict.
fn judge(results: &Path) -> anyhow::Result<bool> {
    let file = results.join(RESULTS_FILE);
    let text = fs::read_to_string(&file).unwrap_or_default();
    for line in text.lines() {
        output::info(&format!("try-install: {line}"));
    }
    let guest_passed = text.lines().last().map(str::trim) == Some(PASSED);
    let upgraded = upgrade_problem(results);
    let verdict = match (&upgraded, guest_passed) {
        (None, true) => format!("{HOST} {PASSED}"),
        (Some(problem), _) => format!("{HOST} failed: {problem}"),
        (None, false) => format!("{HOST} failed: the guest did not pass"),
    };
    output::info(&format!("try-install: {verdict}"));
    let mut kept = text;
    kept.push_str(&verdict);
    kept.push('\n');
    fs::write(&file, kept).with_context(|| format!("could not write {}", file.display()))?;
    Ok(guest_passed && upgraded.is_none())
}

/// Why the data the guest left does not show the upgrade, if it does not: a backup made before
/// an upgrade by this version, and the version file naming this version.
fn upgrade_problem(results: &Path) -> Option<String> {
    let Ok(backups) = fs::read_to_string(results.join(UPGRADED_BACKUPS)) else {
        return Some(format!(
            "the guest listed no backups ({UPGRADED_BACKUPS} is missing)"
        ));
    };
    let found = backups.lines().map(str::trim).any(|name| {
        BackupName::parse(name).is_some_and(|backup| {
            backup.purpose == Purpose::BeforeUpgrade && backup.version == VERSION
        })
    });
    if !found {
        return Some(format!(
            "no backup made before the upgrade to {VERSION} among: {}",
            backups.lines().collect::<Vec<_>>().join(", ")
        ));
    }
    let version = fs::read_to_string(results.join(UPGRADED_VERSION)).unwrap_or_default();
    (!version.contains(VERSION))
        .then(|| format!("the data names another version than {VERSION}: {version}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ops_04_the_host_finds_the_backup_before_the_upgrade_and_the_new_version() {
        let results = tempfile::tempdir().unwrap();
        // A guest that listed no backups at all has not shown the upgrade.
        let missing = upgrade_problem(results.path());
        assert!(
            missing
                .as_deref()
                .is_some_and(|problem| problem.contains("listed no backups")),
            "{missing:?}"
        );
        let backup = BackupName::new(Purpose::BeforeUpgrade, jiff::Timestamp::UNIX_EPOCH, VERSION)
            .to_string();
        fs::write(results.path().join(UPGRADED_BACKUPS), format!("{backup}\n")).unwrap();
        fs::write(
            results.path().join(UPGRADED_VERSION),
            format!("{VERSION}\n"),
        )
        .unwrap();
        assert_eq!(upgrade_problem(results.path()), None);
        // A regular backup is not the one of the upgrade.
        let regular =
            BackupName::new(Purpose::Regular, jiff::Timestamp::UNIX_EPOCH, VERSION).to_string();
        fs::write(results.path().join(UPGRADED_BACKUPS), regular).unwrap();
        assert!(upgrade_problem(results.path()).is_some());
    }

    #[test]
    fn ops_04_a_run_passes_only_when_the_guest_and_the_host_say_so() {
        let results = tempfile::tempdir().unwrap();
        let backup = BackupName::new(Purpose::BeforeUpgrade, jiff::Timestamp::UNIX_EPOCH, VERSION)
            .to_string();
        fs::write(results.path().join(UPGRADED_BACKUPS), backup).unwrap();
        fs::write(results.path().join(UPGRADED_VERSION), VERSION).unwrap();
        fs::write(
            results.path().join(RESULTS_FILE),
            "ok verify\nfailed install: refused\nfailed\n",
        )
        .unwrap();
        assert!(!judge(results.path()).unwrap());
        fs::write(results.path().join(RESULTS_FILE), "ok verify\npassed\n").unwrap();
        assert!(judge(results.path()).unwrap());
        let kept = fs::read_to_string(results.path().join(RESULTS_FILE)).unwrap();
        assert!(kept.ends_with("host: passed\n"), "{kept}");
    }
}
