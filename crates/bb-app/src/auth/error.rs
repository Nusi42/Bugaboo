//! Why a use case did not do what was asked, and how a failed change is answered.

use std::fmt;

use bb_core::ErrorCode;
use bb_policy::PersonRef;

use super::store::AuthStore;
use crate::editing::{Conflict, Stale};

/// Why a use case did not do what was asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppError {
    /// The request was turned away for a reason a person may be told (an error code).
    Refused(ErrorCode),
    /// A `field.*` refusal, with the field it is about (the word of a built-in field or the ID
    /// of an own field), so the client can show it at that field.
    RefusedAtField {
        /// Why the value was refused.
        code: ErrorCode,
        /// The field, as the client names it.
        field: String,
    },
    /// The session of the cookie has ended (it ran out, was ended, or never was one): the same
    /// answer as [`ErrorCode::NotSignedIn`], and the browser is told to forget the cookie and
    /// what it kept (M2-F06). A session that is only not at the stage the request needs (the
    /// password step used where a finished sign-in is needed) is not this: it is still waiting.
    SessionEnded,
    /// The version a change was made from is not the version the record has now. The use case
    /// only names the record; `AuthApi` turns this into [`Self::Conflict`] before the answer goes
    /// out (`editing/conflict.rs`).
    Stale(Box<Stale>),
    /// A stale version, with who changed the record, when, which fields, and the values now.
    Conflict(Box<Conflict>),
    /// A save was refused because somebody took the editing over (`edit.taken-over`), with who,
    /// as the person who saved may see them, so the browser needs no further request to say it.
    TakenOver {
        /// Who holds the lock now.
        by: PersonRef,
    },
    /// Too many failed tries: wait this many seconds.
    Limited {
        /// Seconds until the next try may work.
        retry_after: u64,
    },
    /// Something failed inside the server. The text is for the log only.
    Internal(String),
}

impl AppError {
    /// The error code of a refusal, whatever kind of refusal it is. A session that has ended
    /// is refused as not signed in.
    #[must_use]
    pub const fn code(&self) -> Option<ErrorCode> {
        match self {
            Self::Refused(code) | Self::RefusedAtField { code, .. } => Some(*code),
            Self::SessionEnded => Some(ErrorCode::NotSignedIn),
            Self::Stale(_) | Self::Conflict(_) => Some(ErrorCode::VersionChanged),
            Self::TakenOver { .. } => Some(ErrorCode::EditTakenOver),
            Self::Limited { .. } | Self::Internal(_) => None,
        }
    }
}

/// What a read that may find nothing answers: `None` for "not found", which is a fact about the
/// place or the record that was asked about; every other failure (the store broke) stays a
/// failure and is not turned into an answer.
pub(crate) fn if_there<T>(result: Result<T, AppError>) -> Result<Option<T>, AppError> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(AppError::Refused(ErrorCode::NotFound)) => Ok(None),
        Err(error) => Err(error),
    }
}

impl From<ErrorCode> for AppError {
    fn from(code: ErrorCode) -> Self {
        Self::Refused(code)
    }
}

impl From<bb_auth::AuthError> for AppError {
    fn from(error: bb_auth::AuthError) -> Self {
        Self::Internal(error.to_string())
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Refused(code) | Self::RefusedAtField { code, .. } => {
                write!(formatter, "refused: {code}")
            }
            Self::SessionEnded => write!(formatter, "refused: the session has ended"),
            Self::Stale(_) | Self::Conflict(_) => {
                write!(formatter, "refused: {}", ErrorCode::VersionChanged)
            }
            Self::TakenOver { .. } => write!(formatter, "refused: {}", ErrorCode::EditTakenOver),
            Self::Limited { retry_after } => {
                write!(formatter, "limited for {retry_after} seconds")
            }
            Self::Internal(cause) => write!(formatter, "failed inside the server: {cause}"),
        }
    }
}

impl std::error::Error for AppError {}

/// The answer for a failure of the store: the person gets the standard answer and the cause goes
/// to the log.
pub(crate) fn internal<S: AuthStore>(error: &S::Error) -> AppError {
    AppError::Internal(S::describe(error))
}

/// What a change that did not go through says when it stopped itself (a check inside the
/// transaction failed) and when it hit a duplicate key (two people at once). A rule of the policy
/// engine that stopped it needs no word here: it answers for itself.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Stops {
    aborted: Option<ErrorCode>,
    duplicate: Option<ErrorCode>,
}

impl Stops {
    /// The change cannot stop itself or hit a duplicate: either is an internal failure.
    pub(crate) const NONE: Self = Self {
        aborted: None,
        duplicate: None,
    };

    /// What a change that stopped itself says, and what one that hit a duplicate key says.
    pub(crate) const fn new(aborted: ErrorCode, duplicate: ErrorCode) -> Self {
        Self {
            aborted: Some(aborted),
            duplicate: Some(duplicate),
        }
    }

    /// Both say the same.
    pub(crate) const fn both(code: ErrorCode) -> Self {
        Self::new(code, code)
    }
}

/// The answer for a change that failed: the rule of the policy engine that refused it, what
/// `stops` says for a change that stopped itself or hit a duplicate key, and for anything else an
/// internal failure (the cause goes to the log). The one place that reads the kinds of failure of
/// a change.
pub(crate) fn change_failed<S: AuthStore>(error: &S::Error, stops: Stops) -> AppError {
    if let Some(refusal) = S::as_refusal(error) {
        return refusal.into();
    }
    let code = if S::is_aborted(error) {
        stops.aborted
    } else if S::is_duplicate(error) {
        stops.duplicate
    } else {
        None
    };
    code.map_or_else(|| internal::<S>(error), AppError::Refused)
}
