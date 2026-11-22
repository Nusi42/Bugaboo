//! The apps of AI apps on both databases (AI-22): when the hour of the server is full, a new app
//! takes the place of the first one of the hour that nobody allowed, and never of an allowed one.

use bb_app::auth::AccountStore as _;
use bb_app::ports::{BoxFuture, ChangeStore};
use bb_core::id::IdSource;
use bb_testing::{TestResult, each_database};

use super::*;
use crate::test_support::{at, event, fixture, ids, person};

/// Runs `write` as a change with its event, the way a use case does.
async fn change<T: Send>(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    write: impl for<'t> FnOnce(&'t mut Transaction) -> BoxFuture<'t, Result<T, StoreError>> + Send,
) -> Result<T, StoreError> {
    let event = event(source, 5);
    ChangeStore::record_change(storage, &event, write).await
}

fn app(source: &mut impl IdSource, registered: i64) -> OAuthClient {
    OAuthClient {
        id: OAuthClientId::generate(source),
        name: "Claude Code".to_owned(),
        return_addresses: vec!["http://127.0.0.1/callback".to_owned()],
        registered_at: at(registered),
    }
}

#[tokio::test]
async fn sec_14_a_new_app_pushes_out_the_first_app_of_the_hour_that_nobody_allowed() -> TestResult {
    each_database("sec_14_push_out_unallowed_app", async |db| {
        let fixture = fixture(db).await?;
        let storage = fixture.storage.clone();
        let mut source = ids();
        let ada = person(&mut source, "ada@example.test");
        let stored = ada.clone();
        change(&storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::insert_person(tx, &stored, "hash", at(0)).await })
        })
        .await?;
        // Before the hour, then three apps in it; the first of the hour is allowed.
        let apps = [10, 100, 200, 300].map(|registered| app(&mut source, registered));
        let grant = GrantRecord {
            grant: OAuthGrant {
                id: GrantId::generate(&mut source),
                client: apps[1].id,
                person: ada.id,
                scope: KeyChoice::SeeOnly,
                allowed_at: at(150),
                last_used_at: None,
                ends_at: at(100_000),
                ended_at: None,
                ended: None,
            },
            projects: Vec::new(),
        };
        let stored = apps.to_vec();
        change(&storage, &mut source, move |tx| {
            Box::pin(async move {
                for app in &stored {
                    AuthStorage::insert_oauth_client(tx, app).await?;
                }
                AuthStorage::insert_grant(tx, &grant).await
            })
        })
        .await?;
        let push_out = async |source: &mut _| {
            change(&storage, source, move |tx| {
                Box::pin(async move { AuthStorage::push_out_unallowed_client(tx, at(50)).await })
            })
            .await
        };
        // The allowed app and the one from before the hour stay; the others go, oldest first.
        assert!(push_out(&mut source).await?);
        assert!(storage.oauth_client(apps[2].id).await?.is_none());
        assert!(storage.oauth_client(apps[3].id).await?.is_some());
        assert!(push_out(&mut source).await?);
        assert!(storage.oauth_client(apps[3].id).await?.is_none());
        // Every app of the hour is allowed: nothing makes room.
        assert!(!push_out(&mut source).await?);
        assert!(storage.oauth_client(apps[0].id).await?.is_some());
        assert!(storage.oauth_client(apps[1].id).await?.is_some());
        Ok(())
    })
    .await
}
