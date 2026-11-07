//! Tests of the events: what a change writes with them, on both databases.

use bb_core::id::IdSource as _;
use bb_testing::{TestResult, each_database};

use bb_core::id::{AgentId, RunId};

use super::*;
use crate::test_support::{at, event, ids, open};

fn name(text: &str) -> TechnicalName {
    TechnicalName::new(text).unwrap()
}

#[tokio::test]
async fn adr_0007_a_change_writes_its_event_and_outbox_entry_together() -> TestResult {
    each_database("adr_0007_change_event_outbox", async |db| {
        let database = open(db).await?;
        let mut ids = ids();
        let event = event(&mut ids, 5);
        let answer = database
            .record_change(&event, async |_transaction| Ok(42))
            .await?;
        assert_eq!(answer, 42);
        assert_eq!(
            database.events_about(&event.entity, 10).await?,
            std::slice::from_ref(&event)
        );
        let waiting = database.outbox_after(&name("history"), 10).await?;
        assert_eq!(waiting.len(), 1);
        assert_eq!(waiting[0].0.position, 1);
        assert_eq!(waiting[0].0.event_id, event.id);
        assert_eq!(waiting[0].1, event);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ai_30_an_event_by_an_agent_keeps_the_agent_and_the_person_responsible() -> TestResult {
    each_database("ai_30_event_by_agent", async |db| {
        let database = open(db).await?;
        let mut ids = ids();
        let mut event = event(&mut ids, 5);
        let agent = AgentId::generate(&mut ids);
        let person = bb_core::id::UserId::generate(&mut ids);
        event.actor = Actor::Agent { agent, person };
        database.record_change(&event, async |_| Ok(())).await?;
        let read = database.events_about(&event.entity, 10).await?;
        assert_eq!(read, std::slice::from_ref(&event));
        assert_eq!(read[0].actor.person(), Some(person));
        assert_eq!(read[0].actor.agent(), Some(agent));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn test_concurrent_events_of_a_kind_are_found_oldest_first() -> TestResult {
    each_database("test_concurrent_events_of_kind", async |db| {
        let database = open(db).await?;
        let mut ids = ids();
        let first = event(&mut ids, 5);
        let second = event(&mut ids, 6);
        for each in [&first, &second] {
            database.record_change(each, async |_| Ok(())).await?;
        }
        let found = database.events_of_kind(&first.entity.kind, 10).await?;
        assert_eq!(found, [first.clone(), second]);
        let none = EntityKind::new("comment")?;
        assert!(database.events_of_kind(&none, 10).await?.is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn adr_0007_events_are_listed_in_the_order_they_were_committed() -> TestResult {
    each_database("adr_0007_events_in_commit_order", async |db| {
        let database = open(db).await?;
        let mut ids = ids();
        // Two events of one record whose IDs are in the other order than their commits: the
        // ID has random bits, so within one millisecond it says nothing about the order.
        let smaller = event(&mut ids, 5);
        let mut bigger = event(&mut ids, 5);
        bigger.entity = smaller.entity.clone();
        assert!(smaller.id.stored() < bigger.id.stored());
        for each in [&bigger, &smaller] {
            database.record_change(each, async |_| Ok(())).await?;
        }
        let expected = [bigger.clone(), smaller.clone()];
        assert_eq!(database.events_about(&smaller.entity, 10).await?, expected);
        assert_eq!(
            database.events_of_kind(&smaller.entity.kind, 10).await?,
            expected
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn adr_0007_many_follow_up_events_get_one_position_each_in_order() -> TestResult {
    each_database("adr_0007_follow_ups_in_many_changes", async |db| {
        let database = open(db).await?;
        let mut ids = ids();
        let event = event(&mut ids, 5);
        // More than one statement holds, so the chunks must join up.
        let follow_ups: Vec<DomainEvent> = (0..250)
            .map(|at| crate::test_support::event(&mut ids, 6 + at))
            .collect();
        let queued = follow_ups.clone();
        database
            .record_change(&event, async move |transaction| {
                transaction.follow_ups.extend(queued);
                Ok(())
            })
            .await?;
        let waiting = database.outbox_after(&name("history"), 1000).await?;
        let mut expected = vec![event];
        expected.extend(follow_ups);
        let seen: Vec<(i64, DomainEvent)> = waiting
            .into_iter()
            .map(|(entry, event)| (entry.position, event))
            .collect();
        assert_eq!(
            seen.iter()
                .map(|(position, _)| *position)
                .collect::<Vec<_>>(),
            (1..=251).collect::<Vec<_>>()
        );
        assert_eq!(
            seen.into_iter().map(|(_, event)| event).collect::<Vec<_>>(),
            expected
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn bad_example_a_failed_change_leaves_no_event() -> TestResult {
    each_database("adr_0007_failed_change", async |db| {
        let database = open(db).await?;
        let mut ids = ids();
        let event = event(&mut ids, 5);
        let failed: Result<(), _> = database
            .record_change(&event, async |_transaction| Err(StoreError::Contended))
            .await;
        assert!(matches!(failed, Err(StoreError::Contended)));
        assert!(database.events_about(&event.entity, 10).await?.is_empty());
        assert!(
            database
                .outbox_after(&name("history"), 10)
                .await?
                .is_empty()
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn bad_example_the_same_event_twice_is_refused_and_takes_no_position() -> TestResult {
    each_database("adr_0007_duplicate_event", async |db| {
        let database = open(db).await?;
        let mut ids = ids();
        let first = event(&mut ids, 5);
        database.record_change(&first, async |_| Ok(())).await?;
        let again = database.record_change(&first, async |_| Ok(())).await;
        assert!(again.unwrap_err().is_duplicate());
        let second = event(&mut ids, 6);
        database.record_change(&second, async |_| Ok(())).await?;
        let positions: Vec<i64> = database
            .outbox_after(&name("history"), 10)
            .await?
            .iter()
            .map(|(entry, _)| entry.position)
            .collect();
        assert_eq!(positions, [1, 2], "no gap after the refused event");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn consumers_remember_their_position_and_never_go_back() -> TestResult {
    each_database("adr_0007_consumer_positions", async |db| {
        let database = open(db).await?;
        let mut ids = ids();
        for seconds in 1..=3 {
            let event = event(&mut ids, seconds);
            database.record_change(&event, async |_| Ok(())).await?;
        }
        let notifications = name("notifications");
        let history = name("history");
        assert_eq!(database.outbox_after(&notifications, 10).await?.len(), 3);
        database.outbox_done(&notifications, 2, at(10)).await?;
        let rest = database.outbox_after(&notifications, 10).await?;
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0].0.position, 3);
        database.outbox_done(&notifications, 1, at(11)).await?;
        assert_eq!(database.outbox_position(&notifications).await?, 2);
        assert_eq!(
            database.outbox_after(&history, 10).await?.len(),
            3,
            "each consumer has its own place"
        );
        let reserved = database.outbox_done(&name("head"), 9, at(12)).await;
        assert!(matches!(reserved, Err(StoreError::ReservedName)));
        Ok(())
    })
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn many_changes_at_once_get_every_position_exactly_once() -> TestResult {
    each_database("adr_0007_concurrent_changes", async |db| {
        let database = open(db).await?;
        let mut source = ids();
        let mut tasks = Vec::new();
        for seconds in 0..12 {
            let database = database.clone();
            let event = event(&mut source, seconds);
            tasks.push(tokio::spawn(async move {
                database.record_change(&event, async |_| Ok(())).await
            }));
        }
        for task in tasks {
            task.await??;
        }
        let positions: Vec<i64> = database
            .outbox_after(&name("history"), 100)
            .await?
            .iter()
            .map(|(entry, _)| entry.position)
            .collect();
        assert_eq!(positions, (1..=12).collect::<Vec<i64>>());
        let _ = source.now();
        Ok(())
    })
    .await
}

/// What a use case sees: only the port, not the database.
async fn through_the_port(
    store: &impl bb_app::ports::ChangeStore<Error = StoreError>,
    event: &DomainEvent,
) -> Result<(), StoreError> {
    store
        .record_change(event, |_transaction| Box::pin(async { Ok(()) }))
        .await
}

#[tokio::test]
async fn the_store_is_the_adapter_of_the_change_port() -> TestResult {
    each_database("adr_0007_change_port", async |db| {
        let database = open(db).await?;
        let event = event(&mut ids(), 5);
        through_the_port(&database, &event).await?;
        assert_eq!(database.events_about(&event.entity, 10).await?.len(), 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ai_38_an_event_written_in_a_run_carries_the_run() -> TestResult {
    each_database("ai_38_event_in_run", async |db| {
        let database = open(db).await?;
        let mut ids = ids();
        let (mut inside, outside) = (event(&mut ids, 5), event(&mut ids, 6));
        inside.run = Some(RunId::generate(&mut ids)?);
        database.record_change(&inside, async |_| Ok(())).await?;
        database.record_change(&outside, async |_| Ok(())).await?;
        assert_eq!(
            database.events_about(&inside.entity, 10).await?[0].run,
            inside.run
        );
        assert_eq!(
            database.events_about(&outside.entity, 10).await?[0].run,
            None
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn comp_09_a_text_is_marked_by_the_actor_and_the_channel_and_a_person_keeps_the_mark()
-> TestResult {
    each_database("comp_09_text_mark", async |db| {
        let database = open(db).await?;
        let mut ids = ids();
        let comment = EntityKind::new("comment")?;
        let mut written = event(&mut ids, 5);
        written.entity.kind = comment.clone();
        written.changes = vec![FieldChange {
            field: "text".to_owned(),
            before: None,
            after: Some(serde_json::json!("Done by the agent")),
        }];
        let agent = AgentId::generate(&mut ids);
        let person = bb_core::id::UserId::generate(&mut ids);
        written.actor = Actor::Agent { agent, person };
        database.record_change(&written, async |_| Ok(())).await?;
        let mut by_hand = event(&mut ids, 6);
        by_hand.entity = written.entity.clone();
        by_hand.changes = vec![FieldChange {
            field: "text".to_owned(),
            before: Some(serde_json::json!("Done by the agent")),
            after: Some(serde_json::json!("Done by the agent, checked")),
        }];
        database.record_change(&by_hand, async |_| Ok(())).await?;
        let marks = database
            .text_marks_of(&comment, &[written.entity.id])
            .await?;
        assert_eq!(marks.len(), 1);
        assert_eq!(marks[0].written_by, bb_core::mark::WrittenBy::Agent);
        assert_eq!(marks[0].agent, Some(agent));
        assert_eq!(marks[0].part.as_str(), "text");
        // A person's own comment has no mark.
        let mut own = event(&mut ids, 7);
        own.entity.kind = comment.clone();
        own.changes = by_hand.changes.clone();
        database.record_change(&own, async |_| Ok(())).await?;
        assert!(
            database
                .text_marks_of(&comment, &[own.entity.id])
                .await?
                .is_empty()
        );
        Ok(())
    })
    .await
}
