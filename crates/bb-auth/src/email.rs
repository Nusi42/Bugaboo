//! E-mail addresses as sign-in names, and the list of approved domains for signing up (AUTH-06).
//!
//! Addresses are compared in lower case, with spaces around them removed. The check is
//! deliberately small: one `@`, something on each side, a dot in the domain, no spaces or control
//! characters. Whether the mailbox exists is something only a message can show (M4).

use bb_core::settings::list_items;

/// The longest address (RFC 5321).
const MAX_LENGTH: usize = 254;
/// The longest part before the `@` (RFC 5321).
const MAX_LOCAL_LENGTH: usize = 64;

/// An e-mail address in its one spelling: lower case, trimmed.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EmailAddress(String);

impl EmailAddress {
    /// The address `text`, or `None` when it cannot be one.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let address = text.trim().to_lowercase();
        let (local, domain) = address.split_once('@')?;
        let plain = |part: &str| {
            !part.is_empty() && !part.chars().any(|c| c.is_whitespace() || c.is_control())
        };
        let valid = address.len() <= MAX_LENGTH
            && local.len() <= MAX_LOCAL_LENGTH
            && plain(local)
            && plain(domain)
            && !domain.contains('@')
            && domain.contains('.')
            && !domain.starts_with('.')
            && !domain.ends_with('.')
            && !domain.contains("..");
        valid.then_some(Self(address))
    }

    /// The address as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The part after the `@`.
    #[must_use]
    pub fn domain(&self) -> &str {
        self.0.split_once('@').map_or("", |(_, domain)| domain)
    }
}

/// The domains of `list`, a list separated by commas, in one spelling: without spaces, without a
/// leading `@` and in lower case. The one reading of the approved domains for signing up.
pub fn listed_domains(list: &str) -> impl Iterator<Item = String> {
    list_items(list)
        .map(|item| item.trim_start_matches('@').to_lowercase())
        .filter(|item| !item.is_empty())
}

/// Whether the domain of `address` is on `list`, a list of domains separated by commas (spaces
/// and capitals do not matter). An empty list names nobody.
#[must_use]
pub fn domain_is_listed(address: &EmailAddress, list: &str) -> bool {
    listed_domains(list).any(|item| item == address.domain())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_01_addresses_are_compared_in_one_spelling() {
        let address = EmailAddress::parse("  Ada.Lovelace@Example.COM ").unwrap();
        assert_eq!(address.as_str(), "ada.lovelace@example.com");
        assert_eq!(address.domain(), "example.com");
    }

    #[test]
    fn auth_01_text_that_is_not_an_address_is_refused() {
        for bad in [
            "",
            "ada",
            "@example.com",
            "ada@",
            "ada@example",
            "a b@example.com",
            "ada@exa mple.com",
            "ada@@example.com",
            "ada@example..com",
            "ada@.example.com",
            "ada@example.com.",
            "a\u{0}b@example.com",
            &format!("{}@example.com", "a".repeat(65)),
        ] {
            assert!(EmailAddress::parse(bad).is_none(), "{bad:?}");
        }
    }

    #[test]
    fn auth_06_only_listed_domains_match_exactly() {
        let ada = EmailAddress::parse("ada@example.com").unwrap();
        assert!(domain_is_listed(&ada, "example.com"));
        assert!(domain_is_listed(&ada, " other.org , @Example.com "));
        assert!(!domain_is_listed(&ada, ""));
        assert!(!domain_is_listed(&ada, " , "));
        assert!(!domain_is_listed(&ada, "mail.example.com"));
        assert!(!domain_is_listed(&ada, "example.com.evil.test"));
        let sub = EmailAddress::parse("ada@mail.example.com").unwrap();
        assert!(!domain_is_listed(&sub, "example.com"));
    }
}
