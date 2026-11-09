//! What an agent writes and whom the AI is told wrote it (AI-32, AI-36, ADR-0029): the authors of
//! the brief are the people who wrote it, a rule agent only proposes, and a change to many issues
//! counts one change for each issue.

use bb_core::ErrorCode;
use bb_core::agent::{AgentState, LimitKind};
use bb_core::agent_records::{Question, Review};
use bb_core::agent_work::{HandOverPower, ReviewAnswer};
use bb_core::ai::AuthorKind;
use bb_core::id::AgentNoteId;
use bb_core::relation::Direction;
use bb_policy::StartingRole;
use serde_json::json;

use super::agent_limits::{limit, the_agent};
use super::agent_tools::{fence_above, used};
use super::agent_work::{handed, handed_with, plain_hand_over, start_attempt};
use super::agent_work_rules::{found_by_search, made, rule_world};
use super::agents::refused;
use super::ai_tools::without_nonces;
use super::scenarios::token;
use super::scenarios_issues::an_issue;
use super::scenarios_relations::a_kind;
use super::workflows_rules::member;
use super::world::{Outcome, World};
use crate::agent_work::hand_over;
use crate::files::{StartUploadQuery, start_upload};
use crate::many_changes::{
    ApplyManyChanges, ManyChange, ManyChangeAction, ManyChangeTarget, apply_many_changes,
};
use crate::relations::{AddRelation, add_relation};

#[tokio::test]
async fn ai_36_each_text_of_the_brief_names_the_kind_of_the_person_who_wrote_it() -> Outcome {
    let world = World::new().await?;
    let given = handed(&world).await?;
    let guest = member(
        &world,
        "guest@example.test",
        StartingRole::Guest,
        given.project,
    )
    .await?;
    let outsider = world.person("outsider@example.test").await?;
    // The hand-over was made by a Guest, one question was answered by a Team member and one by
    // nobody who can be named, and a review was written by somebody with no role.
    {
        let mut shared = world.store.lock();
        let work = shared.state.work.works[0].id;
        shared.state.work.hand_overs[0].person = guest.person.id;
        let now = world.ctx().now();
        let mut ids = bb_core::id::SequentialIds::default();
        for (answer, by) in [
            ("The answer of the team member", Some(given.team.person.id)),
            ("The answer nobody can be named for", None),
        ] {
            shared.state.work.questions.push(Question {
                id: AgentNoteId::generate(&mut ids),
                work,
                text: "A question of the agent".to_owned(),
                answer: Some(answer.to_owned()),
                answered_by: by,
                answered_at: Some(now),
                asked_at: now,
            });
        }
        shared.state.work.reviews.push(Review {
            id: AgentNoteId::generate(&mut ids),
            work,
            answer: ReviewAnswer::SendBack,
            note: Some("The note of the reviewer".to_owned()),
            by: Some(outsider.person.id),
            at: now,
        });
    }
    let taken = used(
        &world,
        &given.key,
        "take",
        json!({"issue": given.issue.to_string()}),
    )
    .await?;
    let text = without_nonces(&taken.text);
    let phrase = |kind: AuthorKind| format!("text written by {}", kind.phrase());
    for (needle, kind) in [
        ("Please look at the login button", AuthorKind::Guest),
        ("The answer of the team member", AuthorKind::Member),
        ("The answer nobody can be named for", AuthorKind::Outsider),
        ("The note of the reviewer", AuthorKind::Outsider),
        ("A question of the agent", AuthorKind::Agent),
    ] {
        let above = fence_above(&text, needle);
        assert!(
            above
                .as_deref()
                .is_some_and(|line| line.contains(&phrase(kind))),
            "{needle}: {above:?}\n{text}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn ai_32_a_rule_agent_is_refused_on_relations_and_uploads_of_an_issue_it_was_not_handed()
-> Outcome {
    let world = World::new().await?;
    let given = rule_world(&world).await?;
    made(&world, &given).await?;
    let other = an_issue(&world, given.project).await?;
    found_by_search(&world);
    world
        .ctx()
        .run_rule_matches(&json!({ "project": given.project.to_string() }))
        .await?;
    let kind = a_kind(&world).await?;
    let linked = add_relation(
        &world.ctx(),
        AddRelation {
            session_token: given.key.clone(),
            issue: given.issue.to_string(),
            other: other.to_string(),
            kind,
            direction: Direction::Forward,
        },
    )
    .await;
    let linked = linked.unwrap_err();
    assert!(refused(&linked, ErrorCode::Forbidden), "{linked:?}");
    let uploaded = start_upload(
        &world.ctx(),
        StartUploadQuery {
            session_token: given.key.clone(),
            issue: given.issue.to_string(),
            comment: None,
            name: "log.txt".to_owned(),
            size: 10,
        },
    )
    .await;
    assert!(refused(&uploaded.unwrap_err(), ErrorCode::Forbidden));
    Ok(())
}

#[tokio::test]
async fn ai_37_a_change_to_many_issues_counts_one_change_for_each_issue() -> Outcome {
    let world = World::new().await?;
    let first = handed_with(&world, |asked| {
        asked.powers = vec![HandOverPower::ChangeFields];
        asked.fields = vec!["priority".to_owned()];
    })
    .await?;
    let mut issues = vec![first.issue];
    for _ in 0..3 {
        let more = an_issue(&world, first.project).await?;
        let mut asked = plain_hand_over(&first.team.session_token, first.agent, &[more]);
        asked.powers = vec![HandOverPower::ChangeFields];
        asked.fields = vec!["priority".to_owned()];
        hand_over(&world.ctx(), asked).await?;
        issues.push(more);
    }
    for issue in &issues {
        start_attempt(&world, &first.key, *issue).await?;
    }
    limit(&world, LimitKind::ChangesAMinute, 2);
    let ctx = world.ctx();
    ctx.expect_change();
    let answer = apply_many_changes(
        &ctx,
        ApplyManyChanges {
            session_token: first.key.clone(),
            issues: issues
                .iter()
                .map(|issue| ManyChangeTarget {
                    issue: issue.to_string(),
                    version: 1,
                })
                .collect(),
            action: ManyChangeAction::Change {
                change: ManyChange {
                    priority: Some(bb_core::issue::Priority::High),
                    ..ManyChange::default()
                },
            },
        },
    )
    .await;
    ctx.settle_agent_change(answer.is_ok());
    // Two issues were changed, the third found the limit reached: nothing more is changed and the
    // agent is paused.
    assert!(refused(&answer.unwrap_err(), ErrorCode::AgentLimitReached));
    assert_eq!(the_agent(&world).state, AgentState::Paused);
    let changed = world
        .store
        .lock()
        .state
        .issues
        .iter()
        .filter(|issue| issue.priority == bb_core::issue::Priority::High)
        .count();
    assert_eq!(changed, 2);
    let _ = token(&world);
    Ok(())
}
