//! The `bb-load-check` program: judges a run of `cargo xtask load` against its book. `cargo xtask
//! load` runs it after the driver; so can a person.
//!
//! `bb-load-check --book <file> (--database-url <address> | --data-dir <folder>)`

#![forbid(unsafe_code)]

use std::io::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use bb_core::edit::LEASE_SECONDS;
use bb_load_check::{Afterwards, Book, Failure, check};
use bb_store::{DATABASE_FILE, Database};

// The library uses these; Cargo gives every dependency to the program too
// (`unused_crate_dependencies`).
use bb_app as _;
use serde as _;

/// Where the database of the run is and what its people wrote.
struct Arguments {
    book: PathBuf,
    database_url: Option<String>,
    data_dir: Option<PathBuf>,
}

fn arguments() -> Result<Arguments, Failure> {
    let mut book = None;
    let mut database_url = None;
    let mut data_dir = None;
    let mut given = std::env::args().skip(1);
    while let Some(flag) = given.next() {
        let value = given
            .next()
            .ok_or_else(|| format!("{flag} needs a value"))?;
        match flag.as_str() {
            "--book" => book = Some(PathBuf::from(value)),
            "--database-url" => database_url = Some(value),
            "--data-dir" => data_dir = Some(PathBuf::from(value)),
            other => return Err(format!("unknown flag {other}").into()),
        }
    }
    Ok(Arguments {
        book: book.ok_or("--book is needed")?,
        database_url,
        data_dir,
    })
}

async fn open(arguments: &Arguments) -> Result<Database, Failure> {
    match (&arguments.database_url, &arguments.data_dir) {
        (Some(url), None) => Ok(Database::connect(url).await?),
        (None, Some(folder)) => Ok(Database::connect_file(&folder.join(DATABASE_FILE)).await?),
        _ => Err("name the database with --database-url or --data-dir, one of them".into()),
    }
}

async fn run() -> Result<Vec<String>, Failure> {
    let arguments = arguments()?;
    let book: Book = serde_json::from_str(&std::fs::read_to_string(&arguments.book)?)?;
    let database = open(&arguments).await?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    // The server ran on the clock of this computer, and the leases are over by the end of a run.
    // The purge belongs to the job runner of the server (`bb-jobs`): no row is asked to be gone.
    let afterwards = Afterwards {
        now: i64::try_from(now)?,
        lease_seconds: LEASE_SECONDS,
        purged: false,
    };
    let found = check(&database, &book, &afterwards).await?;
    database.close().await;
    Ok(found)
}

#[tokio::main]
async fn main() -> ExitCode {
    let mut out = std::io::stdout();
    let mut err = std::io::stderr();
    match run().await {
        Ok(found) if found.is_empty() => {
            let _ = writeln!(out, "every rule held");
            ExitCode::SUCCESS
        }
        Ok(found) => {
            let _ = writeln!(err, "{} things did not hold:", found.len());
            for line in found {
                let _ = writeln!(err, "{line}");
            }
            ExitCode::FAILURE
        }
        Err(error) => {
            let _ = writeln!(err, "{error}");
            ExitCode::FAILURE
        }
    }
}
