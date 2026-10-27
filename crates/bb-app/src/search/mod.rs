//! Searching (QRY-04, QRY-05, SEC-17): what a person may ask a search about, and the port the
//! compiler of the store implements. The routes of the search build on this module.

mod name_choices;
mod names;
mod readable;
mod request;
mod run;
mod target;

pub use bb_searchlang::Mistake;
pub use name_choices::{
    NAME_CHOICES_MAX, NameChoiceKind, NameChoices, NameChoicesQuery, name_choices,
};
#[cfg(test)]
pub(crate) use names::NAME_RESOLUTIONS_MAX;
pub(crate) use names::NameCache;
pub(crate) use readable::{Looked, readable_of, word_is_taken};
pub use readable::{ReadableFieldsQuery, SearchFields, SearchScope, readable_fields};
pub use request::{
    GroupCount, GroupKey, Names, OwnReach, ProjectLink, SearchFailure, SearchHit, SearchRequest,
    SearchStore, UnknownName,
};
pub(crate) use run::search_page;
pub use run::{
    SearchCount, SearchCountQuery, SearchGroups, SearchGroupsQuery, SearchPage, SearchQuery,
    SearcherQuery, ShownKey, search, search_count, search_groups, searcher,
};
pub use target::{FieldTarget, Searchable, TextPlace, target_of};
