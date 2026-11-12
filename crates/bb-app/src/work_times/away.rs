//! Entering, changing and removing away times (PLAN-11, ADR-0030, decision 2): one's own, or
//! another person's with *Enter work times for others*. Bugaboo stores no reason. The note is read
//! and written by its person only: whoever enters or changes the days of another person's away
//! time never sees or writes it, and it is no part of an event or a notification. The person is
//! told when somebody else enters, changes or removes one of their away times (the reader of the
//! outbox, `notifications`).

use bb_core::ErrorCode;
use bb_core::event::RecordKind;
use bb_core::id::{AwayTimeId, UserId};
use bb_core::version::{FIRST_VERSION, next};
use bb_core::work_times::NOTE_CHARS_MAX;
use bb_core::work_times::record::AwayTime;
use bb_policy::{Authorized, Place};

use super::types::{AwayDays, ChangeAwayTime, ChangedAwayTime, EnterAwayTime, RemoveAwayTime};
use super::whose::Whose;
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, Done, Stops, by, change_failed, entity, internal};
use crate::event_members::away_time_recorded;
use crate::input::clean_text;
use crate::projects::Why;

/// The note as it is kept: none when empty. Only the person writes a note; for another person's
/// away time a note is refused, never silently dropped.
fn note_of(whose: &Whose, note: Option<&str>) -> Result<Option<String>, AppError> {
    match note.map(str::trim) {
        None | Some("") => Ok(None),
        Some(_) if !whose.own => Err(ErrorCode::Forbidden.into()),
        Some(text) => clean_text(text, NOTE_CHARS_MAX).map(Some),
    }
}

/// The answer to a change made on a version the away time no longer has: the structured conflict,
/// which `AuthApi` turns into the answer with the version it has now.
fn stale(viewer: UserId, id: AwayTimeId) -> AppError {
    AppError::stale(
        entity(RecordKind::AwayTime, id),
        Place::Organization,
        viewer,
    )
}

/// What a change that did not go through answers: the conflict when the away time changed first,
/// else what the store says.
fn lost<S: AuthStore>(why: &Why, error: &S::Error, viewer: UserId, id: AwayTimeId) -> AppError {
    if why.said() == Some(ErrorCode::VersionChanged) {
        stale(viewer, id)
    } else {
        change_failed::<S>(error, Stops::NONE)
    }
}

/// The days of a request, checked.
fn checked(days: &AwayDays) -> Result<(), AppError> {
    AwayTime::check(
        days.first,
        days.last,
        days.first_afternoon_only,
        days.last_morning_only,
    )
    .map_err(|_| ErrorCode::AwayTimeInvalid.into())
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The stored away time, as an actor who may change it, with whose it is.
    async fn away_time_to_change(
        &self,
        actor: &Actor,
        id: AwayTimeId,
    ) -> Result<(AwayTime, Whose), AppError> {
        let time = self
            .store()
            .away_time(id)
            .await
            .map_err(|error| internal::<S>(&error))?
            .ok_or(ErrorCode::NotFound)?;
        let whose = self.work_times_to_change(actor, Some(time.person)).await?;
        Ok((time, whose))
    }
}

/// Enters an away time for the reader, or for another person with *Enter work times for others*.
///
/// # Errors
/// `work-times.away-time-invalid` for days that are not an away time, `access.forbidden` for a note
/// on another person's away time, and the refusals of `work_times_to_change`.
pub async fn enter_away_time<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: EnterAwayTime,
) -> Result<Authorized<ChangedAwayTime>, AppError> {
    let actor = ctx.work_times_actor(&input.session_token).await?;
    let whose = ctx.work_times_to_change(&actor, input.person).await?;
    checked(&input.days)?;
    let note = note_of(&whose, input.days.note.as_deref())?;
    let id = AwayTimeId::generate(&mut ctx.ids())
        .map_err(|error| AppError::Internal(error.to_string()))?;
    let time = AwayTime {
        id,
        person: whose.person,
        first: input.days.first,
        last: input.days.last,
        first_afternoon_only: input.days.first_afternoon_only,
        last_morning_only: input.days.last_morning_only,
        note,
        entered_by: actor.person.id,
        version: FIRST_VERSION,
    };
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::AwayTime, id),
        away_time_recorded(None, Some(&time)),
    )?;
    let stored = time.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move { S::insert_away_time(transaction, &stored).await })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    let away = ctx.away_view(&whose, &time).await?;
    Ok(whose.evaluation.authorize(ChangedAwayTime { away }))
}

/// Changes the days of an away time, and the note when it is the person's own.
///
/// # Errors
/// `work-times.away-time-invalid`, `access.forbidden` for a note on another person's away time,
/// `edit.version-changed` when the away time was changed meanwhile, `resource.not-found` for an
/// away time that is not there, and the refusals of `work_times_to_change`.
pub async fn change_away_time<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangeAwayTime,
) -> Result<Authorized<ChangedAwayTime>, AppError> {
    let actor = ctx.work_times_actor(&input.session_token).await?;
    let (before, whose) = ctx.away_time_to_change(&actor, input.id).await?;
    checked(&input.days)?;
    if before.version != input.version {
        return Err(stale(actor.person.id, before.id));
    }
    // Somebody who enters work times for another person never reads or writes the note: it stays
    // as it is.
    let note = if whose.own {
        note_of(&whose, input.days.note.as_deref())?
    } else {
        note_of(&whose, input.days.note.as_deref())?;
        before.note.clone()
    };
    let after = AwayTime {
        first: input.days.first,
        last: input.days.last,
        first_afternoon_only: input.days.first_afternoon_only,
        last_morning_only: input.days.last_morning_only,
        note,
        version: next(before.version),
        ..before.clone()
    };
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::AwayTime, after.id),
        away_time_recorded(Some(&before), Some(&after)),
    )?;
    let (stored, version, why) = (after.clone(), before.version, Why::default());
    let stop = why.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if S::update_away_time(transaction, &stored, version).await? {
                    Ok(())
                } else {
                    Err(stop.stop::<S>(ErrorCode::VersionChanged))
                }
            })
        })
        .await
        .map_err(|error| lost::<S>(&why, &error, actor.person.id, after.id))?;
    let away = ctx.away_view(&whose, &after).await?;
    Ok(whose.evaluation.authorize(ChangedAwayTime { away }))
}

/// Removes an away time.
///
/// # Errors
/// `edit.version-changed` when the away time was changed meanwhile, `resource.not-found` for an away
/// time that is not there, and the refusals of `work_times_to_change`.
pub async fn remove_away_time<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: RemoveAwayTime,
) -> Result<Done, AppError> {
    let actor = ctx.work_times_actor(&input.session_token).await?;
    let (before, _) = ctx.away_time_to_change(&actor, input.id).await?;
    if before.version != input.version {
        return Err(stale(actor.person.id, before.id));
    }
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::AwayTime, before.id),
        away_time_recorded(Some(&before), None),
    )?;
    let (id, version, why) = (before.id, before.version, Why::default());
    let stop = why.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if S::delete_away_time(transaction, id, version).await? {
                    Ok(())
                } else {
                    Err(stop.stop::<S>(ErrorCode::VersionChanged))
                }
            })
        })
        .await
        .map_err(|error| lost::<S>(&why, &error, actor.person.id, before.id))?;
    Ok(Done)
}
