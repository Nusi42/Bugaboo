//! The writes of a removal on both databases (PRIV-04, ADR-0044, decision 6): the rows of a kind
//! that name the person by a link go and the others stay; the changes about the person's records
//! and the values that hold their name or address are scrubbed, and the members stay.

use bb_app::auth::{AccountStore as _, InvitationStore as _};
use bb_app::ports::ChangeStore;
use bb_core::event::{Actor, EntityKind, EntityRef, FieldChange, RecordKind};
use bb_core::id::{EntityId, IdSource, InvitationId, IssueId, UserId};
use bb_core::personal::Entity as _;
use bb_core::security_log::Digest;
use bb_core::sign_in::Invitation;
use bb_testing::{TestResult, each_database};
use serde_json::json;

use super::*;
use crate::test_support::{at, event, fixture, ids, person};

/// A change of `field` from `before` to `after`.
fn change(field: &str, before: serde_json::Value, after: serde_json::Value) -> FieldChange {
    FieldChange {
        field: field.to_owned(),
        before: Some(before),
        after: Some(after),
    }
}

/// An event by `actor` about the record `id` of `kind` with `changes`.
fn about(
    source: &mut impl IdSource,
    actor: UserId,
    kind: RecordKind,
    id: EntityId,
    changes: Vec<FieldChange>,
) -> DomainEvent {
    DomainEvent {
        actor: Actor::Person(actor),
        entity: EntityRef {
            kind: EntityKind::from(kind),
            id,
        },
        changes,
        ..event(source, 30)
    }
}

/// An invitation to `email`, made by `by`.
fn invitation(source: &mut impl IdSource, email: &str, by: UserId, token: u8) -> Invitation {
    Invitation {
        id: InvitationId::generate(source),
        token_hash: Digest::new([token; 32]),
        email: email.to_owned(),
        invited_by: Some(by),
        created_at: at(3),
        expires_at: at(9_000),
        used_at: None,
    }
}

/// The changes of every event, as stored.
async fn stored_changes(storage: &AuthStorage) -> Result<Vec<String>, StoreError> {
    let mut select = Query::select();
    select.column(Event::Changes).from(Event::Table);
    storage
        .database()
        .fetch_all(&select)
        .await?
        .iter()
        .map(|row| row.get(Event::Changes))
        .collect()
}

/// The events of the scrub: Lea's name changed, a key she made and deleted, her setting changed,
/// and Ada wrote her name and address into a title. `records` are the key, the setting and the
/// issue.
fn lea_events(
    source: &mut impl IdSource,
    lea_id: UserId,
    ada_id: UserId,
    records: [EntityId; 3],
) -> [DomainEvent; 4] {
    let [key, setting, issue] = records;
    [
        about(
            source,
            lea_id,
            RecordKind::AccessKey,
            key,
            vec![change("name", json!(""), json!("Build robot"))],
        ),
        // Her name changed; her setting changed; Ada wrote her name and address into a title.
        about(
            source,
            lea_id,
            RecordKind::Person,
            EntityId::from(lea_id),
            vec![change("display_name", json!("Lea L."), json!("Lea Lind"))],
        ),
        about(
            source,
            lea_id,
            RecordKind::Setting,
            setting,
            vec![change("display_theme", json!("light"), json!("dark"))],
        ),
        about(
            source,
            ada_id,
            RecordKind::Issue,
            issue,
            vec![
                change(
                    "title",
                    json!("Ask lea lind"),
                    json!("Mail LEA@example.test"),
                ),
                change("priority", json!("low"), json!("high")),
                change("summary", json!("Leadership"), json!("Linden")),
            ],
        ),
    ]
}

#[tokio::test]
async fn priv_04_a_removal_deletes_by_link_and_scrubs_the_changes() -> TestResult {
    each_database("priv_04_removal_writes", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let lea = person(&mut source, "lea@example.test");
        let ada = person(&mut source, "ada@example.test");
        let (lea_id, ada_id) = (lea.id, ada.id);
        // Lea invited somebody, and Ada invited Lea.
        let hers = invitation(&mut source, "new@example.test", lea_id, 1);
        let to_her = invitation(&mut source, "lea@example.test", ada_id, 2);
        ChangeStore::record_change(storage, &event(&mut source, 6), move |tx| {
            Box::pin(async move {
                AuthStorage::insert_person(tx, &lea, "hash", at(1)).await?;
                AuthStorage::insert_person(tx, &ada, "hash", at(1)).await?;
                AuthStorage::insert_invitation(tx, &hers).await?;
                AuthStorage::insert_invitation(tx, &to_her).await
            })
        })
        .await?;
        let issue = EntityId::from(IssueId::generate(&mut source)?);
        let setting = EntityId::from(IssueId::generate(&mut source)?);
        // A key she made and deleted: no row of it is left to find its events by.
        let key = EntityId::from(IssueId::generate(&mut source)?);
        let events = lea_events(&mut source, lea_id, ada_id, [key, setting, issue]);
        for one in &events {
            ChangeStore::record_change(storage, one, |_| Box::pin(async { Ok(()) })).await?;
        }
        let scrub = Scrub {
            kinds: vec![RecordKind::Person, RecordKind::AccessKey],
            records: vec![EntityRef {
                kind: EntityKind::from(RecordKind::Setting),
                id: setting,
            }],
            names: vec!["Lea Lind".to_owned()],
            handles: vec!["lea.lind".to_owned()],
            addresses: vec!["lea@example.test".to_owned()],
        };
        let (deleted, scrubbed) =
            ChangeStore::record_change(storage, &event(&mut source, 50), move |tx| {
                Box::pin(async move {
                    let deleted = AuthStorage::delete_linked(
                        tx,
                        lea_id,
                        &Invitation::MARKS,
                        &[Link::Author, Link::Recipient],
                    )
                    .await?;
                    let scrubbed = AuthStorage::scrub_changes(tx, lea_id, &scrub).await?;
                    Ok((deleted, scrubbed))
                })
            })
            .await?;
        // The invitation she made and the one to her address go.
        assert_eq!(deleted, 2);
        assert_eq!(scrubbed, 4);
        let text = stored_changes(storage).await?.join("\n");
        for gone in [
            "Lea L.",
            "lea lind",
            "LEA@example.test",
            "dark",
            "Build robot",
        ] {
            assert!(!text.contains(gone), "{gone} in {text}");
        }
        // The members stay, and so do values that name nothing of her.
        for kept in [
            "display_name",
            "display_theme",
            "title",
            "high",
            "Leadership",
            "Linden",
        ] {
            assert!(text.contains(kept), "{kept} not in {text}");
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_04_a_removal_deletes_the_jobs_that_name_the_person_by_id_or_address() -> TestResult {
    use bb_app::jobs::JobStore as _;
    use bb_core::id::JobId;
    use bb_core::technical::TechnicalName;
    each_database("priv_04_removal_jobs", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let lea = person(&mut source, "lea@example.test");
        let lea_id = lea.id;
        let kind = TechnicalName::new("mail.send")?;
        let payloads = [
            json!({ "person": lea_id.to_string() }),
            json!({ "to": "Lea@Example.test" }),
            json!({ "to": "ada@example.test" }),
        ];
        let jobs: Vec<JobId> = payloads
            .iter()
            .map(|_| JobId::generate(&mut source))
            .collect::<Result<_, _>>()?;
        ChangeStore::record_change(storage, &event(&mut source, 6), move |tx| {
            Box::pin(async move {
                AuthStorage::insert_person(tx, &lea, "hash", at(1)).await?;
                for (job, payload) in jobs.iter().zip(&payloads) {
                    AuthStorage::enqueue_job(tx, *job, &kind, payload, at(2), at(2)).await?;
                }
                Ok(())
            })
        })
        .await?;
        let deleted = ChangeStore::record_change(storage, &event(&mut source, 50), move |tx| {
            Box::pin(async move {
                let by_id = AuthStorage::delete_linked(
                    tx,
                    lea_id,
                    &bb_core::job::Job::MARKS,
                    &[Link::Subject],
                )
                .await?;
                let by_address =
                    AuthStorage::delete_jobs_with_address(tx, "lea@example.test").await?;
                Ok((by_id, by_address))
            })
        })
        .await?;
        assert_eq!(deleted, (1, 1));
        let mut select = Query::select();
        select.column(Job::Payload).from(Job::Table);
        let left: Vec<String> = storage
            .database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(|row| row.get(Job::Payload))
            .collect::<Result<_, _>>()?;
        assert_eq!(left.len(), 1);
        assert!(left[0].contains("ada@example.test"), "{left:?}");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_04_the_full_text_index_forgets_the_removed_text() -> TestResult {
    use bb_app::privacy::removal::RemovalStore as _;
    use bb_core::id::CommentId;
    use bb_core::text_index::TextSource;
    const WORD: &str = "quixotically";
    each_database("priv_04_removal_text_index", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let world = crate::test_support::world(storage, &mut source, 1).await;
        let issue = world.issues.first().ok_or("an issue")?.id;
        let comment = CommentId::generate(&mut source)?;
        let mut transaction = storage.database().begin().await?;
        crate::text_index::index_text(
            &mut transaction,
            issue,
            TextSource::Comment,
            EntityId::from(comment),
            &format!("{WORD} words of Lea"),
        )
        .await?;
        transaction.commit().await?;
        if let Some(found) = in_the_file(storage, WORD)? {
            assert!(found, "the text is in the file before");
        }
        storage.reindex_removed_texts(&[issue], &[comment]).await?;
        storage.compact_removed_texts().await?;
        let mut rows = Query::select();
        rows.expr(Expr::val(1))
            .from(crate::schema::TextIndexRow::Table)
            .and_where(Expr::col(crate::schema::TextIndexRow::Words).like(like::containing(WORD)));
        assert!(storage.database().fetch_optional(&rows).await?.is_none());
        // On SQLite no page of the file or of its write-ahead log keeps the old text.
        if let Some(found) = in_the_file(storage, WORD)? {
            assert!(!found, "the old text is still in the file");
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn priv_04_a_name_outside_ascii_is_scrubbed_in_any_capitals() -> TestResult {
    each_database("priv_04_removal_scrub_unicode", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let ada = person(&mut source, "ada@example.test").id;
        let issue = EntityId::from(IssueId::generate(&mut source)?);
        // As typed and in capitals: the database folds neither, Rust folds both. A Cyrillic name
        // keeps no letter the database can narrow by, so every row is read.
        let titles = [
            ("Özlem Çelik knows", "ask ÖZLEM ÇELIK"),
            ("Юлия Ким knows", "ask ЮЛИЯ КИМ"),
        ];
        for (before, after) in titles {
            let one = about(
                &mut source,
                ada,
                RecordKind::Issue,
                issue,
                vec![change("title", json!(before), json!(after))],
            );
            ChangeStore::record_change(storage, &one, |_| Box::pin(async { Ok(()) })).await?;
        }
        let names = ["Özlem Çelik", "Юлия Ким"];
        assert_eq!(
            storage.columns_holding(&names).await?,
            vec!["event.changes".to_owned()]
        );
        let scrub = Scrub {
            names: names.iter().map(|name| (*name).to_owned()).collect(),
            ..Scrub::default()
        };
        let scrubbed = ChangeStore::record_change(storage, &event(&mut source, 50), move |tx| {
            Box::pin(async move { AuthStorage::scrub_changes(tx, ada, &scrub).await })
        })
        .await?;
        assert_eq!(scrubbed, 2);
        assert_eq!(storage.columns_holding(&names).await?, Vec::<String>::new());
        let text = stored_changes(storage).await?.join("\n");
        assert!(text.contains("title"), "{text}");
        Ok(())
    })
    .await
}

/// Whether the SQLite file or its write-ahead log holds `word` anywhere; none on PostgreSQL.
fn in_the_file(storage: &AuthStorage, word: &str) -> Result<Option<bool>, std::io::Error> {
    let Some(file) = storage.database().file() else {
        return Ok(None);
    };
    let mut bytes = std::fs::read(&file)?;
    let mut wal = file.into_os_string();
    wal.push("-wal");
    if let Ok(more) = std::fs::read(&wal) {
        bytes.extend(more);
    }
    Ok(Some(
        bytes
            .windows(word.len())
            .any(|window| window == word.as_bytes()),
    ))
}
