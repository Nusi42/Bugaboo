//! An in-memory store for the tests of the use cases (M1-F18, M1-F52): the same ports as the
//! database adapter, kept in plain lists, with the two failures that tests need to cause — a
//! security log that cannot be written and a store that fails when a change is committed.
//!
//! A change runs on a copy of the state and replaces the state only when it succeeded, together
//! with its domain event, as the database does with a transaction; a change that stops itself or
//! fails leaves nothing behind.

use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard};

use bb_core::access::{Assignment, Group, GroupMember};
use bb_core::comment::{Comment, CommentVersion, Mention};
use bb_core::edit::{Draft, EditLock, EditNotice};
use bb_core::event::DomainEvent;
use bb_core::follow::Follower;
use bb_core::history::HistoryEntry;
use bb_core::id::{ProjectId, UserId};
use bb_core::issue::{AssignedPerson, Issue, IssueAlias, IssueType};
use bb_core::person::Person;
use bb_core::project::{Project, ProjectAlias};
use bb_core::relation::{IssueParent, Relation, RelationKind};
use bb_core::security_log::Digest;
use bb_core::sign_in::{Invitation, SetupCode, SignInSession, StoredPasskey, TwoStep};
use bb_policy::LevelRules;
use bb_policy::guards::Refusal;
use jiff::Timestamp;

use crate::access::RoleRecord;
use crate::auth::{AccessKeyRecord, SecurityEvent};
use crate::ports::{BoxFuture, ChangeStore};

thread_local! {
    /// Whether the next key to be stored finds its public prefix taken (once). Tests run on a
    /// thread each, so one test cannot set it for another.
    pub(super) static CLASH_ONCE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The next access key that is stored is refused once, as if its public prefix were taken.
pub(crate) fn clash_the_next_key_prefix() {
    CLASH_ONCE.set(true);
}

/// What the fake store can fail with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FakeError {
    /// A change stopped itself (a check inside it failed).
    Aborted,
    /// A key was used twice.
    Duplicate,
    /// A rule of the policy engine stopped a change.
    Refused(Refusal),
    /// Other writers kept changing the same row (the test made it, see `contended_replaces`).
    Contended,
    /// The store itself failed (the test made it).
    Failed,
}

impl fmt::Display for FakeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

/// Everything the store holds. Cloned for a change and put back when the change succeeded.
#[derive(Clone, Default)]
pub(crate) struct State {
    pub(crate) people: Vec<(Person, Option<String>)>,
    pub(crate) sessions: Vec<SignInSession>,
    pub(crate) two_steps: Vec<TwoStep>,
    pub(crate) totp_secrets: Vec<(UserId, Vec<u8>)>,
    pub(crate) backup_codes: Vec<(UserId, Digest, Option<Timestamp>)>,
    pub(crate) passkeys: Vec<StoredPasskey>,
    pub(crate) invitations: Vec<Invitation>,
    pub(crate) access_keys: Vec<AccessKeyRecord>,
    pub(crate) setup_code: Option<SetupCode>,
    pub(crate) settings: Vec<super::fake_settings::FakeSetting>,
    pub(crate) starting_roles_made: bool,
    pub(crate) legacy_administrators: Vec<UserId>,
    pub(crate) roles: Vec<RoleRecord>,
    pub(crate) rules: LevelRules,
    pub(crate) groups: Vec<Group>,
    pub(crate) members: Vec<GroupMember>,
    pub(crate) assignments: Vec<Assignment>,
    pub(crate) projects: Vec<Project>,
    pub(crate) aliases: Vec<ProjectAlias>,
    pub(crate) sign_up_version: i64,
    pub(crate) locks: Vec<EditLock>,
    /// How many times replacing a lock still fails as contended (a test sets it). It is shared
    /// between the copies of the state, so a change that fails does not give the count back.
    pub(crate) contended_replaces: Arc<std::sync::atomic::AtomicUsize>,
    pub(crate) drafts: Vec<Draft>,
    pub(crate) notices: Vec<EditNotice>,
    pub(crate) issue_types: Vec<IssueType>,
    pub(crate) issues: Vec<Issue>,
    pub(crate) assigned_people: Vec<AssignedPerson>,
    pub(crate) issue_aliases: Vec<IssueAlias>,
    pub(crate) counters: Vec<(ProjectId, i64)>,
    pub(crate) relation_kinds: Vec<RelationKind>,
    pub(crate) relations: Vec<Relation>,
    pub(crate) parents: Vec<IssueParent>,
    pub(crate) comments: Vec<Comment>,
    pub(crate) comment_versions: Vec<CommentVersion>,
    pub(crate) mentions: Vec<Mention>,
    pub(crate) followers: Vec<Follower>,
    pub(crate) history: Vec<HistoryEntry>,
    /// How many events the history has finished (their place in the outbox).
    pub(crate) history_position: i64,
    pub(crate) notifications: Vec<bb_core::notification::Notification>,
    pub(crate) handles: Vec<bb_core::handle::PersonHandle>,
    pub(crate) sign_up_invitations: Vec<bb_core::sign_up_invitation::SignUpInvitation>,
    pub(crate) sign_up_mail: Vec<bb_core::sign_up_invitation::SignUpMail>,
    /// How many events the notifications reader has finished (their place in the outbox).
    pub(crate) notification_position: i64,
    /// The password of the mail server, as the secrets table would hold it (not encrypted here).
    pub(crate) mail_password: Option<Vec<u8>>,
    pub(crate) field_definitions: Vec<crate::fields::Definition>,
    pub(crate) field_values: Vec<super::fake_fields::FakeValue>,
    pub(crate) tags: Vec<bb_core::tags::Tag>,
    pub(crate) tag_projects: Vec<bb_core::tags::TagProject>,
    pub(crate) issue_tags: Vec<bb_core::tags::IssueTag>,
    pub(crate) label_groups: Vec<bb_core::tags::LabelGroup>,
    pub(crate) labels: Vec<bb_core::tags::Label>,
    pub(crate) issue_labels: Vec<bb_core::tags::IssueLabel>,
    pub(crate) jobs: Vec<bb_core::job::Job>,
    pub(crate) many_changes_runs: Vec<bb_core::many_changes::ManyChangeRun>,
    pub(crate) layouts: Vec<bb_core::layout::StoredLayout>,
    pub(crate) workflows: Vec<bb_core::workflow::StoredWorkflow>,
    pub(crate) workflow_uses: Vec<bb_core::workflow::WorkflowUse>,
}

/// The state, what was recorded, and the failures a test asked for.
#[derive(Default)]
pub(crate) struct Shared {
    pub(crate) state: State,
    pub(crate) events: Vec<DomainEvent>,
    pub(crate) log: Vec<SecurityEvent>,
    /// How many times the engine asked for the rows of a person at a place.
    pub(crate) policy_loads: usize,
    /// How many reads of any kind the store answered.
    pub(crate) reads: usize,
    /// The security log cannot be written.
    pub(crate) log_fails: bool,
    /// Changes fail as the database does when it breaks.
    pub(crate) changes_fail: bool,
    /// The reads of what the engine decides with fail as the database does when it breaks.
    pub(crate) policy_reads_fail: bool,
    /// How many more times the saving of the progress of a run fails.
    pub(crate) progress_failures: usize,
}

/// The in-memory store. Cheap to clone; all clones are the same store.
#[derive(Clone, Default)]
pub(crate) struct FakeStore {
    shared: Arc<Mutex<Shared>>,
}

/// The open "transaction" of a change: a copy of the state.
pub(crate) struct Tx {
    pub(crate) state: State,
    /// The events of what a change did besides what its own event says.
    pub(crate) follow_ups: Vec<DomainEvent>,
}

impl FakeStore {
    pub(crate) fn lock(&self) -> MutexGuard<'_, Shared> {
        self.shared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The events recorded so far.
    pub(crate) fn event_count(&self) -> usize {
        self.lock().events.len()
    }

    /// The entries of the security log so far.
    pub(crate) fn log_count(&self) -> usize {
        self.lock().log.len()
    }

    /// Makes the security log fail (or work again).
    pub(crate) fn fail_security_log(&self, fails: bool) {
        self.lock().log_fails = fails;
    }

    /// Makes every change fail (or work again): the store breaks while a change is committed.
    pub(crate) fn fail_changes(&self, fails: bool) {
        self.lock().changes_fail = fails;
    }

    /// Makes the reads of what the engine decides with fail (or work again): the store breaks
    /// while a request asks who may do what.
    pub(crate) fn fail_policy_reads(&self, fails: bool) {
        self.lock().policy_reads_fail = fails;
    }

    /// Makes the next `times` savings of the progress of a run fail, as the database does when it
    /// breaks for a while.
    pub(crate) fn fail_many_changes_progress(&self, times: usize) {
        self.lock().progress_failures = times;
    }

    /// Reads from the state.
    pub(crate) fn read<T>(&self, read: impl FnOnce(&State) -> T) -> T {
        let mut shared = self.lock();
        shared.reads += 1;
        read(&shared.state)
    }

    /// How many reads the store answered so far, of any kind.
    pub(crate) fn reads(&self) -> usize {
        self.lock().reads
    }

    /// How many times the engine loaded the rows of a person at a place.
    pub(crate) fn policy_loads(&self) -> usize {
        self.lock().policy_loads
    }
}

impl ChangeStore for FakeStore {
    type Transaction = Tx;
    type Error = FakeError;

    async fn record_changes<'a, T: Send + 'a>(
        &'a self,
        events: &'a [DomainEvent],
        change: impl for<'t> FnOnce(&'t mut Tx) -> BoxFuture<'t, Result<T, FakeError>> + Send + 'a,
    ) -> Result<T, FakeError> {
        let state = {
            let shared = self.lock();
            if shared.changes_fail {
                return Err(FakeError::Failed);
            }
            shared.state.clone()
        };
        let mut tx = Tx {
            state,
            follow_ups: Vec::new(),
        };
        let result = change(&mut tx).await?;
        let mut shared = self.lock();
        shared.state = tx.state;
        shared.events.extend(events.iter().cloned());
        shared.events.extend(tx.follow_ups);
        Ok(result)
    }

    async fn read_snapshot<'a, T: Send + 'a>(
        &'a self,
        read: impl for<'t> FnOnce(&'t mut Tx) -> BoxFuture<'t, Result<T, FakeError>> + Send + 'a,
    ) -> Result<T, FakeError> {
        let state = {
            let mut shared = self.lock();
            shared.reads += 1;
            shared.state.clone()
        };
        read(&mut Tx {
            state,
            follow_ups: Vec::new(),
        })
        .await
    }
}
