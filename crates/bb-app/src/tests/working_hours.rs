//! The rules of working hours on the in-memory store (TIME-04, TIME-08, ADR-0035, decision 4): the
//! comparison of one's own week with the time booked on issues reads the port of time spent, here
//! its fake, and nobody gets it for another person.

use bb_core::time::day::date_of;
use jiff::tz::TimeZone;

use super::scenarios::token;
use super::scenarios_working_hours::{started, typed};
use super::world::{Outcome, World};
use crate::context::Clock as _;
use crate::working_hours::{BookedWeekQuery, booked_week};

#[tokio::test]
async fn time_04_the_week_compares_the_working_hours_with_the_time_booked_on_issues() -> Outcome {
    let world = World::new().await?;
    let today = date_of(world.clock.now(), &TimeZone::UTC);
    let yesterday = today.yesterday()?;
    typed(&world, yesterday).await?;
    started(&world).await?;
    let person = world.admin.person.id;
    world.store.book(person, yesterday, 300);
    world.store.book(person, yesterday, 30);
    // Time booked by somebody else is not in it.
    world
        .store
        .book(bb_core::id::UserId::from_random([9; 16]), yesterday, 600);
    let week = booked_week(
        &world.ctx(),
        BookedWeekQuery {
            session_token: token(&world),
            date: today,
        },
    )
    .await?
    .into_value();
    assert_eq!(week.days.len(), 7);
    let day = week
        .days
        .iter()
        .find(|day| day.date == yesterday)
        .ok_or("no row of yesterday")?;
    assert_eq!(day.worked.get(), 480);
    assert_eq!(day.booked.get(), 330);
    assert_eq!(day.not_booked, 150);
    let free = week
        .days
        .iter()
        .find(|day| day.date != yesterday && day.date != today)
        .ok_or("no other day")?;
    assert_eq!(
        (free.worked.get(), free.booked.get(), free.not_booked),
        (0, 0, 0)
    );
    Ok(())
}
