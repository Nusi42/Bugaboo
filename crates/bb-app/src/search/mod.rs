//! Searching (QRY-04, QRY-05, SEC-17): what a person may ask a search about, and the port the
//! compiler of the store implements. The routes of the search build on this module.

mod group_filter;
mod name_choices;
mod names;
mod readable;
mod request;
mod run;
mod target;
mod totals;

pub use bb_searchlang::{MAX_STEPS, Mistake, ROUND_TRIP_GUARD};
pub use name_choices::{
    NAME_CHOICES_MAX, NameChoiceKind, NameChoices, NameChoicesQuery, name_choices,
};
#[cfg(test)]
pub(crate) use names::NAME_RESOLUTIONS_MAX;
pub(crate) use names::NameCache;
pub use readable::{FieldName, ReadableFieldsQuery, SearchFields, SearchScope, readable_fields};
pub(crate) use readable::{Looked, lengths_readable_in, readable_of, word_is_taken};
pub use request::{
    FieldPlace, GroupBy, GroupKey, GroupPage, GroupPlace, GroupTotal, KeyedChoice, MapStep, Names,
    OwnReach, ProjectLink, ReadableWhere, SUMS_MAX, SearchFailure, SearchHit, SearchRequest,
    SearchStore, Sum, SumAsked, Total, Totals, TotalsAsked, UnknownName, WithheldText, WorkOf,
};
pub use run::{
    GroupShown, SearchCount, SearchCountQuery, SearchGroups, SearchGroupsQuery, SearchPage,
    SearchQuery, SearcherQuery, ShownKey, search, search_count, search_groups, searcher,
};
pub(crate) use run::{Prepared, blocking_kinds, search_page, search_page_within, stopped};
pub use target::{FieldTarget, Searchable, TextPlace, target_of};
pub(crate) use totals::search_totals_within;
pub use totals::{
    GROUPS_PAGE, GroupOrder, GroupTotalShown, SearchTotals, SearchTotalsQuery, SumShown, SumWord,
    TotalShown, search_totals, totals_list,
};
