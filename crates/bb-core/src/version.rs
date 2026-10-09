//! The version of a record that people change (ADR-0019): it counts up with every change, and a
//! change that was made from an older version is refused.

/// The version a new record starts with.
pub const FIRST_VERSION: i64 = 1;
