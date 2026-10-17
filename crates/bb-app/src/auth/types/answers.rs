//! What the sign-in use cases give back: the views the browser app shows.

use std::fmt;
use std::time::Duration;

use bb_core::access_key::KeyChoice;
use bb_core::id::{AccessKeyId, InvitationId, PasskeyId, ProjectId, SessionId, UserId};
use bb_core::person::Person;
use bb_policy::{Evaluation, Permission, Topic};
use jiff::Timestamp;
use serde_json::Value;

use crate::access::ProjectLabel;

/// The person as the browser app shows them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersonView {
    /// The ID of the person.
    pub id: UserId,
    /// The name others see.
    pub display_name: String,
    /// The e-mail address used to sign in.
    pub email: String,
}

impl From<&Person> for PersonView {
    fn from(person: &Person) -> Self {
        Self {
            id: person.id,
            display_name: person.display_name.clone(),
            email: person.email.clone(),
        }
    }
}

/// A session that was just made: the two values the browser gets in cookies.
#[derive(Clone)]
pub struct Started {
    /// The secret for the `__Host-bb_session` cookie.
    pub session_token: String,
    /// The token for the `__Host-bb_csrf` cookie (derived from the session secret).
    pub csrf_token: String,
    /// How long the cookies are kept.
    pub lifetime: Duration,
    /// Who it is for.
    pub person: PersonView,
    /// Whether this session still needs the second step before it opens anything.
    pub second_step_needed: Option<SecondStepMethods>,
}

/// What signing up on one's own comes to.
#[derive(Clone, Debug)]
pub enum SignedUp {
    /// The account works and the person is signed in.
    SignedIn(Started),
    /// E-mail is on, so the account works only after the person opened the link in the e-mail that
    /// was sent to the address; no session was made (M1-F38).
    ConfirmationNeeded,
}

/// Which ways there are to finish signing in when the password was right.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SecondStepMethods {
    /// A code from the code app.
    pub code_app: bool,
    /// A passkey.
    pub passkey: bool,
    /// A backup code (only when the code app is set up).
    pub backup_code: bool,
}

/// A challenge for a passkey: the browser passes `options` to the device and sends the answer
/// back with `challenge_id`.
#[derive(Clone, Debug)]
pub struct PasskeyChallenge {
    /// Names the waiting challenge.
    pub challenge_id: String,
    /// What the device is asked, as the passkey library writes it.
    pub options: Value,
}

/// A person is signed in: what the API needs to know about a request.
#[derive(Clone, Debug)]
pub struct Authenticated {
    /// Who it is.
    pub person: PersonView,
    /// The session the request came with.
    pub session_id: SessionId,
    /// When the session started.
    pub started_at: Timestamp,
    /// What the person may do for the whole organization (the engine's answer for this request);
    /// the browser app shows its menus from it, and handlers wrap their answers with it. Empty
    /// while the person still has to set up a second step.
    pub access: Evaluation,
    /// The person must set up a second step before they can go on (a role of theirs, or the
    /// organization, asks for two-step sign-in and they have none): the session can do nothing
    /// else, from this request on (ADR-0025, decision 7).
    pub second_step_setup_required: bool,
}

/// One session in *Where you're signed in*.
#[derive(Clone, Debug)]
pub struct SessionView {
    /// The ID to end it with.
    pub id: SessionId,
    /// When the person signed in.
    pub started_at: Timestamp,
    /// The last time it was used.
    pub last_seen_at: Timestamp,
    /// The network it came from, shortened.
    pub network: Option<String>,
    /// The name of the browser, if known. The app writes "Firefox on Windows" from the names.
    pub browser: Option<String>,
    /// The name of the system, if known.
    pub system: Option<String>,
    /// Whether this is the session of the request.
    pub current: bool,
}

/// What the app needs to know before anybody signs in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuthStatus {
    /// No administrator exists yet: the first-start setup with the setup code is open.
    pub setup_needed: bool,
    /// People may sign up on their own.
    pub sign_up_open: bool,
    /// Passkeys can be used: the public address of the server has a host name.
    pub passkeys_available: bool,
}

/// One passkey in the list.
#[derive(Clone, Debug)]
pub struct PasskeyView {
    /// The ID to remove it with.
    pub id: PasskeyId,
    /// The name the person gave it.
    pub label: String,
    /// When it was added.
    pub created_at: Timestamp,
    /// When it was last used to sign in.
    pub last_used_at: Option<Timestamp>,
}

/// How a person's second step is set up.
#[derive(Clone, Debug)]
pub struct TwoStepStatus {
    /// Whether a code app is set up.
    pub code_app: bool,
    /// How many backup codes are left.
    pub backup_codes_left: i64,
    /// The passkeys, at most [`MAX_PASSKEYS_LISTED`].
    pub passkeys: Vec<PasskeyView>,
    /// There are more passkeys than the list holds.
    pub truncated: bool,
}

/// What the setup screen of the code app shows.
#[derive(Clone)]
pub struct CodeAppSetup {
    /// The address the code app reads.
    pub otpauth_url: String,
    /// The QR picture of the address (PNG, base64).
    pub qr_png_base64: String,
    /// The secret as text, for typing.
    pub secret: String,
}

/// An invitation as the administrator sees it.
#[derive(Clone, Debug)]
pub struct InvitationView {
    /// The ID of the invitation.
    pub id: InvitationId,
    /// The address it is for.
    pub email: String,
    /// When it was made.
    pub created_at: Timestamp,
    /// When it stops working.
    pub expires_at: Timestamp,
}

/// A new invitation: the secret is shown only now.
#[derive(Clone)]
pub struct CreatedInvitation {
    /// The invitation.
    pub invitation: InvitationView,
    /// The address of the page the link opens, with the secret after the `#`. The page is on this
    /// server; the browser app puts the server's address in front.
    pub link_path: String,
}

/// An access key in the list of its owner. The secret is never part of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessKeyView {
    /// The ID to delete it with.
    pub id: AccessKeyId,
    /// What the person called it.
    pub name: String,
    /// The public start of the key, after `bb_`.
    pub prefix: String,
    /// *What it may do*.
    pub choice: KeyChoice,
    /// The single permissions, when the choice is *More options*.
    pub permissions: Vec<Permission>,
    /// The projects the key is limited to (the names of those the person may not see are left
    /// out); empty for every project.
    pub projects: Vec<ProjectLabel>,
    /// When it was made.
    pub created_at: Timestamp,
    /// When it stops working.
    pub expires_at: Timestamp,
    /// The last time it was used, to the minute.
    pub last_used_at: Option<Timestamp>,
    /// Whether it has stopped working already.
    pub expired: bool,
}

/// A new access key: the secret is shown only now.
#[derive(Clone)]
pub struct CreatedAccessKey {
    /// The key as it will be listed.
    pub key: AccessKeyView,
    /// The whole key, `bb_...`. It is not stored and cannot be shown again.
    pub secret: String,
}

/// A page of the access keys of a person.
#[derive(Clone, Debug)]
pub struct AccessKeyPage {
    /// The keys, newest first.
    pub items: Vec<AccessKeyView>,
    /// The cursor of the next page, if there is one.
    pub next_cursor: Option<String>,
}

/// How long a new access key lives in this organization: what the form of *Create an access key*
/// offers (the two organization settings for the default and the longest time of a key).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessKeyRules {
    /// How many days a new key lives when the person picks no other date (never more than the
    /// longest time).
    pub default_days: i64,
    /// The most days a key can live.
    pub longest_days: i64,
    /// The single permissions a key can be limited to, under *More options*, in the order of the
    /// catalogue, each with its topic (the sentences are the messages of the catalogue).
    pub permissions: Vec<(Permission, Topic)>,
}

/// A project a person may choose for an access key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectChoice {
    /// The ID of the project.
    pub id: ProjectId,
    /// Its short name.
    pub short_name: String,
    /// Its name.
    pub name: String,
}

/// A page of the projects a person may choose for an access key.
#[derive(Clone, Debug)]
pub struct ProjectChoicePage {
    /// The projects, by short name.
    pub items: Vec<ProjectChoice>,
    /// The cursor of the next page, if there is one.
    pub next_cursor: Option<String>,
}

/// How many people with a role still have to set up a second step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SecondStepGap {
    /// The people with the role who have no second step yet.
    pub still_to_set_up: u64,
}

/// Who may sign up on their own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignUpPolicy {
    /// Whether signing up without an invitation is on.
    pub enabled: bool,
    /// The e-mail domains that may sign up.
    pub domains: Vec<String>,
    /// Counts up with every change; a change is made on the version that was looked at. `0` until
    /// the policy is changed the first time.
    pub version: i64,
}

/// The most sessions *Where you're signed in* lists.
pub const MAX_SESSIONS_LISTED: usize = 50;

/// The most passkeys the second-step answer lists (the most a person can have).
pub const MAX_PASSKEYS_LISTED: usize = 20;

/// A page of invitations.
#[derive(Clone, Debug)]
pub struct InvitationPage {
    /// The invitations, newest first.
    pub items: Vec<InvitationView>,
    /// The cursor of the next page, if there is one.
    pub next_cursor: Option<String>,
}

/// The sessions of a person.
#[derive(Clone, Debug)]
pub struct SessionList {
    /// Newest first, at most [`MAX_SESSIONS_LISTED`].
    pub sessions: Vec<SessionView>,
    /// There are more sessions than the list holds.
    pub truncated: bool,
}

/// What an invitation link is for, shown on the page the link opens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvitationLookup {
    /// The address the account will get.
    pub email: String,
}

/// The backup codes made by [`RenewBackupCodes`] and [`ConfirmCodeApp`], shown once.
#[derive(Clone)]
pub struct BackupCodes(pub Vec<String>);

/// One session of the person was ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionEnded {
    /// Whether it was the session of the request itself, so the person is signed out here.
    pub own: bool,
}

/// Nothing to report; the request worked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Done;

// The secrets of a new session, a code app and a new invitation never show in a log or a failed
// test: `Debug` writes who it is for and hides the rest.
impl fmt::Debug for Started {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Started")
            .field("person", &self.person.id)
            .field("second_step_needed", &self.second_step_needed)
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for CodeAppSetup {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CodeAppSetup(<hidden>)")
    }
}

impl fmt::Debug for CreatedInvitation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CreatedInvitation")
            .field("invitation", &self.invitation.id)
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for CreatedAccessKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CreatedAccessKey")
            .field("key", &self.key.id)
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for BackupCodes {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "BackupCodes({} hidden)", self.0.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sec_16_debug_output_of_secrets_shows_none_of_them() {
        let person = PersonView {
            id: UserId::from_random([7; 16]),
            display_name: "Ada".to_owned(),
            email: "ada@example.test".to_owned(),
        };
        let started = Started {
            session_token: "SESSION-SECRET".to_owned(),
            csrf_token: "CSRF-SECRET".to_owned(),
            lifetime: Duration::from_mins(1),
            person: person.clone(),
            second_step_needed: None,
        };
        let setup = CodeAppSetup {
            otpauth_url: "otpauth://totp/OTPAUTH-SECRET".to_owned(),
            qr_png_base64: "QR-SECRET".to_owned(),
            secret: "TOTP-SECRET".to_owned(),
        };
        let created = CreatedInvitation {
            invitation: InvitationView {
                id: InvitationId::from_random([9; 16]),
                email: "bob@example.test".to_owned(),
                created_at: Timestamp::UNIX_EPOCH,
                expires_at: Timestamp::UNIX_EPOCH,
            },
            link_path: "/invitation#LINK-SECRET".to_owned(),
        };
        let codes = BackupCodes(vec!["BACKUP-SECRET".to_owned(), "BACKUP-TWO".to_owned()]);
        let key = CreatedAccessKey {
            key: AccessKeyView {
                id: AccessKeyId::from_random([5; 16]),
                name: "Nightly report".to_owned(),
                prefix: "PREFIX00".to_owned(),
                choice: KeyChoice::SeeOnly,
                permissions: Vec::new(),
                projects: Vec::new(),
                created_at: Timestamp::UNIX_EPOCH,
                expires_at: Timestamp::UNIX_EPOCH,
                last_used_at: None,
                expired: false,
            },
            secret: "bb_KEY-SECRET".to_owned(),
        };
        let shown = format!("{started:?} {setup:?} {created:?} {codes:?} {key:?}");
        for secret in [
            "SESSION-SECRET",
            "CSRF-SECRET",
            "OTPAUTH-SECRET",
            "QR-SECRET",
            "TOTP-SECRET",
            "LINK-SECRET",
            "BACKUP-SECRET",
            "BACKUP-TWO",
            "KEY-SECRET",
            "PREFIX00",
            "Nightly report",
            "ada@example.test",
            "bob@example.test",
        ] {
            assert!(!shown.contains(secret), "{secret} shows in {shown}");
        }
        assert!(shown.contains(&person.id.to_string()));
        assert!(shown.contains("BackupCodes(2 hidden)"));
    }
}
