//! The port of a search: what the compiler of the store is given and what it answers (QRY-05,
//! SEC-17). The store compiles the checked tree into one statement; it never parses or checks.

use std::future::Future;

use bb_core::id::{
    ChoiceId, FieldId, GroupId, IssueId, IssueTypeId, LabelId, ProjectId, RelationKindId, TagId,
    UserId,
};
use bb_core::issue::{Issue, Priority, StatusGroup};
use bb_core::people_kinds::PeopleKind;
use bb_core::project::ShortName;
use bb_core::tags::IssueSight;
use bb_core::time::Minutes;
use bb_core::time::day::FirstDay;
use bb_core::time::range::LengthRange;
use bb_searchlang::CheckedSearch;
use jiff::Timestamp;
use jiff::tz::TimeZone;

use super::target::Searchable;
use crate::issues::IssueStore;

/// The texts of an issue that a bare `contains` leaves out (AI-03): the title and the description
/// are searched with the other texts, so a field kept from AI would be found through them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WithheldText {
    /// The title is kept from AI.
    pub title: bool,
    /// The description is kept from AI.
    pub description: bool,
}

impl WithheldText {
    /// Whether any text is withheld.
    #[must_use]
    pub const fn any(self) -> bool {
        self.title || self.description
    }
}

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

impl ProjectLink {
    /// The project with this short name among `links`: the one rule for reading a project by its
    /// current short name, for the use case that resolves the names and for the compiler.
    #[must_use]
    pub fn named<'a>(links: &'a [Self], short_name: &ShortName) -> Option<&'a Self> {
        links.iter().find(|link| link.short_name == *short_name)
    }

    /// The same for a text as a search writes it (in any case: the rule of `ShortName`).
    #[must_use]
    pub fn named_in<'a>(links: &'a [Self], text: &str) -> Option<&'a Self> {
        Self::named(links, &text.parse::<ShortName>().ok()?)
    }
}

/// A choice of an own field with the keys that find it: its name, its number and how it is shown
/// (`name_key`), and its number read as a whole number (a comparison goes by it, and counts only the
/// choices that have one, where any has).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyedChoice {
    /// The keys.
    pub keys: Vec<String>,
    /// The choice.
    pub id: ChoiceId,
    /// Its number as a whole number (`0410` is 410); none without a number.
    pub number: Option<u64>,
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
    /// The people the person may see by name (*Who sees people*), by the kind of the field they are
    /// named in and the key of their name: a name allowed for one kind is not allowed for another.
    pub people: Vec<(PeopleKind, String, Vec<UserId>)>,
    /// The groups the person may see by name.
    pub groups: Vec<(String, Vec<GroupId>)>,
    /// The groups after `member of` that the person may ask about: those they are in, or all of
    /// them when they manage groups.
    pub member_groups: Vec<(String, Vec<GroupId>)>,
    /// The issues that an Issue ID names, among those the person reads, by the ID as the
    /// language writes it (capitals).
    pub issues: Vec<(String, IssueId)>,
    /// The names of the search that stand for nothing the person may know, with where they are.
    pub unknown: Vec<UnknownName>,
    /// The choices of each own field with a list, in the order of the list, each with the keys that
    /// find it (its name, its number, how it is shown).
    pub choices: Vec<(FieldId, Vec<KeyedChoice>)>,
    /// The tags the person may know, by the key of their name.
    pub tags: Vec<(String, Vec<TagId>)>,
    /// The labels the person may know, by the key of their name.
    pub labels: Vec<(String, Vec<LabelId>)>,
}

/// A name in a search that stands for nothing the person may know: it does not exist, or they may
/// not see it, which is told alike (ADR-0005 amendment M5, decision 9).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownName {
    /// The bytes of the text it stands on.
    pub span: bb_searchlang::Span,
    /// The word of the field it is a value of; empty for a helper (`under`, `blocks`, …).
    pub field: String,
    /// The name as written.
    pub text: String,
}

/// Where the values of an own field count: only in the projects the field reaches, and for the
/// issue types it is used for. Rows that stay on an issue that moved elsewhere are no values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnReach {
    /// The field.
    pub field: FieldId,
    /// The projects of the search that the field reaches.
    pub projects: Vec<ProjectId>,
    /// The issue types the field is limited to; `None` while it is used for all of them.
    pub issue_types: Option<Vec<IssueTypeId>>,
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
    /// The day the person's week starts on, for *this week* and *last week*.
    pub first_day: FirstDay,
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
    /// The built-in texts a bare `contains` must not look in, because an administrator kept them
    /// from AI (AI-03). Nothing is withheld for a person's own search.
    pub withheld_text: WithheldText,
    /// The work the search is cut to: for an agent, the issues of its open work and of its open
    /// rules, whatever its person sees (ADR-0029, decision 3); `None` for a person. The store
    /// reads the issues itself, so a rule that found thousands cuts nothing off.
    pub only: Option<WorkOf>,
    /// The last issue of the page before. The store reads its place in the order itself, so a
    /// cursor holds nothing but this ID.
    pub after: Option<IssueId>,
    /// How many issues a page holds at most.
    pub limit: u64,
}

/// Whose work a search is cut to, and when it is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkOf {
    /// The agent.
    pub agent: bb_core::id::AgentId,
    /// The moment work that ran out stops counting.
    pub at: Timestamp,
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
    /// A tag.
    Tag(TagId),
    /// A label.
    Label(LabelId),
    /// An issue type.
    IssueType(IssueTypeId),
    /// A choice of an own field.
    Choice(ChoiceId),
    /// A priority.
    Priority(Priority),
    /// A status group.
    StatusGroup(StatusGroup),
    /// A range of lengths of time (*Estimated time*, *Time left*, *Time spent*): a length is
    /// grouped by its range, never by its own minutes.
    Length(LengthRange),
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

    /// The totals of *Estimated time* and *Time left* of the issues the search finds, in one
    /// statement (ADR-0035, decision 4): only of the fields among the readable ones of the request,
    /// like a filter. *Time spent* is read through the port of time spent. Like `search_count`, it
    /// ignores `after` and `limit`.
    fn search_lengths(
        &self,
        request: &SearchRequest,
    ) -> impl Future<Output = Result<LengthTotals, SearchFailure>> + Send;
}

/// The totals of the lengths of time of the issues a search finds, for the use case of the totals:
/// none for a field that is not readable or that no issue found has a value of.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LengthTotals {
    /// Whether the search found any issue: a total of nothing is not shown. Never given to a
    /// person (PRIV-13).
    pub found_any: bool,
    /// The sum of *Estimated time*.
    pub estimated: Option<Minutes>,
    /// The sum of *Time left*, each issue's typed or worked out.
    pub time_left: Option<Minutes>,
}
