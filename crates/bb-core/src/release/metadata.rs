//! The release of this build, read from `[workspace.metadata.release]` of the workspace's
//! `Cargo.toml` when the program is compiled (ADR-0047, decision 5: written once there).
//!
//! The reading runs at compile time: a missing or malformed entry stops the build, so the program
//! never starts with a release it cannot name. It reads only what the table holds, `date` and
//! `minor-date` as a quoted `YYYY-MM-DD` and `support-years` as a whole number, each on a line of
//! its own.

use jiff::civil::Date;

/// The text of the workspace's manifest.
const MANIFEST: &str = include_str!("../../../../Cargo.toml");

/// The heading of the table.
const TABLE: &str = "[workspace.metadata.release]";

/// The day this version was released.
pub const RELEASE_DATE: Date = date_of(b"date = \"");

/// The day the first release of this minor version (`X.Y.0`) came out: the support period of a
/// minor release counts from it, whatever patch release follows (ADR-0047, decision 5).
pub const MINOR_RELEASE_DATE: Date = date_of(b"minor-date = \"");

/// The day written as `"YYYY-MM-DD"` by the entry of the table that starts with `start`.
const fn date_of(start: &[u8]) -> Date {
    let at = value_of(MANIFEST.as_bytes(), start);
    let year = digits(MANIFEST.as_bytes(), at, 4);
    let month = digits(MANIFEST.as_bytes(), at + 5, 2);
    let day = digits(MANIFEST.as_bytes(), at + 8, 2);
    assert!(
        MANIFEST.as_bytes()[at + 4] == b'-'
            && MANIFEST.as_bytes()[at + 7] == b'-'
            && MANIFEST.as_bytes()[at + 10] == b'"',
        "a day of the release in Cargo.toml must be written as \"YYYY-MM-DD\""
    );
    Date::constant(as_i16(year), as_i8(month), as_i8(day))
}

/// How many years each minor release from 1.0 on gets security fixes.
pub const SUPPORT_YEARS: i16 = {
    let at = value_of(MANIFEST.as_bytes(), b"support-years = ");
    let mut end = at;
    while end < MANIFEST.len() && MANIFEST.as_bytes()[end].is_ascii_digit() {
        end += 1;
    }
    assert!(
        end > at && end - at <= 2,
        "support-years in Cargo.toml must be a whole number of years"
    );
    as_i16(digits(MANIFEST.as_bytes(), at, end - at))
};

/// Where the value of the entry that starts with `start` begins, inside the table.
const fn value_of(text: &[u8], start: &[u8]) -> usize {
    let mut line = find(text, TABLE.as_bytes());
    assert!(
        line < text.len(),
        "Cargo.toml has no [workspace.metadata.release] table"
    );
    loop {
        // The next line of the table; the table ends at the next heading or the end of the file.
        while line < text.len() && text[line] != b'\n' {
            line += 1;
        }
        line += 1;
        assert!(
            line < text.len() && text[line] != b'[',
            "an entry of [workspace.metadata.release] is missing in Cargo.toml"
        );
        if starts_with(text, line, start) {
            return line + start.len();
        }
    }
}

/// Where `needle` first stands in `text`, or the length of `text` when it is not there.
const fn find(text: &[u8], needle: &[u8]) -> usize {
    let mut at = 0;
    while at + needle.len() <= text.len() {
        if starts_with(text, at, needle) {
            return at;
        }
        at += 1;
    }
    text.len()
}

const fn starts_with(text: &[u8], at: usize, start: &[u8]) -> bool {
    if at + start.len() > text.len() {
        return false;
    }
    let mut index = 0;
    while index < start.len() {
        if text[at + index] != start[index] {
            return false;
        }
        index += 1;
    }
    true
}

/// The number written by the `count` digits at `at`.
const fn digits(text: &[u8], at: usize, count: usize) -> u16 {
    assert!(count <= 4, "a number in Cargo.toml is too long");
    let mut value: u16 = 0;
    let mut index = 0;
    while index < count {
        let byte = text[at + index];
        assert!(
            byte.is_ascii_digit(),
            "a number in Cargo.toml has a non-digit"
        );
        value = value * 10 + (byte - b'0') as u16;
        index += 1;
    }
    value
}

/// A number of at most four digits, which always fits.
const fn as_i16(value: u16) -> i16 {
    value.cast_signed()
}

/// A month or a day: two digits, which always fit.
const fn as_i8(value: u16) -> i8 {
    assert!(value <= 99, "a month or a day in Cargo.toml is too big");
    value.to_le_bytes()[0].cast_signed()
}
