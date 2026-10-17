//! The bodies and queries of the routes of editing.

use super::*;

/// A record, by its kind and ID.
#[derive(Deserialize, Serialize, ToSchema)]
pub(crate) struct RecordRefBody {
    /// The kind of record, like `issue`.
    pub(super) kind: String,
    /// The ID of the record.
    pub(super) id: String,
}

impl RecordRefBody {
    pub(super) fn record(&self) -> Result<EntityRef, ApiError> {
        Ok(EntityRef {
            kind: EntityKind::new(&self.kind).map_err(|_| ApiError::new(ErrorCode::NotFound))?,
            id: EntityId::from_str(&self.id).map_err(|_| ApiError::new(ErrorCode::NotFound))?,
        })
    }
}

impl From<&EntityRef> for RecordRefBody {
    fn from(record: &EntityRef) -> Self {
        Self {
            kind: record.kind.as_str().to_owned(),
            id: record.id.to_string(),
        }
    }
}

/// One thing being edited: one part of one record.
#[derive(Deserialize, ToSchema)]
pub(crate) struct EditWhatBody {
    /// The record.
    pub(super) record: RecordRefBody,
    /// Which long text of the record, like `description`.
    pub(super) part: String,
}

impl EditWhatBody {
    pub(super) fn what(&self) -> Result<EditWhat, ApiError> {
        Ok(EditWhat {
            record: self.record.record()?,
            part: TechnicalName::new(&self.part)
                .map_err(|_| ApiError::new(ErrorCode::InvalidRequest))?,
        })
    }
}

/// Which thing a person wants the lock on, and from which open editor.
#[derive(Deserialize, ToSchema)]
pub(crate) struct TakeLockBody {
    /// The thing that is going to be edited.
    pub(super) what: EditWhatBody,
    /// The open editor (a tab): an ID the browser makes when the editor opens.
    pub(super) editor_id: String,
}

/// The editor has the lock.
#[derive(Serialize, ToSchema)]
pub(crate) struct TakenLockBody {
    /// The lock token to save with and to send with every heartbeat.
    pub(super) lock_token: String,
    /// When the lock runs out if no heartbeat renews it.
    pub(super) expires_at: String,
    /// How long a lease lasts, in seconds.
    pub(super) lease_seconds: i64,
    /// How often to send a heartbeat, in seconds.
    pub(super) heartbeat_seconds: i64,
}

/// Somebody else is editing.
#[derive(Serialize, ToSchema)]
pub(crate) struct HeldLockBody {
    /// Who is editing, as the person who asks may see them.
    pub(super) holder: PersonRef,
    /// Since when.
    pub(super) since: String,
    /// The holder is the person who asks, in another window.
    pub(super) yours_elsewhere: bool,
    /// The person who asks may take the editing over (*Edit anyway*).
    pub(super) can_edit_anyway: bool,
}

impl From<&HeldLock> for HeldLockBody {
    fn from(held: &HeldLock) -> Self {
        Self {
            holder: held.holder.clone(),
            since: held.since.to_string(),
            yours_elsewhere: held.yours_elsewhere,
            can_edit_anyway: held.can_edit_anyway,
        }
    }
}

/// Which thing to ask about, in the address.
#[derive(Deserialize, ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct EditWhatQuery {
    /// The kind of record, like `issue`.
    pub(super) kind: String,
    /// The ID of the record.
    pub(super) id: String,
    /// Which long text of the record, like `description`.
    pub(super) part: String,
}

/// Whether somebody is editing a thing now.
#[derive(Serialize, ToSchema)]
pub(crate) struct LockHolderBody {
    /// Who is editing, or nothing when the thing is free.
    pub(super) held: Option<HeldLockBody>,
}

impl Masked for LockHolderBody {}

impl From<&LockHolder> for LockHolderBody {
    fn from(state: &LockHolder) -> Self {
        Self {
            held: state.held.as_ref().map(HeldLockBody::from),
        }
    }
}

/// The answer to asking for a lock.
#[derive(Serialize, ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub(crate) enum LockOutcomeBody {
    /// The editor has the lock.
    Taken(TakenLockBody),
    /// Somebody else is editing.
    Held(HeldLockBody),
}

impl Masked for LockOutcomeBody {}

impl From<&LockOutcome> for LockOutcomeBody {
    fn from(outcome: &LockOutcome) -> Self {
        match outcome {
            LockOutcome::Taken(TakenLock {
                token,
                expires_at,
                lease_seconds,
                heartbeat_seconds,
            }) => Self::Taken(TakenLockBody {
                lock_token: token.to_string(),
                expires_at: expires_at.to_string(),
                lease_seconds: *lease_seconds,
                heartbeat_seconds: *heartbeat_seconds,
            }),
            LockOutcome::Held(held) => Self::Held(HeldLockBody::from(held)),
        }
    }
}

/// The text so far of a draft, with the version it started from.
#[derive(Deserialize, ToSchema)]
pub(crate) struct DraftTextBody {
    /// The text as it stands now.
    pub(super) text: String,
    /// The version of the record the person started from; nothing for a new record.
    pub(super) base_version: Option<i64>,
}

impl From<DraftTextBody> for DraftText {
    fn from(body: DraftTextBody) -> Self {
        Self {
            text: body.text,
            base_version: body.base_version,
        }
    }
}

/// A heartbeat: the editor is still open.
#[derive(Deserialize, ToSchema)]
pub(crate) struct HeartbeatBody {
    /// The thing that is being edited.
    pub(super) what: EditWhatBody,
    /// The lock token the lock was taken with.
    pub(super) lock_token: String,
    /// The text so far: kept as the draft of the person, also when the lock is gone.
    pub(super) draft: Option<DraftTextBody>,
}

/// Whether the editor still has the lock, and if not, why.
#[derive(Serialize, ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub(crate) enum LockStatusBody {
    /// The lease is renewed.
    Held {
        /// When it runs out now.
        expires_at: String,
    },
    /// No heartbeat came for a minute, or the lock was let go. The draft is kept.
    RanOut,
    /// Somebody took the editing over (*Edit anyway*). The draft is kept.
    TakenOver {
        /// Who, as the person who asks may see them.
        by: PersonRef,
    },
}

impl From<&LockStatus> for LockStatusBody {
    fn from(status: &LockStatus) -> Self {
        match status {
            LockStatus::Held { expires_at } => Self::Held {
                expires_at: expires_at.to_string(),
            },
            LockStatus::RanOut => Self::RanOut,
            LockStatus::TakenOver { by } => Self::TakenOver { by: by.clone() },
        }
    }
}

/// A note for the person who is editing.
#[derive(Serialize, ToSchema)]
pub(crate) struct NoticeBody {
    /// What happened.
    pub(super) kind: NoticeKind,
    /// Who did it, as the person who asks may see them.
    pub(super) from: PersonRef,
    /// When.
    pub(super) at: String,
}

impl From<&NoticeView> for NoticeBody {
    fn from(notice: &NoticeView) -> Self {
        Self {
            kind: notice.kind,
            from: notice.from.clone(),
            at: notice.at.to_string(),
        }
    }
}

/// The answer to a heartbeat.
#[derive(Serialize, ToSchema)]
pub(crate) struct HeartbeatAnswerBody {
    /// Whether the editor still has the lock.
    pub(super) lock: LockStatusBody,
    /// What others did since the last heartbeat; each note is shown once.
    pub(super) notices: Vec<NoticeBody>,
    /// The text that came with the heartbeat was kept as the draft of the person.
    pub(super) draft_saved: bool,
}

impl Masked for HeartbeatAnswerBody {}

impl From<&HeartbeatAnswer> for HeartbeatAnswerBody {
    fn from(heartbeat: &HeartbeatAnswer) -> Self {
        Self {
            lock: LockStatusBody::from(&heartbeat.lock),
            notices: heartbeat.notices.iter().map(NoticeBody::from).collect(),
            draft_saved: heartbeat.draft_saved,
        }
    }
}

/// Which lock to let go of, and the token it was taken with.
#[derive(Deserialize, ToSchema)]
pub(crate) struct ReleaseLockBody {
    /// The thing that was edited.
    pub(super) what: EditWhatBody,
    /// The lock token the lock was taken with.
    pub(super) lock_token: String,
}

/// Which thing somebody else is editing.
#[derive(Deserialize, ToSchema)]
pub(crate) struct EditWhatOnlyBody {
    /// The thing somebody else is editing.
    pub(super) what: EditWhatBody,
}

/// Take the editing over from the person who is editing.
#[derive(Deserialize, ToSchema)]
pub(crate) struct EditAnywayBody {
    /// The thing somebody else is editing.
    pub(super) what: EditWhatBody,
    /// The open editor (a tab) that takes over.
    pub(super) editor_id: String,
}

/// What a draft is for: a part of a record (`kind`, `id` and `part`), or the form that makes a new
/// record of a kind in a project (`kind` and `project`).
#[derive(Deserialize, ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct DraftTargetBody {
    /// The kind of record, like `issue`.
    pub(super) kind: String,
    /// The ID of the record, for a draft of a part of a record.
    pub(super) id: Option<String>,
    /// Which long text of the record, for a draft of a part of a record.
    pub(super) part: Option<String>,
    /// The project, for a draft of a new record.
    pub(super) project: Option<String>,
}

impl DraftTargetBody {
    pub(super) fn target(&self) -> Result<DraftTarget, ApiError> {
        let invalid = || ApiError::new(ErrorCode::InvalidRequest);
        match (&self.id, &self.part, &self.project) {
            (Some(id), Some(part), None) => Ok(DraftTarget::Editing(
                EditWhatBody {
                    record: RecordRefBody {
                        kind: self.kind.clone(),
                        id: id.clone(),
                    },
                    part: part.clone(),
                }
                .what()?,
            )),
            (None, None, Some(project)) => Ok(DraftTarget::NewRecord {
                project: body_id::<ProjectId>(project)?,
                kind: EntityKind::new(&self.kind).map_err(|_| invalid())?,
            }),
            _ => Err(invalid()),
        }
    }
}

/// A draft to keep.
#[derive(Deserialize, ToSchema)]
pub(crate) struct SaveDraftBody {
    /// What the draft is for.
    pub(super) target: DraftTargetBody,
    /// The text and the version it started from.
    pub(super) draft: DraftTextBody,
}

/// A draft as its author sees it.
#[derive(Serialize, ToSchema)]
pub(crate) struct DraftBody {
    /// The text.
    pub(super) text: String,
    /// The version of the record the person started from.
    pub(super) base_version: Option<i64>,
    /// When the draft last changed.
    pub(super) saved_at: String,
}

impl Masked for DraftBody {}

impl From<&DraftView> for DraftBody {
    fn from(draft: &DraftView) -> Self {
        Self {
            text: draft.text.clone(),
            base_version: draft.base_version,
            saved_at: draft.saved_at.to_string(),
        }
    }
}

/// The draft of the person for a target, if there is one.
#[derive(Serialize, ToSchema)]
pub(crate) struct DraftSlotBody {
    /// The draft, or nothing.
    pub(super) draft: Option<DraftBody>,
}

impl Masked for DraftSlotBody {}

impl From<&DraftSlot> for DraftSlotBody {
    fn from(slot: &DraftSlot) -> Self {
        Self {
            draft: slot.draft.as_ref().map(DraftBody::from),
        }
    }
}

/// The records on the screen.
#[derive(Deserialize, ToSchema)]
pub(crate) struct CheckVersionsBody {
    /// The records, at most the number the generated client calls `versionsMax`.
    pub(super) records: Vec<RecordRefBody>,
}

/// The version of one record on the screen.
#[derive(Serialize, ToSchema)]
pub(crate) struct RecordVersionBody {
    /// The record.
    pub(super) record: RecordRefBody,
    /// Its version now; nothing when it is gone or the person may not know about it (the two look
    /// the same).
    pub(super) version: Option<i64>,
}

/// The versions of the records asked about, in the order asked.
#[derive(Serialize, ToSchema)]
pub(crate) struct VersionsBody {
    /// One entry for each record asked about.
    pub(super) items: Vec<RecordVersionBody>,
}

impl Masked for VersionsBody {}

impl From<&Versions> for VersionsBody {
    fn from(checked: &Versions) -> Self {
        Self {
            items: checked
                .items
                .iter()
                .map(|item| RecordVersionBody {
                    record: RecordRefBody::from(&item.record),
                    version: item.version,
                })
                .collect(),
        }
    }
}
