//! Agent work in the store, on both databases (AI-31 to AI-35, ADR-0029): which work is open, what
//! an agent may change on each issue of it, what its searches reach, and what the writes of the
//! work do to versions. The use cases are tested on the fake store; what only the real store
//! decides is tested here.

use bb_app::agent_work::{AgentWorkRecords as _, GrantStore as _};
use bb_app::agents::{AgentReach, AgentWorkStore as _, Hold};
use bb_app::search::WorkOf;
use bb_core::agent_records::{
    Attempt, AttemptResult, GivingState, HandOver, IssueWork, Question, ResultKind, Review,
    RuleMatch, StandingRule,
};
use bb_core::agent_work::{AgentWorkState, CheckResult, HandOverPower, ReviewAnswer, RuleField};
use bb_core::id::{
    AgentId, AgentNoteId, AgentWorkId, AttemptId, HandOverId, IssueId, SavedSearchId,
    SequentialIds, StandingRuleId, UserId,
};
use bb_testing::{Failure, TestResult, each_database};
use jiff::Timestamp;

use super::tests::{Data, request_with, seed};
use super::tests_agent::{a_stored_person, agent_fields, an_agent};
use super::tests_more::titles_of;
use crate::AuthStorage;
use crate::test_support::{at, event};
use bb_app::issues::IssueStore as _;
use bb_app::ports::ChangeStore as _;

/// The moment the tests ask at, and when the hand-overs and rules made here end.
pub(super) const NOW: i64 = 100;
pub(super) const ENDS: i64 = 1_000;

/// Ids that differ for every record, from a counter.
pub(super) struct Made {
    pub(super) source: SequentialIds,
    pub(super) step: i64,
}

impl Made {
    pub(super) fn new() -> Self {
        Self {
            source: SequentialIds::default(),
            step: 1_000,
        }
    }

    pub(super) fn bytes(&mut self) -> [u8; 16] {
        self.step += 1;
        let mut bytes = [0_u8; 16];
        bytes[8..].copy_from_slice(&self.step.to_be_bytes());
        bytes
    }
}

/// The storage of a test, the issues of the search tests, a person and one agent of theirs.
pub(super) struct Given {
    pub(super) fixture: crate::test_support::Fixture,
    pub(super) data: Data,
    pub(super) owner: UserId,
    pub(super) fixer: AgentId,
}

pub(super) async fn given(
    db: &bb_testing::TestDatabase,
) -> Result<Given, Box<dyn std::error::Error + Send + Sync>> {
    let fixture = crate::test_support::fixture(db).await?;
    let data = seed(&fixture.storage).await;
    let owner = a_stored_person(&fixture.storage).await?;
    let fixer = an_agent(&fixture.storage, owner, "Fixer", 1).await?;
    Ok(Given {
        fixture,
        data,
        owner,
        fixer,
    })
}

/// A hand-over of the agent for these issues, with its work; the powers and fields ticked.
pub(super) async fn hand_over(
    storage: &AuthStorage,
    made: &mut Made,
    (person, agent): (UserId, AgentId),
    issues: &[IssueId],
    ticked: (Vec<HandOverPower>, Vec<String>),
) -> Result<(HandOver, Vec<IssueWork>), crate::StoreError> {
    let record = HandOver {
        id: HandOverId::from_random(made.bytes()),
        agent,
        person,
        note: "Look into it".to_owned(),
        powers: ticked.0,
        fields: ticked.1,
        budget_attempts: 3,
        budget_minutes: None,
        budget_cost: None,
        plan_needs_approval: false,
        accept_by_itself: false,
        state: GivingState::Open,
        ends_at: at(ENDS),
        made_at: at(0),
        version: 1,
    };
    let mut works: Vec<IssueWork> = Vec::new();
    for issue in issues {
        let project = storage
            .issue(*issue)
            .await?
            .ok_or(crate::StoreError::Corrupt("an issue of a test"))?
            .project;
        works.push(IssueWork {
            id: AgentWorkId::from_random(made.bytes()),
            issue: *issue,
            project,
            hand_over: record.id,
            agent,
            person,
            state: AgentWorkState::WaitingForTheAgent,
            gave_up: None,
            stopped_for: None,
            closed: false,
            accepted_by_itself: false,
            status_pending: false,
            made_at: at(0),
            version: 1,
        });
    }
    let (stored, kept) = (record.clone(), works.clone());
    let step = made.step;
    storage
        .record_change(&event(&mut made.source, step), move |transaction| {
            Box::pin(async move {
                AuthStorage::insert_hand_over(transaction, &stored).await?;
                for work in &kept {
                    AuthStorage::insert_work(transaction, work).await?;
                }
                Ok(())
            })
        })
        .await?;
    Ok((record, works))
}

/// A standing rule of the agent that found these issues.
pub(super) async fn rule_that_found(
    storage: &AuthStorage,
    made: &mut Made,
    (person, agent): (UserId, AgentId),
    data: &Data,
    found: &[IssueId],
    ends_at: Timestamp,
) -> Result<StandingRule, crate::StoreError> {
    let rule = StandingRule {
        id: StandingRuleId::from_random(made.bytes()),
        agent,
        person,
        search: SavedSearchId::generate(&mut made.source)
            .map_err(|_| crate::StoreError::Corrupt("a saved search ID"))?,
        project: data.project,
        fields: vec![RuleField::Type],
        creates_in: None,
        state: GivingState::Open,
        ends_at,
        made_at: at(0),
        version: 1,
    };
    let (stored, issues) = (rule.clone(), found.to_vec());
    let step = made.step;
    storage
        .record_change(&event(&mut made.source, step), move |transaction| {
            Box::pin(async move {
                AuthStorage::insert_standing_rule(transaction, &stored).await?;
                for issue in &issues {
                    AuthStorage::insert_match(
                        transaction,
                        &RuleMatch {
                            rule: stored.id,
                            issue: *issue,
                            at: at(1),
                        },
                    )
                    .await?;
                }
                Ok(())
            })
        })
        .await?;
    Ok(rule)
}

/// Closes a piece of work, as a review does; whether it was at `version`.
pub(super) async fn close(
    storage: &AuthStorage,
    made: &mut Made,
    work: AgentWorkId,
    version: i64,
) -> Result<bool, crate::StoreError> {
    made.step += 1;
    let step = made.step;
    let closed = std::sync::Arc::new(std::sync::Mutex::new(false));
    let seen = std::sync::Arc::clone(&closed);
    storage
        .record_change(&event(&mut made.source, step), move |transaction| {
            Box::pin(async move {
                let done =
                    AuthStorage::close_work(transaction, work, false, false, version).await?;
                *seen
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = done;
                Ok(())
            })
        })
        .await?;
    let done = *closed
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Ok(done)
}

/// What the agent holds on these issues, as a read asks for them.
async fn holds(
    storage: &AuthStorage,
    agent: AgentId,
    now: Timestamp,
    issues: &[IssueId],
) -> Result<std::collections::BTreeMap<IssueId, Hold>, Failure> {
    Ok(storage.agent_holds(agent, now, issues.to_vec()).await?)
}

#[tokio::test]
async fn ai_33_closed_work_is_not_reached_and_a_hand_over_of_two_issues_keeps_the_other()
-> TestResult {
    each_database("ai_33_closed_work", async |db| {
        let Given {
            fixture,
            data,
            owner,
            fixer,
        } = given(db).await?;
        let storage = &fixture.storage;
        let mut made = Made::new();
        let (_, works) = hand_over(
            storage,
            &mut made,
            (owner, fixer),
            &[data.issues[0].id, data.issues[1].id],
            (Vec::new(), Vec::new()),
        )
        .await?;
        let now = at(NOW);
        let two = [data.issues[0].id, data.issues[1].id];
        let reached = holds(storage, fixer, now, &two).await?;
        assert_eq!(reached.len(), 2);
        // A review accepts the first: the hand-over stays open for the second.
        assert!(close(storage, &mut made, works[0].id, 1).await?);
        assert!(
            !close(storage, &mut made, works[0].id, 1).await?,
            "a second close on the old version finds the work changed"
        );
        let reached = holds(storage, fixer, now, &two).await?;
        assert_eq!(
            reached.keys().copied().collect::<Vec<_>>(),
            [data.issues[1].id],
            "the closed work is not the agent's any more, the other piece is"
        );
        assert!(reached[&data.issues[1].id].handed);
        assert!(
            storage
                .open_work_of(fixer, data.issues[0].id)
                .await?
                .is_none(),
            "no open work on the closed issue"
        );
        assert!(
            storage
                .open_work_of(fixer, data.issues[1].id)
                .await?
                .is_some()
        );
        // The issue can be handed over again: the closed work is not open work.
        hand_over(
            storage,
            &mut made,
            (owner, fixer),
            &[data.issues[0].id],
            (Vec::new(), Vec::new()),
        )
        .await?;
        let reached = holds(storage, fixer, now, &two).await?;
        assert_eq!(reached.len(), 2);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ai_31_an_agent_has_one_open_work_for_an_issue_whatever_two_requests_do() -> TestResult {
    each_database("ai_31_one_open_work", async |db| {
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
        let (_, first) = hand_over(
            storage,
            &mut made,
            (owner, fixer),
            &[issue],
            (Vec::new(), Vec::new()),
        )
        .await?;
        // A second open piece of work of the same agent for the issue is refused by the index.
        let second = hand_over(
            storage,
            &mut made,
            (owner, fixer),
            &[issue],
            (Vec::new(), Vec::new()),
        )
        .await;
        assert!(second.is_err(), "two open pieces of work for one issue");
        // Another agent may work on it too.
        hand_over(
            storage,
            &mut made,
            (owner, helper),
            &[issue],
            (Vec::new(), Vec::new()),
        )
        .await?;
        // Work that ended does not count: stop it, and the issue can be handed over again.
        made.step += 1;
        let step = made.step;
        let id = first[0].id;
        storage
            .record_change(&event(&mut made.source, step), move |transaction| {
                Box::pin(async move {
                    AuthStorage::stop_work_of_agent(
                        transaction,
                        fixer,
                        bb_core::agent_work::StopCause::AgentStopped,
                    )
                    .await?;
                    let _ = id;
                    Ok(())
                })
            })
            .await?;
        hand_over(
            storage,
            &mut made,
            (owner, fixer),
            &[issue],
            (Vec::new(), Vec::new()),
        )
        .await?;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ai_31_the_allowance_follows_the_ticks_and_ends_with_the_hand_over_and_the_rule()
-> TestResult {
    each_database("ai_31_allowance", async |db| {
        let Given {
            fixture,
            data,
            owner,
            fixer,
        } = given(db).await?;
        let storage = &fixture.storage;
        let mut made = Made::new();
        let [a, b, c, d] = [0, 1, 2, 3].map(|at| data.issues[at].id);
        hand_over(
            storage,
            &mut made,
            (owner, fixer),
            &[a],
            (
                vec![HandOverPower::ChangeFields, HandOverPower::ChangeStatus],
                vec!["title".to_owned(), "priority".to_owned()],
            ),
        )
        .await?;
        // Nothing ticked: it reads and reports, and changes nothing.
        hand_over(
            storage,
            &mut made,
            (owner, fixer),
            &[b],
            (Vec::new(), vec!["title".to_owned()]),
        )
        .await?;
        // Sub-issues ticked.
        hand_over(
            storage,
            &mut made,
            (owner, fixer),
            &[c],
            (vec![HandOverPower::CreateSubIssues], Vec::new()),
        )
        .await?;
        rule_that_found(storage, &mut made, (owner, fixer), &data, &[d], at(ENDS)).await?;
        let four = [a, b, c, d];
        let reached = holds(storage, fixer, at(NOW), &four).await?;
        let of = |issue: IssueId| {
            reached
                .get(&issue)
                .map(|hold| hold.allowance.clone())
                .unwrap_or_default()
        };
        assert_eq!(of(a).fields, ["title", "priority"]);
        assert!(of(a).status && !of(a).sub_issues);
        assert!(of(b).fields.is_empty() && !of(b).status, "ticks not given");
        assert!(of(c).sub_issues && of(c).fields.is_empty());
        assert_eq!(of(d).fields, ["issue-type"], "a rule gives its fields only");
        assert_eq!(reached.len(), 4);
        assert!(!reached[&d].handed, "a rule hands nothing over");
        assert!(
            storage
                .agent_reach(fixer, at(NOW))
                .await?
                .projects
                .contains(&data.project)
        );
        // After the end of the hand-overs and the rule the agent reaches nothing.
        assert!(holds(storage, fixer, at(ENDS + 1), &four).await?.is_empty());
        assert_eq!(
            storage.agent_reach(fixer, at(ENDS + 1)).await?,
            AgentReach::default()
        );
        // A rule that was ended is not read.
        made.step += 1;
        let step = made.step;
        storage
            .record_change(&event(&mut made.source, step), move |transaction| {
                Box::pin(async move { AuthStorage::end_grants_of_agent(transaction, fixer).await })
            })
            .await?;
        assert!(holds(storage, fixer, at(NOW), &four).await?.is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ai_33_a_search_of_an_agent_is_cut_to_its_open_work_and_its_rules() -> TestResult {
    each_database("ai_33_cut_to_work", async |db| {
        let Given {
            fixture,
            data,
            owner,
            fixer,
        } = given(db).await?;
        let storage = &fixture.storage;
        let mut made = Made::new();
        let [a, b, c, d] = [0, 1, 2, 3].map(|at| data.issues[at].id);
        let (_, works) = hand_over(
            storage,
            &mut made,
            (owner, fixer),
            &[a, b],
            (Vec::new(), Vec::new()),
        )
        .await?;
        hand_over(
            storage,
            &mut made,
            (owner, fixer),
            &[c],
            (Vec::new(), Vec::new()),
        )
        .await?;
        rule_that_found(storage, &mut made, (owner, fixer), &data, &[d], at(ENDS)).await?;
        let expired = rule_that_found(
            storage,
            &mut made,
            (owner, fixer),
            &data,
            &[data.issues[4].id],
            at(NOW - 1),
        )
        .await?;
        let _ = expired;
        close(storage, &mut made, works[0].id, 1).await?;
        let mut asked = request_with(&data, "", None, 100, agent_fields(), Vec::new());
        asked.only = Some(WorkOf {
            agent: fixer,
            at: at(NOW),
        });
        let mut found = titles_of(storage, &asked).await?;
        found.sort();
        let mut expected: Vec<String> = [b, c, d]
            .iter()
            .map(|id| {
                data.issues
                    .iter()
                    .find(|issue| issue.id == *id)
                    .map(|issue| issue.title.clone())
                    .unwrap_or_default()
            })
            .collect();
        expected.sort();
        assert_eq!(
            found, expected,
            "open work and a rule that works; not closed work, not an ended rule"
        );
        // A person's search is not cut.
        asked.only = None;
        assert!(titles_of(storage, &asked).await?.len() > expected.len());
        Ok(())
    })
    .await
}

/// An attempt with a check, a question and a review for the pieces of work: the first piece has the
/// first three, the second the review.
fn records_of(
    made: &mut Made,
    mine: &[IssueWork],
    owner: UserId,
) -> (Attempt, AttemptResult, Question, Review) {
    let attempt = Attempt {
        id: AttemptId::from_random(made.bytes()),
        work: mine[0].id,
        number: 1,
        plan: Some("Plan".to_owned()),
        plan_approved: true,
        reported_minutes: 5,
        reported_cost: 7,
        started_at: at(2),
        ended_at: None,
        run: None,
    };
    let result = AttemptResult {
        id: AgentNoteId::from_random(made.bytes()),
        attempt: attempt.id,
        kind: ResultKind::Check,
        label: "Tests".to_owned(),
        link: None,
        outcome: Some(CheckResult::Passed),
    };
    let question = Question {
        id: AgentNoteId::from_random(made.bytes()),
        work: mine[0].id,
        text: "Which browser?".to_owned(),
        answer: Some("Firefox".to_owned()),
        answered_by: Some(owner),
        answered_at: Some(at(4)),
        asked_at: at(3),
    };
    let review = Review {
        id: AgentNoteId::from_random(made.bytes()),
        work: mine[1].id,
        answer: ReviewAnswer::SendBack,
        note: Some("Again".to_owned()),
        by: Some(owner),
        at: at(4),
    };
    (attempt, result, question, review)
}

#[tokio::test]
async fn ai_33_the_details_of_a_page_of_work_are_read_together_and_in_order() -> TestResult {
    each_database("ai_33_details", async |db| {
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
        let (_, mine) = hand_over(
            storage,
            &mut made,
            (owner, fixer),
            &[issue, data.issues[1].id],
            (Vec::new(), Vec::new()),
        )
        .await?;
        let (_, theirs) = hand_over(
            storage,
            &mut made,
            (owner, helper),
            &[issue],
            (Vec::new(), Vec::new()),
        )
        .await?;
        let (attempt, result, question, review) = records_of(&mut made, &mine, owner);
        made.step += 1;
        let step = made.step;
        let (a, r, q, v) = (
            attempt.clone(),
            result.clone(),
            question.clone(),
            review.clone(),
        );
        storage
            .record_change(&event(&mut made.source, step), move |transaction| {
                Box::pin(async move {
                    AuthStorage::insert_attempt(transaction, &a).await?;
                    AuthStorage::insert_result(transaction, &r).await?;
                    AuthStorage::insert_question(transaction, &q).await?;
                    AuthStorage::insert_review(transaction, &v).await
                })
            })
            .await?;
        let details = storage
            .details_of_works(vec![mine[0].clone(), mine[1].clone()])
            .await?;
        assert_eq!(details.hand_overs.len(), 1, "one hand-over of both");
        assert_eq!(details.attempts, [attempt]);
        assert_eq!(details.results, [result]);
        assert_eq!(details.questions, [question]);
        assert_eq!(details.reviews, [review]);
        let none = storage.details_of_works(vec![theirs[0].clone()]).await?;
        assert!(none.attempts.is_empty() && none.questions.is_empty() && none.reviews.is_empty());
        assert!(
            storage
                .details_of_works(Vec::new())
                .await?
                .hand_overs
                .is_empty()
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ai_33_writes_of_work_count_versions_and_end_only_what_is_open() -> TestResult {
    each_database("ai_33_versions", async |db| {
        let Given {
            fixture,
            data,
            owner,
            fixer,
        } = given(db).await?;
        let storage = &fixture.storage;
        let mut made = Made::new();
        let (record, works) = hand_over(
            storage,
            &mut made,
            (owner, fixer),
            &[data.issues[0].id],
            (Vec::new(), Vec::new()),
        )
        .await?;
        assert!(storage.agent_has_open_giving(fixer).await?);
        // Looked at: counts the version up on the version read, and only then.
        made.step += 1;
        let step = made.step;
        let id = works[0].id;
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let kept = std::sync::Arc::clone(&seen);
        storage
            .record_change(&event(&mut made.source, step), move |transaction| {
                Box::pin(async move {
                    let first = AuthStorage::mark_work_seen(transaction, id, 1).await?;
                    let again = AuthStorage::mark_work_seen(transaction, id, 1).await?;
                    kept.lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .extend([first, again]);
                    Ok(())
                })
            })
            .await?;
        assert_eq!(
            *seen
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
            [true, false]
        );
        assert_eq!(storage.issue_work(id).await?.map(|w| w.version), Some(2));
        // Ending the grants counts the versions up once, and leaves what ended alone.
        for _ in 0..2 {
            made.step += 1;
            let step = made.step;
            storage
                .record_change(&event(&mut made.source, step), move |transaction| {
                    Box::pin(
                        async move { AuthStorage::end_grants_of_agent(transaction, fixer).await },
                    )
                })
                .await?;
        }
        let ended = storage.hand_over(record.id).await?.ok_or("no hand-over")?;
        assert_eq!(ended.state, GivingState::Ended);
        assert_eq!(ended.version, 2, "counted up once, not again");
        Ok(())
    })
    .await
}
