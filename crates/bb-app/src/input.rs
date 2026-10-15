//! What a use case does with the short texts people type: the rule of `bb_core::text`, answered as
//! the error a person can read.

use bb_core::ErrorCode;
use bb_core::tags::TAG_NAME_MAX;
use bb_core::text::{LONG_TEXT_MAX, NAME_MAX, clean, clean_long, has_invisible_format};

use crate::auth::AppError;

/// `text` as it is kept (trimmed, not empty, at most `longest` characters, no control
/// characters), or `request.invalid`.
pub(crate) fn clean_text(text: &str, longest: usize) -> Result<String, AppError> {
    clean(text, longest)
        .map(str::to_owned)
        .ok_or_else(|| ErrorCode::InvalidRequest.into())
}

/// A long text (a description, a comment) as it is kept, or `request.invalid`: at most
/// `LONG_TEXT_MAX` characters and no control characters but line breaks and tabs.
pub(crate) fn clean_long_text(text: &str) -> Result<String, AppError> {
    clean_long(text, LONG_TEXT_MAX)
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

/// The name of a tag or a label as it is kept: a name of at most `TAG_NAME_MAX` characters that
/// holds no invisible or turning character (`request.invalid`), so two names cannot look alike.
pub(crate) fn clean_tag_name(name: &str) -> Result<String, AppError> {
    if has_invisible_format(name) {
        return Err(ErrorCode::InvalidRequest.into());
    }
    clean_text(name, TAG_NAME_MAX)
}

/// What a person typed to find a tag: nothing at all (the first tags are offered) or text that
/// could be the name of a tag, so a search text is as long as a name at most.
pub(crate) fn clean_tag_search(text: &str) -> Result<String, AppError> {
    if text.trim().is_empty() {
        Ok(String::new())
    } else {
        clean_tag_name(text)
    }
}
