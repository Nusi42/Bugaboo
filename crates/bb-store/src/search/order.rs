//! The order of a search and the place a page ends at (M3-F24).
//!
//! Every key of `sort by` is an expression of the issue: its own column, or a single value taken
//! from the rows that belong to it (the first by the order of the key). The ID is always the last
//! key, so issues with the same values keep one order and a cursor that carries the values of the
//! last issue never skips or repeats one. Issues without a value come last, in either direction,
//! on both databases (they differ in where they put a `NULL`). Text is ordered by its bytes on both
//! (PostgreSQL's collation is set to `C`), so the same issues come in the same order everywhere.

use bb_app::search::FieldTarget;
use bb_core::id::IssueId;
use bb_searchlang::Direction;
use sea_query::{
    Alias, Cond, DynIden, Expr, ExprTrait as _, Func, IntoIden as _, JoinType, Order, Query,
    SelectStatement,
};

use super::source::{Domain, Subject};
use super::{CompileError, Compiler, nothing};
use crate::db::{Dialect, StoredId as _};
use crate::schema::{
    FieldChoiceRow, GroupRow, IssueRow, IssueTypeRow, LabelRow, PersonRow, ProjectRow, TagRow,
};

/// What a key holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Text,
    Whole,
}

/// A value of a key of the order, as the last issue of the page before has it: read by the store
/// from that issue, never written into a cursor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum SortValue {
    /// No value; such issues come last in either direction.
    Empty,
    /// A text, as written (compared by its bytes).
    Text(String),
    /// A whole number (a moment, a rank).
    Whole(i64),
}

/// What a group is, which says how the value of a row is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GroupValue {
    /// A text that is no person's.
    Text,
    /// A number or a moment.
    Whole,
    /// The ID of a thing.
    Id(IdOf),
    /// The place of the priority.
    Priority,
    /// The place of the status group.
    StatusGroup,
}

/// What an ID in a group is the ID of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum IdOf {
    Tag,
    Label,
    Person,
    Group,
    Issue,
    Project,
    IssueType,
    Choice,
}

/// How a field splits the issues into groups.
pub(super) enum Grouping {
    /// One value for each issue.
    One(Expr, GroupValue),
    /// Any number of values: the rows of the values of the issue, the value as the group is named,
    /// and the condition of an issue that has none.
    Many {
        rows: Box<SelectStatement>,
        value: Expr,
        what: GroupValue,
        without: Cond,
    },
}

/// One key of the order.
pub(super) struct Key {
    pub(super) value: Expr,
    pub(super) kind: Kind,
    pub(super) order: Order,
}

/// The table that gives the name of a thing a column holds an ID of: its table, its ID and its name.
fn names_of(domain: Domain) -> Option<(DynIden, DynIden, DynIden)> {
    let of = |table: DynIden, id: DynIden, name: DynIden| Some((table, id, name));
    match domain {
        Domain::Person => of(
            PersonRow::Table.into_iden(),
            PersonRow::Id.into_iden(),
            PersonRow::DisplayName.into_iden(),
        ),
        Domain::Group => of(
            GroupRow::Table.into_iden(),
            GroupRow::Id.into_iden(),
            GroupRow::Name.into_iden(),
        ),
        Domain::Tag => of(
            TagRow::Table.into_iden(),
            TagRow::Id.into_iden(),
            TagRow::NameKey.into_iden(),
        ),
        Domain::Label => of(
            LabelRow::Table.into_iden(),
            LabelRow::Id.into_iden(),
            LabelRow::NameKey.into_iden(),
        ),
        Domain::Choice(_) => of(
            FieldChoiceRow::Table.into_iden(),
            FieldChoiceRow::Id.into_iden(),
            FieldChoiceRow::Name.into_iden(),
        ),
        Domain::Issue => of(
            IssueRow::Table.into_iden(),
            IssueRow::Id.into_iden(),
            IssueRow::Code.into_iden(),
        ),
        Domain::Type => of(
            IssueTypeRow::Table.into_iden(),
            IssueTypeRow::Id.into_iden(),
            IssueTypeRow::Name.into_iden(),
        ),
        Domain::Project => of(
            ProjectRow::Table.into_iden(),
            ProjectRow::Id.into_iden(),
            ProjectRow::ShortName.into_iden(),
        ),
        _ => None,
    }
}

fn bigint(value: Expr) -> Expr {
    Func::cast_as(value, Alias::new("BIGINT")).into()
}

impl Compiler<'_> {
    /// A text in the order of its bytes on both databases.
    fn bytes(&self, text: Expr) -> Expr {
        match self.dialect {
            Dialect::Sqlite => text,
            Dialect::Postgres => Expr::cust_with_expr("$1 COLLATE \"C\"", text),
        }
    }

    /// The value of a field that stands for an issue in the order, and what it holds. An issue with
    /// many values is placed by its smallest, or by its largest when the key runs the other way
    /// (`desc`), so it is where the best of its values would put it.
    fn sort_value(&self, target: FieldTarget, desc: bool) -> Result<(Expr, Kind), CompileError> {
        let (subject, domain) = self.subject_of(target);
        let pick = |value: Expr| {
            if desc {
                Func::max(value)
            } else {
                Func::min(value)
            }
        };
        let first = |value: Expr| match &subject {
            Subject::Column(_) => value,
            Subject::Linked(rows, _) => {
                let mut rows = (**rows).clone();
                rows.clear_selects().expr(pick(value));
                super::scalar(rows)
            }
        };
        match domain {
            Domain::Priority | Domain::StatusGroup => {
                Ok((bigint(self.rank(&subject, domain)?), Kind::Whole))
            }
            Domain::Text | Domain::Status => {
                Ok((self.bytes(Self::plain(&subject, first)), Kind::Text))
            }
            Domain::Number(_) | Domain::YesNo | Domain::Time(_) => {
                Ok((bigint(Self::plain(&subject, first)), Kind::Whole))
            }
            named => {
                let (table, id, name) =
                    names_of(named).ok_or(CompileError::Unsupported("sorting by this field"))?;
                Ok((
                    self.bytes(Self::named(&subject, table, id, name, pick)),
                    Kind::Text,
                ))
            }
        }
    }

    /// The value as it is stored.
    fn plain(subject: &Subject, first: impl Fn(Expr) -> Expr) -> Expr {
        match subject {
            Subject::Column(value) | Subject::Linked(_, value) => first(value.clone()),
        }
    }

    /// The name of the thing the value is the ID of, by a join on the rows (or on the column).
    fn named(
        subject: &Subject,
        table: DynIden,
        id: DynIden,
        name: DynIden,
        pick: impl Fn(Expr) -> sea_query::FunctionCall,
    ) -> Expr {
        let looked_up = Alias::new("named");
        let name_of = Expr::col((looked_up.clone(), name));
        match subject {
            Subject::Column(value) => {
                let mut one = Query::select();
                one.expr(name_of)
                    .from_as(table, looked_up.clone())
                    .and_where(Expr::col((looked_up, id)).eq(value.clone()));
                super::scalar(one)
            }
            Subject::Linked(rows, value) => {
                let mut rows = (**rows).clone();
                rows.clear_selects().expr(pick(name_of)).join_as(
                    JoinType::InnerJoin,
                    table,
                    looked_up.clone(),
                    Expr::col((looked_up, id)).eq(value.clone()),
                );
                super::scalar(rows)
            }
        }
    }

    /// The place of the value among the levels of its domain (the priority, the status group), or
    /// nothing when it is in none.
    fn rank(&self, subject: &Subject, domain: Domain) -> Result<Expr, CompileError> {
        let Subject::Column(value) = subject else {
            return Ok(Expr::null());
        };
        let mut ranked: Option<sea_query::CaseStatement> = None;
        for (at, level) in self.levels(domain)?.into_iter().enumerate() {
            if level.is_empty() {
                continue;
            }
            let rank = i64::try_from(at).unwrap_or(i64::MAX);
            let when = Cond::all().add(value.clone().is_in(level));
            ranked = Some(match ranked {
                None => Expr::case(when, rank),
                Some(case) => case.case(when, rank),
            });
        }
        Ok(ranked.map_or_else(Expr::null, |case| case.finally(Expr::null()).into()))
    }

    /// How a field groups the issues. A field with one value for an issue is a value of the
    /// issue; a field with many (tags, labels, assigned people, an own field with several
    /// values) puts the issue in the group of each of its values, and an issue with none in the
    /// group of no value. A group is named by the identity of the thing (its ID, or the word of a
    /// built-in choice), never by a name: the use case shows it as the person may see it.
    pub(super) fn grouping(&self, word: &str) -> Result<Grouping, CompileError> {
        let target = self.target(word)?;
        let (subject, domain) = self.subject_of(target);
        let what = group_value_of(domain);
        match subject.clone() {
            Subject::Column(value) => {
                let value = match what {
                    GroupValue::Id(_) | GroupValue::Whole => value,
                    GroupValue::Text => self.bytes(value),
                    GroupValue::Priority | GroupValue::StatusGroup => {
                        bigint(self.rank(&subject, domain)?)
                    }
                };
                Ok(Grouping::One(value, what))
            }
            Subject::Linked(rows, value) => {
                let mut rows = *rows;
                rows.clear_selects();
                let value = match (domain, what) {
                    (_, GroupValue::Text) => self.bytes(value),
                    (_, GroupValue::Whole) => bigint(value),
                    _ => value,
                };
                Ok(Grouping::Many {
                    rows: Box::new(rows),
                    value,
                    what,
                    without: subject.empty(),
                })
            }
        }
    }

    /// The keys of the order of this search, the ID last.
    pub(super) fn keys(&self) -> Result<Vec<Key>, CompileError> {
        let sort = &self.request.search.search().sort;
        if sort.is_empty() {
            let newest = Key {
                value: Expr::col((IssueRow::Table, IssueRow::CreatedAt)),
                kind: Kind::Whole,
                order: Order::Desc,
            };
            // With words to find and no order asked for, the issues whose title has the words
            // come first, then the newest, then the ID (ADR-0016, amendment M5, item 5).
            return Ok(match self.title_hit() {
                Some(hit) => vec![hit, newest],
                None => vec![newest],
            });
        }
        sort.iter()
            .map(|key| {
                let target = self.target(&key.field.word)?;
                let down = matches!(
                    key.direction,
                    Some(Direction::HighestFirst | Direction::NewestFirst | Direction::ZToA)
                );
                let up = matches!(
                    key.direction,
                    Some(Direction::LowestFirst | Direction::OldestFirst | Direction::AToZ)
                );
                // Without a word: the newest, the highest, A to Z. What the key holds is found out
                // first, as the direction it reads in without a word depends on it.
                let desc = down || (!up && kind_of(self.subject_of(target).1) == Kind::Whole);
                let (value, kind) = self.sort_value(target, desc)?;
                Ok(Key {
                    value,
                    kind,
                    order: if desc { Order::Desc } else { Order::Asc },
                })
            })
            .collect()
    }

    /// The key "the title alone has the words" (1 or 0, never empty) of the first `contains` on its
    /// own that every issue found must satisfy. It is an expression of the issue like any other
    /// key, so the store reads its value for the last issue of a page by itself.
    fn title_hit(&self) -> Option<Key> {
        let text = self
            .request
            .search
            .search()
            .filter
            .as_ref()
            .and_then(first_words)?;
        let words = bb_core::text_words::search_words(text);
        let in_title = self.words_in(
            crate::text_index::TextScope::Source(bb_core::text_index::TextSource::Title),
            text,
        );
        if words.is_empty() {
            return None;
        }
        Some(Key {
            value: Expr::case(in_title, 1_i64).finally(0_i64).into(),
            kind: Kind::Whole,
            order: Order::Desc,
        })
    }

    /// The direction of the ID: that of the last key, so ties keep the direction of the list.
    pub(super) fn id_order(keys: &[Key]) -> Order {
        keys.last().map_or(Order::Desc, |key| key.order.clone())
    }

    /// Puts the order on the select, issues without a value last.
    pub(super) fn order_by(select: &mut SelectStatement, keys: &[Key]) {
        for key in keys {
            select.order_by_expr(key.value.clone().is_null(), Order::Asc);
            select.order_by_expr(key.value.clone(), key.order.clone());
        }
        select.order_by_expr(
            Expr::col((IssueRow::Table, IssueRow::Id)),
            Self::id_order(keys),
        );
    }

    /// The issues after the last one of the page before, whose values of the keys are `last`.
    pub(super) fn after(keys: &[Key], last: &[SortValue], id: IssueId) -> Cond {
        let later = |value: &Expr, order: &Order, than: sea_query::Value| {
            if matches!(order, Order::Desc) {
                value.clone().lt(than)
            } else {
                value.clone().gt(than)
            }
        };
        let same = |key: &Key, last: &SortValue| match last {
            SortValue::Empty => key.value.clone().is_null(),
            SortValue::Text(text) => key.value.clone().eq(text.clone()),
            SortValue::Whole(number) => key.value.clone().eq(*number),
        };
        let mut any = Cond::any();
        for (at, (key, value)) in keys.iter().zip(last).enumerate() {
            let mut all = Cond::all();
            for (before, seen) in keys.iter().zip(last).take(at) {
                all = all.add(same(before, seen));
            }
            let after = match value {
                // Issues without a value are last, so none comes after one of them by this key.
                SortValue::Empty => nothing(),
                SortValue::Text(text) => Cond::any()
                    .add(later(&key.value, &key.order, text.clone().into()))
                    .add(key.value.clone().is_null()),
                SortValue::Whole(number) => Cond::any()
                    .add(later(&key.value, &key.order, (*number).into()))
                    .add(key.value.clone().is_null()),
            };
            any = any.add(all.add(after));
        }
        let mut all = Cond::all();
        for (key, value) in keys.iter().zip(last) {
            all = all.add(same(key, value));
        }
        let column = Expr::col((IssueRow::Table, IssueRow::Id));
        all = all.add(if matches!(Self::id_order(keys), Order::Desc) {
            column.lt(id.stored())
        } else {
            column.gt(id.stored())
        });
        any.add(all)
    }
}

/// The words of the first `contains` on its own that every issue found has to satisfy: one of the
/// items of the `and`, not inside an `or` or a `not`.
fn first_words(filter: &bb_searchlang::Expr) -> Option<&str> {
    match filter {
        bb_searchlang::Expr::Contains { text, .. } => Some(&text.text),
        bb_searchlang::Expr::And { items, .. } => items.iter().find_map(first_words),
        _ => None,
    }
}

/// What a key holds, by the domain of the values it sorts.
const fn kind_of(domain: Domain) -> Kind {
    match domain {
        Domain::Priority
        | Domain::StatusGroup
        | Domain::Number(_)
        | Domain::YesNo
        | Domain::Time(_) => Kind::Whole,
        _ => Kind::Text,
    }
}

/// What the groups of a field are, by the domain of its values.
const fn group_value_of(domain: Domain) -> GroupValue {
    match domain {
        Domain::Person => GroupValue::Id(IdOf::Person),
        Domain::Group => GroupValue::Id(IdOf::Group),
        Domain::Issue => GroupValue::Id(IdOf::Issue),
        Domain::Project => GroupValue::Id(IdOf::Project),
        Domain::Type => GroupValue::Id(IdOf::IssueType),
        Domain::Choice(_) => GroupValue::Id(IdOf::Choice),
        Domain::Priority => GroupValue::Priority,
        Domain::StatusGroup => GroupValue::StatusGroup,
        Domain::Number(_) | Domain::YesNo | Domain::Time(_) => GroupValue::Whole,
        Domain::Tag => GroupValue::Id(IdOf::Tag),
        Domain::Label => GroupValue::Id(IdOf::Label),
        Domain::Text | Domain::Status => GroupValue::Text,
    }
}
