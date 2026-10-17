//! What signing in stores (AUTH-01…03, AUTH-06, AUTH-09, SEC-12): the sign-in data of a person,
//! the sessions, the invitations and the setup code. Every field carries its personal-data mark
//! (ADR-0015). The secrets themselves (hashes, keys) are credentials: they are stored, never
//! logged, and never put into the changes of an event.

use jiff::Timestamp;

use crate::id::{InvitationId, PasskeyId, SessionId, UserId};
use crate::network::NetworkPrefix;
use crate::personal::Entity;
use crate::security_log::Digest;

words! {
    /// How far a session has come. A session that has only had the password checked cannot do
    /// anything but finish signing in.
    pub enum SessionStage {
        /// The password was right and a second step is still needed.
        SecondStepPending => "second_step_pending",
        /// Signed in.
        Active => "active",
    }
}

words! {
    /// What happened, as written in the security log (SEC-15). One list, so the log, the tests
    /// and the docs use the same words.
    pub enum SecurityAction {
        /// A person signed in.
        SignInSucceeded => "sign_in.succeeded",
        /// The e-mail address or the password was wrong.
        SignInFailed => "sign_in.failed",
        /// A sign-in was turned away because of too many failed tries.
        SignInLimited => "sign_in.limited",
        /// The password was right and the second step is still to do.
        SignInSecondStepNeeded => "sign_in.second_step_needed",
        /// The second step was done.
        SecondStepSucceeded => "second_step.succeeded",
        /// The second step was wrong.
        SecondStepFailed => "second_step.failed",
        /// A half-finished sign-in was ended after too many wrong codes.
        SecondStepGivenUp => "second_step.given_up",
        /// A backup code was used up.
        BackupCodeUsed => "backup_code.used",
        /// A person signed in with a passkey alone.
        PasskeySignInSucceeded => "passkey.sign_in_succeeded",
        /// A passkey did not check out.
        PasskeySignInFailed => "passkey.sign_in_failed",
        /// A passkey was added.
        PasskeyAdded => "passkey.added",
        /// A passkey was removed.
        PasskeyRemoved => "passkey.removed",
        /// Two-step sign-in with a code app was switched on.
        TwoStepEnabled => "two_step.enabled",
        /// Two-step sign-in with a code app was switched off.
        TwoStepDisabled => "two_step.disabled",
        /// New backup codes were made; the old ones stopped working.
        BackupCodesRenewed => "backup_codes.renewed",
        /// A session ended (sign-out, or ended from the list).
        SessionEnded => "session.ended",
        /// All sessions of a person ended.
        SessionsEndedAll => "session.ended_all",
        /// A person changed their password.
        PasswordChanged => "password.changed",
        /// Somebody with the permission made an invitation.
        InvitationCreated => "invitation.created",
        /// Somebody with the permission took an invitation back.
        InvitationRevoked => "invitation.revoked",
        /// Somebody accepted an invitation and made an account.
        InvitationAccepted => "invitation.accepted",
        /// An invitation link did not work.
        InvitationRefused => "invitation.refused",
        /// Somebody signed up on their own.
        SignUpAccepted => "sign_up.accepted",
        /// A sign-up was turned away.
        SignUpRefused => "sign_up.refused",
        /// Somebody with the permission changed who may sign up.
        SignUpPolicyChanged => "sign_up.policy_changed",
        /// The first administrator was made with the setup code.
        SetupCompleted => "setup.completed",
        /// A wrong setup code was tried.
        SetupFailed => "setup.failed",
        /// The starting roles were made (once, at the first start).
        StartingRolesCreated => "starting_roles.created",
        /// A role was made.
        RoleCreated => "role.created",
        /// A role was changed: its name, its permissions or its two-step switch.
        RoleChanged => "role.changed",
        /// A role was deleted.
        RoleDeleted => "role.deleted",
        /// A starting role got its starting permissions back.
        RoleReset => "role.reset",
        /// A group was made.
        GroupCreated => "group.created",
        /// A group was renamed.
        GroupRenamed => "group.renamed",
        /// A group was deleted.
        GroupDeleted => "group.deleted",
        /// A person was put into a group.
        GroupMemberAdded => "group.member_added",
        /// A person was taken out of a group.
        GroupMemberRemoved => "group.member_removed",
        /// A role, or *No access*, was given to a person or a group.
        AssignmentCreated => "assignment.created",
        /// A role, or *No access*, was taken away.
        AssignmentRemoved => "assignment.removed",
        /// The level of a field for a role was changed.
        FieldLevelChanged => "field_level.changed",
        /// How a role sees one kind of people information was changed.
        PeopleLevelChanged => "people_level.changed",
        /// A project was made.
        ProjectCreated => "project.created",
        /// A setting that touches security was changed.
        SettingChanged => "setting.changed",
        /// A setting that touches security went back to its default.
        SettingReset => "setting.reset",
        /// An administrator locked a setting that touches security.
        SettingLocked => "setting.locked",
        /// An administrator unlocked a setting that touches security.
        SettingUnlocked => "setting.unlocked",
        /// A person made an access key.
        AccessKeyCreated => "access_key.created",
        /// A person deleted an access key.
        AccessKeyDeleted => "access_key.deleted",
        /// An access key was turned away: it does not work, or it was used where no key may go.
        AccessKeyRefused => "access_key.refused",
        /// A project got another name.
        ProjectRenamed => "project.renamed",
        /// A project got another short name; the old one stays with it.
        ProjectShortNameChanged => "project.short_name_changed",
        /// A project was moved into another project or to the top level.
        ProjectMoved => "project.moved",
        /// Somebody with *Edit anyway* released an edit lock that another person held.
        EditLockReleased => "edit_lock.released",
        /// An administrator entered or removed the password of the mail server.
        MailPasswordChanged => "mail.password_changed",
    }
}

/// The codes that entries written before M4 hold, and the codes that stand for them now.
///
/// The security log is a hash chain: an entry is never rewritten, because every entry holds the
/// hash of the one before it and a rewritten action would break the chain. So the entries of an
/// older program keep their words in the database, and whoever shows the log (the API, the
/// screen, an export) reads them through [`shown_code`]. This table is the only place the old
/// words stand in the code; new entries are written with the new ones.
pub const FORMER_CODES: [(&str, &str); 6] = [
    ("role.made", "role.created"),
    ("group.made", "group.created"),
    ("assignment.made", "assignment.created"),
    ("project.made", "project.created"),
    ("starting_roles.made", "starting_roles.created"),
    ("access_key.made", "access_key.created"),
];

/// The code to show for a stored one: the new code for an old one, the code itself otherwise.
#[must_use]
pub fn shown_code(stored: &str) -> &str {
    FORMER_CODES
        .iter()
        .find(|(old, _)| *old == stored)
        .map_or(stored, |(_, new)| new)
}

/// The password of a person, as a PHC string (Argon2id with its parameters and salt).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct PasswordLogin {
    /// Whose password it is.
    #[personal(category = "identity", basis = "contract")]
    pub person: UserId,
    /// The password hash. The password itself is never stored.
    #[personal(category = "credentials", basis = "contract")]
    pub hash: String,
    /// When the password was set.
    #[personal(category = "activity", basis = "contract")]
    pub changed_at: Timestamp,
}

/// One place where a person is signed in (or on the way to being signed in).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct SignInSession {
    /// The ID people see in *Where you're signed in*. It opens nothing.
    #[not_personal]
    pub id: SessionId,
    /// Whose session it is.
    #[personal(category = "identity", basis = "contract")]
    pub person: UserId,
    /// The hash of the 256-bit secret in the cookie. The secret itself is never stored.
    #[personal(category = "credentials", basis = "contract")]
    pub token_hash: Digest,
    /// Waiting for the second step, or signed in.
    #[not_personal]
    pub stage: SessionStage,
    /// When the person signed in.
    #[personal(category = "activity", basis = "contract")]
    pub created_at: Timestamp,
    /// The last time the session was used, for the idle timeout.
    #[personal(category = "activity", basis = "contract")]
    pub last_seen_at: Timestamp,
    /// Where the person signed in from, shortened.
    #[personal(category = "technical", basis = "contract")]
    pub network: Option<NetworkPrefix>,
    /// A short description of the browser and the system, like "Firefox on Windows".
    #[personal(category = "technical", basis = "contract")]
    pub device: String,
}

/// A 30-second step of the clock, which a code of the app belongs to (RFC 6238).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TotpStep(i64);

impl TotpStep {
    /// The step before any code was accepted.
    pub const NONE: Self = Self(0);

    /// The step with this number.
    #[must_use]
    pub const fn new(number: i64) -> Self {
        Self(number)
    }

    /// The number of the step, as the database stores it.
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }
}

/// The two-step sign-in with a code app (TOTP) of a person. The shared secret is a stored secret
/// (envelope encrypted), not a column here.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct TwoStep {
    /// Whose it is.
    #[personal(category = "identity", basis = "contract")]
    pub person: UserId,
    /// Whether the person proved they can make codes. Until then it is not asked for.
    #[not_personal]
    pub confirmed: bool,
    /// The newest 30-second step a code was accepted for, so a code cannot be used twice.
    #[personal(category = "credentials", basis = "contract")]
    pub last_step: TotpStep,
    /// When the person started setting it up.
    #[personal(category = "activity", basis = "contract")]
    pub created_at: Timestamp,
}

/// One backup code, kept as a hash.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct BackupCode {
    /// Whose it is.
    #[personal(category = "identity", basis = "contract")]
    pub person: UserId,
    /// The hash of the code.
    #[personal(category = "credentials", basis = "contract")]
    pub code_hash: Digest,
    /// When it was used. A used code never works again.
    #[personal(category = "activity", basis = "contract")]
    pub used_at: Option<Timestamp>,
}

/// A passkey a person registered.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct StoredPasskey {
    /// The ID of the passkey in Bugaboo.
    #[not_personal]
    pub id: PasskeyId,
    /// Whose it is.
    #[personal(category = "identity", basis = "contract")]
    pub person: UserId,
    /// The ID the device gave the passkey.
    #[personal(category = "credentials", basis = "contract")]
    pub credential_id: Vec<u8>,
    /// The public key and the counter, as the passkey library keeps them.
    #[personal(category = "credentials", basis = "contract")]
    pub data: String,
    /// The name the person gave it, like "Work laptop".
    #[personal(category = "content", basis = "contract")]
    pub label: String,
    /// When it was added.
    #[personal(category = "activity", basis = "contract")]
    pub created_at: Timestamp,
    /// When it was last used to sign in.
    #[personal(category = "activity", basis = "contract")]
    pub last_used_at: Option<Timestamp>,
}

/// An invitation to make an account (AUTH-06).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct Invitation {
    /// The ID of the invitation.
    #[not_personal]
    pub id: InvitationId,
    /// The hash of the secret in the link. The secret is shown once, to the administrator.
    #[personal(category = "credentials", basis = "contract")]
    pub token_hash: Digest,
    /// The e-mail address the account will get.
    #[personal(category = "identity", basis = "contract")]
    pub email: String,
    /// The administrator who made it; nobody for the invitation a sign-up makes.
    #[personal(category = "identity", basis = "contract")]
    pub invited_by: Option<UserId>,
    /// When it was made.
    #[personal(category = "activity", basis = "contract")]
    pub created_at: Timestamp,
    /// When it stops working (7 days after it was made).
    #[not_personal]
    pub expires_at: Timestamp,
    /// When it was accepted. An accepted invitation never works again.
    #[personal(category = "activity", basis = "contract")]
    pub used_at: Option<Timestamp>,
}

/// The one-time code for the first start (AUTH-09), kept as a hash.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct SetupCode {
    /// The hash of the code.
    #[not_personal]
    pub code_hash: Digest,
    /// When the server made it.
    #[not_personal]
    pub created_at: Timestamp,
}
