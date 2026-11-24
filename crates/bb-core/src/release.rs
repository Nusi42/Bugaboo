//! Releases (SEC-18, COMP-03, COMP-04, DIST-07; ADR-0047): the public keys a release is checked
//! with, the check of a signature, the release of this build with its support period, and the text
//! of `security.txt`.
//!
//! The keys, the version, the support period and our contact are written here once; the program
//! (`bugaboo update verify`, *About*, `/.well-known/security.txt`) and `cargo xtask release` (the
//! signatures, the release notes, `SECURITY.md`) read them from here.

mod metadata;
mod security_txt;
mod signature;
#[cfg(test)]
mod tests;

pub use metadata::{MINOR_RELEASE_DATE, RELEASE_DATE, SUPPORT_YEARS};
pub use security_txt::{
    OUR_SECURITY_CONTACT, SECURITY_POLICY, SECURITY_TXT_PATH, contact_address, security_txt,
};
pub use signature::{
    CHECKSUMS_FILE, KEYS_ARE_FOR_TESTS, RELEASE_PUBLIC_KEYS, ReleaseKey, SignatureProblem,
    TrustedComment, checksum_entries, checksum_line, checksum_of, signature_name, verify,
};

use jiff::civil::Date;
use jiff::{Span, ToSpan as _};

/// The version of this build: `[workspace.package]` of `Cargo.toml` (ADR-0040).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// How long a release gets security fixes (ADR-0047, decision 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SupportPeriod {
    /// A preview (before 1.0): only the newest release gets fixes, so this one gets them until the
    /// next release comes out.
    UntilNextRelease,
    /// A release from 1.0 on: fixes until this day.
    Until(Date),
}

/// The support period of the release `version` whose minor version (`X.Y.0`) came out on
/// `minor_released`: a preview until the next release, from 1.0 on [`SUPPORT_YEARS`] from the day of
/// `X.Y.0`, so a patch release never adds time (ADR-0047, decision 5). `None` when `version` is no
/// version number.
#[must_use]
pub fn support_period(version: &str, minor_released: Date) -> Option<SupportPeriod> {
    let (major, _, _) = numbers(version)?;
    if major == 0 {
        return Some(SupportPeriod::UntilNextRelease);
    }
    minor_released
        .checked_add(i64::from(SUPPORT_YEARS).years())
        .ok()
        .map(SupportPeriod::Until)
}

/// The support period of this build.
#[must_use]
pub fn this_support_period() -> Option<SupportPeriod> {
    support_period(VERSION, MINOR_RELEASE_DATE)
}

/// How long `security.txt` stays valid after the day of the release (RFC 9116 `Expires`, M9-Q16):
/// an old installation shows its age.
#[must_use]
pub fn security_txt_lifetime() -> Span {
    1.year()
}

/// Whether the version `theirs` is older than `ours`, by the rules of Semantic Versioning: the
/// three numbers first; with the same numbers a pre-release (`0.6.0-rc1`) is older than the
/// release itself, and two pre-releases are compared part by part (numbers as numbers, below
/// words). What follows a `+` does not count. `None` when either is no version number.
#[must_use]
pub fn is_older(theirs: &str, ours: &str) -> Option<bool> {
    Some(compare(theirs, ours)? == std::cmp::Ordering::Less)
}

/// How the version `theirs` ranks against `ours` by Semantic Versioning, `None` when either is no
/// version number.
fn compare(theirs: &str, ours: &str) -> Option<std::cmp::Ordering> {
    let (theirs_numbers, theirs_pre) = (numbers(theirs)?, pre_release(theirs));
    let (ours_numbers, ours_pre) = (numbers(ours)?, pre_release(ours));
    Some(
        theirs_numbers
            .cmp(&ours_numbers)
            .then_with(|| match (theirs_pre, ours_pre) {
                (None, None) => std::cmp::Ordering::Equal,
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (Some(theirs), Some(ours)) => compare_pre(theirs, ours),
            }),
    )
}

/// The pre-release part of a version: what follows the first `-`, without what follows a `+`.
fn pre_release(version: &str) -> Option<&str> {
    let version = version
        .split_once('+')
        .map_or(version, |(version, _)| version);
    version.split_once('-').map(|(_, pre)| pre)
}

/// Two pre-release parts, identifier by identifier: numbers by their value and below words,
/// words by their ASCII order, and a shorter list that agrees so far below a longer one.
fn compare_pre(theirs: &str, ours: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let rank = |identifier: &str| identifier.parse::<u64>().ok();
    let mut theirs = theirs.split('.');
    let mut ours = ours.split('.');
    loop {
        match (theirs.next(), ours.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(a), Some(b)) => {
                let order = match (rank(a), rank(b)) {
                    (Some(a), Some(b)) => a.cmp(&b),
                    (Some(_), None) => Ordering::Less,
                    (None, Some(_)) => Ordering::Greater,
                    (None, None) => a.cmp(b),
                };
                if order != Ordering::Equal {
                    return order;
                }
            }
        }
    }
}

/// The three numbers of a version like `0.6.0`, `1.2.3-test` or `1.2.3+build`.
fn numbers(version: &str) -> Option<(u64, u64, u64)> {
    let version = version
        .split_once('+')
        .map_or(version, |(version, _)| version);
    let plain = version.split_once('-').map_or(version, |(plain, _)| plain);
    let mut parts = plain.split('.').map(|part| {
        if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
            None
        } else {
            part.parse::<u64>().ok()
        }
    });
    let numbers = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(numbers)
}
