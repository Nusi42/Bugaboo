//! The port of a search: what the compiler of the store is given and what it answers (QRY-05,
//! SEC-17). The store compiles the checked tree into one statement; it never parses or checks.

use std::future::Future;

use bb_core::id::{
    ChoiceId, FieldId, GroupId, IssueId, IssueTypeId, LabelId, ProjectId, RelationKindId,
    ReleaseId, TagId, UserId,
};
use bb_core::issue::{Issue, Priority, StatusGroup};
use bb_core::people_kinds::PeopleKind;
use bb_core::project::ShortName;
use bb_core::tags::IssueSight;
use bb_core::time::day::FirstDay;
use bb_core::time::range::LengthRange;
use bb_searchlang::CheckedSearch;
use jiff::Timestamp;
use jiff::tz::TimeZone;

use super::target::{FieldTarget, Searchable};
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
    /// The releases that have a name, among those of the projects the person sees.
    pub releases: Vec<(String, Vec<ReleaseId>)>,
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
    /// The releases the person may know, those of the projects they see (ADR-0049, decision 4): a
    /// release field reads only these, so another release is no value for them. Read only for a
    /// search that names a release field (in its filter, its order or its groups); none (not read)
    /// else, and a release field cannot be read without them.
    pub releases_known: Option<Vec<ReleaseId>>,
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
    /// A release.
    Release(ReleaseId),
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
    /// The people of the set of this place in [`GroupBy::people`], as one group: people the
    /// reader sees alike (by the same role, or as *Someone*) are one group (ADR-0022).
    People(u32),
}

/// The place of a group in the order of the groups, by its key: the one order of groups, for the
/// store (its order, and where a page of groups starts) and for the pages of the totals.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct GroupPlace {
    /// The group without a value comes last.
    pub last: bool,
    /// The text of the key: a text as written, an ID as the hex of its bytes (the order the
    /// databases keep IDs in).
    pub text: String,
    /// The number of the key: a number, the start of a range of lengths, the place of a priority or
    /// a status group in the order of a list.
    pub number: i64,
}

impl GroupKey {
    /// The place of a group in the order of the groups: the group without a value last, a
    /// priority and a status group by the order of a list (`LIST_ORDER`, not the order that
    /// sorting and comparing use), a text and an ID by their bytes, a number by its size.
    #[must_use]
    pub fn place(&self) -> GroupPlace {
        /// The place of a word among all the words of its kind.
        fn place_in<T: PartialEq>(all: &[T], one: &T) -> i64 {
            all.iter()
                .position(|each| each == one)
                .and_then(|place| i64::try_from(place).ok())
                .unwrap_or(0)
        }
        /// The hex of the bytes of an ID, which sorts as the bytes do.
        fn hex(bytes: [u8; 16]) -> String {
            use std::fmt::Write as _;
            bytes.iter().fold(String::new(), |mut hex, byte| {
                let _ = write!(hex, "{byte:02x}");
                hex
            })
        }
        let (text, number) = match self {
            Self::Empty => {
                return GroupPlace {
                    last: true,
                    text: String::new(),
                    number: 0,
                };
            }
            Self::Whole(number) => (String::new(), *number),
            Self::Length(range) => (String::new(), i64::from(range.from)),
            Self::People(place) => (String::new(), i64::from(*place)),
            Self::Text(text) => (text.clone(), 0),
            Self::Person(id) => (hex(id.to_bytes()), 0),
            Self::Group(id) => (hex(id.to_bytes()), 0),
            Self::Issue(id) => (hex(id.to_bytes()), 0),
            Self::Project(id) => (hex(id.to_bytes()), 0),
            Self::Tag(id) => (hex(id.to_bytes()), 0),
            Self::Label(id) => (hex(id.to_bytes()), 0),
            Self::IssueType(id) => (hex(id.to_bytes()), 0),
            Self::Choice(id) => (hex(id.to_bytes()), 0),
            Self::Release(id) => (hex(id.to_bytes()), 0),
            Self::Priority(each) => (String::new(), place_in(&Priority::LIST_ORDER, each)),
            Self::StatusGroup(each) => (String::new(), place_in(&StatusGroup::LIST_ORDER, each)),
        };
        GroupPlace {
            last: false,
            text,
            number,
        }
    }
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

/// One step away on the relation map (ISS-16), for the store: the issues one line away from the
/// `near` ones, through the kinds of relation chosen and, with `part_of`, sub-issues and parents,
/// that the person of the request reads and that are not `known` yet; by ID, at most `most`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapStep {
    /// The issues of the step before.
    pub near: Vec<IssueId>,
    /// Every issue on the map so far.
    pub known: Vec<IssueId>,
    /// The kinds of relation chosen.
    pub kinds: Vec<RelationKindId>,
    /// Whether *part of* is chosen.
    pub part_of: bool,
    /// The most issues the step answers.
    pub most: u64,
    /// When the walk began: the whole walk has the time of one search, and a step gets what is
    /// left of it (a step past it is `TooSlow`).
    pub began: std::time::Instant,
}

/// What the search use cases need from storage.
pub trait SearchStore: IssueStore + 'static {
    /// One step away on the relation map, walked only through issues the person of `request`
    /// reads (its sight and, for an agent, its work), in one statement that stops at `step.most`.
    fn map_step(
        &self,
        request: &SearchRequest,
        step: &MapStep,
    ) -> impl Future<Output = Result<Vec<IssueId>, SearchFailure>> + Send;

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

    /// The totals of the issues the search finds (QRY-08, ADR-0050 decision 7): the count and the
    /// sums asked for, of each group of the field `asked.by` (none without it) and of all the
    /// issues together, with a value counted only where `asked` says the reader may read it. Like
    /// `search_count`, it ignores `after` and `limit`.
    fn search_totals(
        &self,
        request: &SearchRequest,
        asked: &TotalsAsked,
    ) -> impl Future<Output = Result<Totals, SearchFailure>> + Send;

    /// The first issues of each cell of a board in the order of the search, at most `window.take`
    /// of a cell after its first `window.skip`, in one statement whatever the number of cells.
    /// The counts of the cells are [`SearchStore::search_totals`] with [`GroupBy::then`].
    fn search_cell_pages(
        &self,
        request: &SearchRequest,
        cells: &Cells,
        window: &CellWindow,
    ) -> impl Future<Output = Result<Vec<CellHit>, SearchFailure>> + Send;
}

/// How the issues of a search fall into the cells of a board: the column of each value of the
/// field of the columns, and the rows (the grouping of the totals).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cells {
    /// The search word of the field of the columns, a field of one value.
    pub columns: String,
    /// The column of each value of that field, by the place of the column; a value that is not
    /// here is in no column.
    pub column_of: Vec<(GroupKey, usize)>,
    /// The rows, as the totals of the board grouped them; none for a board without rows.
    pub rows: Option<GroupBy>,
}

/// One cell of a board: its column, and its row on a board with rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellAt {
    /// The column, by its place.
    pub column: usize,
    /// The group of the row; none on a board without rows.
    pub row: Option<GroupKey>,
}

/// An issue of a cell.
#[derive(Clone, Debug)]
pub struct CellHit {
    /// The cell.
    pub at: CellAt,
    /// The issue.
    pub issue: Issue,
}

/// Which issues of the cells a statement reads: after the first `skip` of each, at most `take`;
/// only those of one cell for *Show more*.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellWindow {
    /// How many issues of each cell come before.
    pub skip: u64,
    /// How many issues of each cell are read at most.
    pub take: u64,
    /// The one cell; none for every cell.
    pub only: Option<CellAt>,
    /// The groups of the rows whose cells are read, on a board with rows: the rows the board shows.
    pub rows: Vec<GroupKey>,
}

/// The most sums one total may ask for, `count` not counted: the one limit of the totals, which
/// the store sizes its columns by.
pub const SUMS_MAX: usize = 8;

/// What the totals of a search are asked for: the field the issues are grouped by, if any, the
/// sums of each group, and whether the total of all of them is wanted.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TotalsAsked {
    /// The field the issues are grouped by; none for the total of all of them only.
    pub by: Option<GroupBy>,
    /// The sums, in the order the answer gives them; at most [`SUMS_MAX`].
    pub sums: Vec<SumAsked>,
    /// Whether the total of all the issues is wanted. Without it no statement is spent on it and
    /// [`Totals::all`] counts nothing.
    pub all: bool,
}

/// The field the issues of a total are grouped by.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupBy {
    /// The word of the field, among the fields of the request.
    pub word: String,
    /// The people that are one group each set ([`GroupKey::People`]); empty while every person is
    /// a group of their own.
    pub people: Vec<Vec<UserId>>,
    /// A second field, of one value for an issue, that splits each group again (the columns of a
    /// board inside its rows, [`GroupTotal::then`]); none for one grouping.
    pub then: Option<String>,
    /// A page of the groups in the order of their places; none for every group. The store cuts
    /// the page in the statement where the order of its values is the order of the places (texts,
    /// numbers, lengths, IDs), and answers every group otherwise, which the caller pages.
    pub page: Option<GroupPage>,
}

impl GroupBy {
    /// The issues grouped by the field `word`: every person a group of their own, every group
    /// read, no second field.
    #[must_use]
    pub fn of(word: impl Into<String>) -> Self {
        Self {
            word: word.into(),
            people: Vec::new(),
            page: None,
            then: None,
        }
    }

    /// The same, with the people of each set as one group.
    #[must_use]
    pub fn with_people(mut self, people: Vec<Vec<UserId>>) -> Self {
        self.people = people;
        self
    }

    /// The same, with the groups read as `page` says.
    #[must_use]
    pub fn paged(mut self, page: Option<GroupPage>) -> Self {
        self.page = page;
        self
    }

    /// The same, each group split again by the field of one value `word`.
    #[must_use]
    pub fn split_by(mut self, word: impl Into<String>) -> Self {
        self.then = Some(word.into());
        self
    }
}

/// A page of the groups of a total.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupPage {
    /// The place of the last group of the page before; none for the first page.
    pub after: Option<GroupPlace>,
    /// How many groups at most.
    pub limit: u64,
}

/// One sum of a total: the field it adds up, and where the reader may read its values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SumAsked {
    /// The field: *Estimated time*, *Time left*, *Time spent* or an own field of numbers.
    pub target: FieldTarget,
    /// Where a value of the field counts.
    pub readable: ReadableWhere,
}

/// Where the values of a summed field count (PRIV-13, SEC-04): the one answer of the use case,
/// which the statement adds to each value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadableWhere {
    /// On every issue the search finds: the field is among the fields of the request, which are
    /// readable everywhere it looks.
    Everywhere,
    /// Only on the issues of these places; nowhere when there are none.
    In(Vec<FieldPlace>),
}

/// A project where the reader may read a field, and on which issue types.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldPlace {
    /// The project.
    pub project: ProjectId,
    /// The issue types the reader may read the field on there; `None` for all of them.
    pub types: Option<Vec<IssueTypeId>>,
}

/// The totals of a search: one row for each group, and one for all the issues together.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Totals {
    /// The groups, in the order of their keys (the group without a value last); empty when the
    /// totals were not grouped.
    pub groups: Vec<GroupTotal>,
    /// All the issues the search finds: an issue in several groups counts once. Nothing when it
    /// was not asked for (`TotalsAsked::all`).
    pub all: Total,
}

/// The total of one group.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupTotal {
    /// What the group is.
    pub key: GroupKey,
    /// Its value of the second field ([`GroupBy::then`]); none without one.
    pub then: Option<GroupKey>,
    /// Its total.
    pub total: Total,
}

/// The count and the sums of some issues.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct Total {
    /// How many issues the search finds there.
    pub issues: u64,
    /// The sums, in the order they were asked for.
    pub sums: Vec<Sum>,
}

/// One sum of a total, over the values the reader may read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct Sum {
    /// The sum, in the unit of the field (whole minutes for a length, the stored number of an own
    /// field); none when no issue gave a value.
    pub sum: Option<i64>,
    /// How many issues gave a value the reader may read.
    pub valued: u64,
}

impl Total {
    /// The total of two parts with no issue in common: the counts and the sums added.
    #[must_use]
    pub fn plus(&self, other: &Self) -> Self {
        Self {
            issues: self.issues.saturating_add(other.issues),
            sums: (0..self.sums.len().max(other.sums.len()))
                .map(|at| {
                    let one = self.sums.get(at).copied().unwrap_or_default();
                    let two = other.sums.get(at).copied().unwrap_or_default();
                    Sum {
                        sum: match (one.sum, two.sum) {
                            (Some(one), Some(two)) => Some(one.saturating_add(two)),
                            (one, two) => one.or(two),
                        },
                        valued: one.valued.saturating_add(two.valued),
                    }
                })
                .collect(),
        }
    }
}
