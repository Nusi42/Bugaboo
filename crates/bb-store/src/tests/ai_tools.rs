//! The marks *Never sent to AI* and the log of tool calls on both databases (AI-03, AI-05).

use bb_app::ai::NeverSentStore;
use bb_app::ai::tools::ToolCallStore;
use bb_app::ports::ChangeStore;
use bb_core::ai::{
    NeverSentMark, NeverSentSubject, Tool, ToolCall, ToolCaller, ToolEffect, ToolOutcome,
};
use bb_core::fields::Field;
use bb_core::id::{ToolCallId, UserId};
use bb_testing::{TestResult, each_database};

use super::*;
use crate::test_support::{at, event, fixture, ids, world};

/// Writes a mark inside a change; a mark that finds another version stops the change.
async fn put(
    storage: &AuthStorage,
    source: &mut bb_core::id::SequentialIds,
    mark: NeverSentMark,
) -> Result<(), StoreError> {
    let change = event(source, 9);
    ChangeStore::record_change(storage, &change, move |tx| {
        Box::pin(async move {
            if AuthStorage::put_never_sent_mark(tx, &mark).await? {
                Ok(())
            } else {
                Err(StoreError::Corrupt("another version"))
            }
        })
    })
    .await
}

#[tokio::test]
async fn ai_03_a_mark_is_written_once_and_changed_on_the_version_it_was_read_at() -> TestResult {
    each_database("ai_03_marks", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let project = world(&storage, &mut source, 0).await.project;
        let field = NeverSentSubject::Field(Field::Description);
        let of_project = NeverSentSubject::Project(project);
        assert!(storage.never_sent_marks().await?.is_empty());
        for subject in [field, of_project] {
            let first = NeverSentMark {
                subject,
                marked: true,
                version: 1,
            };
            put(&storage, &mut source, first).await?;
        }
        // A second first write finds the row there: nothing is written over it.
        let again = NeverSentMark {
            subject: field,
            marked: false,
            version: 1,
        };
        assert!(put(&storage, &mut source, again).await.is_err());
        let second = NeverSentMark {
            subject: field,
            marked: false,
            version: 2,
        };
        put(&storage, &mut source, second).await?;
        let marks = storage.never_sent_marks().await?;
        assert_eq!(marks.len(), 2);
        assert!(marks.contains(&second));
        Ok(())
    })
    .await
}

fn call(source: &mut bb_core::id::SequentialIds, seconds: i64) -> ToolCall {
    ToolCall {
        id: ToolCallId::generate(source).unwrap(),
        person: UserId::generate(source),
        caller: ToolCaller::AiApp,
        app: Some("An app".to_owned()),
        tool: Some(Tool::GetIssue),
        effect: Some(ToolEffect::Reads),
        bytes_in: 40,
        bytes_out: 900,
        at: at(seconds),
        outcome: ToolOutcome::Done,
    }
}

#[tokio::test]
async fn ai_05_the_log_of_tool_calls_keeps_newest_first_and_prunes_by_time() -> TestResult {
    each_database("ai_05_tool_calls", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let (old, new) = (call(&mut source, 10), call(&mut source, 100));
        storage.append_tool_call(&old).await?;
        storage.append_tool_call(&new).await?;
        assert_eq!(storage.tool_calls(10).await?, vec![new.clone(), old]);
        assert_eq!(storage.prune_tool_calls(at(50)).await?, 1);
        assert_eq!(storage.tool_calls(10).await?, vec![new]);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn ai_05_a_reference_of_the_assistant_is_used_once_until_it_runs_out() -> TestResult {
    use bb_app::ai::AiStore;
    each_database("ai_05_references", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let (now, until) = (at(100), at(700));
        assert!(storage.use_assistant_reference("one", now, until).await?);
        assert!(!storage.use_assistant_reference("one", now, until).await?);
        assert!(storage.use_assistant_reference("two", now, until).await?);
        // After it ran out the word is forgotten; the reference itself is no longer good.
        assert!(
            storage
                .use_assistant_reference("one", at(800), at(1400))
                .await?
        );
        Ok(())
    })
    .await
}
