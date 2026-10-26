//! The port of a search: what the compiler of the store is given and what it answers (QRY-05,
//! SEC-17). The store compiles the checked tree into one statement; it never parses or checks.

use std::future::Future;

use bb_core::id::{
    ChoiceId, FieldId, GroupId, IssueId, IssueTypeId, ProjectId, RelationKindId, UserId,
};
use bb_core::issue::{Issue, StatusGroup};
use bb_core::tags::IssueSight;
use bb_searchlang::CheckedSearch;
use jiff::Timestamp;
use jiff::tz::TimeZone;

use super::target::Searchable;
use crate::issues::IssueStore;

/// A project the person sees, as the compiler needs it to read `project is WEB` and `within WEB`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectLink {
    /// The project.
    pub id: ProjectId,
    /// The project it sits in.
    pub parent: Option<ProjectId>,
    /// Its short name.
    pub short_name: String,
}

/// What the names in a search stand for, resolved by the use case with the rules of the person
/// before the statement is made (ADR-0005 amendment M5, decision 9): a name is the key of
/// `bb_core::names::name_key`, and only things the person may know are in these lists, so a name
/// they cannot see is as unknown as one that does not exist. The compiler never compares text with
/// the case rules of a database.
#[derive(Clone, Debug, Default)]
pub struct Names {
    /// The keys of the statuses that have a name.
    pub statuses: Vec<(String, Vec<String>)>,
    /// The issue types that have a name.
    pub issue_types: Vec<(String, Vec<IssueTypeId>)>,
    /// The people the person may see by name (*Who sees people*), by the key of their name.
    pub people: Vec<(String, Vec<UserId>)>,
    /// The groups the person may see by name.
    pub groups: Vec<(String, Vec<GroupId>)>,
    /// The choices of each own field with a list, in the order of the list.
    pub choices: Vec<(FieldId, Vec<(String, ChoiceId)>)>,
}

/// A value a search sorts by, as it is kept in a cursor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SortValue {
    /// No value; such issues come last in either direction.
    Empty,
    /// A text, in lower case.
    Text(String),
    /// A whole number (a moment, a rank, a count).
    Whole(i64),
}

/// Where a page ended: the sort values of its last issue and its ID, which is the last key so that
/// issues with the same value keep one order (M3-F24).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchPosition {
    /// The values of the keys of `sort by`, in order.
    pub values: Vec<SortValue>,
    /// The ID of the issue.
    pub id: IssueId,
}

/// Everything a search is compiled from. The person's rights are in it as data (`sight`, the
/// fields), decided by the use case with the policy engine; the compiler adds them to the
/// statement and decides nothing.
#[derive(Clone, Debug)]
pub struct SearchRequest {
    /// The search, checked against `fields`.
    pub search: CheckedSearch,
    /// The fields the person may read, with what each reads.
    pub fields: Vec<Searchable>,
    /// Which issues the person reads (SEC-06, ADR-0004).
    pub sight: IssueSight,
    /// Who asks, for `me`.
    pub me: UserId,
    /// The moment of the request, for the relative times.
    pub now: Timestamp,
    /// The zone of the person's region, which says where a day begins.
    pub zone: TimeZone,
    /// The projects the person sees.
    pub projects: Vec<ProjectLink>,
    /// What the names of statuses, types and choices stand for.
    pub names: Names,
    /// The keys of the statuses of each status group, in the order of the groups.
    pub status_groups: Vec<(StatusGroup, Vec<String>)>,
    /// The kinds of relation that mean *blocks*.
    pub blocking_kinds: Vec<RelationKindId>,
    /// The last issue of the page before.
    pub after: Option<SearchPosition>,
    /// How many issues a page holds at most.
    pub limit: u64,
}

/// An issue found, with where it stands in the order.
#[derive(Clone, Debug)]
pub struct SearchHit {
    /// The issue.
    pub issue: Issue,
    /// Where it stands, for the cursor.
    pub position: SearchPosition,
}

/// How many issues a group of a search holds.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct GroupCount {
    /// What the group is: the value of the field as text, `None` for the issues without one.
    pub value: Option<String>,
    /// How many issues the search finds in it.
    pub issues: u64,
}

/// Why a search did not run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchFailure {
    /// It took longer than the limit (SEC-17).
    TooSlow,
    /// The store failed, or the search cannot be compiled; the text is for the log.
    Failed(String),
}

/// What the search use cases need from storage.
pub trait SearchStore: IssueStore + 'static {
    /// A page of the issues the search finds, in its order.
    fn search_page(
        &self,
        request: &SearchRequest,
    ) -> impl Future<Output = Result<Vec<SearchHit>, SearchFailure>> + Send;

    /// How many issues the search finds (the same search, without the paging).
    fn search_count(
        &self,
        request: &SearchRequest,
    ) -> impl Future<Output = Result<u64, SearchFailure>> + Send;

    /// How many issues each group of the field `by` holds, for the same search.
    fn search_groups(
        &self,
        request: &SearchRequest,
        by: &str,
    ) -> impl Future<Output = Result<Vec<GroupCount>, SearchFailure>> + Send;
}
