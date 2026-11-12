//! An agent and the dates of an issue (PLAN-03, ADR-0043, decision 6): a date is a member a
//! hand-over can name, and an agent changes the date it was named for and no other.

use bb_core::ErrorCode;
use bb_core::given::SetTo;

use super::agent_work::{handed_with, query, start_attempt};
use super::agent_work_narrowing::ticking;
use super::agents::refused;
use super::world::{Outcome, World};
use crate::issues::{ChangeIssue, change_issue, issue};

#[tokio::test]
async fn plan_03_an_agent_changes_a_date_only_when_the_hand_over_names_it() -> Outcome {
    let world = World::new().await?;
    let given = handed_with(&world, ticking(&["due-date"])).await?;
    start_attempt(&world, &given.key, given.issue).await?;
    let version = issue(&world.ctx(), query(&given.key, given.issue))
        .await?
        .into_value()
        .issue
        .version;
    let day = |text: &str| text.parse().map(SetTo::Value).ok();
    // The date that was named is the agent's to change.
    let mut change = ChangeIssue::of(&given.key, given.issue.to_string(), version);
    change.due_date = day("2026-11-14");
    change_issue(&world.ctx(), change).await?;
    // The one that was not named is not.
    let mut other = ChangeIssue::of(&given.key, given.issue.to_string(), version + 1);
    other.start_date = day("2026-11-10");
    let result = change_issue(&world.ctx(), other).await;
    assert!(refused(
        &result.err().ok_or("accepted")?,
        ErrorCode::Forbidden
    ));
    Ok(())
}
