//! The time factor in the answer of a change of the people (PLAN-14, ADR-0030, amendment M8): it
//! belongs to the version of the issue the answer names, never to a later one.

use bb_core::ErrorCode;
use bb_core::id::{IssueId, UserId};
use bb_core::plan::TimeFactor;
use bb_core::plan::record::IssueTimeFactor;
use bb_policy::StartingRole;

use super::fake_plan::Meanwhile;
use super::scenarios::token;
use super::scenarios_plan::an_assigned_issue;
use super::workflows_rules::member;
use super::world::{Outcome, World};
use crate::auth::AppError;
use crate::issues::{ChangeIssue, ChangedIssue, change_issue};
use crate::plan::{TimeFactorQuery, time_factor};

/// Somebody sets the time factor of `issue` to ×1.5 just before or just after the next read of a
/// time factor.
fn factor_set_meanwhile(world: &World, issue: &str, when: Meanwhile) -> Result<(), String> {
    let id: IssueId = issue.parse().map_err(|_| "no issue ID")?;
    let person = world.admin.person.id;
    world.store.set_factor_meanwhile(
        IssueTimeFactor {
            issue: id,
            factor: TimeFactor::new(150).ok_or("no factor")?,
            person,
            set_by: person,
        },
        when,
    );
    Ok(())
}

/// The percent of the time factor of `issue` as its route reads it now.
async fn percent_now(world: &World, issue: &str) -> Result<u16, AppError> {
    Ok(time_factor(
        &world.ctx(),
        TimeFactorQuery {
            session_token: token(world),
            issue: issue.to_owned(),
        },
    )
    .await?
    .into_value()
    .percent)
}

/// A change of the people of `issue` at `version`.
fn people_change(world: &World, issue: &str, version: i64, people: Vec<UserId>) -> ChangeIssue {
    let mut change = ChangeIssue::of(&token(world), issue.to_owned(), version);
    change.assigned_to = Some(people);
    change
}

/// Setting a factor counts the version of the issue up, so the check of the version in the write
/// stops a change planned before it: the reset the plan decided on an older factor is never
/// written over the newer one. It is the check of the version every change has, seen from here.
#[tokio::test]
async fn plan_14_a_factor_set_after_the_plan_read_it_is_a_newer_version_the_write_refuses()
-> Outcome {
    let world = World::new().await?;
    let issue = an_assigned_issue(&world).await?;
    // The person on the bar taken off: the plan reads the factor (there is none), which goes back
    // to ×1, and the factor is set right after that read, before the change is written.
    factor_set_meanwhile(&world, &issue, Meanwhile::AfterNextRead)?;
    let refused =
        change_issue::<_, ChangedIssue>(&world.ctx(), people_change(&world, &issue, 1, Vec::new()))
            .await;
    assert_eq!(
        refused.as_ref().err().and_then(AppError::code),
        Some(ErrorCode::VersionChanged),
        "{refused:?}"
    );
    // Nothing of the change was written: the factor set meanwhile stays.
    assert_eq!(percent_now(&world, &issue).await?, 150);
    Ok(())
}

#[tokio::test]
async fn plan_14_a_change_that_writes_nothing_never_answers_a_later_time_factor() -> Outcome {
    let world = World::new().await?;
    let issue = an_assigned_issue(&world).await?;
    let same = vec![world.admin.person.id];
    // The same people again: nothing is written, and the answer is the issue at version 1. The
    // factor is set before the answer reads it, so it belongs to version 2: the answer has none,
    // and the screen reads it again.
    factor_set_meanwhile(&world, &issue, Meanwhile::BeforeNextRead)?;
    let answered = change_issue::<_, ChangedIssue>(
        &world.ctx(),
        people_change(&world, &issue, 1, same.clone()),
    )
    .await?
    .into_value();
    assert_eq!(answered.issue.version, 1);
    assert_eq!(answered.time_factor, None);
    // Without a change meanwhile the answer holds the factor of the version it names.
    let answered =
        change_issue::<_, ChangedIssue>(&world.ctx(), people_change(&world, &issue, 2, same))
            .await?
            .into_value();
    let factor = answered.time_factor.ok_or("no time factor")?;
    assert_eq!((factor.version, factor.percent), (2, 150));
    Ok(())
}

#[tokio::test]
async fn plan_14_a_written_change_that_keeps_the_first_person_never_answers_a_later_factor()
-> Outcome {
    let world = World::new().await?;
    let issue = an_assigned_issue(&world).await?;
    let first = world.admin.person.id;
    let id: IssueId = issue.parse().map_err(|_| "no issue ID")?;
    let project = world
        .store
        .read(|state| {
            state
                .issues
                .iter()
                .find(|each| each.id == id)
                .map(|each| each.project)
        })
        .ok_or("no issue")?;
    let second = member(
        &world,
        "bob@example.test",
        StartingRole::TeamMember,
        project,
    )
    .await?
    .person
    .id;
    // Somebody is added after the person on the bar, who stays: the change is written (version
    // 2), and the factor is set before the answer reads it, so it belongs to version 3. The
    // answer names version 2 and has no factor; the screen reads it again.
    factor_set_meanwhile(&world, &issue, Meanwhile::BeforeNextRead)?;
    let answered = change_issue::<_, ChangedIssue>(
        &world.ctx(),
        people_change(&world, &issue, 1, vec![first, second]),
    )
    .await?
    .into_value();
    assert_eq!(answered.issue.version, 2);
    assert_eq!(answered.time_factor, None);
    // Nothing set meanwhile: the answer holds the factor of the version it names.
    let answered = change_issue::<_, ChangedIssue>(
        &world.ctx(),
        people_change(&world, &issue, 3, vec![first]),
    )
    .await?
    .into_value();
    let factor = answered.time_factor.ok_or("no time factor")?;
    assert_eq!((factor.version, factor.percent), (4, 150));
    Ok(())
}
