//! What can go wrong while the sample is made. Every error says in plain words what it was doing;
//! the words are in `messages/en.json` like all text.

use std::fmt;
use std::path::PathBuf;

use bb_core::message::{self, Message};

/// Why the sample organization could not be made.
#[derive(Debug)]
pub enum SampleError {
    /// The list of names cannot be used.
    Names(String),
    /// The system gave no secure random numbers (for the passwords).
    Random,
    /// A folder or a file could not be written or read.
    Folder {
        /// The folder or file.
        path: PathBuf,
        /// What the system said.
        reason: String,
    },
    /// The folder has other things in it and was not made by this tool, so it is left alone.
    NotASampleFolder(PathBuf),
    /// The database or its key files failed.
    Store(String),
    /// A use case refused a step the sample took, like a person would.
    Step {
        /// What the sample was doing.
        step: &'static str,
        /// What the use case said.
        reason: String,
    },
    /// The security log of the finished sample does not verify.
    LogBroken(String),
    /// The start moment is not a moment in time.
    Start(i64),
    /// The address of the PostgreSQL database is not in the environment.
    DatabaseUrl,
}

impl SampleError {
    /// The error as a message a person can read.
    #[must_use]
    pub fn message(&self) -> Message {
        match self {
            Self::Names(reason) => message::sample_error_names(reason),
            Self::Random => Message::from(message::SAMPLE_ERROR_RANDOM),
            Self::Folder { path, reason } => message::sample_error_folder(path.display(), reason),
            Self::NotASampleFolder(path) => {
                message::sample_error_not_a_sample_folder(path.display())
            }
            Self::Store(reason) => message::sample_error_store(reason),
            Self::Step { step, reason } => message::sample_error_step(step, reason),
            Self::LogBroken(reason) => message::sample_error_log_broken(reason),
            Self::Start(start) => message::sample_error_start(start),
            Self::DatabaseUrl => message::sample_error_database_url(crate::DATABASE_URL_VARIABLE),
        }
    }

    /// A folder error from the error of the file system.
    pub(crate) fn folder(path: impl Into<PathBuf>, error: &std::io::Error) -> Self {
        Self::Folder {
            path: path.into(),
            reason: error.to_string(),
        }
    }

    /// A step the use cases refused.
    pub(crate) fn step(step: &'static str, error: &impl fmt::Display) -> Self {
        Self::Step {
            step,
            reason: error.to_string(),
        }
    }
}

impl fmt::Display for SampleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message().render())
    }
}

impl std::error::Error for SampleError {}

impl From<bb_store::StoreError> for SampleError {
    fn from(error: bb_store::StoreError) -> Self {
        Self::Store(error.to_string())
    }
}
