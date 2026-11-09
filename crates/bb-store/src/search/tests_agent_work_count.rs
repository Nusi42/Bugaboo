//! How much open work an agent has, as the limit of the rules that hand over is judged, on both
//! databases (AI-32, ADR-0029, WP-14).

use bb_app::agent_work::{AgentWorkRecords as _, GrantStore as _};
use bb_core::agent_work::AgentWorkState;
use bb_core::id::{AgentWorkId, HandOverId};
use bb_testing::{TestResult, each_database};

use super::tests_agent_work::{ENDS, Given, Made, NOW, close, given, hand_over};
use crate::AuthStorage;
use crate::test_support::{at, event};
use bb_app::ports::ChangeStore as _;

async fn set_state(
    storage: &AuthStorage,
    made: &mut Made,
    work: AgentWorkId,
    state: AgentWorkState,
    gave_up: Option<&'static str>,
) -> Result<(), crate::StoreError> {
    made.step += 1;
    let step = made.step;
    storage
        .record_change(&event(&mut made.source, step), move |transaction| {
            Box::pin(async move {
                AuthStorage::set_work_state(transaction, work, state, gave_up, false, 1).await?;
                Ok(())
            })
        })
        .await
}

async fn end(
    storage: &AuthStorage,
    made: &mut Made,
    hand_over: HandOverId,
) -> Result<(), crate::StoreError> {
    made.step += 1;
    let step = made.step;
    storage
        .record_change(&event(&mut made.source, step), move |transaction| {
            Box::pin(async move {
                AuthStorage::end_hand_over(transaction, hand_over, 1).await?;
                Ok(())
            })
        })
        .await
}

#[tokio::test]
async fn m7_wp14_the_open_work_of_an_agent_leaves_out_what_ended_in_every_way() -> TestResult {
    each_database("m7_wp14_open_work_count", async |db| {
        let Given {
            fixture,
            data,
            owner,
            fixer,
        } = given(db).await?;
        let storage = &fixture.storage;
        let mut made = Made::new();
        let issues: Vec<_> = data.issues.iter().take(6).map(|issue| issue.id).collect();
        let mut works = Vec::new();
        let mut hand_overs = Vec::new();
        for issue in &issues {
            let (record, kept) = hand_over(
                storage,
                &mut made,
                (owner, fixer),
                &[*issue],
                (Vec::new(), Vec::new()),
            )
            .await?;
            hand_overs.push(record);
            works.push(kept[0].clone());
        }
        let count = async |at_second: i64| storage.open_work_count_of(fixer, at(at_second)).await;
        // All six are open: waiting for the agent counts.
        assert_eq!(count(NOW).await?, 6);
        // Closed by a review, stopped, and given up: not open any more.
        assert!(close(storage, &mut made, works[0].id, 1).await?);
        set_state(
            storage,
            &mut made,
            works[1].id,
            AgentWorkState::Stopped,
            None,
        )
        .await?;
        set_state(
            storage,
            &mut made,
            works[2].id,
            AgentWorkState::AgentGaveUp,
            Some("no steps"),
        )
        .await?;
        assert_eq!(count(NOW).await?, 3);
        // A hand-over that ended gives no open work.
        end(storage, &mut made, hand_overs[3].id).await?;
        assert_eq!(count(NOW).await?, 2);
        // A hand-over past its end gives none either, before the job that ends it has run.
        assert_eq!(count(ENDS - 1).await?, 2);
        assert_eq!(count(ENDS).await?, 0);
        assert_eq!(count(ENDS + 1).await?, 0);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn m7_f234_the_projects_to_look_at_are_those_of_rules_that_hand_over_and_work() -> TestResult
{
    each_database("m7_f234_rule_projects", async |db| {
        use bb_core::agent_records::{GivingState, StandingRule};
        use bb_core::agent_work::RuleWay;
        use bb_core::id::{SavedSearchId, StandingRuleId};
        let Given {
            fixture,
            data,
            owner,
            fixer,
        } = given(db).await?;
        let storage = &fixture.storage;
        let mut made = Made::new();
        let none = storage.hand_over_rule_projects(fixer, at(NOW)).await?;
        assert!(none.is_empty());
        // A rule that proposes, one that ended and one that ran out do not count; two rules that
        // hand over in the project count once, one of them with no end.
        let specs = [
            (RuleWay::Propose, GivingState::Open, Some(at(ENDS))),
            (RuleWay::HandOver, GivingState::Ended, Some(at(ENDS))),
            (RuleWay::HandOver, GivingState::Open, Some(at(NOW - 1))),
            (RuleWay::HandOver, GivingState::Open, Some(at(ENDS))),
            (RuleWay::HandOver, GivingState::Open, None),
        ];
        let mut writes = Vec::new();
        for (way, state, ends_at) in specs {
            writes.push(StandingRule {
                id: StandingRuleId::from_random(made.bytes()),
                agent: fixer,
                person: owner,
                search: SavedSearchId::generate(&mut made.source)
                    .map_err(|_| crate::StoreError::Corrupt("a saved search ID"))?,
                project: data.project,
                way,
                powers: Vec::new(),
                accept_by_itself: false,
                fields: Vec::new(),
                creates_in: None,
                state,
                ends_at,
                made_at: at(0),
                version: 1,
            });
        }
        let step = made.step + 1;
        storage
            .record_change(&event(&mut made.source, step), move |transaction| {
                Box::pin(async move {
                    for one in &writes {
                        AuthStorage::insert_standing_rule(transaction, one).await?;
                    }
                    Ok(())
                })
            })
            .await?;
        let found = storage.hand_over_rule_projects(fixer, at(NOW)).await?;
        assert_eq!(found, vec![data.project]);
        Ok(())
    })
    .await
}
