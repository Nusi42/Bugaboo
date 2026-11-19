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
    TrustedComment, checksum_line, checksum_of, signature_name, verify,
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

/// Whether the version `theirs` is older than `ours`, by the three numbers of each (a suffix
/// after `-` is left out). `None` when either is no version number.
#[must_use]
pub fn is_older(theirs: &str, ours: &str) -> Option<bool> {
    Some(numbers(theirs)? < numbers(ours)?)
}

/// The three numbers of a version like `0.6.0` or `1.2.3-test`.
fn numbers(version: &str) -> Option<(u64, u64, u64)> {
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
