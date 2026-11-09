//! What an agent writes under an issue it was handed, a comment, a relation, a file or a
//! sub-issue, only while an attempt of it runs, and then in the run of the attempt (M7-F120).

use bb_core::ErrorCode;
use bb_core::agent_work::HandOverPower;
use bb_core::relation::Direction;

use super::agent_work::{Handed, handed_with, plain_hand_over, report_attempt};
use super::agent_work_narrowing::new_under;
use super::agent_work_review::{ready, report};
use super::agents::refused;
use super::scenarios_issues::an_issue;
use super::scenarios_relations::a_kind;
use super::world::{Outcome, World};
use crate::agent_work::{end_attempt, hand_over};
use crate::auth::AppError;
use crate::comments::{AddComment, add_comment};
use crate::files::{StartUploadQuery, start_upload};
use crate::issues::create_issue;
use crate::relations::{AddRelation, add_relation};

/// What the agent writes, one by one: each is tried by `write` and says whether it was let through.
async fn tries(
    world: &World,
    given: &Handed,
    other: bb_core::id::IssueId,
) -> Vec<(&'static str, Result<(), AppError>)> {
    let comment = add_comment(
        &world.ctx(),
        AddComment {
            session_token: given.key.clone(),
            issue: given.issue.to_string(),
            text: "A note of the agent".to_owned(),
        },
    )
    .await
    .map(drop);
    let kind = a_kind(world).await.ok();
    let relation = match kind {
        Some(kind) => add_relation(
            &world.ctx(),
            AddRelation {
                session_token: given.key.clone(),
                issue: given.issue.to_string(),
                other: other.to_string(),
                kind,
                direction: Direction::Forward,
            },
        )
        .await
        .map(drop),
        None => Err(AppError::Internal("no relation kind".to_owned())),
    };
    let upload = start_upload(
        &world.ctx(),
        StartUploadQuery {
            session_token: given.key.clone(),
            issue: given.issue.to_string(),
            comment: None,
            name: "log.txt".to_owned(),
            size: 10,
        },
    )
    .await
    .map(drop);
    let sub_issue = match new_under(world, given, Some(given.issue)).await {
        Ok(input) => create_issue(&world.ctx(), input).await.map(drop),
        Err(error) => Err(error),
    };
    vec![
        ("comment", comment),
        ("relation", relation),
        ("upload", upload),
        ("sub-issue", sub_issue),
    ]
}

#[tokio::test]
async fn m7_f120_a_comment_a_relation_a_file_and_a_sub_issue_need_an_attempt_that_runs() -> Outcome
{
    let world = World::new().await?;
    let given = handed_with(&world, |asked| {
        asked.powers = vec![HandOverPower::CreateSubIssues];
    })
    .await?;
    let other = an_issue(&world, given.project)
        .await
        .map_err(AppError::Internal)?;
    // The agent sees the other end of a relation only when that issue is handed over to it too.
    hand_over(
        &world.ctx(),
        plain_hand_over(&given.team.session_token, given.agent, &[other]),
    )
    .await?;
    // Waiting for the agent and ready for review: no attempt runs, nothing is let through.
    for (name, outcome) in tries(&world, &given, other).await {
        assert!(
            refused(&outcome.unwrap_err(), ErrorCode::Forbidden),
            "{name} before the take"
        );
    }
    report_attempt(&world.ctx(), report(&given.key, given.issue, None, 5)).await?;
    for (name, outcome) in tries(&world, &given, other).await {
        assert!(
            outcome.is_ok(),
            "{name} while the attempt runs: {outcome:?}"
        );
    }
    let run = {
        let shared = world.store.lock();
        shared
            .events
            .iter()
            .filter(|event| matches!(event.actor, bb_core::event::Actor::Agent { .. }))
            .filter(|event| matches!(event.entity.kind.as_str(), "comment" | "relation" | "issue"))
            .map(|event| event.run)
            .collect::<Vec<_>>()
    };
    assert!(
        !run.is_empty() && run.iter().all(Option::is_some),
        "{run:?}"
    );
    end_attempt(&world.ctx(), ready(&given.key, given.issue)).await?;
    for (name, outcome) in tries(&world, &given, other).await {
        assert!(
            refused(&outcome.unwrap_err(), ErrorCode::Forbidden),
            "{name} after the submit"
        );
    }
    Ok(())
}
