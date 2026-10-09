//! Editing long text without losing it (ISS-17, ISS-11, ADR-0019): the lease of an edit lock, the
//! draft of a person, and the notes that reach the person who is editing at their next heartbeat.
//!
//! Every time and number of the mechanism is a named constant here, written once: the lease (60 s),
//! the heartbeat (15 s), the keep period of a draft (30 days, owner's answer M3-Q9) and how many
//! things one watch request may ask about. The browser reads them from the generated client
//! (`cargo xtask gen api`), so it never writes them a second time.
//!
//! A lock or a draft is about one *thing being edited*: one part (`description`) of one record. A
//! record is named by its [`EntityRef`], so the mechanism works for any kind of record, issues and
//! comments included, without knowing them.

use std::fmt;

use jiff::{SignedDuration, Timestamp};

use crate::event::{EntityKind, EntityRef};
use crate::id::{EditorId, LockToken, ProjectId, UserId};
use crate::personal::Entity;
use crate::technical::TechnicalName;

/// How long a lock lasts after the last heartbeat, in seconds.
pub const LEASE_SECONDS: i64 = 60;
/// How often the open editor sends a heartbeat, in seconds.
pub const HEARTBEAT_SECONDS: i64 = 15;
/// How long a draft is kept after its last change, in days (M3-Q9).
pub const DRAFT_KEEP_DAYS: i64 = 30;
/// The most things one watch request asks about: what is on the screen.
pub const WATCH_MAX: u32 = 50;
/// The most characters of the text of a draft.
pub const DRAFT_TEXT_MAX: usize = 100_000;
/// The kind of job that deletes the drafts that outlived their keep period (ADR-0017).
pub const PURGE_DRAFTS_JOB: &str = "editing.purge_drafts";

const SECONDS_PER_DAY: i64 = 86_400;

/// How long a lock lasts after the last heartbeat.
#[must_use]
pub const fn lease() -> SignedDuration {
    SignedDuration::from_secs(LEASE_SECONDS)
}

/// How long a draft is kept after its last change.
#[must_use]
pub const fn draft_keep() -> SignedDuration {
    SignedDuration::from_secs(DRAFT_KEEP_DAYS * SECONDS_PER_DAY)
}

/// The moment a lease that is renewed at `now` ends. `None` only for a moment so late that it
/// cannot be written.
#[must_use]
pub fn lease_ends(now: Timestamp) -> Option<Timestamp> {
    now.checked_add(lease()).ok()
}

/// The moment before which a draft is too old to keep, at `now`.
#[must_use]
pub fn draft_cutoff(now: Timestamp) -> Option<Timestamp> {
    now.checked_sub(draft_keep()).ok()
}

/// The text is not the name of a part of a record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidPart;

impl fmt::Display for InvalidPart {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("the part of a record is a short technical name, like description")
    }
}

impl std::error::Error for InvalidPart {}

/// One thing being edited: one part of one record (*the description of this issue*).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EditWhat {
    /// The record.
    pub record: EntityRef,
    /// Which long text of the record.
    pub part: TechnicalName,
}

// A kind of record and the name of a part: no personal data.
impl crate::personal::NotPersonal for EditWhat {}

/// What a draft is for: a text being changed, or a form that makes a new record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DraftTarget {
    /// A part of a record that exists. One draft for each person and part.
    Editing(EditWhat),
    /// The form that makes a record of this kind in this project (*New issue*). One draft for
    /// each person, project and form.
    NewRecord {
        /// The project the new record is for.
        project: ProjectId,
        /// The kind of record the form makes.
        kind: EntityKind,
    },
}

impl DraftTarget {
    /// The stable text a draft is found by, together with the person. Written here and nowhere
    /// else: the stores keep it as a column and never build it themselves.
    #[must_use]
    pub fn key(&self) -> String {
        match self {
            Self::Editing(what) => format!(
                "edit/{}/{}/{}",
                what.record.kind.as_str(),
                what.record.id,
                what.part
            ),
            Self::NewRecord { project, kind } => format!("new/{project}/{}", kind.as_str()),
        }
    }
}

// The name of a kind, IDs and the name of a part: no personal data.
impl crate::personal::NotPersonal for DraftTarget {}

words! {
    /// What the person who is editing is told at their next heartbeat.
    pub enum NoticeKind {
        /// Somebody asked them to finish (*Ask … to finish*).
        FinishAsked => "finish_asked",
        /// Somebody took the editing over (*Edit anyway*); their text is kept as their draft.
        TakenOver => "taken_over",
    }
}

words_in_api!(
    NoticeKind,
    "What the person who is editing is told: somebody asked them to finish, or somebody took the editing over."
);

/// The lease on one thing being edited: who holds it, in which editor, until when (ADR-0019). A
/// row lives only while the lease runs; an expired row is deleted, not kept.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct EditLock {
    /// The record that is being edited.
    #[not_personal]
    pub record: EntityRef,
    /// Which long text of the record.
    #[not_personal]
    pub part: TechnicalName,
    /// Who holds the lock.
    #[personal(category = "activity", basis = "legitimate_interests")]
    pub holder: UserId,
    /// The open editor (a tab) that holds it.
    #[not_personal]
    pub editor: EditorId,
    /// The token the editor saves with.
    #[not_personal]
    pub token: LockToken,
    /// When the lock was taken.
    #[not_personal]
    pub taken_at: Timestamp,
    /// When the lock runs out if no heartbeat renews it.
    #[not_personal]
    pub expires_at: Timestamp,
}

impl EditLock {
    /// What the lock is for.
    #[must_use]
    pub fn what(&self) -> EditWhat {
        EditWhat {
            record: self.record.clone(),
            part: self.part.clone(),
        }
    }

    /// Whether the lease has run out at `now`.
    #[must_use]
    pub fn has_expired(&self, now: Timestamp) -> bool {
        self.expires_at <= now
    }
}

/// What happens when an editor asks for the lock on a thing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Taking {
    /// The editor has the lock now (with this token, which is the old one when the same editor
    /// asks again).
    Granted(EditLock),
    /// Somebody holds it; the lock is theirs.
    Held(EditLock),
}

impl Taking {
    /// The decision, from the lock that is stored for the thing (`existing`, if there is a row)
    /// and the lock the editor `wanted`. A free thing, or one whose lease ran out, is taken; the
    /// same editor asking again keeps its token and gets a new lease; anybody else is told who
    /// holds it. Written once: the stores apply it with a conditional write.
    #[must_use]
    pub fn decide(existing: Option<&EditLock>, wanted: &EditLock, now: Timestamp) -> Self {
        match existing {
            Some(lock) if !lock.has_expired(now) => {
                if lock.holder == wanted.holder && lock.editor == wanted.editor {
                    Self::Granted(EditLock {
                        expires_at: wanted.expires_at,
                        ..lock.clone()
                    })
                } else {
                    Self::Held(lock.clone())
                }
            }
            _ => Self::Granted(wanted.clone()),
        }
    }
}

/// A text a person wrote and has not saved (*Draft*, ISS-11): private to its author, kept until it
/// is saved or thrown away, at the longest [`DRAFT_KEEP_DAYS`] after its last change.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct Draft {
    /// Whose draft it is. Nobody else can read it.
    #[personal(category = "identity", basis = "legitimate_interests")]
    pub author: UserId,
    /// What it is a draft of.
    #[not_personal]
    pub target: DraftTarget,
    /// The text. It can hold anything the person typed.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub text: String,
    /// The version of the record the person started from, for the comparison when they come back.
    /// `None` for a draft of a new record.
    #[not_personal]
    pub base_version: Option<i64>,
    /// When the draft last changed: the keep period counts from here.
    #[not_personal]
    pub saved_at: Timestamp,
}

/// A note for the person who was editing, shown at their next heartbeat (M3-Q3).
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct EditNotice {
    /// Who the note is for.
    #[personal(category = "activity", basis = "legitimate_interests")]
    pub recipient: UserId,
    /// The record that was being edited.
    #[not_personal]
    pub record: EntityRef,
    /// Which long text of the record.
    #[not_personal]
    pub part: TechnicalName,
    /// What happened.
    #[not_personal]
    pub kind: NoticeKind,
    /// Who did it.
    #[personal(category = "activity", basis = "legitimate_interests")]
    pub from: UserId,
    /// When.
    #[not_personal]
    pub at: Timestamp,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::RecordKind;
    use crate::id::{EntityId, SequentialIds};

    fn what(ids: &mut SequentialIds) -> EditWhat {
        EditWhat {
            record: EntityRef {
                kind: RecordKind::Project.into(),
                id: EntityId::from(ProjectId::generate(ids).unwrap()),
            },
            part: TechnicalName::new("description").unwrap(),
        }
    }

    #[test]
    fn adr_0019_the_lease_is_a_minute_and_the_heartbeat_a_quarter_of_it() {
        assert_eq!(lease(), SignedDuration::from_secs(60));
        assert_eq!(LEASE_SECONDS, 4 * HEARTBEAT_SECONDS);
        assert_eq!(draft_keep(), SignedDuration::from_hours(30 * 24));
    }

    #[test]
    fn a_lease_runs_out_at_its_end() {
        let ids = &mut SequentialIds::default();
        let now = Timestamp::from_second(1_800_000_000).unwrap();
        let lock = EditLock {
            record: what(ids).record,
            part: TechnicalName::new("description").unwrap(),
            holder: UserId::generate(ids),
            editor: EditorId::generate(ids),
            token: LockToken::generate(ids),
            taken_at: now,
            expires_at: lease_ends(now).unwrap(),
        };
        assert!(!lock.has_expired(now));
        assert!(lock.has_expired(lease_ends(now).unwrap()));
    }

    #[test]
    fn iss_17_a_lock_is_taken_when_free_or_expired_and_held_otherwise() {
        let ids = &mut SequentialIds::default();
        let now = Timestamp::from_second(1_800_000_000).unwrap();
        let lock_of = |holder: UserId, editor: EditorId, at: Timestamp| EditLock {
            record: what(&mut SequentialIds::default()).record,
            part: TechnicalName::new("description").unwrap(),
            holder,
            editor,
            token: LockToken::from_random([9; 16]),
            taken_at: at,
            expires_at: lease_ends(at).unwrap(),
        };
        let (ana, ben) = (UserId::generate(ids), UserId::generate(ids));
        let (first, second) = (EditorId::generate(ids), EditorId::generate(ids));
        let held = lock_of(ana, first, now);
        let later = lease_ends(now).unwrap();
        // Free: granted as asked.
        let wanted = lock_of(ben, second, now);
        assert_eq!(
            Taking::decide(None, &wanted, now),
            Taking::Granted(wanted.clone())
        );
        // Held by somebody else: told who.
        assert_eq!(
            Taking::decide(Some(&held), &wanted, now),
            Taking::Held(held.clone())
        );
        // The lease ran out: free again.
        assert_eq!(
            Taking::decide(Some(&held), &lock_of(ben, second, later), later),
            Taking::Granted(lock_of(ben, second, later))
        );
        // The same editor again: same token, new lease.
        let again = lock_of(ana, first, later);
        let Taking::Granted(kept) = Taking::decide(Some(&held), &again, now) else {
            panic!("the same editor is granted the lock again");
        };
        assert_eq!(kept.token, held.token);
        assert_eq!(kept.expires_at, again.expires_at);
        // The same person in another window is somebody else.
        let other_window = lock_of(ana, second, now);
        assert!(matches!(
            Taking::decide(Some(&held), &other_window, now),
            Taking::Held(_)
        ));
    }

    #[test]
    fn iss_11_each_target_of_a_draft_has_its_own_key() {
        let ids = &mut SequentialIds::default();
        let first = DraftTarget::Editing(what(ids));
        let second = DraftTarget::Editing(what(ids));
        let project = ProjectId::generate(ids).unwrap();
        let kind = EntityKind::new("issue").unwrap();
        let new_one = DraftTarget::NewRecord {
            project,
            kind: kind.clone(),
        };
        let keys = [first.key(), second.key(), new_one.key()];
        assert_eq!(
            keys.iter().collect::<std::collections::HashSet<_>>().len(),
            3
        );
        assert!(keys[0].starts_with("edit/project/"));
        assert_eq!(new_one.key(), format!("new/{project}/issue"));
        assert_eq!(
            new_one.key(),
            DraftTarget::NewRecord { project, kind }.key()
        );
    }
}
