//! `cargo xtask upgrade-test` (TEST-UPGRADE, ADR-0046 decision 5, architecture §11 "The upgrade
//! test"): a database made by the program of the previous milestone is upgraded by the program of
//! `HEAD`, on SQLite and on PostgreSQL, and reads the same afterwards.
//!
//! 1. The previous milestone is the newest tag `mN-done` before `HEAD`, which Git finds; no tag
//!    is written down here.
//! 2. Its sample program is built once, with optimisations, in a `git worktree` under `.work/`
//!    with a build folder of its own. Only the program is kept, in `target/upgrade/<tag>/`; the
//!    worktree and its build folder are deleted, and so is the program of an older tag. A later
//!    run finds the program there and builds nothing. The try-out of the install
//!    (`cargo xtask try-install`) takes the server program of the same tag the same way, for
//!    Windows and, built in WSL, for Linux ([`OldProgram`]).
//! 3. On each database the old program makes the small sample organization, the way
//!    `cargo xtask dataset sample` does.
//! 4. The driver `web/tooling/upgrade/main.ts` starts the program of `HEAD` on that data folder
//!    (the server makes the backup `before-upgrade` and runs every migration, or refuses to start),
//!    signs in as an administrator of the sample, and holds what the API reads against the dump
//!    the old program wrote: the counts of issues, comments, time entries, work days, away times,
//!    own themes and agents, a list of known records found by their keys, the backup and its
//!    record, and the security log, which must verify.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, bail};
use bb_dev_words::database::DatabaseKind;

use crate::dataset::{self, Arguments, in_folder, load_layout, program_command};
use crate::db;
use crate::output;
use crate::release::folder::{LINUX, Platform};
use crate::repo::Repo;
use crate::steps::{self, Profile, SAMPLE_BINARY, SERVER_BINARY};
use crate::wsl;

/// The tags that mark a finished milestone; the newest before `HEAD` is the previous milestone.
pub(crate) const MILESTONE_TAGS: &str = "m*-done";
/// Where the old program is kept between runs, one folder for its tag.
const PROGRAMS: &str = "target/upgrade";
/// Where the worktree of the old build and the runs of the test go; both are deleted after.
const WORK: &str = ".work/upgrade";
/// The driver that starts the new program on the old data and reads it through the API.
const DRIVER: &str = "web/tooling/upgrade/main.ts";
/// How this checkout builds (the C runtime inside the Windows programs, for one): an old tag is
/// built with the settings of `HEAD`, so its program starts where this one does.
const BUILD_SETTINGS: &str = ".cargo/config.toml";
/// The database of the throwaway cluster that holds the old sample on PostgreSQL.
const DATABASE: &str = "bbupgrade";

/// The newest milestone tag before `HEAD`, as Git finds it.
pub(crate) fn previous_tag(root: &Path) -> anyhow::Result<String> {
    let found = Command::new("git")
        .args([
            "describe",
            "--tags",
            "--abbrev=0",
            "--match",
            MILESTONE_TAGS,
            "HEAD^",
        ])
        .current_dir(root)
        .output()
        .context("could not run git")?;
    let tag = String::from_utf8_lossy(&found.stdout).trim().to_owned();
    if !found.status.success() || tag.is_empty() {
        bail!(
            "Git found no tag {MILESTONE_TAGS} before HEAD: {}",
            String::from_utf8_lossy(&found.stderr).trim()
        );
    }
    Ok(tag)
}

/// The folder that keeps the old program of `tag`.
fn kept_folder(root: &Path, tag: &str) -> PathBuf {
    in_folder(root, PROGRAMS).join(tag)
}

/// A program of an older tag that is kept: the sample program of the upgrade test, or the server
/// program for the try-out of the install, for this system or (built in WSL) for Linux.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OldProgram {
    Sample,
    Server,
    LinuxServer,
}

impl OldProgram {
    fn package(self) -> &'static str {
        match self {
            Self::Sample => SAMPLE_BINARY,
            Self::Server | Self::LinuxServer => "bb-server",
        }
    }

    /// The name of the kept file, inside the folder of its tag; the Linux program in a folder of
    /// its own, since its name is the same as the server's without an ending.
    fn kept_name(self) -> PathBuf {
        match self {
            Self::Sample => {
                PathBuf::from(format!("{SAMPLE_BINARY}{}", std::env::consts::EXE_SUFFIX))
            }
            Self::Server => {
                PathBuf::from(format!("{SERVER_BINARY}{}", std::env::consts::EXE_SUFFIX))
            }
            Self::LinuxServer => Path::new(LINUX.word).join(linux_name(LINUX)),
        }
    }
}

/// The name of the server program for `platform`.
fn linux_name(platform: Platform) -> String {
    format!("{SERVER_BINARY}{}", platform.end)
}

/// The file name of the sample program on this system.
#[cfg(test)]
fn program_name() -> String {
    OldProgram::Sample
        .kept_name()
        .to_string_lossy()
        .into_owned()
}

/// The old sample program of `tag`: the one kept, or built now. Everything else under
/// [`PROGRAMS`] goes, so the disk keeps the programs of one tag.
pub(crate) fn old_program(root: &Path, tag: &str) -> anyhow::Result<PathBuf> {
    kept_program(root, tag, OldProgram::Sample)
}

/// The program `which` of `tag`: the one kept, or built now.
pub(crate) fn kept_program(root: &Path, tag: &str, which: OldProgram) -> anyhow::Result<PathBuf> {
    let folder = kept_folder(root, tag);
    let program = folder.join(which.kept_name());
    keep_only(&in_folder(root, PROGRAMS), tag)?;
    if program.is_file() {
        output::info(&format!(
            "the {} program of {tag} is kept in {}",
            which.package(),
            folder.display()
        ));
        return Ok(program);
    }
    let work = in_folder(root, WORK).join("build");
    remove(&work)?;
    let built = build_in_worktree(root, tag, &work, which, &program);
    // The worktree and its build folder go whether the build went well or not.
    let _ = git(
        root,
        &["worktree", "remove", "--force", &work.to_string_lossy()],
    );
    remove(&work)?;
    let _ = git(root, &["worktree", "prune"]);
    built?;
    Ok(program)
}

/// Deletes every folder under `programs` but the one of `tag`.
fn keep_only(programs: &Path, tag: &str) -> anyhow::Result<()> {
    let Ok(entries) = fs::read_dir(programs) else {
        return Ok(());
    };
    for entry in entries {
        let entry = entry?;
        if entry.file_name() != tag {
            remove(&entry.path())?;
        }
    }
    Ok(())
}

fn remove(path: &Path) -> anyhow::Result<()> {
    match fs::remove_dir_all(path) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            Err(error).with_context(|| format!("could not delete {}", path.display()))
        }
        _ => Ok(()),
    }
}

fn git(root: &Path, arguments: &[&str]) -> anyhow::Result<()> {
    let status = Command::new("git")
        .args(arguments)
        .current_dir(root)
        .status()
        .context("could not run git")?;
    if !status.success() {
        bail!("git {} failed", arguments.join(" "));
    }
    Ok(())
}

/// The command that builds `which` with optimisations in the worktree at `work`, with a build
/// folder of its own inside it, and where the program then is. The Linux program is built in the
/// WSL distribution of the checkout at `root` ([`wsl::build_distribution`]); `work_in_wsl` is the
/// worktree as WSL names it.
fn build_command(
    root: &Path,
    work: &Path,
    work_in_wsl: Option<&str>,
    which: OldProgram,
) -> (Command, PathBuf) {
    let target = work.join("target");
    if let (OldProgram::LinuxServer, Some(in_wsl)) = (which, work_in_wsl) {
        let triple = LINUX.target.unwrap_or_default();
        let script = format!(
            "CARGO_TARGET_DIR=target/wsl cargo build --release --locked --package {} --target {triple}",
            which.package()
        );
        let mut command = Command::new(wsl::PROGRAM);
        command.args(wsl::arguments(
            wsl::build_distribution(root).as_deref(),
            &["--cd", in_wsl, "--exec", "bash", "-lc", &script],
        ));
        let built = target
            .join("wsl")
            .join(triple)
            .join("release")
            .join(linux_name(LINUX));
        return (command, built);
    }
    let mut command = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()));
    command
        .args([
            "build",
            "--release",
            "--locked",
            "--package",
            which.package(),
        ])
        .arg("--config")
        .arg(root.join(BUILD_SETTINGS))
        .env("CARGO_TARGET_DIR", &target)
        .current_dir(work);
    let name = which.kept_name();
    let built = target
        .join("release")
        .join(name.file_name().unwrap_or_default());
    (command, built)
}

/// Builds `which` of `tag` in a worktree at `work` and copies the program to `keep`.
fn build_in_worktree(
    root: &Path,
    tag: &str,
    work: &Path,
    which: OldProgram,
    keep: &Path,
) -> anyhow::Result<()> {
    output::info(&format!(
        "building the {} program of {tag} ...",
        which.package()
    ));
    git(
        root,
        &["worktree", "add", "--detach", &work.to_string_lossy(), tag],
    )?;
    let in_wsl = match which {
        OldProgram::LinuxServer => Some(crate::release::assemble::wsl_path(
            work,
            wsl::build_distribution(root).as_deref(),
        )?),
        OldProgram::Sample | OldProgram::Server => None,
    };
    let (mut command, built) = build_command(root, work, in_wsl.as_deref(), which);
    let status = command.status().context("could not start the build")?;
    if !status.success() {
        bail!("the {} program of {tag} did not build", which.package());
    }
    if let Some(folder) = keep.parent() {
        fs::create_dir_all(folder)
            .with_context(|| format!("could not make {}", folder.display()))?;
    }
    keep_whole(&built, keep).with_context(|| format!("could not keep {}", built.display()))
}

/// What a program being copied carries after its name until the copy is whole.
const PARTIAL: &str = ".partial";

/// Copies `built` to `kept` through `<kept>.partial` and a rename, so a copy that was cut off is
/// never taken for the kept program by a later run.
fn keep_whole(built: &Path, kept: &Path) -> std::io::Result<()> {
    let mut partial = kept.as_os_str().to_owned();
    partial.push(PARTIAL);
    let partial = PathBuf::from(partial);
    let copied = fs::copy(built, &partial).and_then(|_| fs::rename(&partial, kept));
    if copied.is_err() {
        let _ = fs::remove_file(&partial);
    }
    copied
}

/// Runs the test on both databases; the result says whether both passed.
pub(crate) fn run(repo: &Repo) -> anyhow::Result<bool> {
    let root = repo.root();
    let tag = previous_tag(root)?;
    let old = old_program(root, &tag)?;
    let built = steps::run(root, &[steps::server_build(Profile::Debug)])?;
    let Some(server) = built.executable.filter(|_| built.passed) else {
        return Ok(false);
    };
    let mut passed = true;
    for database in [DatabaseKind::Sqlite, DatabaseKind::Postgres] {
        output::info(&format!(
            "upgrade test: {tag} to HEAD on {} ...",
            database.code()
        ));
        let folder = in_folder(root, WORK).join(database.code());
        remove(&folder)?;
        let result = match database {
            DatabaseKind::Sqlite => run_on(root, &old, &server, &folder, None),
            DatabaseKind::Postgres => db::with_postgres(repo, |postgres| {
                run_on(
                    root,
                    &old,
                    &server,
                    &folder,
                    Some(&postgres.empty_database(DATABASE)?),
                )
            }),
        };
        remove(&folder)?;
        passed &= result?;
    }
    Ok(passed)
}

/// One run: the old program makes the sample in `folder`, the driver upgrades and reads it.
fn run_on(
    root: &Path,
    old: &Path,
    server: &Path,
    folder: &Path,
    url: Option<&str>,
) -> anyhow::Result<bool> {
    let layout = load_layout(root)?;
    let database = if url.is_some() {
        DatabaseKind::Postgres
    } else {
        DatabaseKind::Sqlite
    };
    let made = program_command(
        old,
        root,
        &Arguments {
            large: false,
            check: false,
            seed: None,
            start: dataset::now()?,
            database,
            issues: Some(layout.issues.small_total),
            out: folder,
        },
        (&layout, url),
    )
    .status()
    .with_context(|| format!("could not start {}", old.display()))?;
    if !made.success() {
        return Ok(false);
    }
    let mut driver = crate::tools::command("node");
    driver
        .arg(DRIVER)
        .arg("--sample")
        .arg(folder)
        .arg("--database")
        .arg(database.code())
        .current_dir(root)
        .env("BB_SERVER_BIN", server);
    if let Some(url) = url {
        driver.env(&layout.database_url_variable, url);
    }
    Ok(driver
        .status()
        .context("could not start node; run `cargo xtask setup`")?
        .success())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ops_06_the_previous_milestone_is_found_by_git() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let tag = previous_tag(&root).unwrap();
        assert!(tag.starts_with('m') && tag.ends_with("-done"), "{tag}");
    }

    #[test]
    fn ops_04_the_old_linux_server_is_built_in_wsl_for_the_static_target() {
        let root = tempfile::tempdir().unwrap();
        let work = root.path().join("build");
        let (command, built) = build_command(
            root.path(),
            &work,
            Some("/mnt/c/work/build"),
            OldProgram::LinuxServer,
        );
        assert_eq!(command.get_program(), wsl::PROGRAM);
        let arguments: Vec<String> = command
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect();
        // No distribution is named in the folder's settings: the default one builds.
        assert_eq!(arguments[..2], ["--cd", "/mnt/c/work/build"]);
        let script = arguments.last().unwrap();
        assert!(
            script.contains("--package bb-server --target x86_64-unknown-linux-musl"),
            "{script}"
        );
        assert!(
            built.ends_with("x86_64-unknown-linux-musl/release/bugaboo"),
            "{built:?}"
        );
        // The Windows server and the sample program are built by cargo itself.
        let (command, built) = build_command(root.path(), &work, None, OldProgram::Server);
        assert_ne!(command.get_program(), wsl::PROGRAM);
        assert_eq!(
            built.file_name(),
            OldProgram::Server.kept_name().file_name()
        );
    }

    #[test]
    fn ops_06_the_old_program_is_kept_whole_or_not_at_all() {
        let folder = tempfile::tempdir().unwrap();
        let built = folder.path().join("built");
        fs::write(&built, b"a program").unwrap();
        let kept = folder.path().join(program_name());
        keep_whole(&built, &kept).unwrap();
        assert_eq!(fs::read(&kept).unwrap(), b"a program");
        // A copy that fails leaves neither the program nor the copy on its way.
        let other = folder.path().join("other");
        keep_whole(&folder.path().join("not-built"), &other).unwrap_err();
        let mut left: Vec<_> = fs::read_dir(folder.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        left.sort();
        let mut expected = vec![
            std::ffi::OsString::from("built"),
            kept.file_name().unwrap().to_owned(),
        ];
        expected.sort();
        assert_eq!(left, expected);
    }

    #[test]
    fn ops_06_the_upgrade_test_keeps_only_the_old_program_on_disk() {
        let programs = tempfile::tempdir().unwrap();
        for tag in ["m7-done", "m8-done"] {
            let folder = programs.path().join(tag);
            fs::create_dir_all(&folder).unwrap();
            fs::write(folder.join(program_name()), b"a program").unwrap();
        }
        keep_only(programs.path(), "m8-done").unwrap();
        let left: Vec<_> = fs::read_dir(programs.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(left, ["m8-done"]);
        assert!(
            programs
                .path()
                .join("m8-done")
                .join(program_name())
                .is_file()
        );
    }
}
