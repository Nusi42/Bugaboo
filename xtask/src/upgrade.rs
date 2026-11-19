//! `cargo xtask upgrade-test` (TEST-UPGRADE, ADR-0046 decision 5, architecture §11 "The upgrade
//! test"): a database made by the program of the previous milestone is upgraded by the program of
//! `HEAD`, on SQLite and on PostgreSQL, and reads the same afterwards.
//!
//! 1. The previous milestone is the newest tag `mN-done` before `HEAD`, which Git finds; no tag
//!    is written down here.
//! 2. Its sample program is built once, with optimisations, in a `git worktree` under `.work/`
//!    with a build folder of its own. Only the program is kept, in `target/upgrade/<tag>/`; the
//!    worktree and its build folder are deleted, and so is the program of an older tag. A later
//!    run finds the program there and builds nothing.
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
use crate::repo::Repo;
use crate::steps::{self, Profile, SAMPLE_BINARY};

/// The tags that mark a finished milestone; the newest before `HEAD` is the previous milestone.
pub(crate) const MILESTONE_TAGS: &str = "m*-done";
/// Where the old program is kept between runs, one folder for its tag.
const PROGRAMS: &str = "target/upgrade";
/// Where the worktree of the old build and the runs of the test go; both are deleted after.
const WORK: &str = ".work/upgrade";
/// The driver that starts the new program on the old data and reads it through the API.
const DRIVER: &str = "web/tooling/upgrade/main.ts";
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

/// The file name of the sample program on this system.
fn program_name() -> String {
    format!("{SAMPLE_BINARY}{}", std::env::consts::EXE_SUFFIX)
}

/// The old sample program of `tag`: the one kept, or built now. Everything else under
/// [`PROGRAMS`] goes, so the disk keeps one file.
pub(crate) fn old_program(root: &Path, tag: &str) -> anyhow::Result<PathBuf> {
    let folder = kept_folder(root, tag);
    let program = folder.join(program_name());
    keep_only(&in_folder(root, PROGRAMS), tag)?;
    if program.is_file() {
        output::info(&format!(
            "upgrade test: the program of {tag} is kept in {}",
            folder.display()
        ));
        return Ok(program);
    }
    let work = in_folder(root, WORK).join("build");
    remove(&work)?;
    let built = build_in_worktree(root, tag, &work, &folder);
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

/// Builds the sample program of `tag` in a worktree at `work`, with its own build folder inside
/// it, and copies the program into `keep`.
fn build_in_worktree(root: &Path, tag: &str, work: &Path, keep: &Path) -> anyhow::Result<()> {
    output::info(&format!(
        "upgrade test: building the sample program of {tag} ..."
    ));
    git(
        root,
        &["worktree", "add", "--detach", &work.to_string_lossy(), tag],
    )?;
    let target = work.join("target");
    let status = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()))
        .args(["build", "--release", "--locked", "--package", SAMPLE_BINARY])
        .env("CARGO_TARGET_DIR", &target)
        .current_dir(work)
        .status()
        .context("could not run cargo")?;
    if !status.success() {
        bail!("the sample program of {tag} did not build");
    }
    fs::create_dir_all(keep).with_context(|| format!("could not make {}", keep.display()))?;
    let built = target.join("release").join(program_name());
    keep_whole(&built, &keep.join(program_name()))
        .with_context(|| format!("could not keep {}", built.display()))
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
