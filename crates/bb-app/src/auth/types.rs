//! What the sign-in use cases take and give back. Inputs that change something are named after
//! what they do; inputs that only read end in `Query` (ARCH-EVENTS).

use std::time::Duration;

use bb_core::id::{InvitationId, PasskeyId, SessionId, UserId};
use bb_core::person::Person;
use jiff::Timestamp;
use serde_json::Value;

/// The person as the browser app shows them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersonView {
    /// The ID of the person.
    pub id: UserId,
    /// The name others see.
    pub display_name: String,
    /// The e-mail address used to sign in.
    pub email: String,
    /// Whether the person is an administrator.
    pub administrator: bool,
}

impl From<&Person> for PersonView {
    fn from(person: &Person) -> Self {
        Self {
            id: person.id,
            display_name: person.display_name.clone(),
            email: person.email.clone(),
            administrator: person.administrator,
        }
    }
}

/// A session that was just made: the two values the browser gets in cookies.
#[derive(Clone, Debug)]
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
    /// The passkeys.
    pub passkeys: Vec<PasskeyView>,
}

/// What the setup screen of the code app shows.
#[derive(Clone, Debug)]
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
#[derive(Clone, Debug)]
pub struct CreatedInvitation {
    /// The invitation.
    pub invitation: InvitationView,
    /// The address of the page the link opens, with the secret after the `#`. The page is on this
    /// server; the browser app puts the server's address in front.
    pub link_path: String,
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

/// Start setting up the code app.
pub struct StartCodeApp {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// Finish setting up the code app with the first code.
pub struct ConfirmCodeApp {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The code as typed.
    pub code: String,
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
}

/// Make an account without an invitation.
pub struct SignUp {
    /// The address as typed.
    pub email: String,
    /// The name the person gives.
    pub display_name: String,
    /// The password the person chooses.
    pub password: String,
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

/// A challenge for adding a passkey.
pub struct PasskeyRegistrationQuery {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// The open invitations.
pub struct InvitationsQuery {
    /// The secret of the session of the request (an administrator).
    pub session_token: String,
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
#[derive(Clone, Debug)]
pub struct BackupCodes(pub Vec<String>);

/// Nothing to report; the request worked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Done;
