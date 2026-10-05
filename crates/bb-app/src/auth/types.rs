//! What the sign-in use cases take and give back. Inputs that change something are named after
//! what they do; inputs that only read end in `Query` (ARCH-EVENTS).

use std::fmt;
use std::time::Duration;

use bb_core::access_key::KeyChoice;
use bb_core::id::{AccessKeyId, InvitationId, PasskeyId, ProjectId, RoleId, SessionId, UserId};
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

/// How long a new access key lives in this organization: what the form of *Make an access key*
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
}

// ---- Inputs that change something ----

/// Sign in with e-mail address and password.
pub struct SignIn {
    /// The address as typed.
    pub email: String,
    /// The password as typed.
    pub password: String,
    /// The secret of the session cookie the browser still has, if any: signing in ends that old
    /// session.
    pub previous_session: Option<String>,
}

/// Finish signing in with a code from the code app or a backup code.
pub struct SecondStep {
    /// The secret of the pending session.
    pub session_token: String,
    /// The code as typed.
    pub code: String,
}

/// Finish signing in with a passkey after the password.
pub struct PasskeySecondStep {
    /// The secret of the pending session.
    pub session_token: String,
    /// The challenge the passkey answered.
    pub challenge_id: String,
    /// What the device answered.
    pub credential: Value,
}

/// Sign in with a passkey alone.
pub struct PasskeySignIn {
    /// The challenge the passkey answered.
    pub challenge_id: String,
    /// What the device answered.
    pub credential: Value,
    /// The secret of the session cookie the browser still has, if any: signing in ends that old
    /// session.
    pub previous_session: Option<String>,
}

/// End one session (the one of the request).
pub struct SignOut {
    /// The secret of the session.
    pub session_token: String,
}

/// End every session of the person.
pub struct SignOutEverywhere {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// End one of the person's sessions.
pub struct EndSession {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The session to end.
    pub session_id: SessionId,
}

/// Change the password.
pub struct ChangePassword {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The password now.
    pub current_password: String,
    /// The password from now on.
    pub new_password: String,
}

/// Start setting up the code app. A new way to sign in asks for the password again, so a stolen
/// session cannot add one.
pub struct StartCodeApp {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The password, asked again for this.
    pub password: String,
}

/// Finish setting up the code app with the first code.
pub struct ConfirmCodeApp {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The code as typed.
    pub code: String,
    /// The password, asked again for this.
    pub password: String,
}

/// Switch the code app off.
pub struct RemoveCodeApp {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The password, asked again for this.
    pub password: String,
}

/// Make new backup codes.
pub struct RenewBackupCodes {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The password, asked again for this.
    pub password: String,
}

/// Add a passkey.
pub struct AddPasskey {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The challenge the device answered.
    pub challenge_id: String,
    /// What the device answered.
    pub credential: Value,
    /// The name the person gives it.
    pub label: String,
    /// The password, asked again for this.
    pub password: String,
}

/// Remove a passkey.
pub struct RemovePasskey {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The passkey to remove.
    pub passkey_id: PasskeyId,
    /// The password, asked again for this.
    pub password: String,
}

/// Make an access key. The password is asked again, so a stolen session cannot make a key that
/// outlives it.
pub struct MakeAccessKey {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The password, asked again for this.
    pub password: String,
    /// What the key is called.
    pub name: String,
    /// *What it may do*.
    pub choice: KeyChoice,
    /// The single permissions, for the choice *More options*.
    pub permissions: Vec<Permission>,
    /// The projects the key is limited to; empty for every project.
    pub projects: Vec<ProjectId>,
    /// When it stops working, as a date and time such as `2027-03-01T00:00:00Z`; left out for the
    /// organization's default number of days from now.
    pub expires_at: Option<String>,
}

/// Delete an access key of the person.
pub struct DeleteAccessKey {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The key.
    pub id: AccessKeyId,
}

/// Make an invitation.
pub struct CreateInvitation {
    /// The secret of the session of the request (an administrator).
    pub session_token: String,
    /// The address to invite.
    pub email: String,
}

/// Take an invitation back.
pub struct RevokeInvitation {
    /// The secret of the session of the request (an administrator).
    pub session_token: String,
    /// The invitation.
    pub invitation_id: InvitationId,
}

/// Make an account from an invitation.
pub struct AcceptInvitation {
    /// The secret from the link.
    pub secret: String,
    /// The name the person gives.
    pub display_name: String,
    /// The password the person chooses.
    pub password: String,
    /// The secret of the session cookie the browser still has, if any: signing in ends that old
    /// session.
    pub previous_session: Option<String>,
}

/// Make an account without an invitation.
pub struct SignUp {
    /// The address as typed.
    pub email: String,
    /// The name the person gives.
    pub display_name: String,
    /// The password the person chooses.
    pub password: String,
    /// The secret of the session cookie the browser still has, if any: signing in ends that old
    /// session.
    pub previous_session: Option<String>,
}

/// Change who may sign up on their own.
pub struct ChangeSignUpPolicy {
    /// The secret of the session of the request (an administrator).
    pub session_token: String,
    /// The new rule.
    pub policy: SignUpPolicy,
}

/// Make the first administrator with the setup code.
pub struct CompleteSetup {
    /// The setup code as typed.
    pub code: String,
    /// The name of the administrator.
    pub display_name: String,
    /// The address of the administrator.
    pub email: String,
    /// The password of the administrator.
    pub password: String,
    /// The secret of the session cookie the browser still has, if any: signing in ends that old
    /// session.
    pub previous_session: Option<String>,
}

/// Make a new setup code, if setup is still open (the server does this at every start).
pub struct IssueSetupCode;

// ---- Inputs that only read ----

/// What the app needs to know before anybody signs in.
pub struct StatusQuery;

/// Who is behind this session.
pub struct AuthenticateQuery {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// The sessions of the person.
pub struct SessionsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// How the second step is set up.
pub struct TwoStepQuery {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// A challenge for a passkey as the second step of signing in.
pub struct PasskeySecondStepQuery {
    /// The secret of the pending session.
    pub session_token: String,
}

/// A challenge for signing in with a passkey alone.
pub struct PasskeySignInQuery;

/// A challenge for adding a passkey. The password is asked again, so a stolen session cannot
/// add one.
pub struct PasskeyRegistrationQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The password, asked again for this.
    pub password: String,
}

/// The access keys of the person, one page at a time.
pub struct AccessKeysQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// Where the page after the last one starts (`next_cursor` of the page before), if any.
    pub cursor: Option<String>,
    /// How many keys a page holds at most; more than `bb_core::paging::PAGE_SIZE_MAX` is cut to
    /// it.
    pub limit: u32,
}

/// The projects a person may choose for an access key, one page at a time.
pub struct ProjectChoicesQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// Where the page after the last one starts (`next_cursor` of the page before), if any.
    pub cursor: Option<String>,
    /// How many projects a page holds at most.
    pub limit: u32,
}

/// How long a new access key lives in this organization.
pub struct AccessKeyRulesQuery {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// How many people with a role have no second step yet.
pub struct SecondStepGapQuery {
    /// The secret of the session of the request (somebody who manages roles).
    pub session_token: String,
    /// The role.
    pub role: RoleId,
}

/// The open invitations, one page at a time.
pub struct InvitationsQuery {
    /// The secret of the session of the request (an administrator).
    pub session_token: String,
    /// Where the page after the last one starts (`next_cursor` of the page before), if any.
    pub cursor: Option<String>,
    /// How many invitations a page holds at most; more than `bb_core::paging::PAGE_SIZE_MAX` is
    /// cut to it.
    pub limit: u32,
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

/// Which address an invitation is for.
pub struct InvitationLookupQuery {
    /// The secret from the link.
    pub secret: String,
}

/// Who may sign up on their own.
pub struct SignUpPolicyQuery {
    /// The secret of the session of the request (an administrator).
    pub session_token: String,
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
