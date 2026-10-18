//! E-mail addresses as sign-in names, and the list of approved domains for signing up (AUTH-06).
//!
//! Addresses are compared in lower case, with spaces around them removed. The check is
//! deliberately small: one `@`, a dot-atom before it (the characters of RFC 5322 and single dots,
//! so nothing of the syntax of a mailbox) and labels of letters, digits and hyphens with a dot
//! after it. Whether the mailbox exists is something only a message can show (M4).

use bb_core::settings::list_items;

/// The longest address (RFC 5321).
const MAX_LENGTH: usize = 254;
/// The longest part before the `@` (RFC 5321).
const MAX_LOCAL_LENGTH: usize = 64;

/// Whether `part` is a dot-atom (RFC 5322): words of the characters below, joined by single dots.
/// Nothing of the syntax of a mailbox (`< > " , ; ( ) [ ] : \` and spaces) is one.
fn is_dot_atom(part: &str) -> bool {
    const SPECIAL: &str = "!#$%&'*+-/=?^_`{|}~";
    !part.is_empty()
        && part.split('.').all(|word| {
            !word.is_empty()
                && word
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || SPECIAL.contains(c))
        })
}

/// Whether `part` is one label of a domain: letters, digits and hyphens, not starting or ending
/// with a hyphen. Letters of other scripts are allowed; the mail server decides about them.
fn is_label(part: &str) -> bool {
    !part.is_empty()
        && !part.starts_with('-')
        && !part.ends_with('-')
        && part.chars().all(|c| c.is_alphanumeric() || c == '-')
}

/// An e-mail address in its one spelling: lower case, trimmed.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EmailAddress(String);

impl EmailAddress {
    /// The address `text`, or `None` when it cannot be one.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let address = text.trim().to_lowercase();
        let (local, domain) = address.split_once('@')?;
        let valid = address.len() <= MAX_LENGTH
            && local.len() <= MAX_LOCAL_LENGTH
            && is_dot_atom(local)
            && domain.contains('.')
            && domain.split('.').all(is_label);
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
            // Mailbox syntax and list syntax are no address (M4 exit review, M3).
            "x<a@example.com>",
            "x <a@example.com>",
            "\"x\"@example.com",
            "a,b@example.com",
            "a;b@example.com",
            "(c)a@example.com",
            "a..b@example.com",
            ".a@example.com",
            "a.@example.com",
            "ada@example.com>",
            "ada@exa_mple.com",
            "ada@-example.com",
            "ada@example-.com",
            "ada@example.c<om",
            &format!("{}@example.com", "a".repeat(65)),
        ] {
            assert!(EmailAddress::parse(bad).is_none(), "{bad:?}");
        }
    }

    #[test]
    fn auth_01_the_characters_of_a_dot_atom_are_an_address() {
        for good in [
            "a+1@b.co",
            "o'brien@b.co",
            "a_b-c.d{e}@b.co",
            "a=b/c?@b.co",
            "ada@xn--bcher-kva.de",
        ] {
            assert!(EmailAddress::parse(good).is_some(), "{good:?}");
        }
        // A plus tag is a spelling of the same mailbox for many servers, and to this server
        // another address (12-security, accepted).
        let tagged = EmailAddress::parse("a+1@b.co").unwrap();
        assert_ne!(tagged, EmailAddress::parse("a@b.co").unwrap());
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
