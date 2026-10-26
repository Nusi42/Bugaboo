//! Layouts on both databases (CFG-01, ADR-0027): a layout comes back as it was stored, a save is
//! made on the version that was looked at, and the layouts of a place are found by their kind.

use bb_app::access::PolicyStore as _;
use bb_app::layouts::LayoutStore;
use bb_app::ports::{BoxFuture, ChangeStore};
use bb_core::fields::{Field, FieldKey};
use bb_core::id::{IdSource, LayoutId, ProjectId, RoleId};
use bb_core::layout::{
    Layout, LayoutContent, LayoutKind, LayoutPlace, PlacedField, SectionContent, find,
};
use bb_core::project::{Project, ShortName};
use bb_core::version::FIRST_VERSION;
use bb_core::version::Version;
use bb_testing::{TestResult, each_database};

use super::*;
use crate::test_support::{event, fixture, ids};

/// The first version of a record, as the ports are asked.
const FIRST: Version = Version::new(FIRST_VERSION);

/// Runs `write` as a change with its event, the way a use case does.
async fn changed<T: Send>(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    write: impl for<'t> FnOnce(&'t mut Transaction) -> BoxFuture<'t, Result<T, StoreError>> + Send,
) -> Result<T, StoreError> {
    ChangeStore::record_change(storage, &event(source, 5), write).await
}

fn content(roles: &[RoleId], fields: &[Field]) -> LayoutContent {
    LayoutContent {
        off: Vec::new(),
        sections: vec![
            SectionContent {
                tab: "Basics".to_owned(),
                title: "Main".to_owned(),
                fields: fields
                    .iter()
                    .map(|field| PlacedField {
                        field: FieldKey::from(*field),
                        roles: roles.to_vec(),
                    })
                    .collect(),
            },
            SectionContent {
                tab: String::new(),
                title: String::new(),
                fields: Vec::new(),
            },
        ],
    }
}

#[tokio::test]
async fn cfg_01_a_layout_comes_back_as_stored_and_a_save_needs_the_version_it_started_from()
-> TestResult {
    each_database("cfg_01_store", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let project = ProjectId::generate(&mut source).unwrap();
        let made = Project {
            id: project,
            parent: None,
            short_name: ShortName::new("ACME").unwrap(),
            name: "Acme".to_owned(),
            version: FIRST_VERSION,
        };
        let roles = [
            RoleId::generate(&mut source).unwrap(),
            RoleId::generate(&mut source).unwrap(),
        ];
        let layout = Layout {
            id: LayoutId::generate(&mut source).unwrap(),
            project: Some(project),
            issue_type: None,
            kind: LayoutKind::NewIssue,
            version: FIRST_VERSION,
        };
        let first = content(&roles, &[Field::Priority, Field::Description]);
        let (stored_layout, stored_content) = (layout.clone(), first.clone());
        changed(storage, &mut source, move |tx| {
            Box::pin(async move {
                AuthStorage::insert_project(tx, &made).await?;
                AuthStorage::insert_layout(tx, &stored_layout, &stored_content).await
            })
        })
        .await?;
        let found = storage.stored_layout(layout.id).await?.unwrap();
        assert_eq!(found.layout, layout);
        assert_eq!(found.content, first, "order, tabs and roles come back");

        // The headers of a kind are listed, another kind has none.
        let headers = storage.layout_headers(LayoutKind::NewIssue).await?;
        assert_eq!(headers, vec![layout.clone()]);
        assert!(
            storage
                .layout_headers(LayoutKind::IssuePage)
                .await?
                .is_empty()
        );
        assert_eq!(find(&[project], None, &headers), Some(&layout));
        assert_eq!(
            layout.place(),
            LayoutPlace {
                project: Some(project),
                issue_type: None,
                kind: LayoutKind::NewIssue
            }
        );

        // A save on the version it started from replaces the sections and counts the version up.
        // The priority was taken off the form: its role is kept, to come back with it.
        let second = LayoutContent {
            off: vec![PlacedField {
                field: FieldKey::from(Field::Priority),
                roles: roles[1..].to_vec(),
            }],
            ..content(&roles[..1], &[Field::Description])
        };
        let (id, saved) = (layout.id, second.clone());
        let done = changed(storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::replace_layout(tx, id, &saved, FIRST).await })
        })
        .await?;
        assert!(done);
        let found = storage.stored_layout(id).await?.unwrap();
        assert_eq!(found.layout.version, FIRST_VERSION + 1);
        assert_eq!(found.content, second);

        // A save that started from the old version writes nothing.
        let (stale, other) = (id, first.clone());
        let done = changed(storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::replace_layout(tx, stale, &other, FIRST).await })
        })
        .await?;
        assert!(!done);
        assert_eq!(storage.stored_layout(id).await?.unwrap().content, second);

        // Taking it away needs the current version too, and leaves nothing behind.
        let wrong = changed(storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::delete_layout(tx, id, FIRST).await })
        })
        .await?;
        assert!(!wrong);
        let right = changed(storage, &mut source, move |tx| {
            Box::pin(async move {
                AuthStorage::delete_layout(tx, id, Version::new(FIRST_VERSION + 1)).await
            })
        })
        .await?;
        assert!(right);
        assert!(storage.stored_layout(id).await?.is_none());
        assert!(
            storage
                .layout_headers(LayoutKind::NewIssue)
                .await?
                .is_empty()
        );
        Ok(())
    })
    .await
}
