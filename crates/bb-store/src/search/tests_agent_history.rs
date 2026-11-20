//! The history of an agent in the store, on both databases (AI-37, ADR-0029, amendment M9): the
//! hand-overs it made and the undos of its runs, newest first and then by ID, only those it made
//! for its person.

use bb_app::agent_work::AgentWorkRecords as _;
use bb_app::agents::AgentStore as _;
use bb_app::many_changes::ManyChangeStore as _;
use bb_core::agent_records::Attempt;
use bb_core::event::{Actor, Channel, DomainEvent, EntityKind, EntityRef, FieldChange, RecordKind};
use bb_core::id::{AttemptId, EntityId, EventId, RunId, UserId};
use bb_core::many_changes::{ManyChangeRun, ManyChangeStatus};
use bb_core::run::RunKind;
use bb_testing::{TestResult, each_database};
use serde_json::json;

use super::tests_agent::an_agent;
use super::tests_agent_work::{Made, given, hand_over, hand_over_by};
use crate::AuthStorage;
use crate::test_support::at;
use bb_app::ports::ChangeStore as _;

/// An event on the run with the count an undo writes, by `actor`, with `reason`.
fn undo_event(
    made: &mut Made,
    run: RunId,
    actor: Actor,
    reason: Option<String>,
) -> Result<DomainEvent, bb_core::id::IdError> {
    Ok(DomainEvent {
        id: EventId::generate(&mut made.source)?,
        time: at(50),
        actor,
        channel: Channel::Api,
        entity: EntityRef {
            kind: EntityKind::from(RecordKind::ChangeRun),
            id: EntityId::from(run),
        },
        changes: vec![FieldChange {
            field: "undone".to_owned(),
            before: None,
            after: Some(json!(2)),
        }],
        reason,
        run: None,
    })
}

/// The run of Fixer's attempt on the work, undone by Fixer once and by the person once; the event
/// of Fixer's undo.
async fn run_undone_twice(
    storage: &AuthStorage,
    made: &mut Made,
    (owner, fixer): (UserId, bb_core::id::AgentId),
    work: bb_core::id::AgentWorkId,
) -> Result<DomainEvent, Box<dyn std::error::Error + Send + Sync>> {
    let run = RunId::generate(&mut made.source)?;
    let stored = ManyChangeRun {
        id: run,
        kind: RunKind::Attempt,
        person: owner,
        app: None,
        agent: Some(fixer),
        action: json!({}),
        items: Vec::new(),
        status: ManyChangeStatus::Done,
        done: 0,
        changed: 0,
        skipped: Vec::new(),
        created_at: at(10),
        finished_at: None,
    };
    let attempt = Attempt {
        id: AttemptId::from_random(made.bytes()),
        work,
        number: 1,
        plan: None,
        plan_approved: false,
        reported_minutes: 0,
        reported_cost: 0,
        started_at: at(10),
        ended_at: None,
        run: Some(run),
    };
    let reason = Some(bb_core::run::RUN_UNDONE.to_owned());
    let by_fixer = Actor::Agent {
        agent: fixer,
        person: owner,
    };
    let opened = undo_event(made, run, by_fixer, None)?;
    storage
        .record_change(&opened, move |transaction| {
            Box::pin(async move {
                AuthStorage::insert_many_changes_run(transaction, &stored).await?;
                AuthStorage::insert_attempt(transaction, &attempt).await
            })
        })
        .await?;
    let undone = undo_event(made, run, by_fixer, reason.clone())?;
    let by_person = undo_event(made, run, Actor::Person(owner), reason)?;
    for event in [&undone, &by_person] {
        storage
            .record_change(event, |_| Box::pin(async { Ok(()) }))
            .await?;
    }
    Ok(undone)
}

#[tokio::test]
async fn ai_37_the_store_lists_the_hand_overs_and_undos_of_an_agent() -> TestResult {
    each_database("ai_37_agent_history", async |db| {
        let given = given(db).await?;
        let storage = &given.fixture.storage;
        let (owner, fixer) = (given.owner, given.fixer);
        let helper = an_agent(storage, owner, "Helper", 2).await?;
        let issues: Vec<_> = given.data.issues.iter().map(|issue| issue.id).collect();
        let mut made = Made::new();
        let none = || (Vec::new(), Vec::new());

        // The person's own hand-over to Fixer, which Fixer did not make, and Fixer's two.
        let (_, mine) = hand_over(storage, &mut made, (owner, fixer), &issues[..1], none()).await?;
        let (first, _) = hand_over_by(
            storage,
            &mut made,
            (owner, helper),
            &issues[1..3],
            none(),
            Some(fixer),
        )
        .await?;
        let (second, _) = hand_over_by(
            storage,
            &mut made,
            (owner, helper),
            &issues[3..4],
            none(),
            Some(fixer),
        )
        .await?;

        let undone = run_undone_twice(storage, &mut made, (owner, fixer), mine[0].id).await?;

        let handed = storage.hand_overs_by_agent(fixer, owner, None, 10).await?;
        let mut expected = vec![first.id, second.id];
        expected.sort_by_key(|id| id.to_bytes());
        assert_eq!(
            handed.iter().map(|each| each.id).collect::<Vec<_>>(),
            expected
        );
        let of_first = handed
            .iter()
            .find(|each| each.id == first.id)
            .ok_or("the first hand-over")?;
        assert_eq!(of_first.to, helper);
        assert_eq!(of_first.issues, issues[1..3].to_vec());

        // A page at a time, by the moment and the ID.
        let page = storage.hand_overs_by_agent(fixer, owner, None, 1).await?;
        let last = page.first().ok_or("a page of one")?;
        let after = Some((last.at, EntityId::from(last.id)));
        let rest = storage.hand_overs_by_agent(fixer, owner, after, 10).await?;
        assert_eq!(
            rest.iter().map(|each| each.id).collect::<Vec<_>>(),
            expected[1..]
        );

        let undos = storage.undos_by_agent(fixer, owner, None, 10).await?;
        assert_eq!(undos.len(), 1, "only the undo Fixer made: {undos:?}");
        assert_eq!(undos[0].event.id, undone.id);
        assert_eq!(undos[0].issue, Some(issues[0]));
        let after_it = Some((undone.time, EntityId::from(undone.id)));
        assert!(
            storage
                .undos_by_agent(fixer, owner, after_it, 10)
                .await?
                .is_empty()
        );

        // What the agent did for another person is not in this person's history.
        let other = UserId::generate(&mut made.source);
        assert!(
            storage
                .hand_overs_by_agent(fixer, other, None, 10)
                .await?
                .is_empty()
        );
        assert!(
            storage
                .undos_by_agent(fixer, other, None, 10)
                .await?
                .is_empty()
        );
        Ok(())
    })
    .await
}
