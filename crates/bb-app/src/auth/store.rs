//! The port of the sign-in use cases: what they need from storage.
//!
//! Reads take `&self`. **Writes** take the open transaction and are called from inside
//! `record_change` of [`ChangeStore`], so a change and its domain event are committed together
//! (ARCH-EVENTS). A few technical writes are not domain changes and have their own methods: the
//! time a session was last used, the encrypted secret of the code app, and the security log.

use std::fmt;
use std::future::Future;

use bb_core::access_key::AccessKey;
use bb_core::event::EntityRef;
use bb_core::id::{AccessKeyId, InvitationId, PasskeyId, ProjectId, RoleId, SessionId, UserId};
use bb_core::network::NetworkPrefix;
use bb_core::person::Person;
use bb_core::security_log::Digest;
use bb_core::settings::SettingValue;
use bb_core::sign_in::{
    Invitation, SecurityAction, SetupCode, SignInSession, StoredPasskey, TwoStep,
};
use bb_policy::Permission;
use jiff::Timestamp;
use zeroize::Zeroizing;

use super::types::ProjectChoice;
use crate::access::PolicyStore;
use crate::store_macros::{reads, writes};

/// A person with the password hash they sign in with.
#[derive(Clone)]
pub struct Account {
    /// The person.
    pub person: Person,
    /// The stored password hash.
    pub password_hash: Option<String>,
}

/// A session with the person it belongs to.
#[derive(Clone)]
pub struct SessionRecord {
    /// The session.
    pub session: SignInSession,
    /// The person.
    pub person: Person,
}

// A password hash, a session hash and an e-mail address never show in a log or a failed test:
// `Debug` writes only who it is.
impl fmt::Debug for Account {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Account")
            .field("person", &self.person.id)
            .field("has_password", &self.password_hash.is_some())
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for SessionRecord {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SessionRecord")
            .field("session", &self.session.id)
            .field("person", &self.person.id)
            .finish_non_exhaustive()
    }
}

/// An access key with what it may do and where, as stored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessKeyRecord {
    /// The key (never its secret: only the hash is stored).
    pub key: AccessKey,
    /// The single permissions, for a key whose choice is *More options*; empty otherwise.
    pub permissions: Vec<Permission>,
    /// The projects the key is limited to; empty for every project of its owner.
    pub projects: Vec<ProjectId>,
}

/// The settings that sign-in depends on, as the registry resolves them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthSettings {
    /// Minutes without activity before a session ends.
    pub session_idle_minutes: i64,
    /// Hours a session lasts at most.
    pub session_absolute_hours: i64,
    /// Whether people may sign up on their own.
    pub sign_up_enabled: bool,
    /// The approved e-mail domains, separated by commas.
    pub sign_up_domains: String,
    /// Days a new access key lives when the person picks no other date.
    pub access_key_default_days: i64,
    /// The most days an access key can live.
    pub access_key_longest_days: i64,
}

/// Something to write into the security log.
#[derive(Clone, Debug)]
pub struct SecurityEvent {
    /// When it happened.
    pub time: Timestamp,
    /// Who did it, if known.
    pub person: Option<UserId>,
    /// What happened.
    pub action: SecurityAction,
    /// The record it was about, if one.
    pub subject: Option<EntityRef>,
    /// Where it came from, shortened.
    pub network: Option<NetworkPrefix>,
}

/// What the sign-in use cases need from storage.
pub trait AuthStore: PolicyStore + 'static {
    /// Whether the error is a unique key that was used twice (for example the e-mail address of
    /// two people at the same moment).
    fn is_duplicate(error: &Self::Error) -> bool;

    /// The error a change returns to stop and undo itself: the check it made inside the
    /// transaction failed (a code was used twice, an invitation was taken first). Nothing it
    /// wrote stays, and no event is appended.
    fn aborted() -> Self::Error;

    /// Whether the error is the one from [`Self::aborted`].
    fn is_aborted(error: &Self::Error) -> bool;

    /// The error a change returns when a rule of the policy engine stops it (the last
    /// administrator, no escalation): nothing it wrote stays, and no event is appended.
    fn refused(refusal: bb_policy::guards::Refusal) -> Self::Error;

    /// The rule behind an error made by [`Self::refused`].
    fn as_refusal(error: &Self::Error) -> Option<bb_policy::guards::Refusal>;

    /// The text of an error, for the log.
    fn describe(error: &Self::Error) -> String;

    reads! {
        /// The person with this (lower-case) address and their password hash.
        fn account_by_email(email: &str) -> Option<Account>;
        /// The person with this ID.
        fn person(id: UserId) -> Option<Person>;
        /// The session with this secret hash and its person.
        fn session_by_hash(hash: &Digest) -> Option<SessionRecord>;
        /// The sessions of a person, newest first.
        fn sessions_of(person: UserId) -> Vec<SignInSession>;
        /// The setup code of the first start, if one is waiting.
        fn setup_code() -> Option<SetupCode>;
        /// How the code app of a person is set up.
        fn two_step(person: UserId) -> Option<TwoStep>;
        /// The shared secret of the code app of a person.
        fn totp_secret(person: UserId) -> Option<Zeroizing<Vec<u8>>>;
        /// How many backup codes of a person are unused.
        fn backup_codes_left(person: UserId) -> i64;
        /// The passkeys of a person, oldest first.
        fn passkeys_of(person: UserId) -> Vec<StoredPasskey>;
        /// The invitation with this secret hash.
        fn invitation_by_hash(hash: &Digest) -> Option<Invitation>;
        /// The invitations that have not been used and have not run out, newest first (then by
        /// ID), after the position `after` (the last one of the page before), at most `limit`.
        fn open_invitations(
            now: Timestamp,
            after: Option<(Timestamp, InvitationId)>,
            limit: u64
        ) -> Vec<Invitation>;
        /// The sign-in settings.
        fn auth_settings() -> AuthSettings;
        /// The access key with this secret hash, with what it may do and where.
        fn access_key_by_hash(hash: &Digest) -> Option<AccessKeyRecord>;
        /// The access keys of a person, newest first (then by ID), after the position `after`
        /// (the last one of the page before), at most `limit`.
        fn access_keys_of(
            owner: UserId,
            after: Option<(Timestamp, AccessKeyId)>,
            limit: u64
        ) -> Vec<AccessKeyRecord>;
        /// How many access keys a person has.
        fn access_key_count(owner: UserId) -> i64;
        /// The projects by short name after `after` (a short name, or the start), leaving out the
        /// first `skip` of them, at most `limit`: what a person chooses from for an access key.
        fn projects_after(after: Option<&str>, skip: u64, limit: u64) -> Vec<ProjectChoice>;
        /// Whether the person needs a second step: the organization asks it of everybody, or any
        /// role they hold anywhere (their own rows and their groups', at any place) has the
        /// switch for two-step sign-in (ADR-0025, decision 7).
        fn second_step_required(person: UserId) -> bool;
        /// Whether the person has a second step they can sign in with: a confirmed code app or
        /// a passkey.
        fn has_second_step(person: UserId) -> bool;
        /// The people with the role, their own rows and their groups' at any place, who have no
        /// second step yet.
        fn people_without_second_step(role: RoleId) -> Vec<UserId>;
        /// Records when an access key was used, to the minute. Bookkeeping, not a domain change.
        fn touch_access_key(id: AccessKeyId, at: Timestamp) -> ();
        /// Records when a session was last used. Bookkeeping, not a domain change.
        fn touch_session(id: SessionId, at: Timestamp) -> ();
        /// Stores the shared secret of the code app of a person, encrypted. Replaces an older one.
        fn put_totp_secret(person: UserId, secret: &[u8], at: Timestamp) -> ();
        /// Deletes the shared secret of the code app of a person.
        fn delete_totp_secret(person: UserId) -> ();
        /// Adds an entry to the security log.
        fn append_security_log(event: &SecurityEvent) -> ();
    }

    writes! {
        /// Adds a person with their password hash.
        fn insert_person(person: &Person, password_hash: &str, at: Timestamp) -> ();
        /// Replaces the password hash of a person.
        fn set_password(person: UserId, password_hash: &str, at: Timestamp) -> ();
        /// Adds a session.
        fn insert_session(session: &SignInSession) -> ();
        /// Removes a session; `false` if there was none.
        fn delete_session(id: SessionId) -> bool;
        /// Removes all sessions of a person, except one if given. Returns how many went.
        fn delete_sessions_of(person: UserId, except: Option<SessionId>) -> u64;
        /// Removes sessions nobody can use any more: active ones idle since before `idle_before`
        /// or started before `absolute_before`, and pending ones started before `pending_before`.
        fn purge_sessions(
            idle_before: Timestamp,
            absolute_before: Timestamp,
            pending_before: Timestamp
        ) -> u64;
        /// Adds or replaces the code app record of a person.
        fn upsert_two_step(step: &TwoStep) -> ();
        /// Marks the code app of a person as confirmed.
        fn confirm_two_step(person: UserId) -> ();
        /// Removes the code app record and the backup codes of a person.
        fn delete_two_step(person: UserId) -> ();
        /// Remembers that a code for `step` was used, if that is newer than the newest step used
        /// before. `false` means the code was used already (or the code app is not set up).
        fn accept_totp_step(person: UserId, step: i64) -> bool;
        /// Replaces the backup codes of a person with new ones.
        fn replace_backup_codes(person: UserId, hashes: &[Digest]) -> ();
        /// Uses up a backup code; `false` if there is no such unused code.
        fn use_backup_code(person: UserId, hash: &Digest, at: Timestamp) -> bool;
        /// Adds a passkey.
        fn insert_passkey(passkey: &StoredPasskey) -> ();
        /// Records a sign-in with a passkey and, if given, its new counter.
        fn use_passkey(id: PasskeyId, data: Option<&str>, at: Timestamp) -> ();
        /// Removes a passkey of a person; `false` if there was none.
        fn delete_passkey(person: UserId, id: PasskeyId) -> bool;
        /// Adds an access key with its permissions and projects.
        fn insert_access_key(record: &AccessKeyRecord) -> ();
        /// Removes an access key of its owner; `false` if the owner has no such key.
        fn delete_access_key(owner: UserId, id: AccessKeyId) -> bool;
        /// Removes every access key of a person and returns their IDs.
        fn delete_access_keys_of(owner: UserId) -> Vec<AccessKeyId>;
        /// Adds an invitation.
        fn insert_invitation(invitation: &Invitation) -> ();
        /// Removes an invitation; `false` if there was none.
        fn delete_invitation(id: InvitationId) -> bool;
        /// Uses up an invitation if it is unused and has not run out; `false` otherwise.
        fn use_invitation(id: InvitationId, now: Timestamp) -> bool;
        /// Stores the hash of the setup code, replacing an older one.
        fn put_setup_code(code: &SetupCode) -> ();
        /// Removes the setup code with this hash; `false` if there is none (somebody else used it
        /// first, or the hash is another one).
        fn take_setup_code(hash: &Digest) -> bool;
        /// Removes the setup code, whatever it is (setup is over).
        fn clear_setup_code() -> ();
        /// Stores the value of a setting of the whole organization.
        fn put_setting(key: &str, value: &SettingValue, at: Timestamp) -> ();
    }
}

#[cfg(test)]
mod tests {
    use bb_core::id::{SessionId, UserId};
    use bb_core::network::NetworkPrefix;
    use bb_core::sign_in::SessionStage;

    use super::*;

    #[test]
    fn sec_16_debug_output_of_an_account_and_a_session_hides_hashes_and_addresses() {
        let person = Person {
            id: UserId::from_random([3; 16]),
            display_name: "Ada Lovelace".to_owned(),
            email: "ada@example.test".to_owned(),
            created_at: Timestamp::UNIX_EPOCH,
        };
        let account = Account {
            person: person.clone(),
            password_hash: Some("$argon2id$PASSWORD-HASH".to_owned()),
        };
        let record = SessionRecord {
            session: SignInSession {
                id: SessionId::from_random([4; 16]),
                person: person.id,
                token_hash: Digest::new([0xAB; 32]),
                stage: SessionStage::Active,
                created_at: Timestamp::UNIX_EPOCH,
                last_seen_at: Timestamp::UNIX_EPOCH,
                network: NetworkPrefix::from_stored("203.0.113.0"),
                device: "Firefox|Windows".to_owned(),
            },
            person: person.clone(),
        };
        let shown = format!("{account:?} {record:?}");
        for secret in [
            "PASSWORD-HASH",
            "argon2",
            "ada@example.test",
            "Ada Lovelace",
            "abababab",
            "203.0.113",
        ] {
            assert!(!shown.contains(secret), "{secret} shows in {shown}");
        }
        assert!(shown.contains(&person.id.to_string()));
    }
}
