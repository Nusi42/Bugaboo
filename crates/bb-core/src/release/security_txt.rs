//! The text of `/.well-known/security.txt` (RFC 9116; COMP-03, ADR-0047 decision 4, M9-Q16): the
//! organization's own contact first when an administrator set one (`security.contact`), then ours;
//! an expiry one year after the release of this build; our policy; English and German.

use std::fmt::Write as _;

use jiff::civil::Date;

/// Where the server answers it (RFC 9116, section 3).
pub const SECURITY_TXT_PATH: &str = "/.well-known/security.txt";

/// Where a vulnerability in Bugaboo itself is reported: the private reporting of the repository
/// (`repository` of `[workspace.package]`, written once in `Cargo.toml`).
pub const OUR_SECURITY_CONTACT: &str =
    concat!(env!("CARGO_PKG_REPOSITORY"), "/security/advisories/new");

/// Our policy for reports: `SECURITY.md` of the repository.
pub const SECURITY_POLICY: &str = concat!(env!("CARGO_PKG_REPOSITORY"), "/blob/main/SECURITY.md");

/// The languages reports may be written in.
const LANGUAGES: &str = "en, de";

/// The longest contact address that is taken; a longer one is not an address a person typed.
const CONTACT_MOST: usize = 254;

/// The value of the setting `security.contact` as an address for `Contact:`: an `https://` page or
/// a `mailto:` address as written, or an e-mail address with `mailto:` before it. `None` for an
/// empty setting and for anything else, so a value can never add a line or a field of its own.
#[must_use]
pub fn contact_address(setting: &str) -> Option<String> {
    let value = setting.trim();
    if value.is_empty()
        || value.len() > CONTACT_MOST
        || value
            .chars()
            .any(|letter| letter.is_whitespace() || letter.is_control())
    {
        return None;
    }
    let lower = value.to_ascii_lowercase();
    if lower.starts_with("https://") && value.len() > "https://".len() {
        return Some(value.to_owned());
    }
    let mail = lower
        .strip_prefix("mailto:")
        .map_or(value, |_| &value["mailto:".len()..]);
    let (name, domain) = mail.split_once('@')?;
    (!name.is_empty() && domain.contains('.') && !domain.contains('@') && !mail.contains(':'))
        .then(|| format!("mailto:{mail}"))
}

/// The text of `security.txt` for an organization whose setting `security.contact` is
/// `organization_contact`, for the release made on `released`.
#[must_use]
pub fn security_txt(organization_contact: &str, released: Date) -> String {
    let mut text = String::new();
    if let Some(contact) = contact_address(organization_contact) {
        let _ = writeln!(text, "Contact: {contact}");
    }
    let expires = released
        .checked_add(super::security_txt_lifetime())
        .unwrap_or(released);
    let _ = write!(
        text,
        "Contact: {OUR_SECURITY_CONTACT}\nExpires: {expires}T00:00:00Z\nPolicy: {SECURITY_POLICY}\nPreferred-Languages: {LANGUAGES}\n"
    );
    text
}
