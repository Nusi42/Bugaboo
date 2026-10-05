//! The cookies Bugaboo sets, and the one place that sets them (COMP-13, PRIV-01, SEC-COOKIES).
//!
//! Only strictly necessary cookies exist, so no consent banner is needed: the session and the
//! CSRF token. Every cookie is in the table below, once. The name starts with `__Host-`, which
//! makes browsers refuse the cookie unless it is `Secure`, has `Path=/` and names no `Domain`;
//! the code adds exactly those attributes, `HttpOnly` where script has no business reading the
//! cookie, and `SameSite`. No other code in the product sets a cookie (SEC-COOKIES). Browsers
//! accept `Secure` cookies on `localhost` over plain HTTP, so working on this computer without
//! HTTPS still works.
//!
//! `cargo xtask gen api` reads the table and writes the list "cookies we use" for the user
//! docs (`userDocs/cookies.md`) and the names for the browser client.

use std::fmt::Write as _;
use std::time::Duration;

use axum::http::HeaderValue;
use bb_core::message::{self, MessageKey};

/// The request header that carries the CSRF token on every change request (architecture §7).
pub const CSRF_HEADER: &str = "x-csrf-token";

/// How far a browser may send a cookie along with requests from other websites.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SameSite {
    /// Only for requests that start on Bugaboo's own pages.
    Strict,
    /// Also when a person arrives from a link on another website.
    Lax,
}

impl SameSite {
    const fn attribute(self) -> &'static str {
        match self {
            Self::Strict => "SameSite=Strict",
            Self::Lax => "SameSite=Lax",
        }
    }
}

/// Declares the cookies. One line per cookie: variant, name, whether script is shut out
/// (`HttpOnly`), `SameSite` and the message that says what the cookie is for.
macro_rules! cookies {
    ($($(#[$doc:meta])* $variant:ident => ($name:literal, $http_only:literal, $same_site:ident, $purpose:path)),+ $(,)?) => {
        /// A cookie Bugaboo sets.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Cookie {
            $($(#[$doc])* $variant),+
        }

        impl Cookie {
            /// Every cookie, in the order of the table.
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            /// The name, which starts with `__Host-`.
            #[must_use]
            pub const fn name(self) -> &'static str {
                match self { $(Self::$variant => $name),+ }
            }

            /// Whether script on the page is kept from reading the cookie.
            #[must_use]
            pub const fn http_only(self) -> bool {
                match self { $(Self::$variant => $http_only),+ }
            }

            /// When the browser sends the cookie along.
            #[must_use]
            pub const fn same_site(self) -> SameSite {
                match self { $(Self::$variant => SameSite::$same_site),+ }
            }

            /// The message that says what the cookie is for.
            #[must_use]
            pub const fn purpose(self) -> MessageKey {
                match self { $(Self::$variant => $purpose),+ }
            }
        }
    };
}

cookies! {
    /// Keeps a person signed in. Script never reads it.
    Session => ("__Host-bb_session", true, Lax, message::COOKIE_SESSION_PURPOSE),
    /// The token the browser app sends back in [`CSRF_HEADER`]. Script has to read it, so it
    /// is not `HttpOnly`; it protects against forged requests and opens no door by itself.
    Csrf => ("__Host-bb_csrf", false, Strict, message::COOKIE_CSRF_PURPOSE),
}

/// The value cannot be a cookie value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidCookieValue;

impl std::fmt::Display for InvalidCookieValue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("not a valid cookie value")
    }
}

impl std::error::Error for InvalidCookieValue {}

/// Whether a byte may be in a cookie value (RFC 6265): printable ASCII without space, `"`, `,`,
/// `;` and `\`.
const fn is_cookie_byte(byte: u8) -> bool {
    matches!(byte, 0x21 | 0x23..=0x2B | 0x2D..=0x3A | 0x3C..=0x5B | 0x5D..=0x7E)
}

/// Whether an answer sets or clears any cookie.
#[must_use]
pub fn sets_a_cookie(headers: &axum::http::HeaderMap) -> bool {
    headers.contains_key(axum::http::header::SET_COOKIE)
}

impl Cookie {
    /// The `Set-Cookie` header that gives the cookie `value`. With `max_age` the browser keeps
    /// the cookie that long; without, until the browser closes.
    ///
    /// # Errors
    /// [`InvalidCookieValue`] when the value is empty or has a character a cookie cannot hold
    /// (session and CSRF tokens are base64url text, which is fine).
    pub fn set(
        self,
        value: &str,
        max_age: Option<Duration>,
    ) -> Result<HeaderValue, InvalidCookieValue> {
        if value.is_empty() || !value.bytes().all(is_cookie_byte) {
            return Err(InvalidCookieValue);
        }
        let mut text = self.attributes(value);
        if let Some(max_age) = max_age {
            let _ = write!(text, "; Max-Age={}", max_age.as_secs());
        }
        HeaderValue::from_str(&text).map_err(|_| InvalidCookieValue)
    }

    /// Adds the cookie with `value` to `response`.
    ///
    /// # Errors
    /// [`InvalidCookieValue`] as for [`Self::set`].
    pub fn put(
        self,
        response: &mut axum::response::Response,
        value: &str,
        max_age: Option<Duration>,
    ) -> Result<(), InvalidCookieValue> {
        response
            .headers_mut()
            .append(axum::http::header::SET_COOKIE, self.set(value, max_age)?);
        Ok(())
    }

    /// Adds the header that makes the browser forget the cookie to `response`.
    pub fn forget(self, response: &mut axum::response::Response) {
        response
            .headers_mut()
            .append(axum::http::header::SET_COOKIE, self.clear());
    }

    /// The value the browser sent for this cookie, if it sent one. A request may carry several
    /// `Cookie` headers (HTTP/2 splits them), so all of them are read.
    #[must_use]
    pub fn read(self, headers: &axum::http::HeaderMap) -> Option<String> {
        headers
            .get_all(axum::http::header::COOKIE)
            .iter()
            .filter_map(|line| line.to_str().ok())
            .flat_map(|line| line.split(';'))
            .filter_map(|pair| pair.trim().split_once('='))
            .find(|(name, _)| *name == self.name())
            .map(|(_, value)| value.to_owned())
    }

    /// The `Set-Cookie` header that makes the browser forget the cookie (sign-out).
    #[must_use]
    pub fn clear(self) -> HeaderValue {
        // Built from the table and fixed words only, so it is always a valid header value.
        HeaderValue::from_str(&format!("{}; Max-Age=0", self.attributes("")))
            .unwrap_or_else(|_| HeaderValue::from_static(""))
    }

    /// Whether `headers` (of an answer) make the browser forget this cookie.
    #[must_use]
    pub fn is_cleared_in(self, headers: &axum::http::HeaderMap) -> bool {
        let cleared = self.clear();
        headers
            .get_all(axum::http::header::SET_COOKIE)
            .iter()
            .any(|header| *header == cleared)
    }

    fn attributes(self, value: &str) -> String {
        let http_only = if self.http_only() { "; HttpOnly" } else { "" };
        format!(
            "{}={value}; Path=/; Secure{http_only}; {}",
            self.name(),
            self.same_site().attribute()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(value: &HeaderValue) -> &str {
        value.to_str().unwrap()
    }

    #[test]
    fn sec_12_an_answer_that_forgets_the_cookie_is_recognised() {
        let mut headers = axum::http::HeaderMap::new();
        assert!(!sets_a_cookie(&headers));
        assert!(!Cookie::Session.is_cleared_in(&headers));
        headers.append(
            axum::http::header::SET_COOKIE,
            Cookie::Session.set("abc", None).unwrap(),
        );
        assert!(sets_a_cookie(&headers));
        assert!(!Cookie::Session.is_cleared_in(&headers));
        headers.append(axum::http::header::SET_COOKIE, Cookie::Csrf.clear());
        assert!(!Cookie::Session.is_cleared_in(&headers));
        assert!(Cookie::Csrf.is_cleared_in(&headers));
        headers.append(axum::http::header::SET_COOKIE, Cookie::Session.clear());
        assert!(Cookie::Session.is_cleared_in(&headers));
    }

    #[test]
    fn comp_13_every_cookie_is_host_prefixed_secure_and_has_a_same_site_rule() {
        for cookie in Cookie::ALL {
            assert!(cookie.name().starts_with("__Host-"), "{}", cookie.name());
            let header = cookie.set("abc-DEF_123", None).unwrap();
            let header = text(&header);
            let parts: Vec<&str> = header.split("; ").collect();
            assert_eq!(parts[0], format!("{}=abc-DEF_123", cookie.name()));
            assert!(parts.contains(&"Path=/"), "{header}");
            assert!(parts.contains(&"Secure"), "{header}");
            assert!(
                parts.contains(&"SameSite=Strict") || parts.contains(&"SameSite=Lax"),
                "{header}"
            );
            assert_eq!(parts.contains(&"HttpOnly"), cookie.http_only(), "{header}");
            // `__Host-` forbids a Domain.
            assert!(parts.iter().all(|p| !p.starts_with("Domain")), "{header}");
        }
    }

    #[test]
    fn comp_13_the_session_cookie_is_closed_to_script_and_the_csrf_cookie_is_not() {
        let session = Cookie::Session
            .set("t", Some(Duration::from_hours(1)))
            .unwrap();
        assert_eq!(
            text(&session),
            "__Host-bb_session=t; Path=/; Secure; HttpOnly; SameSite=Lax; Max-Age=3600"
        );
        let csrf = Cookie::Csrf.set("t", None).unwrap();
        assert_eq!(
            text(&csrf),
            "__Host-bb_csrf=t; Path=/; Secure; SameSite=Strict"
        );
    }

    #[test]
    fn comp_13_clearing_keeps_the_attributes_and_ends_the_cookie_now() {
        assert_eq!(
            text(&Cookie::Session.clear()),
            "__Host-bb_session=; Path=/; Secure; HttpOnly; SameSite=Lax; Max-Age=0"
        );
    }

    #[test]
    fn comp_13_the_value_of_a_cookie_is_read_from_all_cookie_headers() {
        let mut headers = axum::http::HeaderMap::new();
        headers.append(
            axum::http::header::COOKIE,
            HeaderValue::from_static("other=1; __Host-bb_session=abc"),
        );
        headers.append(
            axum::http::header::COOKIE,
            HeaderValue::from_static("__Host-bb_csrf=def"),
        );
        assert_eq!(Cookie::Session.read(&headers).as_deref(), Some("abc"));
        assert_eq!(Cookie::Csrf.read(&headers).as_deref(), Some("def"));
        assert_eq!(Cookie::Session.read(&axum::http::HeaderMap::new()), None);
        // A longer name that only ends the same way is another cookie.
        let mut other = axum::http::HeaderMap::new();
        other.append(
            axum::http::header::COOKIE,
            HeaderValue::from_static("x__Host-bb_session=abc"),
        );
        assert_eq!(Cookie::Session.read(&other), None);
    }

    #[test]
    fn comp_13_values_that_could_add_attributes_or_headers_are_refused() {
        for bad in [
            "",
            "a b",
            "a;b",
            "a,b",
            "a\"b",
            "a\\b",
            "a\r\nSet-Cookie: x=y",
            "\u{e4}",
        ] {
            assert_eq!(
                Cookie::Session.set(bad, None),
                Err(InvalidCookieValue),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn comp_13_every_cookie_says_what_it_is_for() {
        for cookie in Cookie::ALL {
            assert!(!message::Message::from(cookie.purpose()).render().is_empty());
        }
    }
}
