//! More of agent work in the store, on both databases: the states a search finds, the work and the
//! proposals of an issue.

use bb_app::agent_work::{AgentWorkRecords as _, Closing, GrantStore as _, close_work_of};
use bb_core::agent_records::{GivingState, Proposal, ProposalState};
use bb_core::agent_work::{AgentWorkState, RuleField};
use bb_core::id::{AgentId, AgentNoteId, AgentWorkId, HandOverId, IssueId};
use std::sync::Arc;
use std::time::Duration;

use bb_testing::{DatabaseKind, TestResult, each_database};
use sea_query::{Alias, Expr, ExprTrait as _, Query};
use tokio::sync::Notify;

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

/// Closes a piece of work as a review does, in a change of its own; whether it went through. With
/// `hold`, the change stays open after the close until it is told to go on.
async fn close_piece(
    storage: &AuthStorage,
    at: bb_core::event::DomainEvent,
    hand_over: HandOverId,
    work: AgentWorkId,
    hold: Option<(Arc<Notify>, Arc<Notify>)>,
) -> bool {
    storage
        .record_change(&at, move |transaction| {
            Box::pin(async move {
                let version = AuthStorage::issue_work_in(transaction, work)
                    .await?
                    .map_or(1, |found| found.version);
                let closing = Closing {
                    work,
                    version,
                    accepted_by_itself: false,
                    status_pending: false,
                };
                close_work_of::<AuthStorage>(transaction, hand_over, closing).await?;
                if let Some((entered, release)) = hold {
                    entered.notify_one();
                    release.notified().await;
                }
                Ok(())
            })
        })
        .await
        .is_ok()
}

/// Waits until the second change is really held up by the first: on PostgreSQL until a session
/// waits for a lock (seen in `pg_stat_activity`), on SQLite, which gives no such view, for a moment
/// longer than the second needs to begin.
async fn wait_until_blocked(storage: &AuthStorage, kind: DatabaseKind) {
    if kind != DatabaseKind::Postgres {
        tokio::time::sleep(Duration::from_millis(400)).await;
        return;
    }
    let mut waiting = Query::select();
    waiting
        .column(Alias::new("pid"))
        .from(Alias::new("pg_stat_activity"))
        .and_where(Expr::col(Alias::new("wait_event_type")).eq("Lock"));
    for _ in 0..200 {
        let rows = storage.database().fetch_all(&waiting).await;
        if rows.is_ok_and(|found| !found.is_empty()) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[tokio::test]
async fn ai_35_two_closes_of_the_two_last_pieces_at_once_leave_no_hand_over_open() -> TestResult {
    each_database("ai_35_parallel_close", async |db| {
        let Given {
            fixture,
            data,
            owner,
            fixer,
        } = given(db).await?;
        let storage = &fixture.storage;
        let mut made = Made::new();
        let issues = [data.issues[0].id, data.issues[1].id];
        let (record, works) = hand_over(
            storage,
            &mut made,
            (owner, fixer),
            &issues,
            (Vec::new(), Vec::new()),
        )
        .await?;
        let (first, second) = (works[0].id, works[1].id);
        let [a, b, c] = [1, 2, 3].map(|step| {
            made.step += 1;
            event(&mut made.source, made.step + step)
        });
        // The first close stays open after its work until the second has begun: the second reads
        // the hand-over as it was, and must not end up with both pieces closed and no end.
        let (entered, release) = (Arc::new(Notify::new()), Arc::new(Notify::new()));
        let hold = Some((Arc::clone(&entered), Arc::clone(&release)));
        let begun = Arc::new(Notify::new());
        let go_on = async {
            begun.notified().await;
            wait_until_blocked(storage, db.kind()).await;
            release.notify_one();
        };
        let late = async {
            entered.notified().await;
            begun.notify_one();
            close_piece(storage, b, record.id, second, None).await
        };
        let (one, two, ()) =
            tokio::join!(close_piece(storage, a, record.id, first, hold), late, go_on);
        assert!(one, "the first close goes through");
        if db.kind() == DatabaseKind::Postgres {
            assert!(!two, "the second finds the hand-over changed under it");
            assert!(close_piece(storage, c, record.id, second, None).await);
        } else {
            assert!(two, "SQLite makes the second wait, and it sees the first");
        }
        let ended = storage.hand_over(record.id).await?.ok_or("no hand-over")?;
        assert_eq!(ended.state, GivingState::Ended, "the last close ended it");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ai_36_the_matches_of_an_agent_are_paged_by_moment_issue_and_rule() -> TestResult {
    each_database("ai_36_matches_since", async |db| {
        let Given {
            fixture,
            data,
            owner,
            fixer,
        } = given(db).await?;
        let storage = &fixture.storage;
        let mut made = Made::new();
        let found: Vec<IssueId> = data.issues.iter().map(|issue| issue.id).collect();
        assert!(found.len() > 3, "the data has issues enough for pages");
        // Two rules find the same issues at the same moment: a page may end between the two rows
        // of one issue.
        let first =
            rule_that_found(storage, &mut made, (owner, fixer), &data, &found, at(ENDS)).await?;
        let second =
            rule_that_found(storage, &mut made, (owner, fixer), &data, &found, at(ENDS)).await?;
        let mut since = (at(0), None);
        let mut given_out = Vec::new();
        loop {
            let page = storage
                .matches_of_agent_since(fixer, at(2), since, 3)
                .await?;
            let Some(last) = page.last() else { break };
            since = (last.at, Some((last.issue, last.rule)));
            given_out.extend(page.iter().map(|each| (each.issue, each.rule)));
        }
        let mut wanted: Vec<(IssueId, bb_core::id::StandingRuleId)> = found
            .iter()
            .flat_map(|issue| [(*issue, first.id), (*issue, second.id)])
            .collect();
        wanted.sort();
        assert_eq!(given_out, wanted, "every row once, none left out, in order");
        // A rule that ran out has no matches to give.
        assert!(
            storage
                .matches_of_agent_since(fixer, at(ENDS + 1), (at(0), None), 10)
                .await?
                .is_empty()
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ai_37_another_agent_in_an_attempt_on_the_issue_is_found_among_all_the_work() -> TestResult
{
    each_database("ai_37_another_in_attempt", async |db| {
        let Given {
            fixture,
            data,
            owner,
            fixer,
        } = given(db).await?;
        let storage = &fixture.storage;
        let mut made = Made::new();
        let issue = data.issues[0].id;
        let (_, works) = hand_over(
            storage,
            &mut made,
            (owner, fixer),
            &[issue],
            (Vec::new(), Vec::new()),
        )
        .await?;
        let work = works[0].id;
        let other = AgentWorkId::from_random(made.bytes());
        assert!(!storage.another_in_attempt(issue, other).await?);
        assert!(!storage.another_in_attempt(issue, work).await?);
        made.step += 1;
        let step = made.step;
        storage
            .record_change(&event(&mut made.source, step), move |transaction| {
                Box::pin(async move {
                    AuthStorage::set_work_state(
                        transaction,
                        work,
                        AgentWorkState::AgentWorking,
                        None,
                        false,
                        1,
                    )
                    .await?;
                    Ok(())
                })
            })
            .await?;
        // Another piece of work (not this one) finds the issue taken; the one that works does not.
        assert!(storage.another_in_attempt(issue, other).await?);
        assert!(!storage.another_in_attempt(issue, work).await?);
        assert_eq!(storage.working_issues_of(fixer).await?, 1);
        Ok(())
    })
    .await
}
