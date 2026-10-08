//! What a use case does with the short texts people type: the rule of `bb_core::text`, answered as
//! the error a person can read.

use bb_core::ErrorCode;
use bb_core::text::{NAME_MAX, clean};

use crate::auth::AppError;

/// `text` as it is kept (trimmed, not empty, at most `longest` characters, no control
/// characters), or `request.invalid`.
pub(crate) fn clean_text(text: &str, longest: usize) -> Result<String, AppError> {
    clean(text, longest)
        .map(str::to_owned)
        .ok_or_else(|| ErrorCode::InvalidRequest.into())
}

/// The position a cursor stands for, read by `read`; `None` for no cursor, and `request.invalid`
/// for text that is none.
pub(crate) fn cursor_or_invalid<T>(
    cursor: Option<&str>,
    read: impl FnOnce(&str) -> Option<T>,
) -> Result<Option<T>, AppError> {
    cursor
        .map(|text| read(text).ok_or(AppError::Refused(ErrorCode::InvalidRequest)))
        .transpose()
}

/// The name of a person, group, role, project or access key as it is kept.
pub(crate) fn clean_name(name: &str) -> Result<String, AppError> {
    clean_text(name, NAME_MAX)
}
