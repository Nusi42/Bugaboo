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

use crate::message::{Message, MessageKey};

/// Declares the codes. One line per code: variant, text, HTTP status. The word, its lists, its
/// serde form and its message (`error.<area>.<name>`, with `-` for `_`) come from [`words!`]; the
/// status is added here.
macro_rules! error_codes {
    ($($(#[$doc:meta])* $variant:ident => ($code:literal, $status:literal)),+ $(,)?) => {
        words! {
            /// Something that can go wrong, as the API reports it. The word is the stable text of
            /// the code, like `auth.wrong_credentials`.
            pub enum ErrorCode {
                $($(#[$doc])* $variant => $code),+
            }
            messages message = "error.";
        }

        impl ErrorCode {
            /// The HTTP status the API answers with.
            #[must_use]
            pub const fn status(self) -> u16 {
                match self { $(Self::$variant => $status),+ }
            }
        }
    };
}

error_codes! {
    /// The e-mail address or the password is wrong. Never says which one.
    WrongCredentials => ("auth.wrong_credentials", 401),
    /// The code of the second step (code app or backup code) is wrong or was used already.
    WrongSecondStepCode => ("auth.wrong_second_step_code", 401),
    /// The setup code of the first start is wrong.
    WrongSetupCode => ("auth.wrong_setup_code", 401),
    /// The new password is shorter than 12 characters.
    PasswordTooShort => ("auth.password_too_short", 400),
    /// The new password is in the list of passwords that are tried first.
    PasswordBreached => ("auth.password_breached", 400),
    /// The invitation link does not work: unknown, used, taken back or expired. Never says which.
    InvitationInvalid => ("auth.invitation_invalid", 400),
    /// Signing up on one's own is not possible here. Never says why.
    SignUpRefused => ("auth.sign_up_refused", 403),
    /// A passkey or a passkey ceremony did not check out.
    PasskeyRejected => ("auth.passkey_rejected", 400),
    /// The person must set up a second step before anything else (a role of theirs, or the
    /// organization, asks for two-step sign-in and they have none yet).
    SecondStepRequired => ("auth.second_step_required", 403),
    /// An access key was used where no key may go: the owner's own sign-in data.
    AccessKeyNotAllowed => ("auth.access_key_not_allowed", 403),
    /// The request needs a signed-in person.
    NotSignedIn => ("auth.not_signed_in", 401),
    /// The person can see the thing but may not do this to it.
    Forbidden => ("access.forbidden", 403),
    /// The group has a *No access* in a project the person may not decide in (or know about), and
    /// taking people out of it or deleting it would lift that *No access* (ADR-0025, decision 6).
    CannotLift => ("access.cannot_lift", 403),
    /// The change would leave nobody who can manage roles and decide who has which role for the
    /// whole organization (ADR-0025, decision 6).
    WouldLockOut => ("access.would_lock_out", 409),
    /// Somebody tried to add or give more than they hold in that place themselves (ADR-0025,
    /// decision 6).
    TooMuch => ("access.too_much", 403),
    /// A role that somebody has cannot be deleted without choosing the role that takes its place.
    RoleInUse => ("role.in_use", 409),
    /// A person or group would have a role and *No access* in the same project.
    AssignmentConflict => ("assignment.conflict", 409),
    /// An administrator locked the setting, so nobody lower down may change it (CFG-06).
    SettingLocked => ("setting.locked", 409),
    /// A list of hosts has an address or a path in it, not only host names (SEC-13).
    SettingHostsNotBare => ("setting.hosts_not_bare", 400),
    /// A move would let somebody take away a *No access* that applies to them (WP-08, M2-F23).
    MoveLiftsNoAccess => ("project.move_lifts_no_access", 409),
    /// A project cannot go inside itself or one of its own sub-projects.
    MoveIntoItself => ("project.move_into_itself", 409),
    /// A project would be nested deeper than a tree of projects may go.
    ProjectTooDeep => ("project.too_deep", 409),
    /// The projects, or who has which role, changed since the move was previewed.
    MovePreviewChanged => ("project.preview_changed", 409),
    /// An issue type that was retired takes no new issues (ISS-02).
    IssueTypeRetired => ("issue.type_retired", 409),
    /// An issue would sit under itself or under one of its own sub-issues, or the chain of
    /// sub-issues would get too long (ISS-03).
    ParentCircle => ("issue.parent_circle", 409),
    /// Those two issues are related in that way already (ISS-04).
    RelationExists => ("relation.exists", 409),
    /// Something holds as many as it may: an issue its relations or sub-issues, the organization
    /// its tags, labels or own fields, a field its choices. A full store always answers this code,
    /// never `request.invalid`. The count includes what the person may not see, as it is a limit of
    /// storage, so the answer names no issue.
    LimitReached => ("limit.reached", 409),
    /// A tag is limited to other projects than the one of the issue (TAG-01).
    TagNotHere => ("tag.not_here", 409),
    /// Some issue holds more than one label of a group that is to be a *Pick one* group (TAG-02).
    PickOneConflict => ("label.pick_one_conflict", 409),
    /// A required own field has no value (CF-03).
    FieldRequired => ("field.required", 400),
    /// A text, or a list of values, is shorter than the rules of the own field allow (CF-03).
    FieldTooShort => ("field.too_short", 400),
    /// A text, or a list of values, is longer than the rules of the own field allow (CF-03).
    FieldTooLong => ("field.too_long", 400),
    /// A number, day, moment or length of time is below the lowest the own field allows (CF-03).
    FieldTooSmall => ("field.too_small", 400),
    /// A number, day, moment or length of time is above the highest the own field allows (CF-03).
    FieldTooBig => ("field.too_big", 400),
    /// A text does not match the pattern of the own field (CF-03).
    FieldNoMatch => ("field.no_match", 400),
    /// A value is not one this kind of own field can hold (CF-01).
    FieldWrongValue => ("field.wrong_value", 400),
    /// A value names a choice that the own field does not offer (CF-04).
    FieldUnknownChoice => ("field.unknown_choice", 400),
    /// A value names a person, group or issue that the person who sets it may not name: it is not
    /// there, or they may not know about it (CF-01). Both look the same.
    FieldUnknownReference => ("field.unknown_reference", 400),
    /// An own field is not used for this project or this issue type (CF-03).
    FieldNotHere => ("field.not_here", 400),
    /// A name of a group, role or project is used already.
    NameTaken => ("name.taken", 409),
    /// There is no such thing, or the person may not know about it.
    NotFound => ("resource.not_found", 404),
    /// The request is not valid.
    InvalidRequest => ("request.invalid", 400),
    /// A list was asked for a field that does not exist, or that the person may not read. Both
    /// look the same, so the answer never tells which fields are hidden (SEC-06, PERF-02).
    UnknownField => ("request.unknown_field", 400),
    /// The cursor of a list page was changed or is not one of ours (PERF-02).
    BadCursor => ("request.bad_cursor", 400),
    /// The mail server turned an e-mail away for good: the sender or the address is not accepted
    /// (NTF-02).
    MailRejected => ("mail.rejected", 502),
    /// The mail server cannot be reached, or said to try later (NTF-02).
    MailUnavailable => ("mail.unavailable", 503),
    /// E-mail is off, or the mail server is not entered (PRIV-08).
    MailOff => ("mail.off", 409),
    /// A change request that does not come from one of Bugaboo's own pages.
    WrongOrigin => ("request.wrong_origin", 403),
    /// A change request without the right token of the session (CSRF).
    WrongToken => ("request.wrong_token", 403),
    /// The address exists but does not do that (wrong HTTP method).
    MethodNotAllowed => ("request.method_not_allowed", 405),
    /// The request took too long to arrive.
    TimedOut => ("request.timed_out", 408),
    /// The request is bigger than allowed.
    TooLarge => ("request.too_large", 413),
    /// Someone else changed the thing since the person loaded it (ADR-0019).
    VersionChanged => ("edit.version_changed", 409),
    /// The edit lock ran out or was released, so the text cannot be saved with it. The text is
    /// kept as the person's draft (ADR-0019).
    EditLockLost => ("edit.lock_lost", 409),
    /// Somebody took the editing over (*Edit anyway*), so the text cannot be saved with the
    /// lock. The text is kept as the person's draft (ADR-0019).
    EditTakenOver => ("edit.taken_over", 409),
    /// Somebody else is editing the thing, so it cannot be taken away now. Nothing is lost: the
    /// page names who is editing (ADR-0019).
    EditInProgress => ("edit.in_progress", 409),
    /// Too many requests in a short time (SEC-14).
    TooManyRequests => ("rate_limit.exceeded", 429),
    /// The server cannot answer right now.
    Unavailable => ("server.unavailable", 503),
    /// The server failed; the person did nothing wrong.
    Internal => ("server.internal", 500),
}

impl ErrorCode {
    /// Whether the code says a value breaks the rule of its field (`field.*`, except
    /// [`Self::FieldNotHere`], which says the field is not used here at all).
    #[must_use]
    pub fn is_field_rule(self) -> bool {
        self.code().starts_with("field.") && self != Self::FieldNotHere
    }
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
    /// The field a `field.*` refusal is about: the word of a built-in field or the ID of an own
    /// field, as the client sent it. Only a field the person may fill in is named, so it reveals
    /// nothing.
    #[serde(skip_serializing_if = "Option::is_none")]
    field: Option<String>,
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
            field: None,
        }
    }

    /// Names the field a `field.*` refusal is about, so the client shows it at that field.
    #[must_use]
    pub fn with_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
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

    /// The field the refusal is about, if it names one.
    #[must_use]
    pub fn field(&self) -> Option<&str> {
        self.field.as_deref()
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
    fn every_field_code_but_not_here_is_a_field_rule() {
        let field_codes: Vec<ErrorCode> = ErrorCode::ALL
            .iter()
            .copied()
            .filter(|code| code.code().starts_with("field."))
            .collect();
        assert!(field_codes.len() > 1);
        for code in field_codes {
            assert_eq!(
                code.is_field_rule(),
                code != ErrorCode::FieldNotHere,
                "{code:?}"
            );
        }
        assert!(!ErrorCode::Forbidden.is_field_rule());
    }

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
