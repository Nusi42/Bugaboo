//! What a field reads: where its values are, and what kind of values they are.
//!
//! A field is either one column of the issue ([`Subject::Column`]) or values kept in rows of
//! another table, any number of them for one issue ([`Subject::Linked`]); "some value is" and "no
//! value is" are then `EXISTS` and `NOT EXISTS`, which is one way to say it for assigned people,
//! tags, labels and the own fields alike (ADR-0006).

use bb_app::search::FieldTarget;
use bb_app::search::OwnReach;
use bb_core::event::{EntityKind, RecordKind};
use bb_core::id::FieldId;
use bb_core::own_fields::FieldType;
use bb_core::tags::IssueSight;
use sea_query::{Alias, Cond, Expr, ExprTrait as _, Query, SelectStatement};

use super::time::Clock;
use crate::db::StoredId as _;
use crate::fields::kind_text;
use crate::schema::{
    AssignedPersonRow, FieldValueRow, IssueLabelRow, IssueParentRow, IssueRow, IssueTagRow,
};
use crate::sight::sight_condition_as;

/// Where the values of a field are.
#[derive(Clone)]
pub(super) enum Subject {
    /// One value in a column of the issue; none when the column is `NULL`.
    Column(Expr),
    /// Any number of values in the rows of a table: the select asks for the rows of the issue
    /// (`SELECT 1 FROM table WHERE table.issue = issue.id`), and the expression is the value.
    Linked(Box<SelectStatement>, Expr),
}

impl Subject {
    /// Some value of the field satisfies `condition`.
    pub(super) fn holds(&self, condition: impl FnOnce(Expr) -> Cond) -> Cond {
        match self {
            Self::Column(value) => condition(value.clone()),
            Self::Linked(rows, value) => {
                let mut rows = (**rows).clone();
                rows.cond_where(condition(value.clone()));
                Cond::all().add(Expr::exists(rows))
            }
        }
    }

    /// The field has no value.
    pub(super) fn empty(&self) -> Cond {
        match self {
            Self::Column(value) => Cond::all().add(value.clone().is_null()),
            Self::Linked(rows, _) => Cond::all().add(Expr::exists((**rows).clone()).not()),
        }
    }
}

/// What kind of values a field holds, which says how a value in a search is read and compared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Domain {
    /// A text, compared as written (a database's case rules are never used, ADR-0005 decision 9).
    Text,
    /// The words of the priority.
    Priority,
    /// The names of the statuses.
    Status,
    /// The names of the status groups.
    StatusGroup,
    /// The names of the issue types.
    Type,
    /// A project by its short name.
    Project,
    /// An issue by its ID.
    Issue,
    /// A person by name or `me`.
    Person,
    /// A group by name.
    Group,
    /// A tag by name.
    Tag,
    /// A label by name.
    Label,
    /// A whole number or a decimal; the number is the count of parts in one.
    Number(i64),
    /// Yes or no.
    YesNo,
    /// A moment or a day.
    Time(Clock),
    /// A choice of an own field.
    Choice(FieldId),
}

/// The rows of a table that belong to the issue being judged.
fn rows_of(
    table: impl sea_query::Iden + Copy + 'static,
    issue: impl sea_query::Iden + Copy + 'static,
) -> SelectStatement {
    let mut rows = Query::select();
    rows.expr(Expr::val(1))
        .from(table)
        .and_where(Expr::col((table, issue)).equals((IssueRow::Table, IssueRow::Id)));
    rows
}

fn issue_column(column: IssueRow) -> Expr {
    Expr::col((IssueRow::Table, column))
}

/// The values of an own field: the rows of the issue for the field that hold a value, only in the
/// projects the field reaches and for the issue types it is used for (a row that stays on an issue
/// that moved elsewhere is no value).
fn own_values(field: FieldId, column: FieldValueRow, reach: &[OwnReach]) -> Subject {
    let mut rows = Query::select();
    rows.expr(Expr::val(1))
        .from(FieldValueRow::Table)
        .and_where(
            Expr::col((FieldValueRow::Table, FieldValueRow::EntityKind))
                .eq(kind_text(&EntityKind::from(RecordKind::Issue))),
        )
        .and_where(
            Expr::col((FieldValueRow::Table, FieldValueRow::EntityId))
                .equals((IssueRow::Table, IssueRow::Id)),
        )
        .and_where(Expr::col((FieldValueRow::Table, FieldValueRow::Field)).eq(field.stored()))
        .and_where(Expr::col((FieldValueRow::Table, column)).is_not_null());
    match reach.iter().find(|each| each.field == field) {
        Some(reached) => {
            rows.and_where(
                issue_column(IssueRow::Project)
                    .is_in(reached.projects.iter().map(|project| project.stored())),
            );
            if let Some(types) = &reached.issue_types {
                rows.and_where(
                    issue_column(IssueRow::IssueType).is_in(types.iter().map(|kind| kind.stored())),
                );
            }
        }
        None => {
            rows.and_where(Expr::val(1).eq(2));
        }
    }
    Subject::Linked(Box::new(rows), Expr::col((FieldValueRow::Table, column)))
}

/// The name the issue at the other end of a row has in the statement.
const OTHER_END: &str = "other_end";

/// A subject whose values are issues counts only the ones the person reads: a row that points to
/// an issue they do not read is no value, so nothing about that issue (that it exists, its code, how
/// many sit under it) can be found out by filtering, sorting or grouping (ADR-0004, PRIV-13).
fn reading_the_other_end(subject: Subject, sight: &IssueSight) -> Subject {
    let Subject::Linked(mut rows, value) = subject else {
        return subject;
    };
    let mut reads = Query::select();
    reads
        .expr(Expr::val(1))
        .from_as(IssueRow::Table, Alias::new(OTHER_END))
        .and_where(Expr::col((Alias::new(OTHER_END), IssueRow::Id)).eq(value.clone()))
        .cond_where(sight_condition_as(sight, OTHER_END));
    rows.and_where(Expr::exists(reads));
    Subject::Linked(rows, value)
}

/// What the target reads.
pub(super) fn subject_of(
    target: FieldTarget,
    sight: &IssueSight,
    reach: &[OwnReach],
) -> (Subject, Domain) {
    use FieldTarget as T;
    let linked = |rows, value| Subject::Linked(Box::new(rows), value);
    match target {
        T::Title => (Subject::Column(issue_column(IssueRow::Title)), Domain::Text),
        T::Description => (
            Subject::Column(issue_column(IssueRow::Description)),
            Domain::Text,
        ),
        T::Status => (
            Subject::Column(issue_column(IssueRow::Status)),
            Domain::Status,
        ),
        T::StatusGroup => (
            Subject::Column(issue_column(IssueRow::Status)),
            Domain::StatusGroup,
        ),
        T::Priority => (
            Subject::Column(issue_column(IssueRow::Priority)),
            Domain::Priority,
        ),
        T::Type => (
            Subject::Column(issue_column(IssueRow::IssueType)),
            Domain::Type,
        ),
        T::Project => (
            Subject::Column(issue_column(IssueRow::Project)),
            Domain::Project,
        ),
        T::ReportedBy => (
            Subject::Column(issue_column(IssueRow::ReportedBy)),
            Domain::Person,
        ),
        T::AssignedTo => (
            linked(
                rows_of(AssignedPersonRow::Table, AssignedPersonRow::Issue),
                Expr::col((AssignedPersonRow::Table, AssignedPersonRow::Person)),
            ),
            Domain::Person,
        ),
        T::Tags => (
            linked(
                rows_of(IssueTagRow::Table, IssueTagRow::Issue),
                Expr::col((IssueTagRow::Table, IssueTagRow::Tag)),
            ),
            Domain::Tag,
        ),
        T::Labels => (
            linked(
                rows_of(IssueLabelRow::Table, IssueLabelRow::Issue),
                Expr::col((IssueLabelRow::Table, IssueLabelRow::Label)),
            ),
            Domain::Label,
        ),
        T::Parent => (
            reading_the_other_end(
                linked(
                    rows_of(IssueParentRow::Table, IssueParentRow::Child),
                    Expr::col((IssueParentRow::Table, IssueParentRow::Parent)),
                ),
                sight,
            ),
            Domain::Issue,
        ),
        T::Created => (
            Subject::Column(issue_column(IssueRow::CreatedAt)),
            Domain::Time(Clock::Micros),
        ),
        T::Changed => (
            Subject::Column(issue_column(IssueRow::UpdatedAt)),
            Domain::Time(Clock::Micros),
        ),
        T::Closed => (
            Subject::Column(issue_column(IssueRow::ClosedAt)),
            Domain::Time(Clock::Micros),
        ),
        T::Own { field, kind } => own_subject(field, kind, reach, sight),
    }
}

/// What an own field of this kind reads: the column of its values that holds them.
fn own_subject(
    field: FieldId,
    kind: FieldType,
    reach: &[OwnReach],
    sight: &IssueSight,
) -> (Subject, Domain) {
    match kind {
        FieldType::Text | FieldType::LongText | FieldType::WebAddress => (
            own_values(field, FieldValueRow::ValueText, reach),
            Domain::Text,
        ),
        FieldType::WholeNumber | FieldType::Duration => (
            own_values(field, FieldValueRow::ValueNumber, reach),
            Domain::Number(1),
        ),
        FieldType::Decimal => (
            own_values(field, FieldValueRow::ValueNumber, reach),
            Domain::Number(bb_core::own_fields::DECIMAL_PARTS),
        ),
        FieldType::YesNo => (
            own_values(field, FieldValueRow::ValueNumber, reach),
            Domain::YesNo,
        ),
        FieldType::Date => (
            own_values(field, FieldValueRow::ValueDate, reach),
            Domain::Time(Clock::Days),
        ),
        FieldType::DateTime => (
            own_values(field, FieldValueRow::ValueDate, reach),
            Domain::Time(Clock::Micros),
        ),
        FieldType::PickOne | FieldType::PickSeveral => (
            own_values(field, FieldValueRow::ValueRef, reach),
            Domain::Choice(field),
        ),
        FieldType::Person | FieldType::People => (
            own_values(field, FieldValueRow::ValueRef, reach),
            Domain::Person,
        ),
        FieldType::Group => (
            own_values(field, FieldValueRow::ValueRef, reach),
            Domain::Group,
        ),
        FieldType::Issue => (
            reading_the_other_end(own_values(field, FieldValueRow::ValueRef, reach), sight),
            Domain::Issue,
        ),
    }
}
