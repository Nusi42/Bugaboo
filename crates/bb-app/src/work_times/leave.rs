//! All work times of a person who leaves the organization (PRIV-05, ADR-0030, amendment M8): their
//! usual work days and work hours, their shares, their away times with the notes and everything
//! that tells about them, and their choice of *Holidays*. *Remove a person* (M9) calls
//! [`delete_work_times_in`] inside the change that ends the membership; [`delete_work_times_of_person`]
//! is the same as a use case of its own, which M8 builds and tests alone.

use bb_core::event::{Actor, RecordKind};
use bb_core::id::UserId;
use bb_core::settings::Scope;
use bb_core::settings::work_times::WORK_TIMES_HOLIDAYS;

use crate::auth::{AppError, AuthStore, Ctx, Stops, change_failed, entity, set_to};
use crate::member::work_times as member;

/// Delete all work times of a person.
pub struct DeleteWorkTimesOfPerson {
    /// The person who leaves.
    pub person: UserId,
}

/// Deletes everything of work times that belongs to `person`, inside a change that is open: the
/// one function of the module that the removal of a person calls. Idempotent.
///
/// # Errors
/// The error of the store.
pub async fn delete_work_times_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    person: UserId,
) -> Result<(), S::Error> {
    S::delete_work_times_of(transaction, person).await?;
    S::clear_setting_value(
        transaction,
        Scope::Person(person),
        WORK_TIMES_HOLIDAYS.definition(),
    )
    .await
    .map(|_| ())
}

/// Deletes all work times of a person who leaves, as a change of its own.
///
/// # Errors
/// An internal error when the store fails.
pub async fn delete_work_times_of_person<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: DeleteWorkTimesOfPerson,
) -> Result<(), AppError> {
    let person = input.person;
    let event = ctx
        .event_maker(Actor::System)
        .create(
            entity(RecordKind::Person, person),
            vec![set_to(member::DELETED, serde_json::json!(true))],
        )
        .map_err(|error| AppError::Internal(error.to_string()))?;
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move { delete_work_times_in::<S>(transaction, person).await })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))
}
