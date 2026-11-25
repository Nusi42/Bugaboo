//! Pins on both databases (CFG-05): a pin is added once and after the others, the places are
//! written as given, a pin is taken away alone or with its target, and each person has their own.

use bb_app::ports::{BoxFuture, ChangeStore};
use bb_core::id::{IdSource, ProjectId, SavedSearchId, UserId};
use bb_core::pin::{Pin, PinTarget};
use bb_testing::{TestResult, each_database};

use super::*;
use crate::test_support::{event, fixture, ids};

async fn changed<T: Send>(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    write: impl for<'t> FnOnce(&'t mut Transaction) -> BoxFuture<'t, Result<T, StoreError>> + Send,
) -> Result<T, StoreError> {
    ChangeStore::record_change(storage, &event(source, 5), write).await
}

async fn pin(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    person: UserId,
    target: PinTarget,
    place: i64,
) -> Result<(), StoreError> {
    changed(storage, source, move |tx| {
        Box::pin(async move { AuthStorage::insert_pin(tx, person, target, place).await })
    })
    .await
}

fn targets(pins: &[Pin]) -> Vec<Option<PinTarget>> {
    pins.iter().map(Pin::points_to).collect()
}

#[tokio::test]
async fn cfg_05_pins_are_kept_by_the_place_they_are_given() -> TestResult {
    each_database("cfg_05_store_pins", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let (ana, ben) = (UserId::generate(&mut source), UserId::generate(&mut source));
        let project = PinTarget::Project(ProjectId::generate(&mut source)?);
        let search = PinTarget::SavedSearch(SavedSearchId::generate(&mut source)?);
        pin(storage, &mut source, ana, project, 0).await?;
        pin(storage, &mut source, ana, search, 1).await?;
        pin(storage, &mut source, ben, search, 0).await?;
        let pins = storage.pins_of(ana).await?;
        assert_eq!(targets(&pins), [Some(project), Some(search)]);
        assert_eq!(pins.iter().map(|pin| pin.place).collect::<Vec<_>>(), [0, 1]);
        // The places are written as given.
        changed(storage, &mut source, move |tx| {
            Box::pin(async move {
                AuthStorage::put_pin_places(tx, ana, &[(project, 1), (search, 0)]).await
            })
        })
        .await?;
        assert_eq!(
            targets(&storage.pins_of(ana).await?),
            [Some(search), Some(project)]
        );
        // A pin is taken away alone, and with its target for everybody.
        assert!(
            changed(storage, &mut source, move |tx| {
                Box::pin(async move { AuthStorage::delete_pin(tx, ana, project).await })
            })
            .await?
        );
        changed(storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::delete_pins_to(tx, &[search]).await })
        })
        .await?;
        assert!(storage.pins_of(ana).await?.is_empty());
        assert!(storage.pins_of(ben).await?.is_empty());
        Ok(())
    })
    .await
}
