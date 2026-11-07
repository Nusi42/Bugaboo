//! More of agent work in the store, on both databases: the states a search finds, the work and the
//! proposals of an issue.

use bb_app::agent_work::{AgentWorkRecords as _, GrantStore as _};
use bb_core::agent_records::{Proposal, ProposalState};
use bb_core::agent_work::{AgentWorkState, RuleField};
use bb_core::id::{AgentId, AgentNoteId, AgentWorkId, IssueId};
use bb_testing::{TestResult, each_database};

use super::tests::request_with;
use super::tests_agent::{agent_fields, an_agent};
use super::tests_agent_work::{ENDS, Given, Made, given, hand_over, rule_that_found};
use super::tests_more::titles_of;
use crate::AuthStorage;
use crate::test_support::{at, event};
use bb_app::ports::ChangeStore as _;

#[tokio::test]
async fn ai_33_agent_state_finds_every_state_until_a_review_and_agent_only_who_still_works()
-> TestResult {
    each_database("ai_33_agent_state_forms", async |db| {
        let Given {
            fixture,
            data,
            owner,
            fixer,
        } = given(db).await?;
        let storage = &fixture.storage;
        let mut made = Made::new();
        let issues: Vec<IssueId> = data.issues.iter().take(4).map(|issue| issue.id).collect();
        let (_, works) = hand_over(
            storage,
            &mut made,
            (owner, fixer),
            &issues,
            (Vec::new(), Vec::new()),
        )
        .await?;
        // Waiting for you, stopped, gave up, closed by a review.
        made.step += 1;
        let step = made.step;
        let ids: Vec<AgentWorkId> = works.iter().map(|work| work.id).collect();
        storage
            .record_change(&event(&mut made.source, step), move |transaction| {
                Box::pin(async move {
                    let states = [
                        AgentWorkState::WaitingForYou,
                        AgentWorkState::Stopped,
                        AgentWorkState::AgentGaveUp,
                        AgentWorkState::ReadyForReview,
                    ];
                    for (id, state) in ids.iter().zip(states) {
                        AuthStorage::set_work_state(transaction, *id, state, None, false, 1)
                            .await?;
                    }
                    AuthStorage::close_work(transaction, ids[3], false, false, 2).await?;
                    Ok(())
                })
            })
            .await?;
        let titled = |index: usize| data.issues[index].title.clone();
        let asked = request_with(
            &data,
            "agent state is \"waiting-for-you\"",
            None,
            100,
            agent_fields(),
            Vec::new(),
        );
        assert_eq!(titles_of(storage, &asked).await?, [titled(0)]);
        let stopped = request_with(
            &data,
            "agent state is \"stopped\"",
            None,
            100,
            agent_fields(),
            Vec::new(),
        );
        assert_eq!(titles_of(storage, &stopped).await?, [titled(1)]);
        // *Agent* is who works now: not on work that stopped or gave up, nor on closed work.
        let working = request_with(
            &data,
            "agent is Fixer",
            None,
            100,
            agent_fields(),
            Vec::new(),
        );
        assert_eq!(titles_of(storage, &working).await?, [titled(0)]);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ai_33_the_work_on_an_issue_is_cut_in_the_statement_and_to_the_agent() -> TestResult {
    each_database("ai_33_work_on_issue", async |db| {
        let Given {
            fixture,
            data,
            owner,
            fixer,
        } = given(db).await?;
        let storage = &fixture.storage;
        let helper = an_agent(storage, owner, "Helper", 2).await?;
        let mut made = Made::new();
        let issue = data.issues[0].id;
        for agent in [fixer, helper] {
            hand_over(
                storage,
                &mut made,
                (owner, agent),
                &[issue],
                (Vec::new(), Vec::new()),
            )
            .await?;
        }
        // The work on an issue: newest first, cut in the statement, only the agent's own.
        let all = storage.work_on_issue(issue, None, 10).await?;
        assert_eq!(all.len(), 2);
        assert_eq!(storage.work_on_issue(issue, None, 1).await?.len(), 1);
        let own = storage.work_on_issue(issue, Some(fixer), 10).await?;
        assert_eq!(own.len(), 1);
        assert_eq!(own[0].agent, fixer);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ai_32_the_proposals_on_an_issue_are_cut_in_the_statement_by_state_agent_and_number()
-> TestResult {
    each_database("ai_32_proposals_on", async |db| {
        let Given {
            fixture,
            data,
            owner,
            fixer,
        } = given(db).await?;
        let storage = &fixture.storage;
        let helper = an_agent(storage, owner, "Helper", 2).await?;
        let mut made = Made::new();
        let issue = data.issues[0].id;
        let rule = rule_that_found(
            storage,
            &mut made,
            (owner, fixer),
            &data,
            &[issue],
            at(ENDS),
        )
        .await?;
        let proposal = |made: &mut Made, agent: AgentId, state: ProposalState| Proposal {
            id: AgentNoteId::from_random(made.bytes()),
            rule: rule.id,
            agent,
            issue,
            field: RuleField::Type,
            before: serde_json::Value::Null,
            after: serde_json::json!("x"),
            state,
            decided_by: None,
            made_at: at(3),
        };
        let all = [
            proposal(&mut made, fixer, ProposalState::Open),
            proposal(&mut made, fixer, ProposalState::Open),
            proposal(&mut made, helper, ProposalState::Open),
            proposal(&mut made, fixer, ProposalState::Discarded),
        ];
        made.step += 1;
        let step = made.step;
        let kept = all.to_vec();
        storage
            .record_change(&event(&mut made.source, step), move |transaction| {
                Box::pin(async move {
                    for each in &kept {
                        AuthStorage::insert_proposal(transaction, each).await?;
                    }
                    Ok(())
                })
            })
            .await?;
        let open = Some(ProposalState::Open);
        assert_eq!(storage.proposals_on(issue, open, None, 10).await?.len(), 3);
        assert_eq!(storage.proposals_on(issue, None, None, 10).await?.len(), 4);
        let cut = storage.proposals_on(issue, open, None, 2).await?;
        assert_eq!(cut.len(), 2, "cut in the statement");
        assert!(cut[0].id > cut[1].id, "newest first");
        let own = storage.proposals_on(issue, open, Some(helper), 10).await?;
        assert_eq!(own.len(), 1);
        assert_eq!(own[0].agent, helper);
        Ok(())
    })
    .await
}
