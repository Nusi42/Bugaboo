//! The scenarios of the use cases of working hours (ADR-0007, M1-F18, TIME-08): the preparation,
//! the call, and how many events the call added. `events.rs` names each use case once and runs
//! them; `working_hours.rs` uses the same preparation for the rules.

use bb_core::working_time::Span;
use jiff::Timestamp;
use jiff::civil::Date;

use super::scenarios::{Added, measured, text, token};
use super::world::World;
use crate::auth::AppError;
use crate::working_hours::{
    ChangeWorkDay, ChangedWorkDay, DeleteOldWorkDays, StartWorkDay, Tap, TapWorkDay,
    change_work_day, delete_old_work_days, start_work_day, tap_work_day,
};

/// The administrator starts their work day; the clock moves on first, past the limit of one
/// change a second.
pub(crate) async fn started(world: &World) -> Result<ChangedWorkDay, AppError> {
    world.clock.advance(1);
    Ok(start_work_day(
        &world.ctx(),
        StartWorkDay {
            session_token: token(world),
            device_zone: None,
        },
    )
    .await?
    .into_value())
}

/// A day of the administrator on `date`, from 08:00 to 16:00 UTC, typed afterwards with a reason.
pub(crate) async fn typed(world: &World, date: Date) -> Result<ChangedWorkDay, AppError> {
    world.clock.advance(1);
    let at = |hour: i8| -> Result<Timestamp, AppError> {
        date.at(hour, 0, 0, 0)
            .to_zoned(jiff::tz::TimeZone::UTC)
            .map(|zoned| zoned.timestamp())
            .map_err(|error| AppError::Internal(error.to_string()))
    };
    Ok(change_work_day(
        &world.ctx(),
        ChangeWorkDay {
            session_token: token(world),
            person: None,
            date,
            version: 0,
            periods: vec![Span::closed(at(8)?, at(16)?)],
            breaks: Vec::new(),
            reason: Some("Typed in afterwards".to_owned()),
            device_zone: None,
        },
    )
    .await?
    .into_value())
}

pub(crate) async fn start_work_day_scenario(world: &World) -> Added {
    measured(&world.store, started(world)).await
}

/// A tap on the day that runs, a minute later.
async fn tapped(world: &World, version: i64, tap: Tap) -> Result<ChangedWorkDay, AppError> {
    world.clock.advance(60);
    Ok(tap_work_day(
        &world.ctx(),
        TapWorkDay {
            session_token: token(world),
            version,
            tap,
        },
    )
    .await?
    .into_value())
}

pub(crate) async fn tap_work_day_scenario(world: &World) -> Added {
    let day = started(world).await.map_err(text)?;
    let on_break = tapped(world, day.day.version, Tap::StartBreak)
        .await
        .map_err(text)?;
    let back = tapped(world, on_break.day.version, Tap::EndBreak)
        .await
        .map_err(text)?;
    measured(&world.store, tapped(world, back.day.version, Tap::End)).await
}

pub(crate) async fn change_work_day_scenario(world: &World) -> Added {
    let date = Date::new(2026, 12, 1).map_err(text)?;
    measured(&world.store, typed(world, date)).await
}

pub(crate) async fn delete_old_work_days_scenario(world: &World) -> Added {
    let old = Date::new(2020, 3, 2).map_err(text)?;
    typed(world, old).await.map_err(text)?;
    measured(
        &world.store,
        delete_old_work_days(
            &world.ctx(),
            DeleteOldWorkDays {
                session_token: token(world),
                person: None,
            },
        ),
    )
    .await
}
