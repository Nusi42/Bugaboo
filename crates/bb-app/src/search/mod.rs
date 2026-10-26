//! Searching (QRY-04, QRY-05, SEC-17): what a person may ask a search about, and the port the
//! compiler of the store implements. The routes of the search build on this module.

mod names;
mod readable;
mod request;
mod run;
mod target;

pub use bb_searchlang::Mistake;
pub(crate) use readable::word_is_taken;
pub use readable::{ReadableFieldsQuery, SearchFields, SearchScope, readable_fields};
pub use request::{
    GroupCount, GroupKey, Names, OwnReach, ProjectLink, SearchFailure, SearchHit, SearchRequest,
    SearchStore, UnknownName,
};
pub(crate) use run::search_page;
pub use run::{
    SearchCount, SearchCountQuery, SearchGroups, SearchGroupsQuery, SearchPage, SearchQuery,
    ShownKey, search, search_count, search_groups,
};
pub use target::{FieldTarget, Searchable, target_of};
