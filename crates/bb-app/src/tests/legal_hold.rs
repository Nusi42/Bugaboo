//! The legal-hold name of a removed person (PRIV-04, ADR-0044, decision 6): it goes in the change
//! in which *Delete them* deletes the person's last kept work day, and not before.

use bb_core::keeping::KeepKind;
use bb_core::person::{HoldReason, PersonStanding, Standing};
use bb_core::working_time::Span;
use jiff::civil::{Date, date};

use bb_policy::StartingRole;

use super::scenarios::token;
use super::scenarios_searches::with_role;
use super::world::{Outcome, World};
use crate::keeping::{DeleteOldRecords, OldRecordsQuery, delete_old_records, old_records};
use crate::people::StandingStore as _;
use crate::working_hours::{ChangeWorkDay, change_work_day};

/// The administrator types a work day of their own afterwards. The administrator stands for the
/// removed person here: the store holds what a removal leaves, and the rule under test reads only
/// the work days and the standing.
async fn typed(world: &World, day: Date) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    world.clock.advance(1);
    let at = |hour: i8| {
        day.at(hour, 0, 0, 0)
            .to_zoned(jiff::tz::TimeZone::UTC)
            .map(|zoned| zoned.timestamp())
    };
    change_work_day(
        &world.ctx(),
        ChangeWorkDay {
            session_token: token(world),
            person: None,
            date: day,
            version: 0,
            periods: vec![Span::closed(at(8)?, at(16)?)],
            breaks: Vec::new(),
            reason: Some("Typed in afterwards".to_owned()),
            device_zone: None,
        },
    )
    .await?;
    Ok(())
}

/// A second administrator, who deletes the old work days: the removed one signs in to nothing.
async fn another_administrator(world: &World) -> Result<String, String> {
    let ben = world
        .person("ben@example.test")
        .await
        .map_err(|error| format!("{error:?}"))?;
    with_role(world, &ben, StartingRole::Administrator, None).await?;
    Ok(ben.session_token)
}

async fn delete_old_work_days(world: &World, session: &str) -> Outcome {
    let counted = old_records(
        &world.ctx(),
        OldRecordsQuery {
            session_token: session.to_owned(),
            kind: KeepKind::WorkDays,
        },
    )
    .await?
    .into_value();
    delete_old_records(
        &world.ctx(),
        DeleteOldRecords {
            session_token: session.to_owned(),
            kind: KeepKind::WorkDays,
            person: None,
            before: counted.before.to_string(),
        },
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn priv_04_deleting_the_last_kept_work_days_clears_the_legal_hold_name() -> Outcome {
    let world = World::new().await?;
    let ben = another_administrator(&world).await?;
    let bob = world.admin.person.id;
    typed(&world, date(2020, 3, 2)).await?;
    // Bob was removed; his name is held for his work days until the end of their keep period.
    world.store.lock().state.standings.push(PersonStanding {
        person: bob,
        standing: Standing::Removed,
        since: world.ctx().now(),
        by: Some(bob),
        label: Some("K7Q2".to_owned()),
        hold_name: Some("Bob Builder".to_owned()),
        hold_reason: Some(HoldReason::WorkDays),
        hold_until: Some(date(2022, 12, 31)),
    });
    delete_old_work_days(&world, &ben).await?;
    let standing = world
        .store
        .standing(bob)
        .await
        .map_err(|error| format!("{error:?}"))?
        .ok_or("bob stays removed")?;
    assert_eq!(standing.label.as_deref(), Some("K7Q2"));
    assert_eq!(
        (
            standing.hold_name,
            standing.hold_reason,
            standing.hold_until
        ),
        (None, None, None)
    );
    Ok(())
}

#[tokio::test]
async fn priv_04_the_legal_hold_name_stays_while_a_work_day_is_kept() -> Outcome {
    let world = World::new().await?;
    let ben = another_administrator(&world).await?;
    let bob = world.admin.person.id;
    typed(&world, date(2020, 3, 2)).await?;
    // A day inside the keep period stays, and with it the name.
    let today = bb_core::time::day::date_of(world.ctx().now(), &jiff::tz::TimeZone::UTC);
    typed(&world, today.yesterday()?).await?;
    world.store.lock().state.standings.push(PersonStanding {
        person: bob,
        standing: Standing::Removed,
        since: world.ctx().now(),
        by: Some(bob),
        label: Some("K7Q2".to_owned()),
        hold_name: Some("Bob Builder".to_owned()),
        hold_reason: Some(HoldReason::WorkDays),
        hold_until: Some(date(2029, 1, 14)),
    });
    delete_old_work_days(&world, &ben).await?;
    let standing = world
        .store
        .standing(bob)
        .await
        .map_err(|error| format!("{error:?}"))?
        .ok_or("bob stays removed")?;
    assert_eq!(standing.hold_name.as_deref(), Some("Bob Builder"));
    Ok(())
}
