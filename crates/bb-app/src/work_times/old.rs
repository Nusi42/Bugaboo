//! The daily job that deletes away times one year after their last day (PRIV-05, ADR-0030,
//! amendment M8; ADR-0017, amendment M8). It is the only deletion job of work times: shares and work
//! hours are deleted only when the person leaves (`leave.rs`), because business data is deleted only
//! explicitly.
//!
//! The job deletes each away time with its note, the events of its changes with their outbox
//! entries and the notifications about it. The event it writes of its own names no person: only how
//! many away times went and from which date on they were old, so the job leaves nothing about a
//! person behind.

use bb_core::event::{Actor, RecordKind};
use bb_core::time::day::date_of;
use bb_core::wording::Region;
use bb_core::work_times::away_cutoff;
use serde_json::json;

use crate::auth::{AppError, AuthStore, Ctx, Stops, change_failed, fixed_entity, set_to};
use crate::member::work_times as member;

/// The name of the one record the events of this job are about: a record of its own, with no
/// person in it.
const JOB_RECORD: &str = "away-times-job";

/// Deletes the away times whose last day is more than one year before today, and returns how many
/// went. Idempotent: a second run on the same day finds nothing.
///
/// # Errors
/// An internal error when the store fails.
pub async fn delete_old_away_times<S: AuthStore>(ctx: &Ctx<'_, S>) -> Result<u64, AppError> {
    let layers = ctx.setting_layers_of(None).await?;
    let today = date_of(ctx.now(), Region::of(&layers).zone());
    let Some(before) = away_cutoff(today) else {
        return Ok(0);
    };
    let maker = ctx.event_maker(Actor::System);
    let record = fixed_entity(RecordKind::WorkTimes, JOB_RECORD)?;
    let deleted = ctx
        .store()
        .record_changes(&[], move |transaction| {
            Box::pin(async move {
                let deleted = S::delete_old_away_times(transaction, before).await?;
                let total: u64 = deleted.iter().map(|(_, count)| *count).sum();
                if total > 0 {
                    let event = maker
                        .create(
                            record,
                            vec![
                                set_to(member::DELETED, json!(total)),
                                set_to(member::BEFORE, json!(before.to_string())),
                            ],
                        )
                        .map_err(|_| S::aborted())?;
                    S::append_follow_up_event(transaction, &event).await?;
                }
                Ok(total)
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    Ok(deleted)
}
