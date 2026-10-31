//! The time of an issue and of a project (TIME-03, PRIV-13): what the use case hands the store and
//! what it makes of the answer, against the fake of the port of time spent. That the compiled search
//! counts only the issues the reader sees and every level below an issue is tested on both
//! databases in `bb-store`.

use bb_core::ErrorCode;
use bb_core::fields::{Field, FieldKey};
use bb_core::given::SetTo;
use bb_core::id::IssueId;
use bb_core::time::{Minutes, TimeLeft};
use serde_json::json;

use super::scenarios::{a_project, token};
use super::scenarios_issues::change_of;
use super::scenarios_time::an_entry;
use super::search_fields::{bob_with_a_role, every_field};
use super::search_run::{asked, hit, issue_of};
use super::world::{Outcome, World};
use crate::auth::AppError;
use crate::issues::{ChangeIssue, change_issue};
use crate::search::{LengthTotals, search};
use crate::time::totals::{IssueTimeQuery, ProjectTimeQuery, issue_time, project_time};

const fn minutes(count: u32) -> Minutes {
    Minutes::new(count)
}

/// Gives the issue an estimate, as a person does.
async fn estimate(world: &World, issue: IssueId, estimated: u32) -> Result<(), String> {
    change_issue(
        &world.ctx(),
        ChangeIssue {
            estimated_time: Some(SetTo::Value(minutes(estimated))),
            ..change_of(&token(world), issue, 1)
        },
    )
    .await
    .map(drop)
    .map_err(|error| error.to_string())
}

/// A project with an issue estimated at 4 hours with 80 minutes spent, and below it what the
/// compiled search finds: one sub-issue with 100 minutes spent, 10 hours estimated and 5 hours
/// left in total.
async fn an_issue_with_sub_issues(world: &World) -> Result<IssueId, String> {
    let project = a_project(world, "ACME").await?;
    let issue = issue_of(world, project).await?;
    estimate(world, issue.id, 240).await?;
    let sub = issue_of(world, project).await?;
    an_entry(world, issue.id, 80).await?;
    an_entry(world, sub.id, 100).await?;
    let mut shared = world.store.lock();
    shared.search_hits = vec![hit(sub)];
    shared.search_lengths = LengthTotals {
        found_any: true,
        estimated: Some(minutes(600)),
        time_left: Some(minutes(300)),
    };
    shared.searches.clear();
    Ok(issue.id)
}

#[tokio::test]
async fn time_03_an_issue_shows_its_own_numbers_and_the_totals_with_its_sub_issues() -> Outcome {
    let world = World::new().await?;
    let issue = an_issue_with_sub_issues(&world).await?;
    let time = issue_time(
        &world.ctx(),
        IssueTimeQuery {
            session_token: token(&world),
            issue: issue.to_string(),
        },
    )
    .await
    .map_err(|error| error.to_string())?
    .into_value();
    assert_eq!(time.estimated_time, Some(minutes(240)));
    assert_eq!(time.time_left, Some(TimeLeft::WorkedOut(minutes(160))));
    assert_eq!(time.time_spent, Some(minutes(80)));
    let with = time
        .with_sub_issues
        .ok_or("no totals with the sub-issues")?;
    assert_eq!(with.estimated_time, Some(minutes(840)));
    assert_eq!(with.time_left, Some(minutes(460)));
    assert_eq!(with.time_spent, Some(minutes(180)));
    // The totals are the compiled search of the sub-issues, with the reader's sight in it: one
    // statement for the lengths and one through the port of time spent.
    let searches = world.store.lock().searches.clone();
    assert_eq!(searches.len(), 2);
    assert!(searches.iter().all(|request| !request.sight.all.is_empty()));
    Ok(())
}

#[tokio::test]
async fn time_03_an_issue_without_sub_issues_has_no_totals() -> Outcome {
    let world = World::new().await?;
    let issue = an_issue_with_sub_issues(&world).await?;
    world.store.lock().search_lengths = LengthTotals::default();
    let time = issue_time(
        &world.ctx(),
        IssueTimeQuery {
            session_token: token(&world),
            issue: issue.to_string(),
        },
    )
    .await
    .map_err(|error| error.to_string())?;
    let shown = time.masked_value().map_err(|error| error.to_string())?;
    assert_eq!(shown["with_sub_issues"], json!(null));
    assert_eq!(shown["estimated_time"], json!(240));
    Ok(())
}

#[tokio::test]
async fn priv_13_a_total_of_a_field_the_reader_may_not_read_is_not_shown() -> Outcome {
    let world = World::new().await?;
    let issue = an_issue_with_sub_issues(&world).await?;
    let shown: Vec<FieldKey> = every_field()
        .into_iter()
        .filter(|field| *field != Field::TimeSpent.into())
        .collect();
    let (bob, _) = bob_with_a_role(&world, &shown).await?;
    world.store.lock().searches.clear();
    let time = issue_time(
        &world.ctx(),
        IssueTimeQuery {
            session_token: bob,
            issue: issue.to_string(),
        },
    )
    .await
    .map_err(|error| error.to_string())?;
    let shown = time.masked_value().map_err(|error| error.to_string())?;
    assert!(shown.get("time_spent").is_none(), "{shown}");
    assert!(
        shown["with_sub_issues"].get("time_spent").is_none(),
        "{shown}"
    );
    assert_eq!(shown["with_sub_issues"]["estimated_time"], json!(840));
    // The time spent of the issues below was never read: only the statement of the lengths ran.
    assert_eq!(world.store.lock().searches.len(), 1);
    Ok(())
}

#[tokio::test]
async fn priv_13_the_totals_of_a_project_the_reader_cannot_see_answer_like_a_missing_one() -> Outcome
{
    let world = World::new().await?;
    an_issue_with_sub_issues(&world).await?;
    let (bob, _) = bob_with_a_role(&world, &[]).await?;
    let missing = project_time(
        &world.ctx(),
        ProjectTimeQuery {
            session_token: bob,
            project: "NOWHERE".to_owned(),
        },
    )
    .await;
    assert!(matches!(
        missing,
        Err(AppError::Refused(ErrorCode::NotFound))
    ));
    let totals = project_time(
        &world.ctx(),
        ProjectTimeQuery {
            session_token: token(&world),
            project: "ACME".to_owned(),
        },
    )
    .await
    .map_err(|error| error.to_string())?
    .into_value();
    assert_eq!(totals.estimated_time, Some(minutes(600)));
    assert_eq!(totals.time_spent, Some(minutes(100)));
    Ok(())
}

#[tokio::test]
async fn qry_05_a_hidden_time_field_answers_like_an_unknown_field() -> Outcome {
    for hidden in [Field::EstimatedTime, Field::TimeLeft, Field::TimeSpent] {
        let world = World::new().await?;
        a_project(&world, "ACME").await?;
        let shown: Vec<FieldKey> = every_field()
            .into_iter()
            .filter(|field| *field != hidden.into())
            .collect();
        let (bob, _) = bob_with_a_role(&world, &shown).await?;
        let mistake = |text: String| {
            let (world, bob) = (&world, bob.clone());
            async move {
                match search(&world.ctx(), asked(&bob, &text)).await {
                    Err(AppError::Mistakes(found)) => Ok(found),
                    other => Err(format!("{text}: {other:?}")),
                }
            }
        };
        let name = hidden.search_word();
        let unknown = mistake("nonsense is 2 hours".to_owned()).await?;
        for text in [
            format!("{name} > 2 hours"),
            format!("sort by {name}"),
            format!("{name} is empty"),
        ] {
            let found = mistake(text.clone()).await?;
            assert_eq!(found[0].key, unknown[0].key, "{text}");
        }
        assert!(world.store.lock().searches.is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn time_03_time_spent_is_searchable_in_hours_and_minutes() -> Outcome {
    let world = World::new().await?;
    a_project(&world, "ACME").await?;
    let admin = token(&world);
    for text in [
        "time spent > 1 hour 30 minutes",
        "estimated time <= 45 minutes sort by time left, highest first",
        "time left is empty",
    ] {
        search(&world.ctx(), asked(&admin, text))
            .await
            .map_err(|error| format!("{text}: {error}"))?;
    }
    // A bare number is no length of time: a mistake that shows how to write one.
    match search(&world.ctx(), asked(&admin, "time spent > 90")).await {
        Err(AppError::Mistakes(found)) => {
            assert_eq!(found[0].key, "search.mistake.wrong-duration");
        }
        other => return Err(format!("a bare number: {other:?}").into()),
    }
    assert_eq!(world.store.lock().searches.len(), 3);
    Ok(())
}
