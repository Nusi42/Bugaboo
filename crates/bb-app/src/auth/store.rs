//! The port of the sign-in use cases: what they need from storage.
//!
//! Reads take `&self`. **Writes** take the open transaction and are called from inside
//! `record_change` of [`ChangeStore`], so a change and its domain event are committed together
//! (ARCH-EVENTS). A few technical writes are not domain changes and have their own methods: the
//! time a session was last used, the encrypted secret of the code app, and the security log.

use std::fmt;
use std::future::Future;

use bb_core::event::EntityRef;
use bb_core::id::{InvitationId, PasskeyId, SessionId, UserId};
use bb_core::network::NetworkPrefix;
use bb_core::person::Person;
use bb_core::security_log::Digest;
use bb_core::settings::SettingValue;
use bb_core::sign_in::{
    Invitation, SecurityAction, SetupCode, SignInSession, StoredPasskey, TwoStep,
};
use jiff::Timestamp;
use zeroize::Zeroizing;

use crate::ports::ChangeStore;

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

/// The reads of the port: `fn name(args) -> Type` becomes a method returning
/// `Result<Type, Self::Error>`.
macro_rules! reads {
    ($($(#[$meta:meta])* fn $name:ident($($arg:ident: $ty:ty),*) -> $ret:ty;)+) => {
        $($(#[$meta])*
        fn $name(&self $(, $arg: $ty)*)
            -> impl Future<Output = Result<$ret, Self::Error>> + Send;)+
    };
}

/// The writes of the port: the same, inside the transaction of a change. They are associated
/// functions, not methods: the closure of a change is called with the transaction and nothing
/// else, so it cannot hold on to the store.
macro_rules! writes {
    ($($(#[$meta:meta])* fn $name:ident($($arg:ident: $ty:ty),*) -> $ret:ty;)+) => {
        $($(#[$meta])*
        fn $name(transaction: &mut Self::Transaction $(, $arg: $ty)*)
            -> impl Future<Output = Result<$ret, Self::Error>> + Send;)+
    };
}

/// What the sign-in use cases need from storage.
pub trait AuthStore: ChangeStore + 'static {
    /// Whether the error is a unique key that was used twice (for example the e-mail address of
    /// two people at the same moment).
    fn is_duplicate(error: &Self::Error) -> bool;

    /// The error a change returns to stop and undo itself: the check it made inside the
    /// transaction failed (a code was used twice, an invitation was taken first). Nothing it
    /// wrote stays, and no event is appended.
    fn aborted() -> Self::Error;

    /// Whether the error is the one from [`Self::aborted`].
    fn is_aborted(error: &Self::Error) -> bool;

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
        /// How many administrators exist.
        fn administrators() -> i64;
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
            administrator: true,
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
