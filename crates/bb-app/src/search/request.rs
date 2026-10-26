//! The port of a search: what the compiler of the store is given and what it answers (QRY-05,
//! SEC-17). The store compiles the checked tree into one statement; it never parses or checks.

use std::future::Future;

use bb_core::id::{
    ChoiceId, FieldId, GroupId, IssueId, IssueTypeId, LabelId, ProjectId, RelationKindId, TagId,
    UserId,
};
use bb_core::issue::{Issue, Priority, StatusGroup};
use bb_core::project::ShortName;
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
    pub short_name: ShortName,
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
    /// The tags the person may know, by the key of their name.
    pub tags: Vec<(String, Vec<TagId>)>,
    /// The labels the person may know, by the key of their name.
    pub labels: Vec<(String, Vec<LabelId>)>,
}

/// Where the values of an own field count: only in the projects the field reaches, and for the
/// issue types it is used for. Rows that stay on an issue that moved elsewhere are no values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnReach {
    /// The field.
    pub field: FieldId,
    /// The projects of the search that the field reaches.
    pub projects: Vec<ProjectId>,
    /// The issue types the field is used for; empty for all.
    pub issue_types: Vec<IssueTypeId>,
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
    /// Where the values of each own field count.
    pub own_reach: Vec<OwnReach>,
    /// The last issue of the page before. The store reads its place in the order itself, so a
    /// cursor holds nothing but this ID.
    pub after: Option<IssueId>,
    /// How many issues a page holds at most.
    pub limit: u64,
}

/// An issue found.
#[derive(Clone, Debug)]
pub struct SearchHit {
    /// The issue.
    pub issue: Issue,
}

/// What a group of a search is, by identity: the store gives the ID or the word, never a name,
/// and the use case shows the thing with the rights of the person (ADR-0022).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum GroupKey {
    /// The issues without a value.
    Empty,
    /// A text that is no person's: a status key, a tag or a label, a text of an own field.
    Text(String),
    /// A number or a moment.
    Whole(i64),
    /// A person.
    Person(UserId),
    /// A group of people.
    Group(GroupId),
    /// An issue the person may read.
    Issue(IssueId),
    /// A project.
    Project(ProjectId),
    /// An issue type.
    IssueType(IssueTypeId),
    /// A choice of an own field.
    Choice(ChoiceId),
    /// A priority.
    Priority(Priority),
    /// A status group.
    StatusGroup(StatusGroup),
}

/// How many issues a group of a search holds.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct GroupCount {
    /// What the group is.
    pub key: GroupKey,
    /// How many issues the search finds in it.
    pub issues: u64,
}

/// Why a search did not run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchFailure {
    /// It took longer than the limit (SEC-17).
    TooSlow,
    /// The cursor names an issue that is not there (or not readable): not a cursor of this list.
    BadCursor,
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
