//! `cargo xtask dataset large [--issues N] [--database sqlite|postgres] [--check]`: makes the large
//! dataset of PERF-01 (architecture §11, "Large dataset") with the `bb-sample` program, one dataset
//! for each database in the folder `large.out` of `tools/dev/sample.json` (inside `.work/`), and
//! says how long it took and how much disk it takes. An older dataset of the database is thrown
//! away first. With `--check` it makes a small one twice on each database instead and holds the
//! runs against each other: the program checks the rules of bulk data itself and prints a
//! fingerprint, which must be the same both times.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};
use std::time::Instant;

use anyhow::{Context as _, bail};
use bb_dev_words::database::DatabaseKind;
use bb_dev_words::sample::Layout;

use crate::dataset::{
    Arguments, CHECK_START, Request, in_folder, load_layout, now, program_command,
};
use crate::db;
use crate::output;
use crate::repo::Repo;

/// Where the migrations are; the name of the newest file says which schema a dataset was made for.
const MIGRATIONS: &str = "crates/bb-store/src/migrations";

/// The file in the folder of a dataset that holds the name of the newest migration at the time the
/// dataset was made (the nightly run compares names, not file times, which a checkout changes).
const MADE_FOR: &str = "made-for";

/// What the program prints before the fingerprint of a run.
const FINGERPRINT_WORD: &str = "Fingerprint ";

/// The bytes the files below `folder` take.
pub(crate) fn folder_size(folder: &Path) -> u64 {
    fs::read_dir(folder).map_or(0, |entries| {
        entries
            .flatten()
            .map(|entry| match entry.metadata() {
                Ok(meta) if meta.is_dir() => folder_size(&entry.path()),
                Ok(meta) => meta.len(),
                Err(_) => 0,
            })
            .sum()
    })
}

/// A size for a person: megabytes or gigabytes.
pub(crate) fn shown(bytes: u64) -> String {
    const MEGA: u64 = 1_048_576;
    let mega = bytes / MEGA;
    if mega >= 1024 {
        format!(
            "{:.1} GB",
            f64::from(u32::try_from(mega).unwrap_or(u32::MAX)) / 1024.0
        )
    } else {
        format!("{mega} MB")
    }
}

/// The fingerprint a run printed.
fn fingerprint(said: &str) -> Option<String> {
    said.lines().find_map(|line| {
        line.split_once(FINGERPRINT_WORD)
            .map(|(_, rest)| rest.trim().to_owned())
    })
}

/// What a run of the program printed.
fn said(made: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&made.stdout),
        String::from_utf8_lossy(&made.stderr)
    )
}

/// The name of the newest migration (`m0022_name.rs`: the files are numbered, `mod.rs` is none).
pub(crate) fn newest_migration(root: &Path) -> Option<String> {
    fs::read_dir(root.join(MIGRATIONS))
        .ok()?
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| {
            name.strip_prefix('m')
                .is_some_and(|rest| rest.starts_with(|first: char| first.is_ascii_digit()))
        })
        .max()
}

/// The migration a dataset in `folder` was made for; none when it has none.
pub(crate) fn made_for(folder: &Path) -> Option<String> {
    fs::read_to_string(folder.join(MADE_FOR))
        .ok()
        .map(|text| text.trim().to_owned())
}

/// The folder of the dataset of `database`.
pub(crate) fn dataset_folder(repo: &Repo, layout: &Layout, database: DatabaseKind) -> PathBuf {
    in_folder(repo.root(), &layout.large.out).join(database.code())
}

pub(crate) fn run(repo: &Repo, program: &Path, request: &Request) -> anyhow::Result<bool> {
    let layout = load_layout(repo.root())?;
    // Making the dataset takes the computer for an hour: one full run at a time, as `ci` and
    // `bench` do. `--check` is small and takes no lock.
    let _lock = (!request.check)
        .then(|| crate::nightly::FullRun::take(repo))
        .transpose()?;
    let databases = if request.databases.is_empty() {
        DatabaseKind::ALL.to_vec()
    } else {
        request.databases.clone()
    };
    let mut all_held = true;
    for database in databases {
        // Every database is tried, so one failure does not hide the other.
        all_held &= if request.check {
            check_on(repo, program, request, &layout, database)
        } else {
            build_on(repo, program, request, &layout, database)?
        };
    }
    Ok(all_held)
}

/// What a run of the program is told besides the database.
struct Run<'a> {
    out: &'a Path,
    start: i64,
    issues: Option<u32>,
}

fn run_program(
    repo: &Repo,
    program: &Path,
    (request, layout): (&Request, &Layout),
    database: DatabaseKind,
    (run, url): (&Run<'_>, Option<&str>),
) -> anyhow::Result<Output> {
    program_command(
        program,
        repo.root(),
        &Arguments {
            large: true,
            seed: request.seed,
            start: run.start,
            database,
            issues: run.issues,
            out: run.out,
        },
        (layout, url),
    )
    .stdin(Stdio::null())
    .output()
    .with_context(|| format!("could not start {}", program.display()))
}

fn build_on(
    repo: &Repo,
    program: &Path,
    request: &Request,
    layout: &Layout,
    database: DatabaseKind,
) -> anyhow::Result<bool> {
    let out = dataset_folder(repo, layout, database);
    if out.exists() {
        fs::remove_dir_all(&out)
            .with_context(|| format!("could not remove the old {}", out.display()))?;
    }
    fs::create_dir_all(&out).with_context(|| format!("could not make {}", out.display()))?;
    output::info(&format!(
        "dataset large: {} issues on {} ...",
        request.issues.unwrap_or(layout.large.issues),
        database.code()
    ));
    let began = Instant::now();
    let run = Run {
        out: &out,
        start: now()?,
        issues: request.issues,
    };
    let both = (request, layout);
    let (made, size) = match database {
        DatabaseKind::Sqlite => {
            let made = run_program(repo, program, both, database, (&run, None))?;
            (made, folder_size(&out))
        }
        DatabaseKind::Postgres => db::with_postgres(repo, |postgres| {
            let url = postgres.empty_database(&layout.large.database)?;
            let made = run_program(repo, program, both, database, (&run, Some(&url)))?;
            let size = postgres.size_of(&layout.large.database)? + folder_size(&out);
            Ok((made, size))
        })?,
    };
    output::info(said(&made).trim());
    if !made.status.success() {
        return Ok(false);
    }
    fs::write(
        out.join(MADE_FOR),
        newest_migration(repo.root()).unwrap_or_default(),
    )
    .with_context(|| format!("could not write {MADE_FOR} in {}", out.display()))?;
    output::info(&format!(
        "dataset large: {} done in {:.1} minutes, {} on disk.",
        database.code(),
        began.elapsed().as_secs_f64() / 60.0,
        shown(size)
    ));
    Ok(true)
}

/// Makes the small dataset twice on `database`; the fingerprints it printed.
fn two_runs(
    repo: &Repo,
    program: &Path,
    (request, layout): (&Request, &Layout),
    database: DatabaseKind,
) -> anyhow::Result<Vec<Option<String>>> {
    let folder = in_folder(repo.root(), &layout.large.check_out);
    let issues = Some(request.issues.unwrap_or(layout.large.check_issues));
    db::with_postgres(repo, |postgres| {
        let mut prints = Vec::new();
        for number in 1..=2 {
            let out = folder.join(format!("{}-{number}", database.code()));
            if out.exists() {
                fs::remove_dir_all(&out)?;
            }
            fs::create_dir_all(&out)?;
            let name = format!("{}_check_{number}", layout.large.database);
            let url = match database {
                DatabaseKind::Sqlite => None,
                DatabaseKind::Postgres => Some(postgres.empty_database(&name)?),
            };
            let run = Run {
                out: &out,
                start: CHECK_START,
                issues,
            };
            let made = run_program(
                repo,
                program,
                (request, layout),
                database,
                (&run, url.as_deref()),
            )?;
            if database == DatabaseKind::Postgres {
                postgres.drop_database(&name)?;
            }
            fs::remove_dir_all(&out)?;
            if !made.status.success() {
                bail!(
                    "The large dataset did not hold its rules on {}:\n{}",
                    database.code(),
                    said(&made)
                );
            }
            prints.push(fingerprint(&said(&made)));
        }
        Ok(prints)
    })
}

fn check_on(
    repo: &Repo,
    program: &Path,
    request: &Request,
    layout: &Layout,
    database: DatabaseKind,
) -> bool {
    let prints = match two_runs(repo, program, (request, layout), database) {
        Ok(prints) => prints,
        Err(error) => {
            output::info(&format!("TEST-SAMPLE-DATA: {error:#}"));
            return false;
        }
    };
    let same = prints.len() == 2 && prints[0].is_some() && prints[0] == prints[1];
    output::info(&if same {
        format!(
            "dataset large --check: two runs on {} hold the rules and are the same ({}).",
            database.code(),
            prints[0].clone().unwrap_or_default()
        )
    } else {
        format!(
            "dataset large --check: two runs on {} are not the same: {prints:?}",
            database.code()
        )
    });
    same
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fingerprint_is_read_from_the_line_the_program_prints() {
        let said = "Checked the large dataset: 3 issues. Fingerprint 0ffb\nCreated.";
        assert_eq!(fingerprint(said).as_deref(), Some("0ffb"));
        assert_eq!(fingerprint("nothing"), None);
    }

    #[test]
    fn the_newest_migration_is_the_highest_numbered_file() {
        let folder = tempfile::tempdir().unwrap();
        let migrations = folder.path().join(MIGRATIONS);
        fs::create_dir_all(&migrations).unwrap();
        for name in ["m0002_b.rs", "m0010_c.rs", "m0009_a.rs", "mod.rs"] {
            fs::write(migrations.join(name), "").unwrap();
        }
        assert_eq!(
            newest_migration(folder.path()).as_deref(),
            Some("m0010_c.rs")
        );
        assert_eq!(made_for(folder.path()), None);
        fs::write(
            folder.path().join(MADE_FOR),
            "m0010_c.rs
",
        )
        .unwrap();
        assert_eq!(made_for(folder.path()).as_deref(), Some("m0010_c.rs"));
    }

    #[test]
    fn sizes_are_shown_in_megabytes_and_gigabytes() {
        assert_eq!(shown(5 * 1_048_576), "5 MB");
        assert_eq!(shown(3 * 1_073_741_824), "3.0 GB");
    }

    #[test]
    fn a_folder_that_is_not_there_has_no_size() {
        assert_eq!(folder_size(Path::new("a-folder-that-is-never-made")), 0);
    }
}
