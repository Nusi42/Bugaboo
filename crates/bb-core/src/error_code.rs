//! Error codes: the one list of everything the API can say went wrong (architecture §7, UX §9).
//!
//! Each code has a stable text (`auth.wrong_credentials`) that clients can rely on, the HTTP
//! status it maps to, and the key of the message a person reads. The API (`bb-api`) answers with
//! [`Problem`]; the TypeScript union of codes is generated from this list.
//!
//! Safety rule: an error never reveals a hidden thing. Whatever the person may not see answers
//! with [`ErrorCode::NotFound`], exactly as if it did not exist, and no error message has a
//! placeholder, so no name, ID or count can leak through it. ARCH-ERRORS checks the table and the
//! catalog.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::message::{self, Message, MessageKey};

/// Declares the codes. One line per code: variant, text, HTTP status, message.
macro_rules! error_codes {
    ($($(#[$doc:meta])* $variant:ident => ($code:literal, $status:literal, $message:path)),+ $(,)?) => {
        /// Something that can go wrong, as the API reports it.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[non_exhaustive]
        pub enum ErrorCode {
            $($(#[$doc])* $variant),+
        }

        impl ErrorCode {
            /// Every code, in the order of the table.
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            /// The stable text of the code, like `auth.wrong_credentials`.
            #[must_use]
            pub const fn code(self) -> &'static str {
                match self { $(Self::$variant => $code),+ }
            }

            /// The HTTP status the API answers with.
            #[must_use]
            pub const fn status(self) -> u16 {
                match self { $(Self::$variant => $status),+ }
            }

            /// The message a person reads.
            #[must_use]
            pub const fn message(self) -> MessageKey {
                match self { $(Self::$variant => $message),+ }
            }
        }
    };
}

error_codes! {
    /// The e-mail address or the password is wrong. Never says which one.
    WrongCredentials => ("auth.wrong_credentials", 401, message::ERROR_AUTH_WRONG_CREDENTIALS),
    /// The request needs a signed-in person.
    NotSignedIn => ("auth.not_signed_in", 401, message::ERROR_AUTH_NOT_SIGNED_IN),
    /// The person can see the thing but may not do this to it.
    Forbidden => ("access.forbidden", 403, message::ERROR_ACCESS_FORBIDDEN),
    /// There is no such thing, or the person may not know about it.
    NotFound => ("resource.not_found", 404, message::ERROR_RESOURCE_NOT_FOUND),
    /// The request is not valid.
    InvalidRequest => ("request.invalid", 400, message::ERROR_REQUEST_INVALID),
    /// The request is bigger than allowed.
    TooLarge => ("request.too_large", 413, message::ERROR_REQUEST_TOO_LARGE),
    /// Someone else changed the thing since the person loaded it (ADR-0019).
    VersionChanged => ("edit.version_changed", 409, message::ERROR_EDIT_VERSION_CHANGED),
    /// Too many requests in a short time (SEC-14).
    TooManyRequests => ("rate_limit.exceeded", 429, message::ERROR_RATE_LIMIT_EXCEEDED),
    /// The server cannot answer right now.
    Unavailable => ("server.unavailable", 503, message::ERROR_SERVER_UNAVAILABLE),
    /// The server failed; the person did nothing wrong.
    Internal => ("server.internal", 500, message::ERROR_SERVER_INTERNAL),
}

/// The text is not the code of any error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnknownErrorCode;

impl fmt::Display for UnknownErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("not the code of any error")
    }
}

impl std::error::Error for UnknownErrorCode {}

impl fmt::Display for ErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl FromStr for ErrorCode {
    type Err = UnknownErrorCode;

    fn from_str(text: &str) -> Result<Self, UnknownErrorCode> {
        Self::ALL
            .iter()
            .copied()
            .find(|code| code.code() == text)
            .ok_or(UnknownErrorCode)
    }
}

impl Serialize for ErrorCode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.code())
    }
}

impl<'de> Deserialize<'de> for ErrorCode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = <std::borrow::Cow<'de, str>>::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

/// The content type of an error answer (RFC 9457).
pub const PROBLEM_CONTENT_TYPE: &str = "application/problem+json";

/// The body of an error answer, as plain data. The HTTP layer only sends it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Problem {
    /// A stable address for this kind of error.
    #[serde(rename = "type")]
    pub kind: String,
    /// The message for people, in English until languages are chosen per request.
    pub title: String,
    /// The HTTP status.
    pub status: u16,
    /// The error code clients react to.
    pub code: ErrorCode,
    /// More help, if there is any; also from the message catalog.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl Problem {
    /// The error answer for `code`.
    #[must_use]
    pub fn new(code: ErrorCode) -> Self {
        Self {
            kind: format!("urn:bugaboo:error:{}", code.code()),
            title: Message::from(code.message()).render(),
            status: code.status(),
            code,
            detail: None,
        }
    }

    /// Adds a longer explanation. It is a catalog message like every other text.
    #[must_use]
    pub fn with_detail(mut self, detail: &Message) -> Self {
        self.detail = Some(detail.render());
        self
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn codes_are_unique_and_shaped_like_area_dot_name() {
        let mut seen = HashSet::new();
        for code in ErrorCode::ALL {
            let text = code.code();
            assert!(seen.insert(text), "{text} is used twice");
            let (area, name) = text.split_once('.').unwrap();
            for part in [area, name] {
                assert!(
                    !part.is_empty() && part.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                    "{text}"
                );
            }
            assert!((400..600).contains(&code.status()), "{text}");
        }
    }

    #[test]
    fn every_code_has_its_own_message_with_no_placeholders() {
        let mut keys = HashSet::new();
        for code in ErrorCode::ALL {
            let key = code.message();
            assert!(keys.insert(key), "{key} is shared");
            assert!(!key.template().contains('{'), "{key} would leak a value");
            assert!(!Message::from(key).render().is_empty());
        }
    }

    #[test]
    fn codes_round_trip_as_text() {
        for code in ErrorCode::ALL {
            assert_eq!(code.code().parse(), Ok(*code));
            let json = serde_json::to_string(code).unwrap();
            assert_eq!(serde_json::from_str::<ErrorCode>(&json).unwrap(), *code);
        }
        assert_eq!("auth.nothing".parse::<ErrorCode>(), Err(UnknownErrorCode));
    }

    #[test]
    fn problem_body_follows_rfc_9457() {
        let problem = Problem::new(ErrorCode::WrongCredentials);
        let json = serde_json::to_value(&problem).unwrap();
        assert_eq!(json["type"], "urn:bugaboo:error:auth.wrong_credentials");
        assert_eq!(json["status"], 401);
        assert_eq!(json["code"], "auth.wrong_credentials");
        assert_eq!(
            json["title"],
            "The e-mail address or the password is not right."
        );
        assert!(json.get("detail").is_none());
    }

    #[test]
    fn wrong_credentials_never_says_which_part_was_wrong() {
        let text = Message::from(ErrorCode::WrongCredentials.message()).render();
        assert!(text.contains("e-mail address or the password"));
    }
}
