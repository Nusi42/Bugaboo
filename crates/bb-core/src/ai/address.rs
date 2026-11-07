//! The address of an OpenAI-compatible model: where it is, and whether that is on the
//! organization's own network (AI-01, PRIV-08).
//!
//! A model on the local network may be reached without encryption, because nothing leaves the
//! network. Any other address is a provider on the internet and is reached over HTTPS only.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// The longest address, in characters.
pub const ADDRESS_MAX: usize = 300;

/// Why an address is not accepted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddressProblem {
    /// It is no web address (`http://` or `https://` and a host), is too long, or holds a user
    /// name, a password, a query or a fragment.
    NotAnAddress,
    /// It is an address on the internet that does not use HTTPS.
    NotEncrypted,
}

/// What a call needs of an address.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelEndpoint {
    /// The address without a trailing slash, like `https://models.example.test/v1`.
    pub base: String,
    /// Whether the connection is encrypted.
    pub encrypted: bool,
}

/// A checked address of an OpenAI-compatible model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelAddress {
    base: String,
    https: bool,
    local: bool,
}

impl ModelAddress {
    /// Reads and checks an address.
    ///
    /// # Errors
    /// [`AddressProblem::NotAnAddress`] for text that is no plain web address,
    /// [`AddressProblem::NotEncrypted`] for an address on the internet that starts with `http://`.
    pub fn parse(text: &str) -> Result<Self, AddressProblem> {
        let text = text.trim();
        let (https, rest) = if let Some(rest) = text.strip_prefix("https://") {
            (true, rest)
        } else if let Some(rest) = text.strip_prefix("http://") {
            (false, rest)
        } else {
            return Err(AddressProblem::NotAnAddress);
        };
        let (authority, path) = rest.split_once('/').unwrap_or((rest, ""));
        let plain = |character: char| !character.is_control() && !character.is_whitespace();
        if text.chars().count() > ADDRESS_MAX
            || !text.chars().all(plain)
            || authority.is_empty()
            || authority.contains(['@', '\\', '?', '#'])
            || path.contains(['?', '#'])
        {
            return Err(AddressProblem::NotAnAddress);
        }
        let host = host_of(authority).ok_or(AddressProblem::NotAnAddress)?;
        let local = is_local_host(host);
        if !https && !local {
            return Err(AddressProblem::NotEncrypted);
        }
        let path = path.trim_matches('/');
        let scheme = if https { "https" } else { "http" };
        let base = if path.is_empty() {
            format!("{scheme}://{authority}")
        } else {
            format!("{scheme}://{authority}/{path}")
        };
        Ok(Self { base, https, local })
    }

    /// Whether the model is on this computer or the organization's own network.
    #[must_use]
    pub const fn is_local(&self) -> bool {
        self.local
    }

    /// The address without a trailing slash.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.base
    }

    /// What a call needs of it.
    #[must_use]
    pub fn endpoint(&self) -> ModelEndpoint {
        ModelEndpoint {
            base: self.base.clone(),
            encrypted: self.https,
        }
    }
}

/// The host of `host[:port]` or `[v6][:port]`, or none when the port is not a number.
fn host_of(authority: &str) -> Option<&str> {
    let (host, port) = if let Some(rest) = authority.strip_prefix('[') {
        let (host, after) = rest.split_once(']')?;
        (host, after.strip_prefix(':'))
    } else {
        match authority.split_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (authority, None),
        }
    };
    let port_ok = port.is_none_or(|port| port.parse::<u16>().is_ok());
    (port_ok && !host.is_empty()).then_some(host)
}

/// Whether a host is this computer or on the organization's own network: `localhost`, a name with
/// no dot or ending in `.local` or `.lan`, or an address of a private range. Link-local addresses
/// (`169.254.0.0/16`, `fe80::/10`, where the metadata services of clouds are) and `.internal`
/// names are not local: they need HTTPS like any address on the internet.
fn is_local_host(host: &str) -> bool {
    let lower = host.to_ascii_lowercase();
    if let Ok(address) = lower.parse::<IpAddr>() {
        return match address.to_canonical() {
            IpAddr::V4(v4) => is_private_v4(v4),
            IpAddr::V6(v6) => is_private_v6(v6),
        };
    }
    !lower.contains('.')
        || [".local", ".lan", ".localhost"]
            .iter()
            .any(|suffix| lower.ends_with(suffix))
}

fn is_private_v4(address: Ipv4Addr) -> bool {
    address.is_loopback() || address.is_private()
}

fn is_private_v6(address: Ipv6Addr) -> bool {
    address.is_loopback() || address.is_unique_local()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_address_on_the_local_network_may_be_plain() {
        for text in [
            "http://localhost:11434/v1",
            "http://127.0.0.1:9123",
            "http://192.168.0.5/v1/",
            "http://10.1.2.3:1234",
            "http://172.20.0.9",
            "http://models.lan/v1",
            "http://gpu-box:8000",
            "http://[::1]:8000",
        ] {
            let address = ModelAddress::parse(text).unwrap_or_else(|_| panic!("{text}"));
            assert!(address.is_local(), "{text}");
        }
    }

    #[test]
    fn an_address_on_the_internet_needs_https() {
        assert_eq!(
            ModelAddress::parse("http://models.example.test/v1"),
            Err(AddressProblem::NotEncrypted)
        );
        for text in [
            "http://8.8.8.8/v1",
            "http://169.254.169.254/latest",
            "http://[fe80::1]/v1",
            "http://metadata.google.internal/v1",
        ] {
            assert_eq!(
                ModelAddress::parse(text),
                Err(AddressProblem::NotEncrypted),
                "{text}"
            );
        }
        let address = ModelAddress::parse("https://models.example.test/v1/").unwrap();
        assert!(!address.is_local());
        assert_eq!(address.as_str(), "https://models.example.test/v1");
    }

    #[test]
    fn text_that_is_no_plain_address_is_refused() {
        for text in [
            "",
            "ftp://x.test",
            "models.example.test",
            "https://",
            "https://user:pw@models.example.test",
            "https://models.example.test/v1?key=1",
            "https://models.example.test/#top",
            "https://models.example.test:port/v1",
            "https://models example.test",
        ] {
            assert_eq!(
                ModelAddress::parse(text),
                Err(AddressProblem::NotAnAddress),
                "{text}"
            );
        }
        assert!(ModelAddress::parse(&format!("https://{}", "a".repeat(400))).is_err());
    }
}
