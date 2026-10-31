//! The work days older than the keep period (PRIV-09, ADR-0035, decision 7): counted when the
//! settings page asks, and deleted only when an administrator says so. No job counts or deletes
//! them. Deleting a day deletes its periods, its breaks and the events of its changes (with the old
//! values and the reasons), and every deletion is written to the security log with whose days and
//! how many (an event for each person holds the count).

use bb_core::ErrorCode;
use bb_core::event::RecordKind;
use bb_core::people_kinds::PeopleKind;
use bb_core::settings::Layers;
use bb_core::sign_in::SecurityAction;
use bb_core::time::day::date_of;
use bb_core::wording::Region;
use bb_core::working_time::keep_years;
use bb_policy::{Authorized, PeopleContext, Permission, PersonRef, Place};
use jiff::civil::Date;
use jiff::{Span, Timestamp};
use serde_json::json;

use super::OLD_PEOPLE_MAX;
use super::store::OldDays;
use super::types::{
    DeleteOldWorkDays, DeletedWorkDays, OldOfPerson, OldWorkDays, OldWorkDaysQuery,
};
use crate::auth::{AppError, AuthStore, Ctx, Stops, by, change_failed, entity, internal, set_to};
use crate::member::work_day as member;

/// The first date that is still inside the keep period: a day with an earlier date is older than
/// it. Counted in whole years from today in the organization's zone.
fn kept_from(layers: &Layers, now: Timestamp) -> Result<Date, AppError> {
    let today = date_of(now, Region::of(layers).zone());
    today
        .checked_sub(Span::new().years(keep_years(layers)))
        .map_err(|_| ErrorCode::InvalidRequest.into())
}

/// How many work days are older than the keep period, for each person and in total.
///
/// # Errors
/// `access.forbidden` without *Change organization settings*.
pub async fn old_work_days<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: OldWorkDaysQuery,
) -> Result<Authorized<OldWorkDays>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let evaluation = ctx
        .require(
            &actor,
            Permission::ChangeOrganizationSettings,
            Place::Organization,
        )
        .await?;
    let layers = ctx.setting_layers_of(None).await?;
    let before = kept_from(&layers, ctx.now())?;
    let counts = ctx
        .store()
        .old_work_days(before)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let ids: Vec<_> = counts.iter().map(|old| old.person).collect();
    ctx.preload_people(&ids).await?;
    let mut people = Vec::with_capacity(counts.len());
    for OldDays { person, count } in &counts {
        let shown = ctx
            .person_ref(
                &evaluation,
                PeopleKind::TimeEntries,
                PeopleContext::Management,
                *person,
            )
            .await?;
        let id = matches!(shown, PersonRef::Name { .. }).then_some(*person);
        people.push(OldOfPerson {
            person: shown,
            id,
            count: *count,
        });
    }
    // In the order of their names; people not shown by name after them.
    people.sort_by_key(|old| name_of(&old.person));
    let truncated = people.len() > OLD_PEOPLE_MAX;
    people.truncate(OLD_PEOPLE_MAX);
    Ok(evaluation.authorize(OldWorkDays {
        keep_years: keep_years(&layers),
        before,
        total: counts.iter().map(|old| old.count).sum(),
        people,
        truncated,
    }))
}

/// The name a person is sorted by: none sorts last.
fn name_of(person: &PersonRef) -> (bool, String) {
    match person {
        PersonRef::Name { name, .. } => (false, name.to_lowercase()),
        _ => (true, String::new()),
    }
}

/// *Delete them*: deletes the work days older than the keep period, of everybody or of one person,
/// with their periods, breaks and the events of their changes, and writes each person's deletion
/// to the security log.
///
/// # Errors
/// `access.forbidden` without *Change organization settings*.
pub async fn delete_old_work_days<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: DeleteOldWorkDays,
) -> Result<Authorized<DeletedWorkDays>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let evaluation = ctx
        .require(
            &actor,
            Permission::ChangeOrganizationSettings,
            Place::Organization,
        )
        .await?;
    let layers = ctx.setting_layers_of(None).await?;
    let before = kept_from(&layers, ctx.now())?;
    let maker = ctx.event_maker(by(actor.person.id));
    let person = input.person;
    let deleted = ctx
        .store()
        .record_changes(&[], move |transaction| {
            Box::pin(async move {
                let deleted = S::delete_old_work_days(transaction, before, person).await?;
                for old in &deleted {
                    let event = maker
                        .create(
                            entity(RecordKind::Person, old.person),
                            vec![
                                set_to(member::DELETED, json!(old.count)),
                                set_to(member::BEFORE, json!(before.to_string())),
                            ],
                        )
                        .map_err(|_| S::aborted())?;
                    S::append_follow_up_event(transaction, &event).await?;
                }
                Ok(deleted)
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    for old in &deleted {
        ctx.note(
            SecurityAction::WorkDaysDeleted,
            Some(actor.person.id),
            Some(entity(RecordKind::Person, old.person)),
        )
        .await;
    }
    Ok(evaluation.authorize(DeletedWorkDays {
        deleted: deleted.iter().map(|old| old.count).sum(),
    }))
}
