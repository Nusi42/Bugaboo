//! Error codes: the one list of everything the API can say went wrong (architecture §7, UX §9).
//!
//! Each code has a stable text (`auth.wrong-credentials`) that clients can rely on, the HTTP
//! status it maps to, and the key of the message a person reads. The API (`bb-api`) answers with
//! [`Problem`]; the TypeScript union of codes is generated from this list.
//!
//! Safety rule: an error never reveals a hidden thing. Whatever the person may not see answers
//! with [`ErrorCode::NotFound`], exactly as if it did not exist, and an error message has a
//! placeholder only for a limit of the product that every code of that kind names ([`ErrorCode::arguments`]),
//! so no name, ID or count of a person's data can leak through it. ARCH-ERRORS checks the table and
//! the catalog.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::message::{Message, MessageKey, Value};
use crate::relation::DATE_EFFECT_KINDS_MOST;
use crate::time::{MINUTES_PER_HOUR, TIME_ENTRY_MINUTES_MAX, TIME_ENTRY_MINUTES_MIN};
use crate::working_time::GERMANY_KEEP_YEARS_LEAST;

/// Declares the codes. One line per code: variant, text, HTTP status. The word, its lists, its
/// serde form and its message (`error.<area>.<name>`, with `-` for `_`) come from [`words!`]; the
/// status is added here.
macro_rules! error_codes {
    ($($(#[$doc:meta])* $variant:ident => ($code:literal, $status:literal)),+ $(,)?) => {
        words! {
            /// Something that can go wrong, as the API reports it. The word is the stable text of
            /// the code, like `auth.wrong-credentials`.
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
    WrongCredentials => ("auth.wrong-credentials", 401),
    /// The code of the second step (code app or backup code) is wrong or was used already.
    WrongSecondStepCode => ("auth.wrong-second-step-code", 401),
    /// The setup code of the first start is wrong.
    WrongSetupCode => ("auth.wrong-setup-code", 401),
    /// The new password is shorter than 12 characters.
    PasswordTooShort => ("auth.password-too-short", 400),
    /// The new password is in the list of passwords that are tried first.
    PasswordBreached => ("auth.password-breached", 400),
    /// The invitation link does not work: unknown, used, taken back or expired. Never says which.
    InvitationInvalid => ("auth.invitation-invalid", 400),
    /// Signing up on one's own is not possible here. Never says why.
    SignUpRefused => ("auth.sign-up-refused", 403),
    /// A passkey or a passkey ceremony did not check out.
    PasskeyRejected => ("auth.passkey-rejected", 400),
    /// The person must set up a second step before anything else (a role of theirs, or the
    /// organization, asks for two-step sign-in and they have none yet).
    SecondStepRequired => ("auth.second-step-required", 403),
    /// An access key was used where no key may go: the owner's own sign-in data.
    AccessKeyNotAllowed => ("auth.access-key-not-allowed", 403),
    /// The agent already has work on this issue that has not ended (one open hand-over for an
    /// issue and an agent).
    AgentWorkOpen => ("agent-work.already-open", 409),
    /// The work is not in the state the request needs (an answer to a question that was answered,
    /// a review of work that is not ready).
    AgentWorkNotReady => ("agent-work.not-ready", 409),
    /// The agent used all the attempts the hand-over gave it.
    AgentWorkBudget => ("agent-work.budget-reached", 409),
    /// The agent reached one of its limits and is paused until its person resumes it (ADR-0029,
    /// decision 9). The change it asked for was not started.
    AgentLimitReached => ("agent.limit-reached", 429),
    /// The request needs a signed-in person.
    NotSignedIn => ("auth.not-signed-in", 401),
    /// The person can see the thing but may not do this to it.
    Forbidden => ("access.forbidden", 403),
    /// The group has a *No access* in a project the person may not decide in (or know about), and
    /// taking people out of it or deleting it would lift that *No access* (ADR-0025, decision 6).
    CannotLift => ("access.cannot-lift", 403),
    /// The change would leave nobody who can manage roles and decide who has which role for the
    /// whole organization (ADR-0025, decision 6).
    WouldLockOut => ("access.would-lock-out", 409),
    /// Somebody tried to add or give more than they hold in that place themselves (ADR-0025,
    /// decision 6).
    TooMuch => ("access.too-much", 403),
    /// A role that somebody has cannot be deleted without choosing the role that takes its place.
    RoleInUse => ("role.in-use", 409),
    /// A person or group would have a role and *No access* in the same project.
    AssignmentConflict => ("assignment.conflict", 409),
    /// An administrator locked the setting, so nobody lower down may change it (CFG-06).
    SettingLocked => ("setting.locked", 409),
    /// A list of hosts has an address or a path in it, not only host names (SEC-13).
    SettingHostsNotBare => ("setting.hosts-not-bare", 400),
    /// A time zone that is not on the list of zones the server carries (M2-F72).
    SettingUnknownTimeZone => ("setting.unknown-time-zone", 400),
    /// A move would let somebody take away a *No access* that applies to them (WP-08, M2-F23).
    MoveLiftsNoAccess => ("project.move-lifts-no-access", 409),
    /// A project cannot go inside itself or one of its own sub-projects.
    MoveIntoItself => ("project.move-into-itself", 409),
    /// A project would be nested deeper than a tree of projects may go.
    ProjectTooDeep => ("project.too-deep", 409),
    /// The projects, or who has which role, changed since the move was previewed.
    MovePreviewChanged => ("project.preview-changed", 409),
    /// An issue type that was retired takes no new issues (ISS-02).
    IssueTypeRetired => ("issue.type-retired", 409),
    /// The project does not offer that issue type (ISS-02).
    IssueTypeNotOffered => ("issue.type-not-offered", 409),
    /// The start date of an issue is after its due date (PLAN-01, ADR-0043, decision 1).
    IssueStartAfterDue => ("issue.start-after-due", 400),
    /// The project does not offer the group that label is in (TAG-02).
    LabelNotOffered => ("label.not-offered", 409),
    /// The workflow of the issue has no status change from the status the issue is in to that one
    /// (WF-01).
    WorkflowNoSuchChange => ("workflow.no-such-change", 409),
    /// The person may not change the status of the issue at all (WF-03).
    WorkflowNotForYou => ("workflow.not-for-you", 403),
    /// The person may change the status, but *Who can do it* of this status change does not name
    /// them (WF-03).
    WorkflowNotNamed => ("workflow.not-named", 403),
    /// *Only if …* of the status change is not true (WF-03).
    WorkflowOnlyIf => ("workflow.only-if", 409),
    /// The status change needs a field that the person cannot fill in. It names no field: the
    /// field may be hidden from the person (WF-04, SEC-06).
    WorkflowCannotFill => ("workflow.cannot-fill", 403),
    /// *Publish* would drop a status that issues are still in, and says nowhere to move them to
    /// (WF-02).
    WorkflowStatusInUse => ("workflow.status-in-use", 409),
    /// A workflow breaks a rule: no status, too many, a name that is empty or too long, a status
    /// change between statuses it does not have (WF-02).
    WorkflowInvalid => ("workflow.invalid", 400),
    /// An issue would sit under itself or under one of its own sub-issues (ISS-03).
    ParentCircle => ("issue.parent-circle", 409),
    /// The chain of sub-issues would get longer than a chain may be (ISS-03).
    ParentTooDeep => ("issue.parent-too-deep", 409),
    /// Those two issues are related in that way already (ISS-04).
    RelationExists => ("relation.exists", 409),
    /// A relation of a kind with a date effect, or giving a kind that effect, would close a loop of
    /// dependencies: the answer names one loop by issue IDs the person may see (PLAN-02, ADR-0043,
    /// decision 4).
    RelationDependencyLoop => ("relation.dependency-loop", 409),
    /// A kind of relation that reads the same from both ends has no direction, so it cannot have a
    /// date effect (PLAN-02, ADR-0043, decision 4).
    RelationKindNoDirection => ("relation-kind.no-direction", 409),
    /// Two kinds of relation carry a date effect already: the answer names them (PLAN-02,
    /// ADR-0043, decision 4).
    RelationKindDateEffectMost => ("relation-kind.date-effect-most", 409),
    /// Something holds as many as it may: an issue its relations or sub-issues, the organization
    /// its tags, labels or own fields, a field its choices. A full store always answers this code,
    /// never `request.invalid`. The count includes what the person may not see, as it is a limit of
    /// storage, so the answer names no issue.
    LimitReached => ("limit.reached", 409),
    /// A timer runs already for the person (maybe started on another device).
    TimerRunning => ("timer.running", 409),
    /// The person's timer does not run (it was stopped meanwhile, maybe on another device).
    TimerNotRunning => ("timer.not-running", 409),
    /// A new start time of a timer is in the future.
    TimerStartInFuture => ("timer.start-in-the-future", 400),
    /// A new start time of a timer is before the end of the person's last entry that has a start.
    TimerStartBeforeLastEntry => ("timer.start-before-last-entry", 400),
    /// A timer ran longer than one time entry can be; its start time has to change first.
    TimerTooLong => ("timer.too-long", 409),
    /// The timer was started or switched less than a second ago.
    TimerTooFast => ("timer.too-fast", 429),
    /// The person booked as many time entries on the day as one day can have.
    TooManyTimeEntries => ("time.too-many-entries", 409),
    /// A time entry of no time.
    TimeEntryTooShort => ("time.entry-too-short", 400),
    /// A time entry longer than a day.
    TimeEntryTooLong => ("time.entry-too-long", 400),
    /// The note of a time entry is too long.
    TimeNoteTooLong => ("time.note-too-long", 400),
    /// A tag is limited to other projects than the one of the issue (TAG-01).
    TagNotHere => ("tag.not-here", 409),
    /// Some issue holds more than one label of a group that is to be a *Pick one* group (TAG-02).
    PickOneConflict => ("label.pick-one-conflict", 409),
    /// A required own field has no value (CF-03).
    FieldRequired => ("field.required", 400),
    /// A text, or a list of values, is shorter than the rules of the own field allow (CF-03).
    FieldTooShort => ("field.too-short", 400),
    /// A text, or a list of values, is longer than the rules of the own field allow (CF-03).
    FieldTooLong => ("field.too-long", 400),
    /// A number, day, moment or length of time is below the lowest the own field allows (CF-03).
    FieldTooSmall => ("field.too-small", 400),
    /// A number, day, moment or length of time is above the highest the own field allows (CF-03).
    FieldTooBig => ("field.too-big", 400),
    /// A text does not match the pattern of the own field (CF-03).
    FieldNoMatch => ("field.no-match", 400),
    /// A value is not one this kind of own field can hold (CF-01).
    FieldWrongValue => ("field.wrong-value", 400),
    /// A value names a choice that the own field does not offer (CF-04).
    FieldUnknownChoice => ("field.unknown-choice", 400),
    /// A value names a person, group or issue that the person who sets it may not name: it is not
    /// there, or they may not know about it (CF-01). Both look the same.
    FieldUnknownReference => ("field.unknown-reference", 400),
    /// An own field is not used for this project or this issue type (CF-03).
    FieldNotHere => ("field.not-here", 400),
    /// A file is bigger than the limit of the project (ATT-03). The page says the limit in words
    /// from the number it read (`getIssueFiles`); the message itself names no number.
    FileTooBig => ("file.too-big", 413),
    /// The organization does not accept this type of file (ATT-03).
    FileTypeBlocked => ("file.type-blocked", 415),
    /// A part of an upload does not start where the file stands now; the upload goes on from there
    /// (ATT-01).
    FileWrongPart => ("file.wrong-part", 409),
    /// An upload was finished before all its bytes were there (ATT-01).
    FileIncomplete => ("file.incomplete", 409),
    /// The disk of the server has no room for a part of an upload: less than
    /// [`DISK_KEPT_FREE_BYTES`](crate::files::DISK_KEPT_FREE_BYTES) would be left. Trying later can
    /// work; the administrator frees room (ATT-01).
    FileNoRoom => ("file.no-room", 507),
    /// A name of a group, role or project is used already.
    NameTaken => ("name.taken", 409),
    /// Another choice of the same field has this number (TIME-13).
    ChoiceNumberTaken => ("choice.number-taken", 409),
    /// There is no such thing, or the person may not know about it.
    NotFound => ("resource.not-found", 404),
    /// The request is not valid.
    InvalidRequest => ("request.invalid", 400),
    /// A list was asked for a field that does not exist, or that the person may not read. Both
    /// look the same, so the answer never tells which fields are hidden (SEC-06, PERF-02).
    UnknownField => ("request.unknown-field", 400),
    /// The cursor of a list page was changed or is not one of ours (PERF-02).
    BadCursor => ("request.bad-cursor", 400),
    /// The mail server turned an e-mail away for good: the sender or the address is not accepted
    /// (NTF-02).
    MailRejected => ("mail.rejected", 502),
    /// The mail server cannot be reached, or said to try later (NTF-02).
    MailUnavailable => ("mail.unavailable", 503),
    /// E-mail is off, or the mail server is not entered (PRIV-08).
    MailOff => ("mail.off", 409),
    /// The mail server is set to send a user name or a password without encryption to another
    /// machine (NTF-02).
    MailInsecure => ("mail.insecure", 409),
    /// AI is off: the organization has not switched it on, or the place does not have it on
    /// (AI-02).
    AiOff => ("ai.off", 409),
    /// The AI provider did not answer, or its answer could not be used. Nothing of what the
    /// provider sent is passed on (AI-01).
    AiUnavailable => ("ai.unavailable", 502),
    /// The model, or the key that the provider asks for, is not entered (AI-01).
    AiNotSetUp => ("ai.not-set-up", 409),
    /// An outside provider cannot be used before an administrator confirms that a data processing
    /// agreement is in place (PRIV-08).
    AiAgreementNeeded => ("ai.agreement-needed", 409),
    /// The address of the model is no web address (AI-01).
    AiAddressInvalid => ("ai.address-invalid", 400),
    /// The address of the model is on the internet and does not start with https:// (AI-01).
    AiAddressNotEncrypted => ("ai.address-not-encrypted", 400),
    /// A proposed change of the assistant cannot be applied: the reference is not one this server
    /// made for this person, or it ran out, or it does not belong to the change sent with it (AI-05).
    AiProposalInvalid => ("ai.proposal-invalid", 409),
    /// A person changed their handle a short time ago and cannot change it again yet (M3-F70).
    HandleChangedLately => ("handle.changed-lately", 429),
    /// A change request that does not come from one of Bugaboo's own pages.
    WrongOrigin => ("request.wrong-origin", 403),
    /// A change request without the right token of the session (CSRF).
    WrongToken => ("request.wrong-token", 403),
    /// The address exists but does not do that (wrong HTTP method).
    MethodNotAllowed => ("request.method-not-allowed", 405),
    /// The request took too long to arrive.
    TimedOut => ("request.timed-out", 408),
    /// The request is bigger than allowed.
    TooLarge => ("request.too-large", 413),
    /// Someone else changed the thing since the person loaded it (ADR-0019).
    VersionChanged => ("edit.version-changed", 409),
    /// The edit lock ran out or was let go, so the text cannot be saved with it. The text is
    /// kept as the person's draft (ADR-0019).
    EditLockLost => ("edit.lock-lost", 409),
    /// Somebody took the editing over (*Edit anyway*), so the text cannot be saved with the
    /// lock. The text is kept as the person's draft (ADR-0019).
    EditTakenOver => ("edit.taken-over", 409),
    /// Somebody else is editing the thing, so it cannot be taken away now. Nothing is lost: the
    /// page names who is editing (ADR-0019).
    EditInProgress => ("edit.in-progress", 409),
    /// The text of a search cannot be run: the answer lists the mistakes in it, each with the key of
    /// its message and where it is (ADR-0005).
    SearchInvalid => ("search.invalid", 400),
    /// The person's last work day was never ended: the answer names its date (`open_day`), and
    /// the person says when it ended before a new one starts (ADR-0035, decision 5).
    WorkDayLeftOpen => ("working-hours.day-left-open", 409),
    /// The work day runs already.
    WorkDayRunning => ("working-hours.day-running", 409),
    /// No work day runs, so there is nothing to take a break from or to end.
    WorkDayNotRunning => ("working-hours.day-not-running", 409),
    /// A break runs already.
    BreakRunning => ("working-hours.break-running", 409),
    /// No break runs, so there is none to end.
    NoBreakRunning => ("working-hours.no-break-running", 409),
    /// The times do not make one work day: a break outside the work, two that overlap, a day
    /// longer than 24 hours or one that overlaps the day before or after it (ADR-0035, decision 5).
    NotAWorkDay => ("working-hours.not-a-work-day", 400),
    /// A start, an end or a break of a work day lies after now: a day records what happened
    /// (ADR-0035, decision 7).
    WorkDayInTheFuture => ("working-hours.in-the-future", 400),
    /// A change of another person's work day, or of one's own past day, needs a reason in the
    /// person's own words (ADR-0035, decision 7).
    ReasonNeeded => ("working-hours.reason-needed", 400),
    /// *Totals only* is on: other people's days are shown as week totals only and are not changed
    /// (ADR-0035, decision 8).
    TotalsOnly => ("working-hours.week-totals-only", 403),
    /// The keep period of work days is set below the two years the preset *Germany* asks for
    /// (ADR-0035, decision 7).
    KeepYearsTooShort => ("working-hours.keep-years-too-short", 400),
    /// A search took longer than the time a search may take (SEC-17, architecture §7 *Search
    /// limits*); the person narrows it down.
    SearchTooSlow => ("search.too-slow", 503),
    /// Too many requests in a short time (SEC-14).
    TooManyRequests => ("rate-limit.exceeded", 429),
    /// The server cannot answer right now.
    Unavailable => ("server.unavailable", 503),
    /// The server failed; the person did nothing wrong.
    Internal => ("server.internal", 500),
}

impl ErrorCode {
    /// The values for the placeholders of the message of this code: the limits of the product the
    /// message names, from the constants that hold them (never a value of a person's data).
    #[must_use]
    pub fn arguments(self) -> Vec<(&'static str, Value)> {
        match self {
            Self::TimeEntryTooShort => vec![(
                "minutes",
                Value::Number(i64::from(TIME_ENTRY_MINUTES_MIN.get())),
            )],
            Self::TimeEntryTooLong => vec![(
                "hours",
                Value::Number(i64::from(TIME_ENTRY_MINUTES_MAX.get() / MINUTES_PER_HOUR)),
            )],
            Self::KeepYearsTooShort => {
                vec![("years", Value::Number(GERMANY_KEEP_YEARS_LEAST))]
            }
            Self::RelationKindDateEffectMost => vec![(
                "most",
                Value::Number(i64::try_from(DATE_EFFECT_KINDS_MOST).unwrap_or(i64::MAX)),
            )],
            _ => Vec::new(),
        }
    }

    /// The English words of the message of this code, with the values of its placeholders.
    #[must_use]
    pub fn text(self) -> Message {
        Message::with_arguments(self.message(), self.arguments())
    }

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
    /// Values for the placeholders of the message: the limits it names ([`ErrorCode::arguments`]);
    /// most messages have none.
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
    /// The error answer for `code`; `params` holds the limits its message names, if it names any.
    #[must_use]
    pub fn new(code: ErrorCode) -> Self {
        let arguments = code.arguments();
        Self {
            kind: format!("urn:bugaboo:error:{}", code.code()),
            title: code.text().render(),
            status: code.status(),
            code,
            message_key: code.message(),
            params: arguments
                .iter()
                .filter_map(|(name, value)| match value {
                    Value::Number(number) => Some(((*name).to_owned(), serde_json::json!(number))),
                    Value::Text(_) | Value::Date(_) => None,
                })
                .collect(),
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
                    !part.is_empty() && part.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
                    "{text}"
                );
            }
            assert!((400..600).contains(&code.status()), "{text}");
        }
    }

    #[test]
    fn every_code_has_its_own_message_and_a_placeholder_only_for_a_limit_of_the_product() {
        let mut keys = HashSet::new();
        for code in ErrorCode::ALL {
            let key = code.message();
            assert!(keys.insert(key), "{key} is shared");
            // A placeholder is filled from a constant of the product, never from data.
            assert_eq!(
                key.template().contains('{'),
                !code.arguments().is_empty(),
                "{key} would leak a value or show a placeholder"
            );
            let text = code.text().render();
            assert!(!text.is_empty() && !text.contains('{'), "{text}");
        }
    }

    #[test]
    fn the_limits_a_message_names_come_from_the_constants() {
        let short = Problem::new(ErrorCode::TimeEntryTooShort);
        let json = serde_json::to_value(&short).unwrap();
        assert_eq!(json["params"], serde_json::json!({"minutes": 1}));
        assert_eq!(json["title"], "A time entry needs at least 1 minute.");
        let long = serde_json::to_value(Problem::new(ErrorCode::TimeEntryTooLong)).unwrap();
        assert_eq!(long["params"], serde_json::json!({"hours": 24}));
        assert_eq!(long["title"], "A time entry can be at most 24 hours long.");
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
        assert_eq!(json["type"], "urn:bugaboo:error:auth.wrong-credentials");
        assert_eq!(json["status"], 401);
        assert_eq!(json["code"], "auth.wrong-credentials");
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
