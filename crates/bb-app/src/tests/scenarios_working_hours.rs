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
    ChangeWorkDay, ChangedWorkDay, DeleteOldWorkDays, EndBreak, EndWorkDay, StartBreak,
    StartWorkDay, change_work_day, delete_old_work_days, end_break, end_work_day, start_break,
    start_work_day,
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
        },
    )
    .await?
    .into_value())
}

pub(crate) async fn start_work_day_scenario(world: &World) -> Added {
    measured(&world.store, started(world)).await
}

pub(crate) async fn start_break_scenario(world: &World) -> Added {
    let day = started(world).await.map_err(text)?;
    world.clock.advance(60);
    measured(
        &world.store,
        start_break(
            &world.ctx(),
            StartBreak {
                session_token: token(world),
                version: day.day.version,
            },
        ),
    )
    .await
}

pub(crate) async fn end_break_scenario(world: &World) -> Added {
    let day = started(world).await.map_err(text)?;
    world.clock.advance(60);
    let on_break = start_break(
        &world.ctx(),
        StartBreak {
            session_token: token(world),
            version: day.day.version,
        },
    )
    .await
    .map_err(text)?
    .into_value();
    world.clock.advance(60);
    measured(
        &world.store,
        end_break(
            &world.ctx(),
            EndBreak {
                session_token: token(world),
                version: on_break.day.version,
            },
        ),
    )
    .await
}

pub(crate) async fn end_work_day_scenario(world: &World) -> Added {
    let day = started(world).await.map_err(text)?;
    world.clock.advance(60);
    measured(
        &world.store,
        end_work_day(
            &world.ctx(),
            EndWorkDay {
                session_token: token(world),
                version: day.day.version,
            },
        ),
    )
    .await
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
