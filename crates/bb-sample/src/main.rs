//! The `bb-sample` program: makes the sample organization in a folder. `cargo xtask dataset
//! sample` runs it; so can a person.

#![forbid(unsafe_code)]

use std::io::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;

use bb_core::message::{self, Message, MessageKey};
use bb_sample::{
    DATABASE_URL_VARIABLE, DEFAULT_SEED, DUMP_FILE, NothingMore, Options, Output, SIGN_IN_FILE,
    SampleError, Target, make,
};
use clap::{Parser, ValueEnum};
use jiff::Timestamp;

// The library uses these; Cargo gives every dependency to the program too
// (`unused_crate_dependencies`).
use bb_app as _;
use bb_auth as _;
use bb_store as _;
use getrandom as _;
use serde as _;
use serde_json as _;
use toml as _;

/// The text of a message without placeholders, for the help texts of the command line.
fn text(key: MessageKey) -> String {
    Message::from(key).render()
}

/// Where the sample is kept.
#[derive(Clone, Copy, Debug, ValueEnum)]
enum DatabaseChoice {
    #[value(name = "sqlite")]
    Sqlite,
    #[value(name = "postgres")]
    Postgres,
}

#[derive(Parser)]
#[command(name = "bb-sample", about = text(message::CLI_ABOUT_SAMPLE))]
struct Args {
    #[arg(long, default_value_t = DEFAULT_SEED, help = text(message::CLI_HELP_SAMPLE_SEED))]
    seed: u64,
    #[arg(long, help = text(message::CLI_HELP_SAMPLE_START))]
    start: i64,
    #[arg(
        long,
        value_enum,
        default_value = "sqlite",
        help = message::cli_help_sample_database(DATABASE_URL_VARIABLE).render()
    )]
    database: DatabaseChoice,
    #[arg(long, help = text(message::CLI_HELP_SAMPLE_OUT))]
    out: PathBuf,
}

async fn run(args: &Args) -> Result<String, SampleError> {
    let start = Timestamp::from_second(args.start).map_err(|_| SampleError::Start(args.start))?;
    let output = Output::prepare(&args.out)?;
    output.mark(args.seed, args.start)?;
    let target = match args.database {
        DatabaseChoice::Sqlite => Target::BuiltIn,
        DatabaseChoice::Postgres => Target::Address(
            std::env::var(DATABASE_URL_VARIABLE).map_err(|_| SampleError::DatabaseUrl)?,
        ),
    };
    let options = Options {
        seed: args.seed,
        start,
    };
    let sample = make(&options, &output.data_folder(), &target, &mut NothingMore).await?;
    sample.verify_security_log().await?;
    output.write(DUMP_FILE, &sample.dump().await?.to_json()?)?;
    output.write(SIGN_IN_FILE, &sample.sign_in_text())?;
    let done = message::sample_done(
        output.sign_in_file().display(),
        output.data_folder().display(),
        sample.people().len(),
        args.seed,
    );
    sample.close().await;
    Ok(done.render())
}

#[tokio::main]
async fn main() -> ExitCode {
    match run(&Args::parse()).await {
        Ok(done) => {
            // Nobody to tell when the console is gone: the sample is made either way.
            let _ = writeln!(std::io::stdout(), "{done}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            let _ = writeln!(std::io::stderr(), "{}", message::server_error(error));
            ExitCode::FAILURE
        }
    }
}

// Used by the tests of the library only; at the end of the file, because the catalog check reads
// a file only up to its first `cfg(test)` (TXT-CATALOG).
#[cfg(test)]
use bb_testing as _;
#[cfg(test)]
use tempfile as _;
