//! The scenarios of the use cases of work times (ADR-0007, M1-F18, PLAN-11, PLAN-15): the
//! preparation, the call, and how many events the call added. `events.rs` names each use case once
//! and runs them; `work_times.rs` uses the same preparation for the rules.

use bb_core::id::{AwayTimeId, UserId};
use bb_core::work_times::UsualDay;
use bb_policy::StartingRole;
use jiff::civil::Date;

use super::scenarios::{Added, a_project, measured, text, token};
use super::scenarios_searches::with_role;
use super::world::World;
use crate::auth::AppError;
use crate::work_times::{
    AwayDays, ChangeAwayTime, ChangeUsualDays, DeleteWorkTimesOfPerson, EnterAwayTime,
    RemoveAwayTime, SetShare, change_away_time, change_usual_days, delete_old_away_times,
    delete_work_times_of_person, enter_away_time, remove_away_time, set_share,
};

/// A date.
pub(crate) fn date(year: i16, month: i8, day: i8) -> Date {
    Date::new(year, month, day).unwrap_or_default()
}

/// The days of an away time with no half days and no note.
pub(crate) fn days(first: Date, last: Date) -> AwayDays {
    AwayDays {
        first,
        last,
        first_afternoon_only: false,
        last_morning_only: false,
        note: None,
    }
}

/// An away time of the administrator, or of `person` when somebody who may enter work times for
/// others (the administrator) enters it.
pub(crate) async fn entered(
    world: &World,
    person: Option<UserId>,
    away: AwayDays,
) -> Result<AwayTimeId, AppError> {
    Ok(enter_away_time(
        &world.ctx(),
        EnterAwayTime {
            session_token: token(world),
            person,
            days: away,
        },
    )
    .await?
    .into_value()
    .away
    .id)
}

/// Seven days with the same work hours, the first `works` days ticked.
pub(crate) fn week(works: usize, minutes: u16) -> [UsualDay; 7] {
    std::array::from_fn(|at| UsualDay {
        works: at < works,
        minutes,
    })
}

pub(crate) async fn change_usual_days_scenario(world: &World) -> Added {
    measured(
        &world.store,
        change_usual_days(
            &world.ctx(),
            ChangeUsualDays {
                session_token: token(world),
                person: None,
                days: week(4, 360),
            },
        ),
    )
    .await
}

pub(crate) async fn enter_away_time_scenario(world: &World) -> Added {
    measured(
        &world.store,
        entered(world, None, days(date(2027, 2, 1), date(2027, 2, 5))),
    )
    .await
}

pub(crate) async fn change_away_time_scenario(world: &World) -> Added {
    let id = entered(world, None, days(date(2027, 2, 1), date(2027, 2, 5)))
        .await
        .map_err(text)?;
    measured(
        &world.store,
        change_away_time(
            &world.ctx(),
            ChangeAwayTime {
                session_token: token(world),
                id,
                version: 1,
                days: days(date(2027, 2, 1), date(2027, 2, 8)),
            },
        ),
    )
    .await
}

pub(crate) async fn remove_away_time_scenario(world: &World) -> Added {
    let id = entered(world, None, days(date(2027, 2, 1), date(2027, 2, 5)))
        .await
        .map_err(text)?;
    measured(
        &world.store,
        remove_away_time(
            &world.ctx(),
            RemoveAwayTime {
                session_token: token(world),
                id,
                version: 1,
            },
        ),
    )
    .await
}

pub(crate) async fn set_share_scenario(world: &World) -> Added {
    let project = a_project(world, "WEB").await?;
    let bob = world.person("bob@example.test").await.map_err(text)?;
    with_role(world, &bob, StartingRole::TeamMember, Some(project)).await?;
    measured(
        &world.store,
        set_share(
            &world.ctx(),
            SetShare {
                session_token: token(world),
                project: project.to_string(),
                person: bob.person.id,
                percent: 60,
            },
        ),
    )
    .await
}

pub(crate) async fn delete_old_away_times_scenario(world: &World) -> Added {
    entered(world, None, days(date(2020, 3, 2), date(2020, 3, 6)))
        .await
        .map_err(text)?;
    let before = world.store.event_count();
    let deleted = delete_old_away_times(&world.ctx()).await.map_err(text)?;
    if deleted != 1 {
        return Err(format!("{deleted} away times were deleted, not 1"));
    }
    Ok(world.store.event_count() - before)
}

pub(crate) async fn delete_work_times_of_person_scenario(world: &World) -> Added {
    let bob = world.person("bob@example.test").await.map_err(text)?;
    entered(
        world,
        Some(bob.person.id),
        days(date(2027, 2, 1), date(2027, 2, 5)),
    )
    .await
    .map_err(text)?;
    measured(
        &world.store,
        delete_work_times_of_person(
            &world.ctx(),
            DeleteWorkTimesOfPerson {
                person: bob.person.id,
            },
        ),
    )
    .await
}
