//! What the editing use cases take and give back (ADR-0019).
//!
//! The lease, the heartbeat and the draft are the state of an open editor, not facts about a
//! record: the inputs that only keep that state end in `Query` like the other bookkeeping of the
//! program (the time a session was last used), and write no domain event. The inputs that other
//! people can learn from (*Ask … to finish*, *Edit anyway*, releasing a stuck lock) are commands
//! with an event (ARCH-EVENTS). Everything that is given back is wrapped in `Authorized<T>`.

use bb_core::edit::{DraftTarget, EditWhat, NoticeKind};
use bb_core::event::EntityRef;
use bb_core::id::{EditorId, LockToken};
use bb_policy::{Masked, PersonRef};
use jiff::Timestamp;
use serde::Serialize;

/// Take the lock on a thing, on the first keystroke.
pub struct TakeLockQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The thing that is going to be edited.
    pub what: EditWhat,
    /// The open editor (tab) that asks.
    pub editor: EditorId,
}

/// Ask who is editing a thing, without taking the lock (the box has just opened).
pub struct ReadLockQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The thing that is going to be edited.
    pub what: EditWhat,
}

/// Whether somebody else is editing a thing now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LockHolder {
    /// Who is editing and since when; nothing when the thing is free. A lock of the person's own
    /// (another window) is shown too, with `yours_elsewhere` set.
    pub held: Option<HeldLock>,
}

impl Masked for LockHolder {}

/// A text that is kept as the draft of the person while they edit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DraftText {
    /// The text as it stands now.
    pub text: String,
    /// The version of the record the person started from; `None` for a new record.
    pub base_version: Option<i64>,
}

/// Renew the lease of a lock, and keep the text so far as the draft.
pub struct HeartbeatQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The thing that is being edited.
    pub what: EditWhat,
    /// The token the lock was taken with.
    pub token: LockToken,
    /// The text so far, kept as the draft of the person even when the lock is gone.
    pub draft: Option<DraftText>,
}

/// Let go of a lock: the text was saved, or the editor was closed.
pub struct ReleaseLockQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The thing that was edited.
    pub what: EditWhat,
    /// The token the lock was taken with.
    pub token: LockToken,
}

/// Ask the person who is editing to finish (*Ask … to finish*): they are told at their next
/// heartbeat.
pub struct AskToFinish {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The thing somebody else is editing.
    pub what: EditWhat,
}

/// Take the editing over (*Edit anyway*). The person who was editing keeps their text as their
/// draft and is told at their next heartbeat.
pub struct EditAnyway {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The thing somebody else is editing.
    pub what: EditWhat,
    /// The open editor (tab) that takes over.
    pub editor: EditorId,
}

/// Release a lock that is stuck, whoever holds it. Recorded in the security log.
pub struct ReleaseStuckLock {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The thing whose lock is stuck.
    pub what: EditWhat,
}

/// Keep a text as the draft of the person.
pub struct SaveDraftQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// What the draft is for.
    pub target: DraftTarget,
    /// The text and the version it started from.
    pub draft: DraftText,
}

/// The draft of the person for a target.
pub struct ReadDraftQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// What the draft is for.
    pub target: DraftTarget,
}

/// Throw the draft of the person away (*Throw away*).
pub struct ThrowAwayDraftQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// What the draft is for.
    pub target: DraftTarget,
}

/// Ask for the versions of some records, for the check every 30 seconds and on return to a tab.
pub struct CheckVersionsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The records on the screen, at most `bb_core::edit::VERSIONS_MAX`.
    pub records: Vec<EntityRef>,
}

/// Delete the drafts that outlived their keep period: what the background job does (ADR-0017).
pub struct PurgeDraftsQuery;

/// The editor has the lock.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TakenLock {
    /// The token to save with and to send with every heartbeat.
    pub token: LockToken,
    /// When the lock runs out if no heartbeat renews it.
    pub expires_at: Timestamp,
    /// How long a lease lasts, in seconds.
    pub lease_seconds: i64,
    /// How often to send a heartbeat, in seconds.
    pub heartbeat_seconds: i64,
}

/// Somebody else is editing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HeldLock {
    /// Who is editing, as the person who asks may see them (*Someone* when they may not).
    pub holder: PersonRef,
    /// Since when.
    pub since: Timestamp,
    /// The holder is the person who asks, in another window.
    pub yours_elsewhere: bool,
    /// The person who asks may take the editing over (*Edit anyway*).
    pub can_edit_anyway: bool,
}

/// The answer to asking for a lock.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum LockOutcome {
    /// The editor has the lock.
    Taken(TakenLock),
    /// Somebody else is editing.
    Held(HeldLock),
}

impl Masked for LockOutcome {}

/// Whether the editor still has the lock, and if not, why. The draft is kept either way.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum LockStatus {
    /// The lease is renewed.
    Held {
        /// When it runs out now.
        expires_at: Timestamp,
    },
    /// No heartbeat came for a minute, or the lock was released.
    RanOut,
    /// Somebody took the editing over (*Edit anyway*).
    TakenOver {
        /// Who, as the person who asks may see them.
        by: PersonRef,
    },
}

/// A note for the person who is editing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NoticeView {
    /// What happened.
    pub kind: NoticeKind,
    /// Who did it, as the person who asks may see them.
    pub from: PersonRef,
    /// When.
    pub at: Timestamp,
}

/// The answer to a heartbeat.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HeartbeatAnswer {
    /// Whether the editor still has the lock.
    pub lock: LockStatus,
    /// What others did since the last heartbeat; each note is shown once.
    pub notices: Vec<NoticeView>,
    /// The text that came with the heartbeat was kept as the draft of the person.
    pub draft_saved: bool,
}

impl Masked for HeartbeatAnswer {}

/// A draft as its author sees it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DraftView {
    /// The text.
    pub text: String,
    /// The version of the record the person started from.
    pub base_version: Option<i64>,
    /// When the draft last changed.
    pub saved_at: Timestamp,
}

impl Masked for DraftView {}

/// The draft of the author for a target, if there is one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DraftSlot {
    /// The draft, or nothing.
    pub draft: Option<DraftView>,
}

impl Masked for DraftSlot {}

/// The version of one record on the screen.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RecordVersion {
    /// The record.
    pub record: EntityRef,
    /// Its version now; nothing when it is gone or the person may not know about it (the two
    /// look the same).
    pub version: Option<i64>,
}

/// The versions of the records asked about, in the order asked.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Versions {
    /// One entry for each record asked about.
    pub items: Vec<RecordVersion>,
}

impl Masked for Versions {}
