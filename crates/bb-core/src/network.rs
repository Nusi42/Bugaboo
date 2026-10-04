//! The shortened network address (PRIV-01): IPv4 keeps three bytes and IPv6 the first 48 bits.
//! Logs and the security log hold this, never the full address. `core::net` only has the address
//! types; this crate still does no I/O.

use core::net::IpAddr;
use std::fmt;

use serde::{Deserialize, Serialize};

/// A network address cut short, like `203.0.113.0` or `2001:db8:abcd::`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NetworkPrefix(String);

impl NetworkPrefix {
    /// The shortened form of `address`. An IPv4 address written as IPv6 (`::ffff:a.b.c.d`) counts
    /// as IPv4.
    #[must_use]
    pub fn from_address(address: IpAddr) -> Self {
        Self(match address.to_canonical() {
            IpAddr::V4(v4) => {
                let [a, b, c, _] = v4.octets();
                format!("{a}.{b}.{c}.0")
            }
            IpAddr::V6(v6) => {
                let [a, b, c, ..] = v6.segments();
                format!("{a:x}:{b:x}:{c:x}::")
            }
        })
    }

    /// The prefix read back from storage, if the text is one that [`Self::from_address`] makes.
    #[must_use]
    pub fn from_stored(text: &str) -> Option<Self> {
        text.parse()
            .ok()
            .map(Self::from_address)
            .filter(|prefix| prefix.as_str() == text)
    }

    /// The text, as written to logs.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NetworkPrefix {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn short(address: &str) -> String {
        NetworkPrefix::from_address(address.parse().unwrap()).to_string()
    }

    #[test]
    fn ipv4_keeps_three_bytes_and_ipv6_48_bits() {
        assert_eq!(short("203.0.113.77"), "203.0.113.0");
        assert_eq!(short("2001:db8:abcd:1234:5678::9"), "2001:db8:abcd::");
    }

    #[test]
    fn only_shortened_text_is_read_back() {
        for text in ["203.0.113.0", "2001:db8:abcd::"] {
            assert_eq!(NetworkPrefix::from_stored(text).unwrap().as_str(), text);
        }
        for text in ["203.0.113.77", "2001:db8:abcd:1234::", "host.example", ""] {
            assert_eq!(NetworkPrefix::from_stored(text), None, "{text}");
        }
    }

    #[test]
    fn an_ipv4_address_written_as_ipv6_counts_as_ipv4() {
        assert_eq!(short("::ffff:203.0.113.77"), "203.0.113.0");
    }
}
