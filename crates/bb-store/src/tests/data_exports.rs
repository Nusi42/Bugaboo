//! The records of *Download my data* on both databases (PRIV-03): one waiting export of a person is
//! found, a finished one is not; the newest export of a person by who asked; the page of every
//! export, newest first; the exports past their keep period, oldest first.

use bb_app::ports::{BoxFuture, ChangeStore};
use bb_core::data_export::{DataExport, ExportState};
use bb_core::id::{DataExportId, IdSource, UserId};
use bb_testing::{TestResult, each_database};

use super::*;
use crate::test_support::{at, event, fixture, ids};

async fn changed<T: Send>(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    write: impl for<'t> FnOnce(&'t mut Transaction) -> BoxFuture<'t, Result<T, StoreError>> + Send,
) -> Result<T, StoreError> {
    ChangeStore::record_change(storage, &event(source, 5), write).await
}

fn export(source: &mut impl IdSource, person: UserId, asked_by: UserId, second: i64) -> DataExport {
    DataExport {
        id: DataExportId::generate(source).unwrap(),
        person,
        asked_by,
        reason: (person != asked_by).then(|| "The person asked by letter".to_owned()),
        state: ExportState::Waiting,
        asked_at: at(second),
        ready_at: None,
        bytes: None,
    }
}

async fn insert(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    export: &DataExport,
) -> Result<(), StoreError> {
    let stored = export.clone();
    changed(storage, source, move |tx| {
        Box::pin(async move {
            AuthStorage::lock_exports_of(tx, stored.person).await?;
            AuthStorage::insert_data_export(tx, &stored).await
        })
    })
    .await
}

#[tokio::test]
async fn priv_03_exports_are_stored_found_and_finished_once() -> TestResult {
    each_database("priv_03_store_exports", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let (person, admin) = (UserId::generate(&mut source), UserId::generate(&mut source));
        let own = export(&mut source, person, person, 10);
        insert(storage, &mut source, &own).await?;
        assert_eq!(storage.data_export(own.id).await?, Some(own.clone()));
        assert_eq!(storage.waiting_export_of(person).await?, Some(own.clone()));
        assert_eq!(storage.waiting_export_of(admin).await?, None);

        let id = own.id;
        let finished = changed(storage, &mut source, move |tx| {
            Box::pin(async move {
                AuthStorage::finish_data_export(tx, id, ExportState::Ready, Some(at(20)), Some(42))
                    .await
            })
        })
        .await?;
        assert!(finished);
        let again = changed(storage, &mut source, move |tx| {
            Box::pin(async move {
                AuthStorage::finish_data_export(tx, id, ExportState::Failed, None, None).await
            })
        })
        .await?;
        assert!(!again, "an export ends once");
        let ready = storage.data_export(id).await?.unwrap();
        assert_eq!(
            (ready.state, ready.ready_at, ready.bytes),
            (ExportState::Ready, Some(at(20)), Some(42))
        );
        assert_eq!(storage.waiting_export_of(person).await?, None);

        let by_admin = export(&mut source, person, admin, 30);
        insert(storage, &mut source, &by_admin).await?;
        assert_eq!(
            storage
                .latest_export_of(person, person)
                .await?
                .map(|found| found.id),
            Some(id)
        );
        assert_eq!(
            storage.latest_export_of(person, admin).await?,
            Some(by_admin.clone())
        );
        let page: Vec<_> = storage
            .data_exports_page(0, 10)
            .await?
            .into_iter()
            .map(|found| found.id)
            .collect();
        assert_eq!(page, [by_admin.id, id], "newest first");
        assert_eq!(storage.data_exports_page(1, 10).await?.len(), 1);
        let old: Vec<_> = storage
            .exports_asked_before(at(25), 10)
            .await?
            .into_iter()
            .map(|found| found.id)
            .collect();
        assert_eq!(old, [id]);

        let deleted = changed(storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::delete_data_export(tx, id).await })
        })
        .await?;
        assert!(deleted);
        assert_eq!(storage.data_export(id).await?, None);
        Ok(())
    })
    .await
}
