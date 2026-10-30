//! The `bb-sample` program: makes the sample organization in a folder. `cargo xtask dataset
//! sample` runs it; so can a person.

#![forbid(unsafe_code)]

use std::io::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;

use bb_core::message::{self, Message, MessageKey};
use bb_dev_words::database::{DatabaseKind, parse as parse_database};
use bb_sample::{
    DEFAULT_SEED, Everything, Large, Layout, Options, Output, SampleError, Target, create,
};
use clap::{CommandFactory as _, FromArgMatches as _, Parser};
use jiff::Timestamp;

// The library uses these; Cargo gives every dependency to the program too
// (`unused_crate_dependencies`).
use bb_app as _;
use bb_auth as _;
use bb_files as _;
use bb_searchlang as _;
use bb_store as _;
use getrandom as _;
use serde as _;
use serde_json as _;
use toml as _;
use zeroize as _;

/// The text of a message without placeholders, for the help texts of the command line.
fn text(key: MessageKey) -> String {
    Message::from(key).render()
}

#[derive(Parser)]
#[command(name = "bb-sample", about = text(message::CLI_ABOUT_SAMPLE))]
struct Args {
    #[arg(long, default_value_t = DEFAULT_SEED, help = text(message::CLI_HELP_SAMPLE_SEED))]
    seed: u64,
    #[arg(long, help = text(message::CLI_HELP_SAMPLE_START))]
    start: i64,
    // Its help names the environment variable of the layout, so it is set in `arguments`.
    #[arg(long, default_value_t = DatabaseKind::Sqlite, value_parser = parse_database)]
    database: DatabaseKind,
    #[arg(long, help = text(message::CLI_HELP_SAMPLE_ISSUES))]
    issues: Option<u32>,
    #[arg(long, help = text(message::CLI_HELP_SAMPLE_LARGE))]
    large: bool,
    #[arg(long, requires = "large", help = text(message::CLI_HELP_SAMPLE_CHECK))]
    check: bool,
    #[arg(long, help = text(message::CLI_HELP_SAMPLE_OUT))]
    out: PathBuf,
}

async fn run(args: &Args, layout: &Layout) -> Result<String, SampleError> {
    let start = Timestamp::from_second(args.start).map_err(|_| SampleError::Start(args.start))?;
    let output = Output::prepare(&args.out, layout)?;
    output.mark(args.seed, args.start)?;
    let target = match args.database {
        DatabaseKind::Sqlite => Target::BuiltIn,
        DatabaseKind::Postgres => Target::Address(
            std::env::var(&layout.database_url_variable)
                .map_err(|_| SampleError::DatabaseUrl(layout.database_url_variable.clone()))?,
        ),
    };
    let options = Options {
        seed: args.seed,
        start,
    };
    let mut checked = None;
    let sample = if args.large {
        let mut large = Large::new(args.issues, args.check)?;
        let sample = create(&options, &output.data_folder(), &target, &mut large).await?;
        checked = Some(sample.check_large(large.facts(), large.is_check()).await?);
        sample
    } else {
        let mut everything = match args.issues {
            Some(issues) => Everything::new(issues)?,
            None => Everything::full()?,
        };
        create(&options, &output.data_folder(), &target, &mut everything).await?
    };
    sample.verify_security_log().await?;
    if let Some(report) = checked {
        // The dump of the sample names its own projects; the large dataset has the check instead.
        let said = message::sample_large_checked(
            format!("{:016x}", report.fingerprint),
            report.issues,
            report.sampled,
        );
        let _ = writeln!(std::io::stdout(), "{}", said.render());
        if let Some(problem) = report.problems.first() {
            return Err(SampleError::Large(format!(
                "{} problems, the first: {problem}",
                report.problems.len()
            )));
        }
    } else {
        output.write_dump(&sample.dump().await?.to_json()?)?;
    }
    output.write_sign_in(&sample.sign_in_text())?;
    let done = message::sample_done(
        output.sign_in_file().display(),
        output.data_folder().display(),
        sample.people().len(),
        args.seed,
    );
    sample.refresh_statistics().await?;
    sample.close().await;
    Ok(done.render())
}

/// The arguments, with the help of `--database` naming the variable of the layout.
fn arguments(layout: &Layout) -> Args {
    let matches = Args::command()
        .mut_arg("database", |argument| {
            argument.help(message::cli_help_sample_database(&layout.database_url_variable).render())
        })
        .get_matches();
    Args::from_arg_matches(&matches).unwrap_or_else(|error| error.exit())
}

/// Reads the layout, takes the arguments and makes the sample.
async fn start() -> Result<String, SampleError> {
    let layout = Layout::bundled()?;
    Box::pin(run(&arguments(&layout), &layout)).await
}

#[tokio::main]
async fn main() -> ExitCode {
    match Box::pin(start()).await {
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
