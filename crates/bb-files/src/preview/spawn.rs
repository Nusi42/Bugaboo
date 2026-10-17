//! The server's side of a preview: runs `bugaboo preview` as a child process, gives it the bytes of
//! the file on its standard input, waits for it for at most [`TIME_LIMIT`], and accepts only a small
//! PNG on its standard output (ADR-0032 §4). The child gets nothing else: no environment, no path
//! of the file, the folder of the previews as its working folder.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use bb_core::files::PreviewKind;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
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

/// Runs the child on the file at `input` and leaves the preview in `output`, which is written here
/// and not by the child, and only when what the child sent is a small PNG.
///
/// # Errors
/// The error of the system when the child cannot be started or the preview cannot be written.
pub(crate) async fn create(
    command: &PreviewCommand,
    kind: PreviewKind,
    input: &Path,
    output: &Path,
) -> io::Result<PreviewOutcome> {
    let (outcome, sent) = match &command.runner {
        Runner::Child { program, arguments } => {
            let work = output.parent().unwrap_or_else(|| Path::new("."));
            run_child(program, arguments, command.time_limit, kind, input, work).await?
        }
        #[cfg(any(test, feature = "testing"))]
        Runner::InProcess => {
            let bytes = tokio::fs::read(input).await?;
            let (code, sent) = tokio::task::spawn_blocking(move || {
                let mut sent = Vec::new();
                let code = super::run_streams(kind, &bytes[..], &mut sent);
                (code, sent)
            })
            .await
            .map_err(io::Error::other)?;
            if code == 0 {
                (PreviewOutcome::Created, sent)
            } else {
                (PreviewOutcome::Without(Some(code)), sent)
            }
        }
    };
    if outcome != PreviewOutcome::Created {
        return Ok(outcome);
    }
    if !is_small_png(&sent) {
        return Ok(PreviewOutcome::Without(Some(0)));
    }
    tokio::fs::write(output, &sent).await?;
    Ok(PreviewOutcome::Created)
}

/// Starts the child with nothing of the server's, feeds it the file, collects what it sends (never
/// more than [`MAX_OUTPUT_BYTES`] and one byte), and waits for it, for at most `time_limit`.
async fn run_child(
    program: &Path,
    arguments: &[OsString],
    time_limit: Duration,
    kind: PreviewKind,
    input: &Path,
    work: &Path,
) -> io::Result<(PreviewOutcome, Vec<u8>)> {
    let mut process = Command::new(program);
    process
        .args(arguments)
        .arg(kind.code())
        .env_clear()
        .current_dir(work)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    #[cfg(windows)]
    process.creation_flags(CREATE_NO_WINDOW);
    let mut child = process.spawn()?;
    let (stdin, stdout) = (child.stdin.take(), child.stdout.take());
    // The file goes to the child piece by piece, and the pipe is closed after it. A child that
    // ends before it has read everything makes this fail, which is not worth knowing.
    let feed = async {
        if let (Some(mut stdin), Ok(mut file)) = (stdin, tokio::fs::File::open(input).await) {
            tokio::io::copy(&mut file, &mut stdin).await.ok();
            stdin.shutdown().await.ok();
        }
    };
    // One byte more than a preview may have is read, so one that is bigger is seen as bigger; the
    // pipe is closed after that, so a child that goes on writing ends.
    let collect = async {
        let mut sent = Vec::new();
        if let Some(stdout) = stdout {
            stdout
                .take(MAX_OUTPUT_BYTES + 1)
                .read_to_end(&mut sent)
                .await
                .ok();
        }
        sent
    };
    let run = async {
        let ((), sent) = tokio::join!(feed, collect);
        (child.wait().await, sent)
    };
    if let Ok((status, sent)) = tokio::time::timeout(time_limit, run).await {
        return Ok((
            match status?.code() {
                Some(0) => PreviewOutcome::Created,
                code => PreviewOutcome::Without(code),
            },
            sent,
        ));
    }
    // Stopping a child that is already gone is no error worth knowing.
    child.kill().await.ok();
    Ok((PreviewOutcome::TimedOut, Vec::new()))
}

/// Whether the bytes are a PNG no bigger than [`MAX_OUTPUT_BYTES`].
fn is_small_png(bytes: &[u8]) -> bool {
    u64::try_from(bytes.len()).is_ok_and(|length| length <= MAX_OUTPUT_BYTES)
        && bytes.starts_with(PNG_SIGNATURE)
}
