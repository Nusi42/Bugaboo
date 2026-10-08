//! What the sign-in use cases take when they change something, named after what they do.

use bb_core::access_key::KeyChoice;
use bb_core::id::{AccessKeyId, InvitationId, PasskeyId, ProjectId, SessionId};
use bb_policy::Permission;
use serde_json::Value;

use super::answers::SignUpPolicy;

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
