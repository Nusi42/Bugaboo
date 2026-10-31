//! The rules of working hours on the in-memory store (TIME-04, TIME-08, ADR-0035, decision 4): the
//! comparison of one's own week with the time booked on issues reads the port of time spent, here
//! its fake, and nobody gets it for another person.

use bb_core::time::day::date_of;
use jiff::tz::TimeZone;

use bb_core::technical::TechnicalName;
use bb_policy::StartingRole;

use super::scenarios::token;
use super::scenarios_working_hours::{started, typed};
use super::world::{Outcome, World};
use crate::access::{CreateStartingRoles, create_starting_roles};
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

#[tokio::test]
async fn sec_08_hr_is_created_once_on_a_server_whose_roles_were_made_before_it() -> Outcome {
    let world = World::new().await?;
    let hr = TechnicalName::new(StartingRole::Hr.code())?;
    let is_hr = |record: &crate::access::RoleRecord| record.role.starting.as_ref() == Some(&hr);
    // A server of before: its starting roles are made, HR is not, and the migration made HR due.
    {
        let mut shared = world.store.lock();
        shared.state.roles.retain(|record| !is_hr(record));
        shared.state.starting_roles_due.push(hr.clone());
    }
    let before = world.store.event_count();
    // Two servers that start at the same moment.
    let (first, second) = (world.ctx(), world.ctx());
    let (one, two) = tokio::join!(
        create_starting_roles(&first, CreateStartingRoles),
        create_starting_roles(&second, CreateStartingRoles)
    );
    one?;
    two?;
    let made = world
        .store
        .read(|state| state.roles.iter().filter(|record| is_hr(record)).count());
    assert_eq!(made, 1);
    assert!(
        world
            .store
            .read(|state| state.starting_roles_due.is_empty())
    );
    assert_eq!(
        world.store.event_count() - before,
        1,
        "one event, of the one role"
    );
    // Another start makes nothing, and writes no event.
    create_starting_roles(&world.ctx(), CreateStartingRoles).await?;
    assert_eq!(world.store.event_count() - before, 1);
    // A role that is there already when the row says it is due is not made twice.
    world.store.lock().state.starting_roles_due.push(hr.clone());
    create_starting_roles(&world.ctx(), CreateStartingRoles).await?;
    assert_eq!(world.store.event_count() - before, 1);
    assert!(
        world
            .store
            .read(|state| state.starting_roles_due.is_empty())
    );
    Ok(())
}
