//! The version of a record that people change (ADR-0019): it counts up with every change, and a
//! change that was made from an older version is refused.

use crate::event::FieldChange;

/// The version a new record starts with.
pub const FIRST_VERSION: i64 = 1;

/// The name of the change that every event of a change of a versioned record holds: the version
/// before and after. It is how the log tells an event that counted the version up from one that
/// did not (a lock, a member of a group), so a stale change is told who really changed the record.
pub const FIELD: &str = "version";

/// The version after `version`: the one place that counts a version up.
#[must_use]
pub const fn next(version: i64) -> i64 {
    version.saturating_add(1)
}

/// The change that counts a record from `before` to the next version.
#[must_use]
pub fn counted_up(before: i64) -> FieldChange {
    FieldChange {
        field: FIELD.to_owned(),
        before: Some(before.into()),
        after: Some(next(before).into()),
    }
}
