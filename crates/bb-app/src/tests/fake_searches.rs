//! The saved-search part of the in-memory store (QRY-06).

use bb_core::id::{ProjectId, SavedSearchId, UserId};
use bb_core::person::Person;
use bb_core::saved_search::{
    ProjectDefaultList, SavedSearch, SearchColumn, SearchShare, ShareTarget,
};
use jiff::Timestamp;

use super::fake::{FakeError, FakeStore, State, Tx};
use crate::saved_searches::{Reach, SavedSearchStore};

fn reaches(state: &State, search: SavedSearchId, reach: &Reach<'_>) -> bool {
    state
        .search_shares
        .iter()
        .filter(|share| share.search == search)
        .any(|share| match share.target {
            ShareTarget::Group(group) => reach.groups.contains(&group),
            ShareTarget::Project(project) => reach.projects.contains(&project),
        })
}

/// The searches of `found` that come after `after`, by ID, at most `limit`.
fn page(
    mut found: Vec<SavedSearch>,
    after: Option<SavedSearchId>,
    skip: u64,
    limit: u64,
) -> Vec<SavedSearch> {
    found.sort_by_key(|search| search.id);
    found
        .into_iter()
        .filter(|search| after.is_none_or(|after| search.id > after))
        .skip(usize::try_from(skip).unwrap_or(usize::MAX))
        .take(usize::try_from(limit).unwrap_or(usize::MAX))
        .collect()
}

impl SavedSearchStore for FakeStore {
    async fn saved_search_in(
        tx: &mut Tx,
        id: SavedSearchId,
    ) -> Result<Option<SavedSearch>, FakeError> {
        Ok(tx
            .state
            .saved_searches
            .iter()
            .find(|search| search.id == id)
            .cloned())
    }

    async fn search_columns_in(
        tx: &mut Tx,
        id: SavedSearchId,
    ) -> Result<Vec<SearchColumn>, FakeError> {
        let mut found: Vec<SearchColumn> = tx
            .state
            .search_columns
            .iter()
            .filter(|column| column.search == id)
            .cloned()
            .collect();
        found.sort_by_key(|column| column.position);
        Ok(found)
    }

    async fn search_shares_in(
        tx: &mut Tx,
        id: SavedSearchId,
    ) -> Result<Vec<SearchShare>, FakeError> {
        let mut found: Vec<SearchShare> = tx
            .state
            .search_shares
            .iter()
            .filter(|share| share.search == id)
            .cloned()
            .collect();
        found.sort_by_key(|share| (share.shared_at, share.target));
        Ok(found)
    }

    async fn project_default_list_in(
        tx: &mut Tx,
        project: ProjectId,
    ) -> Result<Option<ProjectDefaultList>, FakeError> {
        Ok(tx
            .state
            .default_lists
            .iter()
            .find(|list| list.project == project)
            .cloned())
    }

    async fn saved_searches_made_by(
        &self,
        maker: UserId,
        after: Option<SavedSearchId>,
        limit: u64,
    ) -> Result<Vec<SavedSearch>, FakeError> {
        Ok(self.read(|state| {
            let made = state
                .saved_searches
                .iter()
                .filter(|search| search.maker == maker && search.deleted_at.is_none())
                .cloned()
                .collect();
            page(made, after, 0, limit)
        }))
    }

    async fn saved_searches_shared_with(
        &self,
        reach: Reach<'_>,
        after: Option<SavedSearchId>,
        skip: u64,
        limit: u64,
    ) -> Result<Vec<SavedSearch>, FakeError> {
        Ok(self.read(|state| {
            let shared = state
                .saved_searches
                .iter()
                .filter(|search| {
                    search.maker != reach.person
                        && search.deleted_at.is_none()
                        && reaches(state, search.id, &reach)
                })
                .cloned()
                .collect();
            page(shared, after, skip, limit)
        }))
    }

    async fn count_saved_searches_of_in(tx: &mut Tx, maker: UserId) -> Result<u64, FakeError> {
        let count = tx
            .state
            .saved_searches
            .iter()
            .filter(|search| search.maker == maker)
            .count();
        Ok(u64::try_from(count).unwrap_or(u64::MAX))
    }

    async fn columns_of_searches(
        &self,
        ids: &[SavedSearchId],
    ) -> Result<Vec<SearchColumn>, FakeError> {
        Ok(self.read(|state| {
            let mut found: Vec<SearchColumn> = state
                .search_columns
                .iter()
                .filter(|column| ids.contains(&column.search))
                .cloned()
                .collect();
            found.sort_by_key(|column| (column.search, column.position));
            found
        }))
    }

    async fn shares_of_searches(
        &self,
        ids: &[SavedSearchId],
    ) -> Result<Vec<SearchShare>, FakeError> {
        Ok(self.read(|state| {
            let mut found: Vec<SearchShare> = state
                .search_shares
                .iter()
                .filter(|share| ids.contains(&share.search))
                .cloned()
                .collect();
            found.sort_by_key(|share| (share.search, share.shared_at, share.target));
            found
        }))
    }

    async fn searches_shared_with_group(
        &self,
        group: bb_core::id::GroupId,
    ) -> Result<Vec<SavedSearchId>, FakeError> {
        Ok(self.read(|state| {
            let mut found: Vec<SavedSearchId> = state
                .search_shares
                .iter()
                .filter(|share| share.target == ShareTarget::Group(group))
                .map(|share| share.search)
                .collect();
            found.sort();
            found
        }))
    }

    async fn makers_of(&self, ids: &[UserId]) -> Result<Vec<Person>, FakeError> {
        Ok(self.read(|state| {
            state
                .people
                .iter()
                .filter(|(person, _)| ids.contains(&person.id))
                .map(|(person, _)| person.clone())
                .collect()
        }))
    }

    async fn lock_saved_searches_of(_tx: &mut Tx, _maker: UserId) -> Result<(), FakeError> {
        Ok(())
    }

    async fn insert_saved_search(
        tx: &mut Tx,
        search: &SavedSearch,
        columns: &[SearchColumn],
    ) -> Result<(), FakeError> {
        tx.state.saved_searches.push(search.clone());
        tx.state.search_columns.extend(columns.iter().cloned());
        Ok(())
    }

    async fn update_saved_search(
        tx: &mut Tx,
        search: &SavedSearch,
        columns: &[SearchColumn],
        version: i64,
    ) -> Result<bool, FakeError> {
        let Some(stored) = tx
            .state
            .saved_searches
            .iter_mut()
            .find(|stored| stored.id == search.id && stored.version == version)
        else {
            return Ok(false);
        };
        *stored = search.clone();
        tx.state
            .search_columns
            .retain(|column| column.search != search.id);
        tx.state.search_columns.extend(columns.iter().cloned());
        Ok(true)
    }

    async fn set_saved_search_deleted(
        tx: &mut Tx,
        id: SavedSearchId,
        deleted_at: Option<Timestamp>,
        version: i64,
    ) -> Result<bool, FakeError> {
        let Some(stored) = tx
            .state
            .saved_searches
            .iter_mut()
            .find(|stored| stored.id == id && stored.version == version)
        else {
            return Ok(false);
        };
        stored.deleted_at = deleted_at;
        stored.version += 1;
        Ok(true)
    }

    async fn insert_search_share(tx: &mut Tx, share: &SearchShare) -> Result<bool, FakeError> {
        let there = tx
            .state
            .search_shares
            .iter()
            .any(|known| known.search == share.search && known.target == share.target);
        if !there {
            tx.state.search_shares.push(share.clone());
        }
        Ok(!there)
    }

    async fn delete_search_share(
        tx: &mut Tx,
        search: SavedSearchId,
        target: ShareTarget,
    ) -> Result<bool, FakeError> {
        let before = tx.state.search_shares.len();
        tx.state
            .search_shares
            .retain(|share| !(share.search == search && share.target == target));
        Ok(tx.state.search_shares.len() < before)
    }

    async fn put_project_default_list(
        tx: &mut Tx,
        list: &ProjectDefaultList,
    ) -> Result<(), FakeError> {
        tx.state
            .default_lists
            .retain(|known| known.project != list.project);
        tx.state.default_lists.push(list.clone());
        Ok(())
    }

    async fn clear_project_default_list(
        tx: &mut Tx,
        project: ProjectId,
        search: Option<SavedSearchId>,
    ) -> Result<bool, FakeError> {
        let before = tx.state.default_lists.len();
        tx.state.default_lists.retain(|list| {
            !(list.project == project && search.is_none_or(|search| list.search == search))
        });
        Ok(tx.state.default_lists.len() < before)
    }

    async fn delete_shares_with_group(
        tx: &mut Tx,
        group: bb_core::id::GroupId,
    ) -> Result<Vec<SavedSearchId>, FakeError> {
        let target = ShareTarget::Group(group);
        let gone: Vec<SavedSearchId> = tx
            .state
            .search_shares
            .iter()
            .filter(|share| share.target == target)
            .map(|share| share.search)
            .collect();
        tx.state
            .search_shares
            .retain(|share| share.target != target);
        Ok(gone)
    }

    async fn delete_private_searches_of(
        tx: &mut Tx,
        maker: UserId,
    ) -> Result<Vec<SavedSearchId>, FakeError> {
        let state = &mut tx.state;
        let private: Vec<SavedSearchId> = state
            .saved_searches
            .iter()
            .filter(|search| {
                search.maker == maker
                    && !state
                        .search_shares
                        .iter()
                        .any(|share| share.search == search.id)
            })
            .map(|search| search.id)
            .collect();
        state
            .saved_searches
            .retain(|search| !private.contains(&search.id));
        state
            .search_columns
            .retain(|column| !private.contains(&column.search));
        Ok(private)
    }
}
