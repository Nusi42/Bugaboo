//! Changing the usual work days and work hours of a person (PLAN-11, PLAN-15, ADR-0030): one's own,
//! or another person's with *Enter work times for others*, who is then told (the reader of the
//! outbox, `notifications`). The seven days have a version like any record two people may change:
//! a change made on a version that is not the current one is the structured conflict (ADR-0019).

use bb_core::ErrorCode;
use bb_core::event::RecordKind;
use bb_core::id::UserId;
use bb_core::work_times::UsualDays;
use bb_policy::{Authorized, Place};

use super::read::usual_of;
use super::types::{ChangeUsualDays, ChangedUsualDays};
use crate::auth::{AppError, AuthStore, Ctx, Stops, by, change_failed, entity, internal};
use crate::event_members::usual_days_recorded;
use crate::projects::Why;

/// The answer to a change made on a version the usual work days no longer have.
fn stale(viewer: UserId, person: UserId) -> AppError {
    AppError::stale(
        entity(RecordKind::WorkTimes, person),
        Place::Organization,
        viewer,
    )
}

/// New usual work days and work hours: the seven days, each ticked or not, with the work hours of
/// each, on the version the reader looked at. A change to what the person has already writes
/// nothing and tells nobody.
///
/// # Errors
/// `work-times.hours-invalid` for a day of more than 24 hours, the structured conflict for a version
/// the days no longer have, and the refusals of `work_times_to_change`.
pub async fn change_usual_days<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangeUsualDays,
) -> Result<Authorized<ChangedUsualDays>, AppError> {
    let actor = ctx.work_times_actor(&input.session_token).await?;
    let whose = ctx.work_times_to_change(&actor, input.person).await?;
    let after = UsualDays::new(input.days).ok_or(ErrorCode::WorkHoursInvalid)?;
    let person = whose.person;
    let before = ctx
        .store()
        .usual_days(person)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let version = before.map_or(0, |stored| stored.version);
    if version != input.version {
        return Err(stale(actor.person.id, person));
    }
    if before.map_or_else(UsualDays::start, |stored| stored.days) == after {
        return Ok(whose.evaluation.authorize(ChangedUsualDays {
            usual: usual_of(&after),
            version,
        }));
    }
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::WorkTimes, person),
        usual_days_recorded(
            person,
            before.as_ref().map(|stored| &stored.days),
            &after,
            version,
        ),
    )?;
    let why = Why::default();
    let stop = why.clone();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if S::put_usual_days(transaction, person, &after, version).await? {
                    Ok(())
                } else {
                    Err(stop.stop::<S>(ErrorCode::VersionChanged))
                }
            })
        })
        .await
        .map_err(|error| {
            if why.said() == Some(ErrorCode::VersionChanged) {
                stale(actor.person.id, person)
            } else {
                change_failed::<S>(&error, Stops::NONE)
            }
        })?;
    Ok(whose.evaluation.authorize(ChangedUsualDays {
        usual: usual_of(&after),
        version: version + 1,
    }))
}
