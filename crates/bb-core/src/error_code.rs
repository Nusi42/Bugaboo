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

use std::collections::BTreeMap;

use serde::Serialize;

use crate::message::{self, Message, MessageKey};

/// Declares the codes. One line per code: variant, text, HTTP status, message. The word, its
/// lists and its serde form come from [`words!`]; the status and the message are added here.
macro_rules! error_codes {
    ($($(#[$doc:meta])* $variant:ident => ($code:literal, $status:literal, $message:path)),+ $(,)?) => {
        words! {
            /// Something that can go wrong, as the API reports it. The word is the stable text of
            /// the code, like `auth.wrong_credentials`.
            pub enum ErrorCode {
                $($(#[$doc])* $variant => $code),+
            }
        }

        impl ErrorCode {
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
    /// The code of the second step (code app or backup code) is wrong or was used already.
    WrongSecondStepCode => ("auth.wrong_second_step_code", 401, message::ERROR_AUTH_WRONG_SECOND_STEP_CODE),
    /// The setup code of the first start is wrong.
    WrongSetupCode => ("auth.wrong_setup_code", 401, message::ERROR_AUTH_WRONG_SETUP_CODE),
    /// The new password is shorter than 12 characters.
    PasswordTooShort => ("auth.password_too_short", 400, message::ERROR_AUTH_PASSWORD_TOO_SHORT),
    /// The new password is in the list of passwords that are tried first.
    PasswordBreached => ("auth.password_breached", 400, message::ERROR_AUTH_PASSWORD_BREACHED),
    /// The invitation link does not work: unknown, used, taken back or expired. Never says which.
    InvitationInvalid => ("auth.invitation_invalid", 400, message::ERROR_AUTH_INVITATION_INVALID),
    /// Signing up on one's own is not possible here. Never says why.
    SignUpRefused => ("auth.sign_up_refused", 403, message::ERROR_AUTH_SIGN_UP_REFUSED),
    /// A passkey or a passkey ceremony did not check out.
    PasskeyRejected => ("auth.passkey_rejected", 400, message::ERROR_AUTH_PASSKEY_REJECTED),
    /// The person must set up a second step before anything else (a role of theirs, or the
    /// organization, asks for two-step sign-in and they have none yet).
    SecondStepRequired => ("auth.second_step_required", 403, message::ERROR_AUTH_SECOND_STEP_REQUIRED),
    /// An access key was used where no key may go: the owner's own sign-in data.
    AccessKeyNotAllowed => ("auth.access_key_not_allowed", 403, message::ERROR_AUTH_ACCESS_KEY_NOT_ALLOWED),
    /// The request needs a signed-in person.
    NotSignedIn => ("auth.not_signed_in", 401, message::ERROR_AUTH_NOT_SIGNED_IN),
    /// The person can see the thing but may not do this to it.
    Forbidden => ("access.forbidden", 403, message::ERROR_ACCESS_FORBIDDEN),
    /// The group has a *No access* in a project the person may not decide in (or know about), and
    /// taking people out of it or deleting it would lift that *No access* (ADR-0025, decision 6).
    CannotLift => ("access.cannot_lift", 403, message::ERROR_ACCESS_CANNOT_LIFT),
    /// The change would leave nobody who can manage roles and decide who has which role for the
    /// whole organization (ADR-0025, decision 6).
    WouldLockOut => ("access.would_lock_out", 409, message::ERROR_ACCESS_WOULD_LOCK_OUT),
    /// Somebody tried to add or give more than they hold in that place themselves (ADR-0025,
    /// decision 6).
    TooMuch => ("access.too_much", 403, message::ERROR_ACCESS_TOO_MUCH),
    /// A role that somebody has cannot be deleted without choosing the role that takes its place.
    RoleInUse => ("role.in_use", 409, message::ERROR_ROLE_IN_USE),
    /// A person or group would have a role and *No access* in the same project.
    AssignmentConflict => ("assignment.conflict", 409, message::ERROR_ASSIGNMENT_CONFLICT),
    /// An administrator locked the setting, so nobody lower down may change it (CFG-06).
    SettingLocked => ("setting.locked", 409, message::ERROR_SETTING_LOCKED),
    /// A move would let somebody take away a *No access* that applies to them (WP-08, M2-F23).
    MoveLiftsNoAccess => ("project.move_lifts_no_access", 409, message::ERROR_PROJECT_MOVE_LIFTS_NO_ACCESS),
    /// A project cannot go inside itself or one of its own sub-projects.
    MoveIntoItself => ("project.move_into_itself", 409, message::ERROR_PROJECT_MOVE_INTO_ITSELF),
    /// A project would be nested deeper than a tree of projects may go.
    ProjectTooDeep => ("project.too_deep", 409, message::ERROR_PROJECT_TOO_DEEP),
    /// The projects, or who has which role, changed since the move was previewed.
    MovePreviewChanged => ("project.preview_changed", 409, message::ERROR_PROJECT_PREVIEW_CHANGED),
    /// A name of a group, role or project is used already.
    NameTaken => ("name.taken", 409, message::ERROR_NAME_TAKEN),
    /// There is no such thing, or the person may not know about it.
    NotFound => ("resource.not_found", 404, message::ERROR_RESOURCE_NOT_FOUND),
    /// The request is not valid.
    InvalidRequest => ("request.invalid", 400, message::ERROR_REQUEST_INVALID),
    /// A list was asked for a field that does not exist, or that the person may not read. Both
    /// look the same, so the answer never tells which fields are hidden (SEC-06, PERF-02).
    UnknownField => ("request.unknown_field", 400, message::ERROR_REQUEST_UNKNOWN_FIELD),
    /// The cursor of a list page was changed or is not one of ours (PERF-02).
    BadCursor => ("request.bad_cursor", 400, message::ERROR_REQUEST_BAD_CURSOR),
    /// A change request that does not come from one of Bugaboo's own pages.
    WrongOrigin => ("request.wrong_origin", 403, message::ERROR_REQUEST_WRONG_ORIGIN),
    /// A change request without the right token of the session (CSRF).
    WrongToken => ("request.wrong_token", 403, message::ERROR_REQUEST_WRONG_TOKEN),
    /// The address exists but does not do that (wrong HTTP method).
    MethodNotAllowed => ("request.method_not_allowed", 405, message::ERROR_REQUEST_METHOD_NOT_ALLOWED),
    /// The request took too long to arrive.
    TimedOut => ("request.timed_out", 408, message::ERROR_REQUEST_TIMED_OUT),
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

words_in_api!(
    ErrorCode,
    "What went wrong. Clients react to this text, which never changes."
);

/// The content type of an error answer (RFC 9457).
pub const PROBLEM_CONTENT_TYPE: &str = "application/problem+json";

/// The body of an error answer, as plain data. The HTTP layer only sends it.
///
/// Clients react to `code` and show the message for `message_key` and `params` in their own
/// language. `title` is the English text, only a fallback for clients that cannot do that. The
/// fields are private and the only way to make a `Problem` is from an [`ErrorCode`], so no answer
/// carries a text or a status that does not belong to its code.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Problem {
    /// A stable address of the kind of error, `urn:bugaboo:error:<code>`.
    #[serde(rename = "type")]
    kind: String,
    /// The English text, a fallback for clients that cannot look up the message.
    title: String,
    /// The HTTP status.
    status: u16,
    /// What went wrong.
    code: ErrorCode,
    /// The key of the message to show, in the message catalog.
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    message_key: MessageKey,
    /// Values for the placeholders of the message. Empty today: error messages have none.
    params: BTreeMap<String, serde_json::Value>,
    /// The ID of the request, also in the `x-request-id` header, to quote when asking for help.
    #[serde(skip_serializing_if = "Option::is_none")]
    request_id: Option<String>,
}

impl Problem {
    /// The error answer for `code`. `params` stays empty: error messages have no placeholders.
    #[must_use]
    pub fn new(code: ErrorCode) -> Self {
        Self {
            kind: format!("urn:bugaboo:error:{}", code.code()),
            title: Message::from(code.message()).render(),
            status: code.status(),
            code,
            message_key: code.message(),
            params: BTreeMap::new(),
            request_id: None,
        }
    }

    /// Adds the ID of the request, so a person can quote it to support. Set by `bb-api`.
    #[must_use]
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// The error code.
    #[must_use]
    pub const fn code(&self) -> ErrorCode {
        self.code
    }

    /// The HTTP status.
    #[must_use]
    pub const fn status(&self) -> u16 {
        self.status
    }

    /// The key of the message to show.
    #[must_use]
    pub const fn message_key(&self) -> MessageKey {
        self.message_key
    }

    /// The English text, a fallback for clients that cannot look up the message.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The ID of the request, if `bb-api` set one.
    #[must_use]
    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::UnknownWord;

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
        assert_eq!("auth.nothing".parse::<ErrorCode>(), Err(UnknownWord));
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
        assert_eq!(json["message_key"], "error.auth.wrong-credentials");
        assert_eq!(json["params"], serde_json::json!({}));
        assert!(json.get("request_id").is_none());
        let traced = Problem::new(ErrorCode::NotFound).with_request_id("r-1");
        assert_eq!(serde_json::to_value(&traced).unwrap()["request_id"], "r-1");
        assert_eq!(traced.request_id(), Some("r-1"));
        assert_eq!(traced.status(), 404);
    }

    #[test]
    fn wrong_credentials_never_says_which_part_was_wrong() {
        let text = Message::from(ErrorCode::WrongCredentials.message()).render();
        assert!(text.contains("e-mail address or the password"));
    }
}
