//! `cargo xtask gen release`: the parts of `SECURITY.md` that the release decides, written from the
//! one place each is kept (ADR-0047, decisions 2 and 5): the public keys from
//! `bb_core::release::RELEASE_PUBLIC_KEYS`, the supported versions from the version and the
//! release metadata of `Cargo.toml`. `cargo xtask release` writes the same parts into the release
//! notes. DOC-GENERATED fails when `SECURITY.md` is not what the code gives.

use bb_core::release::{
    self, KEYS_ARE_FOR_TESTS, RELEASE_PUBLIC_KEYS, SUPPORT_YEARS, SupportPeriod,
};
use std::fmt::Write as _;

use jiff::civil::Date;

use super::Generated;
use crate::repo::Repo;
use crate::source::read_text;

/// The page that holds the parts.
pub(crate) const PAGE: &str = "SECURITY.md";

/// What the first comment of a part of `SECURITY.md` says besides its name.
const NOTE: &str =
    "by `cargo xtask gen release` from bb-core and Cargo.toml; edit those, not this text -->";

/// The name of the part with the public keys.
pub(crate) const KEYS_PART: &str = "release-keys";

/// The name of the part with the support period.
pub(crate) const SUPPORT_PART: &str = "support-period";

/// The public keys, as a table, and how to check a file with them.
pub(crate) fn keys_part() -> String {
    let mut text = String::from(
        "A release signs its list `SHA256SUMS`, which names every other file of the release with \
         its checksum, with one of these public keys (minisign, Ed25519). A list signed with \
         either one is genuine, and so is every file it names with the right checksum.\n\n\
         | Key | Public key |\n|-----|------------|\n",
    );
    for key in RELEASE_PUBLIC_KEYS {
        let _ = writeln!(text, "| {} | `{}` |", key.name, key.public);
    }
    if KEYS_ARE_FOR_TESTS {
        text.push_str(
            "\n**These are test keys.** Nobody holds their secret keys, so no release is signed \
             with them. The real keys replace them before the first release that people install.\n",
        );
    }
    text.push_str(
        "\nCheck a downloaded file with the Bugaboo you have installed \
         (`bugaboo update verify <file>`), or with the minisign tool and the list \
         (`minisign -Vm SHA256SUMS -P <public key>`, then \
         `sha256sum -c SHA256SUMS --ignore-missing`).",
    );
    text
}

/// Which versions get security fixes, for the release `version` made on `released`, whose minor
/// version (`X.Y.0`) came out on `minor_released`.
pub(crate) fn support_part(version: &str, released: Date, minor_released: Date) -> String {
    let years = format!(
        "From 1.0 on, each minor release gets security fixes for {SUPPORT_YEARS} years from the \
         day its first release (X.Y.0) comes out, as patch releases without new features."
    );
    match release::support_period(version, minor_released) {
        Some(SupportPeriod::Until(day)) => format!(
            "Version {version}, released on {released}, gets security fixes until {day}. {years}"
        ),
        _ => format!(
            "Version {version}, released on {released}, is a free public preview. Until 1.0, \
             only the newest release gets security fixes; a fix comes as the next patch release. \
             {years}"
        ),
    }
}

/// The two parts of `SECURITY.md` and of the release notes for the release `version` of this
/// build, by name.
pub(crate) fn parts(version: &str) -> [(&'static str, String); 2] {
    [
        (KEYS_PART, keys_part()),
        (
            SUPPORT_PART,
            support_part(version, release::RELEASE_DATE, release::MINOR_RELEASE_DATE),
        ),
    ]
}

/// `SECURITY.md` with its parts as the code says.
pub(crate) fn generate(repo: &Repo) -> anyhow::Result<Vec<Generated>> {
    let mut page = read_text(repo, PAGE)?;
    for (name, part) in parts(release::VERSION) {
        page = super::replace_part(PAGE, &page, name, NOTE, &part)?;
    }
    Ok(vec![Generated::new(PAGE, page)])
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;

    #[test]
    fn sec_18_the_page_names_every_public_key() {
        let part = keys_part();
        for key in RELEASE_PUBLIC_KEYS {
            assert!(part.contains(key.public), "{}", key.name);
        }
        assert_eq!(part.contains("test keys"), KEYS_ARE_FOR_TESTS);
    }

    #[test]
    fn comp_04_a_preview_and_a_release_from_1_0_name_their_support() {
        let preview = support_part("0.6.0", date(2026, 12, 1), date(2026, 12, 1));
        assert!(
            preview.contains("preview") && preview.contains("newest release"),
            "{preview}"
        );
        let one = support_part("1.0.0", date(2027, 6, 1), date(2027, 6, 1));
        let until = date(2027 + SUPPORT_YEARS, 6, 1).to_string();
        assert!(one.contains(&until), "{one}");
        // A patch release counts from the day of its minor release, not its own.
        let patch = support_part("1.0.2", date(2028, 1, 5), date(2027, 6, 1));
        assert!(
            patch.contains(&until) && patch.contains("2028-01-05"),
            "{patch}"
        );
    }
}
