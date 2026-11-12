//! Changing the usual work days and work hours of a person (PLAN-11, PLAN-15, ADR-0030): one's own,
//! or another person's with *Enter work times for others*, who is then told (the reader of the
//! outbox, `notifications`).

use bb_core::ErrorCode;
use bb_core::event::RecordKind;
use bb_core::work_times::UsualDays;
use bb_policy::Authorized;

use super::read::usual_of;
use super::types::{ChangeUsualDays, ChangedUsualDays};
use crate::auth::{AppError, AuthStore, Ctx, Stops, by, change_failed, entity, internal};
use crate::event_members::usual_days_recorded;

/// New usual work days and work hours: the seven days, each ticked or not, with the work hours of
/// each. A change to what the person has already writes nothing and tells nobody.
///
/// # Errors
/// `work-times.hours-invalid` for a day of more than 24 hours, and the refusals of
/// `work_times_to_change`.
pub async fn change_usual_days<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangeUsualDays,
) -> Result<Authorized<ChangedUsualDays>, AppError> {
    let actor = ctx
        .work_times_changer(&input.session_token, input.person)
        .await?;
    let whose = ctx.work_times_to_change(&actor, input.person).await?;
    let after = UsualDays::new(input.days).ok_or(ErrorCode::WorkHoursInvalid)?;
    let person = whose.person;
    let before = ctx
        .store()
        .usual_days(person)
        .await
        .map_err(|error| internal::<S>(&error))?;
    if before.unwrap_or_default() == after {
        return Ok(whose.evaluation.authorize(ChangedUsualDays {
            usual: usual_of(&after),
        }));
    }
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::WorkTimes, person),
        usual_days_recorded(person, before.as_ref(), &after),
    )?;
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move { S::put_usual_days(transaction, person, &after).await })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    Ok(whose.evaluation.authorize(ChangedUsualDays {
        usual: usual_of(&after),
    }))
}
