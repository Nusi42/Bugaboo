//! The port of the saved searches: what the use cases need from storage.
//!
//! Like the other ports, reads take `&self` and **writes** are associated functions that run inside
//! `record_change`, so a change and its domain event are committed together (ARCH-EVENTS).

use std::future::Future;

use bb_core::id::{GroupId, ProjectId, SavedSearchId, UserId};
use bb_core::person::Person;
use bb_core::saved_search::{
    ProjectDefaultList, SavedSearch, SearchColumn, SearchShare, ShareTarget,
};
use jiff::Timestamp;

use crate::ports::ChangeStore;
use crate::store_macros::{queries, reads, writes};

/// Who a list of shared searches is for: the groups the person is in and the projects they see,
/// which are the places a search is shared with that reach them.
#[derive(Clone, Copy, Debug)]
pub struct Reach<'a> {
    /// The person; their own searches are in their own list, not in this one.
    pub person: UserId,
    /// The groups they are in.
    pub groups: &'a [GroupId],
    /// The projects they see.
    pub projects: &'a [ProjectId],
}

/// What the saved-search use cases need from storage.
pub trait SavedSearchStore: ChangeStore + 'static {
    queries! {
        /// The saved search with this ID, also one in the trash.
        fn saved_search(id: SavedSearchId) -> Option<SavedSearch> = saved_search_in;
        /// The columns of one saved search, in order.
        fn search_columns(id: SavedSearchId) -> Vec<SearchColumn> = search_columns_in;
        /// The groups and projects one saved search is shared with, in the order they were added.
        fn search_shares(id: SavedSearchId) -> Vec<SearchShare> = search_shares_in;
        /// The saved search a project shows as its list, if it has one.
        fn project_default_list(project: ProjectId) -> Option<ProjectDefaultList>
            = project_default_list_in;
    }

    reads! {
        /// The saved searches a person made that are not in the trash, by ID (oldest first),
        /// after `after`, at most `limit`.
        fn saved_searches_made_by(
            maker: UserId,
            after: Option<SavedSearchId>,
            limit: u64
        ) -> Vec<SavedSearch>;
        /// The saved searches of other people that are shared with a place that reaches the
        /// person and are not in the trash, by ID, after `after`, at most `limit`. Whether a
        /// search reads cleanly for the person is judged after (ADR-0034, decision 4).
        fn saved_searches_shared_with(
            reach: Reach<'_>,
            after: Option<SavedSearchId>,
            limit: u64
        ) -> Vec<SavedSearch>;
        /// How many saved searches a person has, the trash included.
        fn count_saved_searches_of(maker: UserId) -> u64;
        /// The columns of these saved searches, each search's in order.
        fn columns_of_searches(ids: &[SavedSearchId]) -> Vec<SearchColumn>;
        /// The shares of these saved searches.
        fn shares_of_searches(ids: &[SavedSearchId]) -> Vec<SearchShare>;
        /// The people with these IDs that exist: the makers of a page.
        fn makers_of(ids: &[UserId]) -> Vec<Person>;
    }

    writes! {
        /// Adds a saved search with its columns.
        fn insert_saved_search(search: &SavedSearch, columns: &[SearchColumn]) -> ();
        /// Writes the name, text and presentation of a saved search and its columns, counts the
        /// version up to the one in `search`, and says whether the search was still at `version`.
        fn update_saved_search(
            search: &SavedSearch,
            columns: &[SearchColumn],
            version: i64
        ) -> bool;
        /// Puts a saved search into the trash (a date) or takes it out (`None`), writes the
        /// version in `search`, and says whether the search was at `version`.
        fn set_saved_search_deleted(
            id: SavedSearchId,
            deleted_at: Option<Timestamp>,
            version: i64
        ) -> bool;
        /// Shares a saved search; `false` when it was shared with the place already.
        fn insert_search_share(share: &SearchShare) -> bool;
        /// Stops sharing; `false` when it was not shared with the place.
        fn delete_search_share(search: SavedSearchId, target: ShareTarget) -> bool;
        /// Makes the saved search the list of the project, in place of the one it had.
        fn put_project_default_list(list: &ProjectDefaultList) -> ();
        /// Takes the list away, so the project shows the built-in default again; `false` when it
        /// had none. With `search`, only if the list is that search.
        fn clear_project_default_list(project: ProjectId, search: Option<SavedSearchId>) -> bool;
        /// Takes a group out of every share (the group is deleted); the IDs of the searches that
        /// were shared with it.
        fn delete_shares_with_group(group: GroupId) -> Vec<SavedSearchId>;
        /// Deletes the saved searches a person made that are shared with nobody, with their
        /// columns (the person is removed, ADR-0034, decision 8); the shared ones stay. The IDs
        /// of those that went.
        fn delete_private_searches_of(maker: UserId) -> Vec<SavedSearchId>;
    }
}
