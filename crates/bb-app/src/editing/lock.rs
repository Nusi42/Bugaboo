//! Edit locks (ISS-17, ADR-0019): taking the lock on the first keystroke, the heartbeat that
//! renews it, letting go, *Ask … to finish*, *Edit anyway*, and an administrator releasing a stuck
//! lock.
//!
//! A lock is a lease on one thing being edited (one part of one record). The lease and the heartbeat
//! are bookkeeping of the open editor and write no domain event. Taking the editing over, asking
//! the person to finish and releasing somebody else's lock are facts others can learn from, so
//! they are commands with an event; releasing somebody else's lock is also written to the security
//! log.

use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::edit::{
    self, DraftTarget, EditLock, EditNotice, EditWhat, HEARTBEAT_SECONDS, Hold, LEASE_SECONDS,
    NoticeKind, Taking,
};
use bb_core::event::FieldChange;
use bb_core::id::{LockToken, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_core::sign_in::SecurityAction;
use bb_policy::{Authorized, Evaluation, PeopleContext, Permission};
use serde_json::json;

use super::parts::Need;
use super::types::{
    AskToFinish, EditAnyway, HeartbeatAnswer, HeartbeatQuery, HeldLock, LockHolder, LockOutcome,
    LockStatus, NoticeView, ReadLockQuery, ReleaseLockQuery, ReleaseStuckLock, TakeLockQuery,
    TakenLock,
};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, Done, Stops, by, change_failed, internal};

/// The name of the field an event about a lock changes: the part is in the value.
const EVENT_FIELD: &str = "edit_lock";

/// When a lease that starts at `now` ends: an error only where time cannot be written.
fn lease_ends(now: jiff::Timestamp) -> Result<jiff::Timestamp, AppError> {
    edit::lease_ends(now)
        .ok_or_else(|| AppError::Internal("a lease that cannot be written".to_owned()))
}

impl<S: AuthStore> Ctx<'_, S> {
    /// How a person who edits is shown to the viewer: by the kind *"… is editing"*.
    pub(super) async fn editor_ref(
        &self,
        viewer: &Evaluation,
        person: UserId,
    ) -> Result<bb_policy::PersonRef, AppError> {
        self.person_ref(viewer, PeopleKind::Editing, PeopleContext::Ordinary, person)
            .await
    }

    /// A lock held on a thing as the person who asks sees it: who holds it (by the kind *"… is
    /// editing"*), since when, whether it is the person's own in another window, and whether they
    /// may take the editing over. The one place that makes it, for taking and for asking.
    async fn held_lock(
        &self,
        access: &Evaluation,
        actor: &Actor,
        lock: &EditLock,
    ) -> Result<HeldLock, AppError> {
        Ok(HeldLock {
            holder: self.editor_ref(access, lock.holder).await?,
            since: lock.taken_at,
            yours_elsewhere: lock.holder == actor.person.id,
            can_edit_anyway: access.allows(Permission::EditAnyway),
        })
    }

    /// A new lock for the actor, with a fresh token and a lease that starts now.
    fn lock_for(
        &self,
        actor: &Actor,
        what: &EditWhat,
        editor: bb_core::id::EditorId,
    ) -> Result<EditLock, AppError> {
        let now = self.now();
        let expires_at = lease_ends(now)?;
        Ok(EditLock {
            record: what.record.clone(),
            part: what.part.clone(),
            holder: actor.person.id,
            editor,
            token: LockToken::generate(&mut self.ids()),
            taken_at: now,
            expires_at,
        })
    }

    /// The lock that is held on the thing now, if any: an expired row is no lock.
    async fn live_lock(&self, what: &EditWhat) -> Result<Option<EditLock>, AppError> {
        let now = self.now();
        Ok(self
            .store()
            .edit_lock(what.clone())
            .await
            .map_err(|error| internal::<S>(&error))?
            .filter(|lock| !lock.has_expired(now)))
    }

    /// The answer for an editor that has the lock.
    fn taken(lock: &EditLock) -> TakenLock {
        TakenLock {
            token: lock.token,
            expires_at: lock.expires_at,
            lease_seconds: LEASE_SECONDS,
            heartbeat_seconds: HEARTBEAT_SECONDS,
        }
    }

    /// The notes for the actor about a thing, as they may read them.
    async fn notice_views(
        &self,
        viewer: &Evaluation,
        notices: &[EditNotice],
    ) -> Result<Vec<NoticeView>, AppError> {
        let mut views = Vec::with_capacity(notices.len());
        for notice in notices {
            views.push(NoticeView {
                kind: notice.kind,
                from: self.editor_ref(viewer, notice.from).await?,
                at: notice.at,
            });
        }
        Ok(views)
    }
}

/// Says who is editing a thing now, without taking the lock: the box asks when it opens, so a
/// person finds out that somebody is editing before the first key (UX §6). The same rules as
/// *take the lock*: the person must be allowed to change the record.
///
/// # Errors
/// `resource.not_found` for a record the person may not know about, `access.forbidden` when they
/// may not change it.
pub async fn read_lock<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ReadLockQuery,
) -> Result<Authorized<LockHolder>, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let facts = ctx.facts_for_part(&actor, &input.what, Need::See).await?;
    let access = ctx.require(&actor, facts.edit, facts.place).await?;
    let held = match ctx.live_lock(&input.what).await? {
        Some(lock) => Some(ctx.held_lock(&access, &actor, &lock).await?),
        None => None,
    };
    Ok(access.authorize(LockHolder { held }))
}

/// Takes the lock on a thing, on the first keystroke, or says who is editing.
///
/// # Errors
/// `resource.not_found` for a record the person may not know about, `access.forbidden` when they
/// may not change it.
pub async fn take_lock<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: TakeLockQuery,
) -> Result<Authorized<LockOutcome>, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let facts = ctx
        .facts_for_part(&actor, &input.what, Need::Change)
        .await?;
    let access = ctx.require(&actor, facts.edit, facts.place).await?;
    let wanted = ctx.lock_for(&actor, &input.what, input.editor)?;
    let taking = ctx
        .store()
        .take_edit_lock(&wanted, ctx.now())
        .await
        .map_err(|error| internal::<S>(&error))?;
    let outcome = match taking {
        Taking::Granted(lock) => LockOutcome::Taken(Ctx::<S>::taken(&lock)),
        Taking::Held(lock) => LockOutcome::Held(ctx.held_lock(&access, &actor, &lock).await?),
    };
    Ok(access.authorize(outcome))
}

/// Renews the lease of a lock, keeps the text so far as the draft of the person, and hands over
/// the notes others left. The draft is kept whether or not the lock is still held: the text of
/// somebody whose lock ran out or was taken over is never dropped.
///
/// # Errors
/// `resource.not_found` for a record the person may not know about, `access.forbidden` when they
/// may not change it, `request.too_large` for a draft over `bb_core::edit::DRAFT_TEXT_MAX`.
pub async fn heartbeat<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: HeartbeatQuery,
) -> Result<Authorized<HeartbeatAnswer>, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let facts = ctx
        .facts_for_part(&actor, &input.what, Need::Change)
        .await?;
    let access = ctx.require(&actor, facts.edit, facts.place).await?;
    let now = ctx.now();
    let draft_saved = match input.draft {
        Some(text) => {
            ctx.keep_draft(
                actor.person.id,
                &DraftTarget::Editing(input.what.clone()),
                text,
            )
            .await?;
            true
        }
        None => false,
    };
    let ends = lease_ends(now)?;
    let renewed = ctx
        .store()
        .renew_edit_lock(
            &input.what,
            Hold {
                holder: actor.person.id,
                token: input.token,
            },
            now,
            ends,
        )
        .await
        .map_err(|error| internal::<S>(&error))?;
    let notices = ctx
        .store()
        .take_edit_notices(actor.person.id, &input.what)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let lock = if renewed {
        LockStatus::Held { expires_at: ends }
    } else {
        ctx.why_lost(&access, &input.what, &notices).await?
    };
    Ok(access.authorize(HeartbeatAnswer {
        lock,
        notices: ctx.notice_views(&access, &notices).await?,
        draft_saved,
    }))
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Why an editor does not hold the lock any more: somebody holds it now, or the notes say they
    /// took it over, or it simply ran out.
    pub(super) async fn why_lost(
        &self,
        viewer: &Evaluation,
        what: &EditWhat,
        notices: &[EditNotice],
    ) -> Result<LockStatus, AppError> {
        if let Some(lock) = self.live_lock(what).await? {
            return Ok(LockStatus::TakenOver {
                by: self.editor_ref(viewer, lock.holder).await?,
            });
        }
        if let Some(notice) = notices
            .iter()
            .find(|notice| notice.kind == NoticeKind::TakenOver)
        {
            return Ok(LockStatus::TakenOver {
                by: self.editor_ref(viewer, notice.from).await?,
            });
        }
        Ok(LockStatus::RanOut)
    }
}

/// Lets go of a lock: the text was saved, or the editor was closed. A token that holds nothing any
/// more is no error: the lock is gone either way.
///
/// # Errors
/// Only that the session is not one.
pub async fn release_lock<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ReleaseLockQuery,
) -> Result<Done, AppError> {
    // A lock belongs to an editor, an open tab of a signed-in person, so an access key has none
    // (ADR-0019); and it is let go of only with the token the lock was taken with and by the
    // person who holds it: nobody can release another editor's lock without both.
    let person = ctx.active(&input.session_token).await?.person.id;
    ctx.store()
        .release_edit_lock(
            &input.what,
            Hold {
                holder: person,
                token: input.token,
            },
        )
        .await
        .map_err(|error| internal::<S>(&error))?;
    Ok(Done)
}

/// Asks the person who is editing to finish. They read it at their next heartbeat.
///
/// # Errors
/// `resource.not_found` for a record the person may not know about or when nobody is editing,
/// `access.forbidden` when they may not change the record, `request.invalid` when they are the
/// one editing.
pub async fn ask_to_finish<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AskToFinish,
) -> Result<Authorized<Done>, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let facts = ctx.facts_for_part(&actor, &input.what, Need::See).await?;
    let access = ctx.require(&actor, facts.edit, facts.place).await?;
    let holder = ctx
        .live_lock(&input.what)
        .await?
        .ok_or(AppError::Refused(ErrorCode::NotFound))?
        .holder;
    if holder == actor.person.id {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let notice = EditNotice {
        recipient: holder,
        record: input.what.record.clone(),
        part: input.what.part.clone(),
        kind: NoticeKind::FinishAsked,
        from: actor.person.id,
        at: ctx.now(),
    };
    let event = ctx.event(
        by(actor.person.id),
        input.what.record.clone(),
        vec![FieldChange {
            field: EVENT_FIELD.to_owned(),
            before: None,
            after: Some(json!({ "asked_to_finish": input.what.part.as_str() })),
        }],
    )?;
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move { S::put_edit_notice(transaction, &notice).await })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    Ok(access.authorize(Done))
}

/// Takes the editing over (*Edit anyway*). The person who was editing keeps their text as their
/// draft and reads at their next heartbeat that the editing was taken over.
///
/// # Errors
/// `resource.not_found` for a record the person may not know about, `access.forbidden` without
/// *Edit anyway*.
pub async fn edit_anyway<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: EditAnyway,
) -> Result<Authorized<LockOutcome>, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let facts = ctx
        .facts_for_part(&actor, &input.what, Need::Change)
        .await?;
    // *Edit anyway* is the right to take over, and only for a person who may change the record: a
    // lock they could not keep would run out in a minute (M3-F35).
    ctx.require(&actor, facts.edit, facts.place).await?;
    let access = ctx
        .require(&actor, Permission::EditAnyway, facts.place)
        .await?;
    let wanted = ctx.lock_for(&actor, &input.what, input.editor)?;
    let now = wanted.taken_at;
    let me = actor.person.id;
    let event = ctx.event(
        by(me),
        input.what.record.clone(),
        vec![FieldChange {
            field: EVENT_FIELD.to_owned(),
            before: None,
            after: Some(json!({ "taken_over": input.what.part.as_str() })),
        }],
    )?;
    let lock = Arc::new(wanted.clone());
    ctx.store()
        .record_change(&event, {
            let lock = Arc::clone(&lock);
            move |transaction| {
                Box::pin(async move {
                    let former = S::replace_edit_lock(transaction, &lock).await?;
                    // Somebody else's lease that was still running is told; an expired one, or
                    // one of the person's own, is not.
                    if let Some(former) =
                        former.filter(|former| !former.has_expired(now) && former.holder != me)
                    {
                        let notice = EditNotice {
                            recipient: former.holder,
                            record: former.record,
                            part: former.part,
                            kind: NoticeKind::TakenOver,
                            from: me,
                            at: now,
                        };
                        S::put_edit_notice(transaction, &notice).await?;
                    }
                    Ok(())
                })
            }
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    Ok(access.authorize(LockOutcome::Taken(Ctx::<S>::taken(&wanted))))
}

/// Releases a lock that is stuck, whoever holds it. It is written to the security log, because an
/// administrator ends somebody else's editing.
///
/// # Errors
/// `resource.not_found` for a record the person may not know about or a thing nobody is editing,
/// `access.forbidden` without *Edit anyway*.
pub async fn release_stuck_lock<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ReleaseStuckLock,
) -> Result<Authorized<Done>, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let facts = ctx
        .facts_for_part(&actor, &input.what, Need::Change)
        .await?;
    // Like *Edit anyway*: ending somebody's editing is for a person who may change the record.
    ctx.require(&actor, facts.edit, facts.place).await?;
    let access = ctx
        .require(&actor, Permission::EditAnyway, facts.place)
        .await?;
    let event = ctx.event(
        by(actor.person.id),
        input.what.record.clone(),
        vec![FieldChange {
            field: EVENT_FIELD.to_owned(),
            before: Some(json!(input.what.part.as_str())),
            after: None,
        }],
    )?;
    let what = input.what.clone();
    let (me, now) = (actor.person.id, ctx.now());
    let stopped = crate::projects::Why::default();
    let stop = stopped.clone();
    // Whether somebody else's lease that was still running ended: only then an administrator
    // ended somebody's editing, which is what the security log is for.
    let ended = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let ended_in = Arc::clone(&ended);
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                let Some(former) = S::remove_edit_lock(transaction, &what).await? else {
                    return Err(stop.stop::<S>(ErrorCode::NotFound));
                };
                if !former.has_expired(now) && former.holder != me {
                    let notice = EditNotice {
                        recipient: former.holder,
                        record: former.record,
                        part: former.part,
                        kind: NoticeKind::Released,
                        from: me,
                        at: now,
                    };
                    S::put_edit_notice(transaction, &notice).await?;
                    ended_in.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                Ok(())
            })
        })
        .await
        .map_err(|error| stopped.answer::<S>(&error, ErrorCode::NotFound))?;
    if ended.load(std::sync::atomic::Ordering::Relaxed) {
        ctx.note(
            SecurityAction::EditLockReleased,
            Some(actor.person.id),
            Some(input.what.record.clone()),
        )
        .await;
    }
    Ok(access.authorize(Done))
}
