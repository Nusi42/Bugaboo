//! The removal that is not finished on both databases (PRIV-04, ADR-0044, decision 6): its words
//! are a sealed secret that goes with its last step, its job names only the person and the step,
//! a dead job is kept and counted until the removal goes on, and the comments that go are hidden
//! from the change of the removal on.

use bb_app::auth::AccountStore as _;
use bb_app::comments::CommentStore as _;
use bb_app::jobs::JobStore as _;
use bb_app::ports::ChangeStore;
use bb_app::privacy::removal::{PersonWords, RemovalStore as _};
use bb_core::comment::Comment;
use bb_core::id::{CommentId, EntityId, IdSource, JobId, UserId};
use bb_core::settings::privacy::RemovedTexts;
use bb_core::text_index::TextSource;
use bb_core::text_words::search_words;
use bb_testing::{TestResult, each_database};

use super::*;
use crate::schema::{CommentRow, Job as JobRow, TextIndexRow};
use crate::test_support::{at, event, fixture, ids, person, removed, world};
use crate::text_index::{TextScope, TextSearch, index_text, text_issues};

/// Lea, stored as a person.
async fn lea(
    storage: &AuthStorage,
    source: &mut impl IdSource,
) -> Result<UserId, Box<dyn std::error::Error + Send + Sync>> {
    let lea = person(source, "lea@example.test");
    let id = lea.id;
    ChangeStore::record_change(storage, &event(source, 1), move |tx| {
        Box::pin(async move { AuthStorage::insert_person(tx, &lea, "hash", at(1)).await })
    })
    .await?;
    Ok(id)
}

/// The payloads of the jobs that finish a removal, as stored.
async fn finishing(storage: &AuthStorage) -> Result<Vec<(JobId, serde_json::Value)>, StoreError> {
    let mut select = Query::select();
    select
        .columns([JobRow::Id, JobRow::Payload])
        .from(JobRow::Table)
        .and_where(Expr::col(JobRow::Kind).eq("privacy.finish-removal"));
    storage
        .database()
        .fetch_all(&select)
        .await?
        .iter()
        .map(|row| {
            Ok((
                row.id(JobRow::Id)?,
                serde_json::from_str(&row.get::<String>(JobRow::Payload)?)?,
            ))
        })
        .collect()
}

/// Whether the secret of the words of the removal of `person` is stored.
async fn sealed(storage: &AuthStorage, person: UserId) -> Result<bool, StoreError> {
    let name = format!("privacy.removal.{person}");
    Ok(storage
        .database()
        .secret_names()
        .await?
        .iter()
        .any(|stored| stored.as_str() == name))
}

#[tokio::test]
async fn priv_04_the_words_of_a_removal_are_sealed_and_go_with_its_last_step() -> TestResult {
    each_database("priv_04_removal_sealed", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let lea = lea(storage, &mut source).await?;
        let removal = removed(storage, &mut source, lea, RemovedTexts::Keep).await?;
        let pending = storage
            .pending_removal(lea)
            .await?
            .ok_or("the removal is not finished")?;
        assert_eq!(pending.job, removal.job);
        assert!(!pending.comments_go);
        // The words are a secret of their own, which only the key files open.
        assert!(sealed(storage, lea).await?);
        let words = storage
            .removal_words(lea)
            .await?
            .ok_or("the words are kept")?;
        assert_eq!(
            words,
            PersonWords {
                name: "Test Person".to_owned(),
                address: words.address.clone(),
                handles: words.handles.clone(),
            }
        );
        let swept = storage.columns_holding(&["Test Person"]).await?;
        assert!(
            swept.iter().all(|column| !column.starts_with("secret.")),
            "{swept:?}"
        );
        // The job names the person and the step, and no word of them.
        let jobs = finishing(storage).await?;
        let [(job, payload)] = jobs.as_slice() else {
            return Err(format!("one job finishes the removal: {jobs:?}").into());
        };
        assert_eq!(*job, removal.job);
        let members: Vec<&String> = payload.as_object().ok_or("an object")?.keys().collect();
        assert_eq!(members, ["person", "step"]);
        assert!(!payload.to_string().contains("Test Person"));
        // Only the job the row names ends it; then the row and the secret with its key are gone.
        let other = JobId::generate(&mut source)?;
        let (wrong, right) = ChangeStore::record_changes(storage, &[], move |tx| {
            Box::pin(async move {
                let wrong = AuthStorage::end_removal(tx, lea, other).await?;
                let right = AuthStorage::end_removal(tx, lea, removal.job).await?;
                Ok((wrong, right))
            })
        })
        .await?;
        assert!(!wrong && right);
        assert!(storage.pending_removal(lea).await?.is_none());
        assert!(storage.removal_words(lea).await?.is_none());
        assert!(!sealed(storage, lea).await?);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_04_the_dead_job_of_a_removal_is_kept_and_counted_until_the_removal_goes_on()
-> TestResult {
    each_database("priv_04_removal_dead_job", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let lea = lea(storage, &mut source).await?;
        let removal = removed(storage, &mut source, lea, RemovedTexts::Keep).await?;
        let mut dead = Query::update();
        dead.table(JobRow::Table)
            .value(JobRow::State, "dead")
            .value(JobRow::RunAt, 0_i64)
            .and_where(Expr::col(JobRow::Id).eq(removal.job.stored()));
        storage.database().execute(&dead).await?;
        let far = at(1_000_000_000);
        // The purge of dead jobs leaves it; the status page counts the removal as stopped.
        assert_eq!(storage.purge_dead_jobs(far, 100).await?, 0);
        assert_eq!(storage.stopped_removals().await?.len(), 1);
        assert_eq!(storage.database().stopped_removal_count().await?, 1);
        // A new job goes on from the step of the row; the old job is no longer the removal's.
        let next = JobId::generate(&mut source)?;
        let step = storage.stopped_removals().await?.first().ok_or("stopped")?.step.clone();
        let kind = TechnicalName::new("privacy.finish-removal")?;
        let payload = serde_json::json!({ "person": lea, "step": serde_json::from_str::<serde_json::Value>(&step)? });
        let moved = ChangeStore::record_changes(storage, &[], move |tx| {
            Box::pin(async move {
                let moved =
                    AuthStorage::move_removal(tx, lea, removal.job, next, &step, at(200)).await?;
                AuthStorage::enqueue_job(tx, next, &kind, &payload, at(200), at(200)).await?;
                Ok(moved)
            })
        })
        .await?;
        assert!(moved);
        assert_eq!(storage.database().stopped_removal_count().await?, 0);
        assert_eq!(storage.purge_dead_jobs(far, 100).await?, 1);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_04_the_comments_that_go_are_hidden_from_the_change_of_the_removal_on() -> TestResult {
    const WORD: &str = "quixotically";
    each_database("priv_04_removal_hides_comments", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let scene = world(storage, &mut source, 1).await;
        let issue = scene.issues.first().ok_or("an issue")?.id;
        let lea = lea(storage, &mut source).await?;
        let comment = Comment {
            id: CommentId::generate(&mut source)?,
            issue,
            author: lea,
            text: format!("{WORD} words of Lea"),
            created_at: at(10),
            edited_at: None,
            deleted_at: None,
            version: 1,
        };
        let (id, row) = (comment.id, comment.clone());
        ChangeStore::record_change(storage, &event(&mut source, 10), move |tx| {
            Box::pin(async move { AuthStorage::insert_comment(tx, &row).await })
        })
        .await?;
        let mut transaction = storage.database().begin().await?;
        index_text(
            &mut transaction,
            issue,
            TextSource::Comment,
            EntityId::from(id),
            &comment.text,
        )
        .await?;
        transaction.commit().await?;
        let found = async || -> Result<bool, StoreError> {
            let words = search_words(WORD);
            let search = TextSearch {
                words: &words,
                scope: TextScope::Everywhere,
            };
            let select = text_issues(storage.database().dialect(), &search);
            Ok(storage.database().fetch_optional(&select).await?.is_some())
        };
        assert!(found().await?, "found before the removal");
        removed(storage, &mut source, lea, RemovedTexts::Delete).await?;
        // At once: in the trash, which every read leaves out, and out of every search, although
        // the job has not deleted it yet.
        let mut trashed = Query::select();
        trashed
            .column(CommentRow::DeletedAt)
            .from(CommentRow::Table)
            .and_where(Expr::col(CommentRow::Id).eq(id.stored()));
        let row = storage
            .database()
            .fetch_optional(&trashed)
            .await?
            .ok_or("the comment waits")?;
        assert_eq!(row.optional_time(CommentRow::DeletedAt)?, Some(at(100)));
        assert!(!found().await?, "no search finds it");
        let mut rows = Query::select();
        rows.expr(Expr::val(1))
            .from(TextIndexRow::Table)
            .and_where(Expr::col(TextIndexRow::SourceRef).eq(id.stored()));
        assert!(
            storage.database().fetch_optional(&rows).await?.is_some(),
            "the index still has it"
        );
        // Nobody takes it out of the trash while the removal goes on.
        let restored = ChangeStore::record_change(storage, &event(&mut source, 120), move |tx| {
            Box::pin(async move { AuthStorage::set_comment_deleted(tx, id, None, None).await })
        })
        .await?;
        assert!(!restored);
        Ok(())
    })
    .await
}
