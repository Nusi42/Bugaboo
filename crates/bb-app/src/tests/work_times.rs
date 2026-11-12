//! The rules of work times on the in-memory store (PLAN-11, PLAN-15, ADR-0030): what the plan can
//! read through the port, the deletion of old away times with the clock moved, and everything a
//! person who leaves loses.

use bb_core::access::AppliesTo;
use bb_core::holidays::Holiday;
use bb_core::id::{ProjectId, UserId};
use bb_core::settings::SettingValue;
use bb_core::work_times::{Halves, Share};
use bb_policy::{Place, StartingRole};
use jiff::civil::Date;

use super::scenarios::{a_project, text, token};
use super::scenarios_searches::with_role;
use super::scenarios_work_times::{date, days, entered, week};
use super::world::{Outcome, World};
use crate::auth::Started;
use crate::roles::{AssignRole, Given, RemoveAssignment, Subject, assign_role, remove_assignment};
use crate::settings::{ChangeSetting, SettingsScope, change_setting};
use crate::work_times::{
    AwayDays, ChangeUsualDays, DayMinutes, DeleteWorkTimesOfPerson, SetShare, WorkMinutesQuery,
    change_usual_days, delete_old_away_times, delete_work_times_of_person, set_share,
};

/// A project with Bob in it, who has set his usual work days: Monday to Thursday, 6 hours. The
/// clock of a test stands on Friday, 15 January 2027.
async fn bob_in_a_project(world: &World) -> Result<(Started, ProjectId), String> {
    let project = a_project(world, "WEB").await?;
    let bob = world.person("bob@example.test").await.map_err(text)?;
    change_usual_days(
        &world.ctx(),
        ChangeUsualDays {
            session_token: bob.session_token.clone(),
            person: None,
            version: 0,
            days: week(4, 360),
        },
    )
    .await
    .map_err(text)?;
    Ok((bob, project))
}

/// The minutes Bob can work in the project on each day from `first` to `last`, as the plan reads
/// them through the port.
async fn minutes(
    world: &World,
    bob: &Started,
    project: ProjectId,
    first: Date,
    last: Date,
) -> Result<Vec<DayMinutes>, String> {
    let reader = world
        .ctx()
        .evaluate(world.admin.person.id, Place::Project(project))
        .await
        .map_err(text)?;
    let found = world
        .ctx()
        .work_minutes(
            &reader,
            WorkMinutesQuery {
                people: vec![bob.person.id],
                project,
                first,
                last,
            },
        )
        .await
        .map_err(text)?;
    Ok(found.people[0].days.clone())
}

async fn share_of_bob(
    world: &World,
    project: ProjectId,
    bob: UserId,
    percent: u8,
) -> Result<(), String> {
    set_share(
        &world.ctx(),
        SetShare {
            session_token: token(world),
            project: project.to_string(),
            person: bob,
            percent,
        },
    )
    .await
    .map_err(text)?;
    Ok(())
}

#[tokio::test]
async fn plan_15_the_minutes_of_a_day_count_work_hours_share_away_times_and_holidays() -> Outcome {
    let world = World::new().await?;
    let (bob, project) = bob_in_a_project(&world).await?;
    with_role(&world, &bob, StartingRole::TeamMember, Some(project)).await?;
    share_of_bob(&world, project, bob.person.id, 50).await?;
    // Monday 5 to Friday 9 October 2026 and the weekend after: Bob works Monday to Thursday.
    let (monday, sunday) = (date(2026, 10, 5), date(2026, 10, 11));
    // Tuesday afternoon, and all of Wednesday.
    let mut away = days(date(2026, 10, 6), date(2026, 10, 7));
    away.first_afternoon_only = true;
    crate::work_times::enter_away_time(
        &world.ctx(),
        crate::work_times::EnterAwayTime {
            session_token: bob.session_token.clone(),
            person: None,
            days: away,
        },
    )
    .await
    .map_err(text)?;
    let found = minutes(&world, &bob, project, monday, sunday).await?;
    let table: Vec<(u32, Halves, Option<Holiday>)> = found
        .iter()
        .map(|day| (day.minutes, day.away, day.holiday))
        .collect();
    let afternoon = Halves {
        morning: false,
        afternoon: true,
    };
    assert_eq!(
        table,
        [
            // Monday: 6 hours of the share of half.
            (180, Halves::NONE, None),
            // Tuesday: the afternoon is away, so half of that.
            (90, afternoon, None),
            // Wednesday: away the whole day (the last day of the away time).
            (0, Halves::BOTH, None),
            // Thursday: 3 October is not in the week, so no holiday; a work day.
            (180, Halves::NONE, None),
            // Friday, Saturday and Sunday are not usual work days.
            (0, Halves::NONE, None),
            (0, Halves::NONE, None),
            (0, Halves::NONE, None),
        ]
    );
    // Ascension Day, Thursday 14 May 2026: the organization starts with *Germany*, which has it.
    let found = ascension_day(&world, &bob, project).await?;
    assert_eq!(
        (found[0].minutes, found[0].holiday),
        (0, Some(Holiday::AscensionDay))
    );
    // A person who chose *None* has a work day there; one who chose a state has the holiday.
    for (choice, minutes_that_day, holiday) in [
        ("none", 180, None),
        ("de-by", 0, Some(Holiday::AscensionDay)),
    ] {
        change_setting(
            &world.ctx(),
            ChangeSetting {
                session_token: bob.session_token.clone(),
                scope: SettingsScope::Person,
                key: "work-times.holidays".to_owned(),
                value: SettingValue::Text(choice.to_owned()),
                version: None,
            },
        )
        .await
        .map_err(text)?;
        let found = ascension_day(&world, &bob, project).await?;
        assert_eq!(
            (found[0].minutes, found[0].holiday),
            (minutes_that_day, holiday),
            "{choice}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn plan_15_a_share_counts_as_nothing_without_a_role_and_again_when_the_role_returns()
-> Outcome {
    let world = World::new().await?;
    let (bob, project) = bob_in_a_project(&world).await?;
    let monday = date(2026, 10, 5);
    // The share is set while Bob has no role in the project: it is kept, and counts as nothing.
    share_of_bob(&world, project, bob.person.id, 60).await?;
    let none = minutes(&world, &bob, project, monday, monday).await?;
    assert_eq!(none[0].minutes, 0);
    let kept = world
        .store
        .read(|state| state.shares.clone())
        .into_iter()
        .find(|(person, which, _)| *person == bob.person.id && *which == project);
    assert_eq!(kept.map(|(_, _, share)| share), Share::new(60));
    // A role comes: the share counts as before.
    let assignment = assign_role(
        &world.ctx(),
        AssignRole {
            session_token: token(&world),
            subject: Subject::Person(bob.person.id),
            given: Given::Role(
                super::workflows_rules::starting_role(&world, StartingRole::TeamMember).await?,
            ),
            project: Some(project),
            applies_to: AppliesTo::ProjectAndSubProjects,
        },
    )
    .await
    .map_err(text)?
    .into_value()
    .id;
    let counts = minutes(&world, &bob, project, monday, monday).await?;
    assert_eq!(counts[0].minutes, 216);
    // The role goes: nothing counts, and nothing is deleted. It comes back: 216 again.
    remove_assignment(
        &world.ctx(),
        RemoveAssignment {
            session_token: token(&world),
            assignment,
        },
    )
    .await
    .map_err(text)?;
    assert_eq!(
        minutes(&world, &bob, project, monday, monday).await?[0].minutes,
        0
    );
    assert_eq!(world.store.read(|state| state.shares.len()), 1);
    with_role(&world, &bob, StartingRole::TeamMember, Some(project)).await?;
    assert_eq!(
        minutes(&world, &bob, project, monday, monday).await?[0].minutes,
        216
    );
    Ok(())
}

/// Ascension Day 2026, a Thursday: the one day the holidays of the person decide.
async fn ascension_day(
    world: &World,
    bob: &Started,
    project: ProjectId,
) -> Result<Vec<DayMinutes>, String> {
    minutes(world, bob, project, date(2026, 5, 14), date(2026, 5, 14)).await
}

/// The number of away times in the store.
fn away_count(world: &World) -> usize {
    world.store.read(|state| state.away_times.len())
}

#[tokio::test]
async fn plan_11_an_away_time_is_deleted_one_year_after_its_last_day() -> Outcome {
    let world = World::new().await?;
    // The clock stands on 2027-01-15, so a year ago is 2026-01-15.
    let just_over = date(2026, 1, 14);
    let exactly = date(2026, 1, 15);
    let recent = date(2027, 1, 1);
    let first = |last: Date| -> Result<AwayDays, String> {
        Ok(days(
            last.checked_sub(jiff::Span::new().days(2)).map_err(text)?,
            last,
        ))
    };
    for last in [just_over, exactly, recent] {
        entered(&world, None, first(last)?).await.map_err(text)?;
    }
    assert_eq!(delete_old_away_times(&world.ctx()).await.map_err(text)?, 1);
    assert_eq!(away_count(&world), 2);
    // A second round on the same day finds nothing.
    assert_eq!(delete_old_away_times(&world.ctx()).await.map_err(text)?, 0);
    // The next day the one that ended exactly a year ago goes too; the recent one stays.
    world.clock.advance(24 * 3600);
    assert_eq!(delete_old_away_times(&world.ctx()).await.map_err(text)?, 1);
    assert_eq!(away_count(&world), 1);
    // A year after the last day of the recent one, it goes as well.
    world.clock.advance(365 * 24 * 3600);
    assert_eq!(delete_old_away_times(&world.ctx()).await.map_err(text)?, 1);
    assert_eq!(away_count(&world), 0);
    Ok(())
}

#[tokio::test]
async fn plan_11_a_person_who_leaves_loses_all_work_times() -> Outcome {
    let world = World::new().await?;
    let (bob, project) = bob_in_a_project(&world).await?;
    let cleo = world.person("cleo@example.test").await.map_err(text)?;
    share_of_bob(&world, project, bob.person.id, 60).await?;
    share_of_bob(&world, project, cleo.person.id, 40).await?;
    entered(
        &world,
        Some(bob.person.id),
        days(date(2027, 2, 1), date(2027, 2, 5)),
    )
    .await
    .map_err(text)?;
    entered(
        &world,
        Some(cleo.person.id),
        days(date(2027, 2, 1), date(2027, 2, 5)),
    )
    .await
    .map_err(text)?;
    change_setting(
        &world.ctx(),
        ChangeSetting {
            session_token: bob.session_token.clone(),
            scope: SettingsScope::Person,
            key: "work-times.holidays".to_owned(),
            value: SettingValue::Text("de-by".to_owned()),
            version: None,
        },
    )
    .await
    .map_err(text)?;
    delete_work_times_of_person(
        &world.ctx(),
        DeleteWorkTimesOfPerson {
            person: bob.person.id,
        },
    )
    .await
    .map_err(text)?;
    let left = world.store.read(|state| {
        (
            state
                .usual_days
                .iter()
                .any(|(person, _)| *person == bob.person.id),
            state
                .shares
                .iter()
                .any(|(person, _, _)| *person == bob.person.id),
            state
                .away_times
                .iter()
                .any(|time| time.person == bob.person.id),
        )
    });
    assert_eq!(
        left,
        (false, false, false),
        "usual days, shares, away times"
    );
    // The choice of *Holidays* went too: the organization's choice counts again.
    let layers = world
        .ctx()
        .setting_layers_of(Some(bob.person.id))
        .await
        .map_err(text)?;
    assert_eq!(
        bb_core::holidays::Region::chosen(&layers),
        bb_core::holidays::Region::Germany
    );
    // Somebody else's work times stay.
    assert_eq!(away_count(&world), 1);
    assert_eq!(world.store.read(|state| state.shares.len()), 1);
    Ok(())
}

#[tokio::test]
async fn plan_15_a_reader_who_may_not_know_the_work_times_gets_the_minutes_and_no_days_named()
-> Outcome {
    let world = World::new().await?;
    let (bob, project) = bob_in_a_project(&world).await?;
    with_role(&world, &bob, StartingRole::TeamMember, Some(project)).await?;
    // An away time on Thursday 14 May 2026, which is Ascension Day for Germany as well.
    entered(
        &world,
        Some(bob.person.id),
        days(date(2026, 5, 13), date(2026, 5, 13)),
    )
    .await
    .map_err(text)?;
    let guest = world.person("guest@example.test").await.map_err(text)?;
    with_role(&world, &guest, StartingRole::Guest, Some(project)).await?;
    let read_as = |person: UserId| {
        let (world, bob) = (&world, &bob);
        async move {
            let reader = world
                .ctx()
                .evaluate(person, Place::Project(project))
                .await
                .map_err(text)?;
            let found = world
                .ctx()
                .work_minutes(
                    &reader,
                    WorkMinutesQuery {
                        people: vec![bob.person.id],
                        project,
                        first: date(2026, 5, 11),
                        last: date(2026, 5, 14),
                    },
                )
                .await
                .map_err(text)?;
            Ok::<_, String>(found.people[0].clone())
        }
    };
    let seen = read_as(world.admin.person.id).await?;
    let unseen = read_as(guest.person.id).await?;
    assert!(seen.away_visible && !unseen.away_visible);
    assert_eq!(seen.days[2].away, Halves::BOTH);
    assert_eq!(seen.days[3].holiday, Some(Holiday::AscensionDay));
    // The minutes are the same: they shape the bar.
    let of = |person: &crate::work_times::PersonMinutes| -> Vec<u32> {
        person.days.iter().map(|day| day.minutes).collect()
    };
    assert_eq!(of(&seen), of(&unseen));
    assert!(unseen.days.iter().all(|day| day.away == Halves::NONE));
    assert!(unseen.days.iter().all(|day| day.holiday.is_none()));
    Ok(())
}

#[tokio::test]
async fn plan_11_a_read_of_away_times_says_when_it_left_some_out() -> Outcome {
    let world = World::new().await?;
    let admin = world.admin.person.id;
    let base = date(2020, 1, 1);
    let first_id = entered(&world, None, days(base, base))
        .await
        .map_err(text)?;
    {
        let mut shared = world.store.lock();
        let template = shared
            .state
            .away_times
            .iter()
            .find(|time| time.id == first_id)
            .cloned()
            .ok_or("no away time")?;
        for _ in 0..crate::work_times::AWAY_TIMES_MAX {
            shared.state.away_times.push(template.clone());
        }
    }
    let read = crate::work_times::work_times(
        &world.ctx(),
        crate::work_times::WorkTimesQuery {
            session_token: token(&world),
            person: Some(admin),
            first: base,
            last: base,
        },
    )
    .await
    .map_err(text)?
    .into_value();
    assert_eq!(read.away.len(), crate::work_times::AWAY_TIMES_MAX);
    assert!(read.truncated);
    Ok(())
}
