//! The writes of a removal on both databases (PRIV-04, ADR-0044, decision 6): the rows of a kind
//! that name the person by a link go and the others stay; the changes about the person's records
//! and the values that hold their name or address are scrubbed, and the members stay.

use bb_app::auth::{AccountStore as _, InvitationStore as _};
use bb_app::ports::ChangeStore;
use bb_core::event::{Actor, EntityKind, EntityRef, FieldChange, RecordKind};
use bb_core::id::{EntityId, IdSource, InvitationId, IssueId, UserId};
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
        let events = [
            // Her name changed; her setting changed; Ada wrote her name and address into a title.
            about(
                &mut source,
                lea_id,
                RecordKind::Person,
                EntityId::from(lea_id),
                vec![change("display_name", json!("Lea L."), json!("Lea Lind"))],
            ),
            about(
                &mut source,
                lea_id,
                RecordKind::Setting,
                setting,
                vec![change("display_theme", json!("light"), json!("dark"))],
            ),
            about(
                &mut source,
                ada_id,
                RecordKind::Issue,
                issue,
                vec![
                    change(
                        "title",
                        json!("Ask Lea Lind"),
                        json!("Mail lea@example.test"),
                    ),
                    change("priority", json!("low"), json!("high")),
                ],
            ),
        ];
        for one in &events {
            ChangeStore::record_change(storage, one, |_| Box::pin(async { Ok(()) })).await?;
        }
        let scrub = Scrub {
            kinds: vec![RecordKind::Person],
            records: vec![EntityRef {
                kind: EntityKind::from(RecordKind::Setting),
                id: setting,
            }],
            words: vec!["Lea Lind".to_owned(), "lea@example.test".to_owned()],
        };
        let (deleted, scrubbed) =
            ChangeStore::record_change(storage, &event(&mut source, 50), move |tx| {
                Box::pin(async move {
                    let deleted = AuthStorage::delete_linked(
                        tx,
                        lea_id,
                        "invitation",
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
        assert_eq!(scrubbed, 3);
        let text = stored_changes(storage).await?.join("\n");
        for gone in ["Lea L.", "Lea Lind", "lea@example.test", "dark"] {
            assert!(!text.contains(gone), "{gone} in {text}");
        }
        // The members stay, and so do values that name nothing of her.
        for kept in ["display_name", "display_theme", "title", "high"] {
            assert!(text.contains(kept), "{kept} not in {text}");
        }
        Ok(())
    })
    .await
}
