//! Saved searches on both databases (QRY-06): a search keeps its columns in order, a version that
//! moved stops a change, a share is made once, a person is reached through a group or a project,
//! and what is in the trash or made by the person is not in a list of what others shared.

use bb_app::ports::{BoxFuture, ChangeStore};
use bb_core::id::{GroupId, IdSource, ProjectId, SavedSearchId, UserId};
use bb_core::saved_search::{
    Column, Presentation, ProjectDefaultList, SavedSearch, SearchColumn, SearchShare, ShareTarget,
    Spacing,
};
use bb_core::version::FIRST_VERSION;
use bb_testing::{TestResult, each_database};

use super::*;
use crate::test_support::{at, event, fixture, ids, world};

async fn changed<T: Send>(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    write: impl for<'t> FnOnce(&'t mut Transaction) -> BoxFuture<'t, Result<T, StoreError>> + Send,
) -> Result<T, StoreError> {
    ChangeStore::record_change(storage, &event(source, 5), write).await
}

fn presentation() -> Presentation {
    Presentation {
        columns: vec![
            Column {
                field: "title".to_owned(),
                width: Some(300),
            },
            Column {
                field: "status".to_owned(),
                width: None,
            },
        ],
        spacing: Spacing::of("compact").unwrap_or_default(),
        group_by: Some("statusGroup".to_owned()),
        ..Presentation::default()
    }
}

fn search(source: &mut impl IdSource, maker: UserId, name: &str, seconds: i64) -> SavedSearch {
    let presentation = presentation();
    SavedSearch {
        id: SavedSearchId::generate(source).unwrap(),
        name: name.to_owned(),
        text: "status is open".to_owned(),
        maker,
        view: presentation.view,
        spacing: presentation.spacing,
        group_by: presentation.group_by,
        created_at: at(seconds),
        updated_at: at(seconds),
        deleted_at: None,
        version: FIRST_VERSION,
    }
}

async fn insert(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    search: &SavedSearch,
) -> Result<(), StoreError> {
    let (stored, columns) = (search.clone(), presentation().rows(search.id));
    changed(storage, source, move |tx| {
        Box::pin(async move { AuthStorage::insert_saved_search(tx, &stored, &columns).await })
    })
    .await
}

async fn share(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    search: SavedSearchId,
    target: ShareTarget,
) -> Result<bool, StoreError> {
    let share = SearchShare {
        search,
        target,
        shared_at: at(10),
    };
    changed(storage, source, move |tx| {
        Box::pin(async move { AuthStorage::insert_search_share(tx, &share).await })
    })
    .await
}

async fn shared_with(
    storage: &AuthStorage,
    reader: UserId,
    groups: &[GroupId],
    projects: &[ProjectId],
    after: Option<SavedSearchId>,
) -> Result<Vec<SavedSearch>, StoreError> {
    storage
        .saved_searches_shared_with(
            Reach {
                person: reader,
                groups,
                projects,
            },
            after,
            10,
        )
        .await
}

fn ids_of(found: &[SavedSearch]) -> Vec<SavedSearchId> {
    found.iter().map(|search| search.id).collect()
}

#[tokio::test]
async fn qry_06_a_saved_search_keeps_its_columns_in_order_and_is_changed_from_its_version()
-> TestResult {
    each_database("qry_06_store_round_trip", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let maker = UserId::generate(&mut source);
        let made = search(&mut source, maker, "Open bugs", 1);
        insert(storage, &mut source, &made).await?;
        assert_eq!(storage.saved_search(made.id).await?, Some(made.clone()));
        let columns = storage.search_columns(made.id).await?;
        assert_eq!(Presentation::of(&made, &columns), presentation());
        // A change from the version it has is written and counts the version up; from any other, not.
        let mut changed_search = made.clone();
        changed_search.name = "Bugs".to_owned();
        changed_search.version = made.version + 1;
        let fewer: Vec<SearchColumn> = presentation().rows(made.id).into_iter().take(1).collect();
        let (new, kept) = (changed_search.clone(), fewer.clone());
        let applied = changed(storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::update_saved_search(tx, &new, &kept, 1).await })
        })
        .await?;
        assert!(applied);
        assert_eq!(
            storage.saved_search(made.id).await?,
            Some(changed_search.clone())
        );
        assert_eq!(storage.search_columns(made.id).await?, fewer);
        let (stale, kept) = (changed_search.clone(), fewer.clone());
        let applied = changed(storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::update_saved_search(tx, &stale, &kept, 1).await })
        })
        .await?;
        assert!(!applied, "a change from an old version is not written");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_06_the_trash_hides_a_search_from_the_lists_and_keeps_it_for_undo() -> TestResult {
    each_database("qry_06_store_trash", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let maker = UserId::generate(&mut source);
        let made = search(&mut source, maker, "Open bugs", 1);
        insert(storage, &mut source, &made).await?;
        let id = made.id;
        let deleted = changed(storage, &mut source, move |tx| {
            Box::pin(
                async move { AuthStorage::set_saved_search_deleted(tx, id, Some(at(20)), 1).await },
            )
        })
        .await?;
        assert!(deleted);
        let in_trash = storage.saved_search(id).await?.ok_or("gone")?;
        assert_eq!((in_trash.deleted_at, in_trash.version), (Some(at(20)), 2));
        assert!(
            storage
                .saved_searches_made_by(maker, None, 10)
                .await?
                .is_empty()
        );
        // From a version it does not have, nothing is written.
        let again = changed(storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::set_saved_search_deleted(tx, id, None, 1).await })
        })
        .await?;
        assert!(!again);
        let back = changed(storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::set_saved_search_deleted(tx, id, None, 2).await })
        })
        .await?;
        assert!(back);
        assert_eq!(
            ids_of(&storage.saved_searches_made_by(maker, None, 10).await?),
            [id]
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_06_a_search_is_reached_through_a_group_or_a_project_and_never_by_its_maker_twice()
-> TestResult {
    each_database("qry_06_store_reach", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let world = world(storage, &mut source, 1).await;
        let (maker, reader) = (UserId::generate(&mut source), UserId::generate(&mut source));
        let group = GroupId::generate(&mut source).unwrap();
        let (first, second, third, trashed) = (
            search(&mut source, maker, "One", 1),
            search(&mut source, maker, "Two", 2),
            search(&mut source, maker, "Three", 3),
            search(&mut source, maker, "Trashed", 4),
        );
        for each in [&first, &second, &third, &trashed] {
            insert(storage, &mut source, each).await?;
        }
        assert!(share(storage, &mut source, first.id, ShareTarget::Group(group)).await?);
        assert!(!share(storage, &mut source, first.id, ShareTarget::Group(group)).await?);
        share(
            storage,
            &mut source,
            second.id,
            ShareTarget::Project(world.project),
        )
        .await?;
        share(
            storage,
            &mut source,
            trashed.id,
            ShareTarget::Project(world.project),
        )
        .await?;
        let trashed_id = trashed.id;
        changed(storage, &mut source, move |tx| {
            Box::pin(async move {
                AuthStorage::set_saved_search_deleted(tx, trashed_id, Some(at(20)), 1).await
            })
        })
        .await?;
        let groups = [group];
        let projects = [world.project];
        assert_eq!(
            ids_of(&shared_with(storage, reader, &groups, &projects, None).await?),
            [first.id, second.id]
        );
        assert_eq!(
            ids_of(&shared_with(storage, reader, &groups, &[], None).await?),
            [first.id]
        );
        assert_eq!(
            ids_of(&shared_with(storage, reader, &[], &projects, None).await?),
            [second.id]
        );
        assert!(
            shared_with(storage, reader, &[], &[], None)
                .await?
                .is_empty()
        );
        // After a search, the rest.
        assert_eq!(
            ids_of(&shared_with(storage, reader, &groups, &projects, Some(first.id)).await?),
            [second.id]
        );
        // Not the maker's own: that is their own list.
        let own = storage
            .saved_searches_shared_with(
                Reach {
                    person: maker,
                    groups: &groups,
                    projects: &projects,
                },
                None,
                10,
            )
            .await?;
        assert!(own.is_empty());
        // The columns and shares of a page come in one read each, in order.
        let ids = [first.id, second.id, third.id];
        assert_eq!(storage.columns_of_searches(&ids).await?.len(), 6);
        let shares = storage.shares_of_searches(&ids).await?;
        assert_eq!(
            shares.iter().map(|share| share.target).collect::<Vec<_>>(),
            [
                ShareTarget::Group(group),
                ShareTarget::Project(world.project)
            ]
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_06_stopping_a_share_and_deleting_a_group_take_the_share_rows_away() -> TestResult {
    each_database("qry_06_store_shares_go", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let (maker, reader) = (UserId::generate(&mut source), UserId::generate(&mut source));
        let group = GroupId::generate(&mut source).unwrap();
        let (first, third) = (
            search(&mut source, maker, "One", 1),
            search(&mut source, maker, "Three", 3),
        );
        insert(storage, &mut source, &first).await?;
        insert(storage, &mut source, &third).await?;
        let (id, target) = (first.id, ShareTarget::Group(group));
        share(storage, &mut source, id, target).await?;
        let stopped = changed(storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::delete_search_share(tx, id, target).await })
        })
        .await?;
        assert!(stopped);
        let again = changed(storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::delete_search_share(tx, id, target).await })
        })
        .await?;
        assert!(!again, "a share that is not there is not stopped");
        // A group that is deleted takes its shares along.
        share(storage, &mut source, id, target).await?;
        share(storage, &mut source, third.id, target).await?;
        let removed = changed(storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::delete_shares_with_group(tx, group).await })
        })
        .await?;
        assert_eq!(removed, [first.id, third.id]);
        let reaching = shared_with(storage, reader, &[group], &[], None).await?;
        assert!(reaching.is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_06_a_project_has_one_list_and_a_search_that_is_a_list_cannot_go() -> TestResult {
    each_database("qry_06_store_default_list", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let world = world(storage, &mut source, 1).await;
        let maker = UserId::generate(&mut source);
        let (first, second) = (
            search(&mut source, maker, "One", 1),
            search(&mut source, maker, "Two", 2),
        );
        insert(storage, &mut source, &first).await?;
        insert(storage, &mut source, &second).await?;
        let project = world.project;
        assert_eq!(storage.project_default_list(project).await?, None);
        for chosen in [first.id, second.id] {
            let list = ProjectDefaultList {
                project,
                search: chosen,
            };
            changed(storage, &mut source, move |tx| {
                Box::pin(async move { AuthStorage::put_project_default_list(tx, &list).await })
            })
            .await?;
        }
        // The second replaced the first.
        assert_eq!(
            storage
                .project_default_list(project)
                .await?
                .map(|list| list.search),
            Some(second.id)
        );
        let (other, mine) = (first.id, second.id);
        let kept = changed(storage, &mut source, move |tx| {
            Box::pin(async move {
                AuthStorage::clear_project_default_list(tx, project, Some(other)).await
            })
        })
        .await?;
        assert!(!kept, "only the search that is the list is taken away");
        let cleared = changed(storage, &mut source, move |tx| {
            Box::pin(async move {
                AuthStorage::clear_project_default_list(tx, project, Some(mine)).await
            })
        })
        .await?;
        assert!(cleared);
        assert_eq!(storage.project_default_list(project).await?, None);
        // A project or a search that is not there is refused.
        let nowhere = ProjectId::generate(&mut source).unwrap();
        let refused = changed(storage, &mut source, move |tx| {
            Box::pin(async move {
                AuthStorage::put_project_default_list(
                    tx,
                    &ProjectDefaultList {
                        project: nowhere,
                        search: mine,
                    },
                )
                .await
            })
        })
        .await;
        assert!(refused.is_err());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn qry_06_a_person_who_is_removed_takes_the_searches_shared_with_nobody_along() -> TestResult
{
    each_database("qry_06_store_removal", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let world = world(storage, &mut source, 1).await;
        let (maker, other) = (UserId::generate(&mut source), UserId::generate(&mut source));
        let (private, shared, theirs) = (
            search(&mut source, maker, "Private", 1),
            search(&mut source, maker, "Shared", 2),
            search(&mut source, other, "Theirs", 3),
        );
        for each in [&private, &shared, &theirs] {
            insert(storage, &mut source, each).await?;
        }
        share(
            storage,
            &mut source,
            shared.id,
            ShareTarget::Project(world.project),
        )
        .await?;
        let gone = changed(storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::delete_private_searches_of(tx, maker).await })
        })
        .await?;
        assert_eq!(gone, [private.id]);
        assert_eq!(storage.saved_search(private.id).await?, None);
        assert!(storage.search_columns(private.id).await?.is_empty());
        assert!(storage.saved_search(shared.id).await?.is_some());
        assert!(storage.saved_search(theirs.id).await?.is_some());
        Ok(())
    })
    .await
}
