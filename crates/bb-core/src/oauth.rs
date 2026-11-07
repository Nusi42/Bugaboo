//! How an AI app signs in (AI-22, ADR-0042): the records of apps and what a person allowed them,
//! the lives of codes and tokens, the limits of calls, and the rules for the address an app comes
//! back to.
//!
//! An app registers itself ([`OAuthClient`]); a person allows it ([`OAuthGrant`]); the app gets a
//! code, then a short token and a token that renews it ([`OAuthToken`]). Every secret is stored as a
//! hash only. What a grant may do is the words of access keys (*See only*, *See and change*) and a
//! list of projects; it never widens the person (ADR-0025, decision 8).

use std::fmt::Write as _;

use jiff::{SignedDuration, Timestamp};

use crate::access_key::KeyChoice;
use crate::id::{GrantId, OAuthClientId, ProjectId, UserId};
use crate::personal::Entity;
use crate::security_log::Digest;

/// How long an access token lives (ADR-0042, decision 3).
pub const ACCESS_TOKEN_SECONDS: i64 = 15 * 60;
/// How long an authorization code works.
pub const CODE_SECONDS: i64 = 60;
/// Days a renewing token can go unused before it runs out.
pub const REFRESH_UNUSED_DAYS: i64 = 30;
/// Minutes without a call after which a session of an app ends and its run is closed.
pub const SESSION_IDLE_MINUTES: u64 = 30;
/// The most calls one grant (or one access key) may make in a minute.
pub const CALLS_A_MINUTE_FOR_A_GRANT: u32 = 120;
/// The most calls one person may make in a minute over all their grants and keys.
pub const CALLS_A_MINUTE_FOR_A_PERSON: u32 = 240;
/// The most return addresses one app may register.
pub const MOST_RETURN_ADDRESSES: usize = 5;
/// The longest return address, in bytes.
pub const RETURN_ADDRESS_LENGTH_MAX: usize = 2_000;
/// The longest name an app may give itself, in characters.
pub const APP_NAME_LENGTH_MAX: usize = 80;
/// The most grants one person has at the same time (also the most the list shows).
pub const MOST_GRANTS: usize = 100;
/// The most projects one grant can be limited to, like a key.
pub const MOST_GRANT_PROJECTS: usize = crate::access_key::MOST_PROJECTS;
/// The most web addresses an administrator can allow.
pub const MOST_WEB_ADDRESSES: usize = 200;
/// Days after which an app that registered and was never allowed is forgotten.
pub const UNUSED_CLIENT_DAYS: i64 = 7;
/// Days after which a grant that ended is deleted.
pub const ENDED_GRANT_KEEP_DAYS: i64 = 30;

/// The `resource` a token is issued for: the path of the MCP server (RFC 8707). A token is refused
/// anywhere else.
pub const MCP_PATH: &str = "/mcp";
/// The path of the protected-resource metadata (RFC 9728).
pub const PROTECTED_RESOURCE_METADATA_PATH: &str = "/.well-known/oauth-protected-resource";
/// The path of the authorization-server metadata (RFC 8414).
pub const AUTHORIZATION_SERVER_METADATA_PATH: &str = "/.well-known/oauth-authorization-server";
/// The path apps register at (RFC 7591).
pub const REGISTRATION_PATH: &str = "/oauth/register";
/// The path apps swap a code or a renewing token at.
pub const TOKEN_PATH: &str = "/oauth/token";
/// The page of the browser app where a person is asked to allow an app.
pub const CONSENT_PATH: &str = "/connect";

/// The only way to prove a code: the SHA-256 form of PKCE (RFC 7636).
pub const PKCE_METHOD: &str = "S256";

/// The access token's life.
#[must_use]
pub const fn access_token_life() -> SignedDuration {
    SignedDuration::from_secs(ACCESS_TOKEN_SECONDS)
}

/// The life of a code.
#[must_use]
pub const fn code_life() -> SignedDuration {
    SignedDuration::from_secs(CODE_SECONDS)
}

impl KeyChoice {
    /// The choices an AI app can be given: the two plain ones. *Everything I can do* and single
    /// permissions are not offered to apps (ADR-0042, decision 3).
    pub const APP_CHOICES: [Self; 2] = [Self::SeeOnly, Self::SeeAndChange];

    /// Whether what this choice allows includes everything `other` allows: *See and change*
    /// includes *See only*.
    #[must_use]
    pub const fn covers_choice(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::SeeAndChange, Self::SeeOnly | Self::SeeAndChange)
                | (Self::SeeOnly, Self::SeeOnly)
        )
    }

    /// Whether an app can be given this choice.
    #[must_use]
    pub const fn is_for_apps(self) -> bool {
        matches!(self, Self::SeeOnly | Self::SeeAndChange)
    }
}

words! {
    /// What a token is for.
    pub enum TokenKind {
        /// The short token an app sends with every call.
        Access => "access",
        /// The token that renews the access token; it turns over at each use.
        Refresh => "refresh",
    }
}

words! {
    /// Why a grant ended.
    pub enum GrantEnd {
        /// The person took it back.
        TakenBack => "taken-back",
        /// The person signed out everywhere.
        SignedOutEverywhere => "signed-out-everywhere",
        /// An old renewing token was used again: two parties held it.
        TokenReused => "token-reused",
        /// A code was used a second time.
        CodeReused => "code-reused",
    }
}

words! {
    /// Why an app was turned away at registration or at the token endpoint: the error words of
    /// OAuth (RFC 6749 §5.2, RFC 7591 §3.2.2, RFC 8707), which apps read. They are protocol words
    /// and never shown to a person.
    pub enum OAuthProblem {
        /// The request is missing something or is malformed.
        InvalidRequest => "invalid_request",
        /// The app is not known.
        InvalidClient => "invalid_client",
        /// The code, the renewing token or the proof is wrong, has run out or was used.
        InvalidGrant => "invalid_grant",
        /// Only the code and the renewing token are swapped for tokens.
        UnsupportedGrantType => "unsupported_grant_type",
        /// The token is asked for a resource that is not this server's MCP server.
        InvalidTarget => "invalid_target",
        /// A return address that may not be registered.
        InvalidRedirectUri => "invalid_redirect_uri",
        /// The name or the other data of the app cannot be taken.
        InvalidClientMetadata => "invalid_client_metadata",
        /// The person said *Cancel*.
        AccessDenied => "access_denied",
        /// An administrator switched AI apps off.
        TemporarilyUnavailable => "temporarily_unavailable",
    }
}

/// An app that registered itself (RFC 7591). It is a public client: it proves a code with PKCE and
/// holds no secret of its own.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct OAuthClient {
    /// The ID the app is known by (`client_id`).
    #[not_personal]
    pub id: OAuthClientId,
    /// The name the app gave itself. Bugaboo cannot check it.
    #[personal(category = "content", basis = "contract")]
    pub name: String,
    /// The addresses the app comes back to, as registered, one per line.
    #[personal(category = "content", basis = "contract")]
    pub return_addresses: Vec<String>,
    /// When it registered.
    #[not_personal]
    pub registered_at: Timestamp,
}

/// What a person allowed an app: one grant, until it is taken back or runs out.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct OAuthGrant {
    /// The ID of the grant: what the list shows and what taking back names. It opens nothing.
    #[not_personal]
    pub id: GrantId,
    /// The app.
    #[not_personal]
    pub client: OAuthClientId,
    /// The person who allowed it, and whom the app acts for.
    #[personal(category = "identity", basis = "contract")]
    pub person: UserId,

    /// What the app may do: *See only* or *See and change*.
    #[not_personal]
    pub scope: KeyChoice,
    /// When the person pressed *Allow*.
    #[personal(category = "activity", basis = "contract")]
    pub allowed_at: Timestamp,
    /// The last time the app made a call, to the minute.
    #[personal(category = "activity", basis = "contract")]
    pub last_used_at: Option<Timestamp>,
    /// The latest moment the grant works at all, whatever its tokens say.
    #[not_personal]
    pub ends_at: Timestamp,
    /// When it ended, if it did.
    #[personal(category = "activity", basis = "contract")]
    pub ended_at: Option<Timestamp>,
    /// Why it ended.
    #[not_personal]
    pub ended: Option<GrantEnd>,
}

impl OAuthGrant {
    /// Whether the grant works at `now`.
    #[must_use]
    pub fn works_at(&self, now: Timestamp) -> bool {
        self.ended_at.is_none() && now < self.ends_at
    }
}

/// One project a grant is limited to. The grant counts in it and its sub-projects.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct OAuthGrantProject {
    /// The grant.
    #[not_personal]
    pub grant: GrantId,
    /// The project, by ID so moving it changes nothing.
    #[not_personal]
    pub project: ProjectId,
}

/// A token of a grant, as stored: only its hash.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct OAuthToken {
    /// The hash of the secret. The secret is sent once and never stored.
    #[personal(category = "credentials", basis = "contract")]
    pub secret_hash: Digest,
    /// The grant it belongs to.
    #[not_personal]
    pub grant: GrantId,
    /// What it is for.
    #[not_personal]
    pub kind: TokenKind,
    /// When it was made.
    #[not_personal]
    pub created_at: Timestamp,
    /// When it runs out.
    #[not_personal]
    pub expires_at: Timestamp,
    /// When it was used (a renewing token is used once).
    #[not_personal]
    pub used_at: Option<Timestamp>,
}

/// A code an app got after *Allow*, as stored: only its hash. It works once.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct OAuthCode {
    /// The hash of the code.
    #[personal(category = "credentials", basis = "contract")]
    pub secret_hash: Digest,
    /// The grant the code makes work.
    #[not_personal]
    pub grant: GrantId,
    /// The address the code was given for; the exchange has to name the same one.
    #[personal(category = "content", basis = "contract")]
    pub return_address: String,
    /// The PKCE challenge the app sent; the exchange has to prove it.
    #[personal(category = "credentials", basis = "contract")]
    pub challenge: String,
    /// When it runs out.
    #[not_personal]
    pub expires_at: Timestamp,
    /// When it was used.
    #[not_personal]
    pub used_at: Option<Timestamp>,
}

/// A web address an administrator allowed apps to come back to (M7-Q4).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct OAuthWebAddress {
    /// The return address, as an app has to write it.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub address: String,
    /// The administrator who allowed it.
    #[personal(category = "activity", basis = "legitimate_interests")]
    pub added_by: UserId,
    /// When.
    #[not_personal]
    pub added_at: Timestamp,
}

/// Where an app comes back to, and how much that tells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReturnKind {
    /// An address on the person's own computer: `http://127.0.0.1` or `http://[::1]`.
    OnThisComputer,
    /// A web address (`https://…`): an administrator has to allow it.
    Web,
}

/// A return address that was read and is one of the two allowed kinds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReturnAddress<'a> {
    kind: ReturnKind,
    /// Everything before the port, for a loopback address; the whole address otherwise.
    before_port: &'a str,
    /// Everything after the port.
    after_port: &'a str,
    whole: &'a str,
}

const LOOPBACK_V4: &str = "http://127.0.0.1";
const LOOPBACK_V6: &str = "http://[::1]";

impl<'a> ReturnAddress<'a> {
    /// Reads an address. `None` for anything but a loopback address written as an address
    /// (`http://127.0.0.1` or `http://[::1]`, not `localhost`: RFC 8252 §8.3) or an `https` web
    /// address; also for one with a fragment, with a name and password in it, with a space or a
    /// control character, with `..` parts or too long (RFC 6749 §3.1.2).
    #[must_use]
    pub fn read(address: &'a str) -> Option<Self> {
        if address.is_empty()
            || address.len() > RETURN_ADDRESS_LENGTH_MAX
            || address
                .chars()
                .any(|each| each.is_control() || each.is_whitespace() || !each.is_ascii())
            || address.contains('#')
        {
            return None;
        }
        for loopback in [LOOPBACK_V4, LOOPBACK_V6] {
            if let Some(rest) = address.strip_prefix(loopback) {
                let after_port = rest.strip_prefix(':').map_or(rest, |with_port| {
                    let digits = with_port.chars().take_while(char::is_ascii_digit).count();
                    &with_port[digits..]
                });
                let port_ok = rest.strip_prefix(':').is_none_or(|with_port| {
                    let digits = with_port.chars().take_while(char::is_ascii_digit).count();
                    (1..=5).contains(&digits)
                        && with_port[..digits]
                            .parse::<u32>()
                            .is_ok_and(|port| (1..=65_535).contains(&port))
                });
                if !port_ok || !(after_port.is_empty() || after_port.starts_with(['/', '?'])) {
                    return None;
                }
                return Some(Self {
                    kind: ReturnKind::OnThisComputer,
                    before_port: loopback,
                    after_port,
                    whole: address,
                });
            }
        }
        let rest = address.strip_prefix("https://")?;
        let authority = rest.split(['/', '?']).next().unwrap_or_default();
        if authority.is_empty() || authority.contains('@') || rest.contains("/../") {
            return None;
        }
        Some(Self {
            kind: ReturnKind::Web,
            before_port: address,
            after_port: "",
            whole: address,
        })
    }

    /// Where it is.
    #[must_use]
    pub const fn kind(&self) -> ReturnKind {
        self.kind
    }

    /// The address as it was written.
    #[must_use]
    pub const fn as_str(&self) -> &'a str {
        self.whole
    }

    /// Whether `presented` is this registered address: the whole text for a web address, and for a
    /// loopback address everything but the port, because the app picks a free one at each sign-in
    /// (RFC 8252 §7.3).
    #[must_use]
    pub fn accepts(&self, presented: &str) -> bool {
        match self.kind {
            ReturnKind::Web => presented == self.whole,
            ReturnKind::OnThisComputer => ReturnAddress::read(presented).is_some_and(|other| {
                other.kind == ReturnKind::OnThisComputer
                    && other.before_port == self.before_port
                    && other.after_port == self.after_port
            }),
        }
    }
}

/// Whether `presented` is one of the `registered` addresses.
#[must_use]
pub fn is_registered(registered: &[String], presented: &str) -> bool {
    registered
        .iter()
        .filter_map(|each| ReturnAddress::read(each))
        .any(|each| each.accepts(presented))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_22_only_the_two_kinds_of_return_address_are_read() {
        let kind = |text| ReturnAddress::read(text).map(|address| address.kind());
        assert_eq!(
            kind("http://127.0.0.1:5000/cb"),
            Some(ReturnKind::OnThisComputer)
        );
        assert_eq!(
            kind("http://127.0.0.1/cb"),
            Some(ReturnKind::OnThisComputer)
        );
        assert_eq!(
            kind("http://[::1]:49152/callback?x=1"),
            Some(ReturnKind::OnThisComputer)
        );
        assert_eq!(kind("https://app.example.com/cb"), Some(ReturnKind::Web));
        for refused in [
            "http://localhost:5000/cb",
            "http://127.0.0.1.evil.example/cb",
            "http://127.0.0.1@evil.example/cb",
            "http://127.0.0.1:0/cb",
            "http://127.0.0.1:99999/cb",
            "http://example.com/cb",
            "https://",
            "https://user:pass@example.com/cb",
            "https://example.com/cb#fragment",
            "https://example.com/a b",
            "myapp://callback",
            "javascript:alert(1)",
            "",
        ] {
            assert_eq!(kind(refused), None, "{refused}");
        }
    }

    #[test]
    fn ai_22_the_port_of_a_loopback_address_is_not_compared_and_nothing_else_is_loose() {
        let registered = vec!["http://127.0.0.1/callback".to_owned()];
        assert!(is_registered(
            &registered,
            "http://127.0.0.1:51234/callback"
        ));
        assert!(is_registered(&registered, "http://127.0.0.1/callback"));
        assert!(!is_registered(&registered, "http://127.0.0.1:51234/other"));
        assert!(!is_registered(&registered, "http://[::1]:51234/callback"));
        let web = vec!["https://app.example.com/cb".to_owned()];
        assert!(is_registered(&web, "https://app.example.com/cb"));
        assert!(!is_registered(&web, "https://app.example.com/cb2"));
        assert!(!is_registered(&web, "https://app.example.com:8443/cb"));
    }

    #[test]
    fn ai_22_an_app_can_be_given_the_two_plain_choices_only() {
        assert!(KeyChoice::SeeOnly.is_for_apps());
        assert!(KeyChoice::SeeAndChange.is_for_apps());
        assert!(!KeyChoice::Everything.is_for_apps());
        assert!(!KeyChoice::Permissions.is_for_apps());
    }
}

/// Writes `text` for the query of an address or the body of a form: everything but letters,
/// digits and `-._~` as `%` and two hexadecimal digits (RFC 3986 §2.3).
#[must_use]
pub fn percent_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

/// Reads text written by [`percent_encode`], with `+` for a space as forms write it; `None` for a
/// broken escape or text that is not UTF-8.
#[must_use]
pub fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while let Some(&byte) = bytes.get(at) {
        match byte {
            b'+' => out.push(b' '),
            b'%' => {
                let pair = text.get(at + 1..at + 3)?;
                out.push(u8::from_str_radix(pair, 16).ok()?);
                at += 2;
            }
            other => out.push(other),
        }
        at += 1;
    }
    String::from_utf8(out).ok()
}

/// The pairs of a form body or a query (`a=b&c=d`), decoded; `None` when one is broken.
#[must_use]
pub fn form_pairs(body: &str) -> Option<Vec<(String, String)>> {
    body.split('&')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let (name, value) = part.split_once('=').unwrap_or((part, ""));
            Some((percent_decode(name)?, percent_decode(value)?))
        })
        .collect()
}

/// `address` with the `pairs` added to its query, the way an app wants its answer (RFC 6749
/// §4.1.2).
#[must_use]
pub fn with_query(address: &str, pairs: &[(&str, &str)]) -> String {
    let mut out = address.to_owned();
    out.push(if address.contains('?') { '&' } else { '?' });
    let written: Vec<String> = pairs
        .iter()
        .map(|(name, value)| format!("{}={}", percent_encode(name), percent_encode(value)))
        .collect();
    out.push_str(&written.join("&"));
    out
}

#[cfg(test)]
mod form_tests {
    use super::*;

    #[test]
    fn ai_22_text_goes_into_a_query_and_back() {
        let tricky = "a b&c=d/é?";
        assert_eq!(
            percent_decode(&percent_encode(tricky)).as_deref(),
            Some(tricky)
        );
        assert_eq!(
            with_query(
                "http://127.0.0.1:1/cb",
                &[("code", "x y"), ("state", "s&1")]
            ),
            "http://127.0.0.1:1/cb?code=x%20y&state=s%261"
        );
        assert_eq!(
            with_query("https://a.example/cb?x=1", &[("code", "c")]),
            "https://a.example/cb?x=1&code=c"
        );
        assert_eq!(
            form_pairs("grant_type=refresh_token&refresh_token=a%2Bb+c").unwrap(),
            vec![
                ("grant_type".to_owned(), "refresh_token".to_owned()),
                ("refresh_token".to_owned(), "a+b c".to_owned())
            ]
        );
        assert_eq!(percent_decode("%zz"), None);
        assert_eq!(percent_decode("%"), None);
    }
}
