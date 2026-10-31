//! The scenarios of the timer and of time entries for the table of events, and their helpers.

use bb_core::id::{IssueId, TimeEntryId};
use bb_core::time::Minutes;
use jiff::civil::date;

use super::scenarios::{Added, a_project, measured, text, token};
use super::scenarios_issues::an_issue;
use super::world::World;
use crate::time::{
    AddTime, ChangeStartTime, ChangeTimeEntry, CreateStarterTimeFields, DeleteTimeEntry,
    EntryDetails, StartTimer, StopTimer, SwitchTimer, ThrowAwayTimer, add_time, change_start_time,
    change_time_entry, create_starter_time_fields, delete_time_entry, start_timer, stop_timer,
    switch_timer, throw_away_timer,
};

/// An issue of a new project, for the administrator of the world to book on.
pub(crate) async fn an_issue_to_book_on(
    world: &World,
    short_name: &str,
) -> Result<IssueId, String> {
    let project = a_project(world, short_name).await?;
    an_issue(world, project).await
}

/// Starts the administrator's timer on an issue and gives the version of the timer.
pub(crate) async fn a_running_timer(world: &World, issue: IssueId) -> Result<i64, String> {
    Ok(start_timer(
        &world.ctx(),
        StartTimer {
            session_token: token(world),
            issue: issue.to_string(),
        },
    )
    .await
    .map_err(text)?
    .into_value()
    .version)
}

/// An entry of the administrator, booked by hand, of `minutes` minutes on 2027-01-14.
pub(crate) async fn an_entry(
    world: &World,
    issue: IssueId,
    minutes: u32,
) -> Result<(TimeEntryId, i64), String> {
    let made = add_time(
        &world.ctx(),
        AddTime {
            session_token: token(world),
            issue: issue.to_string(),
            date: date(2027, 1, 14),
            minutes: Minutes::new(minutes),
            details: EntryDetails::default(),
        },
    )
    .await
    .map_err(text)?
    .into_value();
    Ok((made.id, made.version))
}

/// A server that has not made the starter fields of time entries: the first start of a server made
/// before they existed. Each field is one change.
pub(crate) async fn create_starter_time_fields_scenario(world: &World) -> Added {
    world.store.lock().state.starter_fields.clear();
    world.store.lock().state.field_definitions.clear();
    measured(
        &world.store,
        create_starter_time_fields(&world.ctx(), CreateStarterTimeFields),
    )
    .await
}

pub(crate) async fn start_timer_scenario(world: &World) -> Added {
    let issue = an_issue_to_book_on(world, "ACME").await?;
    measured(
        &world.store,
        start_timer(
            &world.ctx(),
            StartTimer {
                session_token: token(world),
                issue: issue.to_string(),
            },
        ),
    )
    .await
}

pub(crate) async fn stop_timer_scenario(world: &World) -> Added {
    let issue = an_issue_to_book_on(world, "ACME").await?;
    let version = a_running_timer(world, issue).await?;
    world.clock.advance(2 * 60);
    measured(
        &world.store,
        stop_timer(
            &world.ctx(),
            StopTimer {
                session_token: token(world),
                version,
                details: EntryDetails::default(),
            },
        ),
    )
    .await
}

pub(crate) async fn switch_timer_scenario(world: &World) -> Added {
    let issue = an_issue_to_book_on(world, "ACME").await?;
    let other = an_issue_to_book_on(world, "WEB").await?;
    let version = a_running_timer(world, issue).await?;
    world.clock.advance(2 * 60);
    measured(
        &world.store,
        switch_timer(
            &world.ctx(),
            SwitchTimer {
                session_token: token(world),
                version,
                issue: other.to_string(),
                details: EntryDetails::default(),
            },
        ),
    )
    .await
}

pub(crate) async fn change_start_time_scenario(world: &World) -> Added {
    let issue = an_issue_to_book_on(world, "ACME").await?;
    let version = a_running_timer(world, issue).await?;
    let earlier = world
        .ctx()
        .now()
        .checked_sub(jiff::SignedDuration::from_mins(15))
        .map_err(text)?;
    measured(
        &world.store,
        change_start_time(
            &world.ctx(),
            ChangeStartTime {
                session_token: token(world),
                version,
                started_at: earlier,
            },
        ),
    )
    .await
}

pub(crate) async fn throw_away_timer_scenario(world: &World) -> Added {
    let issue = an_issue_to_book_on(world, "ACME").await?;
    let version = a_running_timer(world, issue).await?;
    measured(
        &world.store,
        throw_away_timer(
            &world.ctx(),
            ThrowAwayTimer {
                session_token: token(world),
                version,
            },
        ),
    )
    .await
}

pub(crate) async fn add_time_scenario(world: &World) -> Added {
    let issue = an_issue_to_book_on(world, "ACME").await?;
    let before = world.store.event_count();
    an_entry(world, issue, 90).await?;
    Ok(world.store.event_count() - before)
}

pub(crate) async fn change_time_entry_scenario(world: &World) -> Added {
    let issue = an_issue_to_book_on(world, "ACME").await?;
    let (entry, version) = an_entry(world, issue, 90).await?;
    measured(
        &world.store,
        change_time_entry(
            &world.ctx(),
            ChangeTimeEntry {
                session_token: token(world),
                entry,
                version,
                date: None,
                minutes: Some(Minutes::new(60)),
                details: EntryDetails::default(),
            },
        ),
    )
    .await
}

pub(crate) async fn delete_time_entry_scenario(world: &World) -> Added {
    let issue = an_issue_to_book_on(world, "ACME").await?;
    let (entry, version) = an_entry(world, issue, 90).await?;
    measured(
        &world.store,
        delete_time_entry(
            &world.ctx(),
            DeleteTimeEntry {
                session_token: token(world),
                entry,
                version,
            },
        ),
    )
    .await
}
