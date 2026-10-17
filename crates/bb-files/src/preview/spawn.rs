//! The server's side of a preview: runs `bugaboo preview` as a child process, waits for it for at
//! most [`TIME_LIMIT`], and accepts only a small PNG from it (ADR-0032 §4).

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use bb_core::files::PreviewKind;
use tokio::io::AsyncReadExt as _;
use tokio::process::Command;

use super::{MAX_OUTPUT_BYTES, TIME_LIMIT};

/// The first bytes of every PNG.
const PNG_SIGNATURE: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// On Windows a child of a service must not open a console window.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// How the child is started: the program and the arguments before the kind of preview, the input
/// and the output (for the server, `bugaboo` and `preview`).
#[derive(Clone, Debug)]
pub struct PreviewCommand {
    runner: Runner,
    time_limit: Duration,
}

/// Who creates the preview.
#[derive(Clone, Debug)]
enum Runner {
    /// A child process: the program and the arguments before the kind, the input and the output.
    Child {
        program: PathBuf,
        arguments: Vec<OsString>,
    },
    /// The tests of the server make the preview themselves.
    #[cfg(any(test, feature = "testing"))]
    InProcess,
}

impl PreviewCommand {
    /// A command that starts `program` with `arguments` first.
    #[must_use]
    pub fn new(program: impl Into<PathBuf>, arguments: &[&str]) -> Self {
        Self {
            runner: Runner::Child {
                program: program.into(),
                arguments: arguments.iter().map(OsString::from).collect(),
            },
            time_limit: TIME_LIMIT,
        }
    }

    /// A command that makes the preview in the process that asks, without a child (tests of the
    /// server only; the limits on pixels, size and output still hold, a crash is not survived).
    #[cfg(any(test, feature = "testing"))]
    #[must_use]
    pub const fn in_process() -> Self {
        Self {
            runner: Runner::InProcess,
            time_limit: TIME_LIMIT,
        }
    }

    /// The command that starts this very program with its subcommand `preview`.
    ///
    /// # Errors
    /// The error of the system when it cannot say which program is running.
    pub fn of_this_program(subcommand: &str) -> io::Result<Self> {
        Ok(Self::new(std::env::current_exe()?, &[subcommand]))
    }

    /// The same command, held to another time than [`TIME_LIMIT`] (the tests of the limit).
    #[must_use]
    pub const fn within(mut self, time_limit: Duration) -> Self {
        self.time_limit = time_limit;
        self
    }
}

/// What came of running the child.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewOutcome {
    /// The child wrote a small PNG.
    Created,
    /// The child ended without a preview: it refused the file (with the exit code it gave) or
    /// crashed (no code).
    Without(Option<i32>),
    /// The child took longer than the limit and was stopped.
    TimedOut,
}

/// Runs the child on `input` and leaves the preview in `output`. The output is removed again
/// unless it is a small PNG.
///
/// # Errors
/// The error of the system when the child cannot be started.
pub(crate) async fn create(
    command: &PreviewCommand,
    kind: PreviewKind,
    input: &Path,
    output: &Path,
) -> io::Result<PreviewOutcome> {
    let outcome = match &command.runner {
        Runner::Child { program, arguments } => {
            run_child(program, arguments, command.time_limit, kind, input, output).await?
        }
        #[cfg(any(test, feature = "testing"))]
        Runner::InProcess => {
            let (input, output) = (input.to_owned(), output.to_owned());
            let code = tokio::task::spawn_blocking(move || super::run(kind, &input, &output))
                .await
                .map_err(io::Error::other)?;
            if code == 0 {
                PreviewOutcome::Created
            } else {
                PreviewOutcome::Without(Some(code))
            }
        }
    };
    if outcome == PreviewOutcome::Created && !is_small_png(output).await {
        tokio::fs::remove_file(output).await.ok();
        return Ok(PreviewOutcome::Without(Some(0)));
    }
    if outcome != PreviewOutcome::Created {
        tokio::fs::remove_file(output).await.ok();
    }
    Ok(outcome)
}

/// Starts the child and waits for it, for at most `time_limit`.
async fn run_child(
    program: &Path,
    arguments: &[OsString],
    time_limit: Duration,
    kind: PreviewKind,
    input: &Path,
    output: &Path,
) -> io::Result<PreviewOutcome> {
    let mut process = Command::new(program);
    process
        .args(arguments)
        .arg(kind.code())
        .arg(input)
        .arg(output)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    #[cfg(windows)]
    process.creation_flags(CREATE_NO_WINDOW);
    let mut child = process.spawn()?;
    if let Ok(status) = tokio::time::timeout(time_limit, child.wait()).await {
        return Ok(match status?.code() {
            Some(0) => PreviewOutcome::Created,
            code => PreviewOutcome::Without(code),
        });
    }
    // Stopping a child that is already gone is no error worth knowing.
    child.kill().await.ok();
    Ok(PreviewOutcome::TimedOut)
}

/// Whether the file is a PNG no bigger than [`MAX_OUTPUT_BYTES`].
async fn is_small_png(path: &Path) -> bool {
    let Ok(file) = tokio::fs::File::open(path).await else {
        return false;
    };
    let Ok(metadata) = file.metadata().await else {
        return false;
    };
    if metadata.len() > MAX_OUTPUT_BYTES {
        return false;
    }
    let mut signature = [0_u8; PNG_SIGNATURE.len()];
    let mut file = file;
    file.read_exact(&mut signature).await.is_ok() && signature == PNG_SIGNATURE
}
