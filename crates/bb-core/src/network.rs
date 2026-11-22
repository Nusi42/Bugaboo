//! The shortened network address (PRIV-01): IPv4 keeps three bytes and IPv6 the first 48 bits.
//! Logs and the security log hold this, never the full address. `core::net` only has the address
//! types; this crate still does no I/O.

use core::net::{IpAddr, Ipv4Addr, Ipv6Addr};
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

/// The group of clients a rate limit counts together (SEC-14). A public address counts by its
/// [`NetworkPrefix`]: one customer of a provider is given a whole `/24` or `/48` and can move inside
/// it at will. An address of a network of its own counts by itself: this computer, the private
/// ranges (`10/8`, `172.16/12`, `192.168/16`), link-local addresses and the shared range of
/// providers (`100.64/10`), where a whole office or the customers of one provider share the first
/// bytes; an IPv6 unique-local address (`fc00::/7`) counts by its `/64`, which one machine owns.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ClientGroup {
    /// A public address, by its network.
    Network(NetworkPrefix),
    /// An address of a network of its own, alone (unique-local: its `/64`).
    Own(IpAddr),
}

impl ClientGroup {
    /// The group `address` is counted in. An IPv4 address written as IPv6 counts as IPv4.
    #[must_use]
    pub fn of(address: IpAddr) -> Self {
        match address.to_canonical() {
            IpAddr::V4(v4) if is_own_v4(v4) => Self::Own(IpAddr::V4(v4)),
            IpAddr::V6(v6) if v6.is_loopback() || v6.is_unicast_link_local() => {
                Self::Own(IpAddr::V6(v6))
            }
            IpAddr::V6(v6) if v6.is_unique_local() => {
                let [a, b, c, d, ..] = v6.segments();
                Self::Own(IpAddr::V6(Ipv6Addr::new(a, b, c, d, 0, 0, 0, 0)))
            }
            public => Self::Network(NetworkPrefix::from_address(public)),
        }
    }
}

/// Whether an IPv4 address is of a network of its own (see [`ClientGroup`]).
fn is_own_v4(address: Ipv4Addr) -> bool {
    let [first, second, ..] = address.octets();
    // The shared range of providers (RFC 6598); `Ipv4Addr::is_shared` is not stable yet.
    let shared = first == 100 && second & 0b1100_0000 == 64;
    address.is_loopback() || address.is_private() || address.is_link_local() || shared
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

    fn group(address: &str) -> ClientGroup {
        ClientGroup::of(address.parse().unwrap())
    }

    #[test]
    fn sec_14_an_address_of_an_own_network_is_counted_alone_and_a_public_one_by_its_network() {
        for own in [
            "127.0.0.1",
            "10.1.2.3",
            "172.16.0.9",
            "172.31.255.1",
            "192.168.0.5",
            "169.254.1.1",
            "100.64.0.7",
            "100.127.255.1",
            "::1",
            "fe80::1",
            "::ffff:192.168.0.5",
        ] {
            let address: IpAddr = own.parse().unwrap();
            assert_eq!(
                group(own),
                ClientGroup::Own(address.to_canonical()),
                "{own}"
            );
        }
        // Unique-local: the `/64`, which one machine owns.
        assert_eq!(group("fd12:3456:789a:1::5"), group("fd12:3456:789a:1::9"));
        assert_ne!(group("fd12:3456:789a:1::5"), group("fd12:3456:789a:2::5"));
        // Public: the network, also just outside the own ranges.
        for (one, other) in [
            ("203.0.113.7", "203.0.113.200"),
            ("172.32.0.1", "172.32.0.2"),
            ("100.128.0.1", "100.128.0.2"),
            ("2001:db8:1:2::1", "2001:db8:1:ffff::9"),
        ] {
            assert_eq!(group(one), group(other), "{one}");
            assert!(matches!(group(one), ClientGroup::Network(_)), "{one}");
        }
    }
}
