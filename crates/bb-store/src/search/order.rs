//! The order of a search and the place a page ends at (M3-F24).
//!
//! Every key of `sort by` is an expression of the issue: its own column, or a single value taken
//! from the rows that belong to it (the first by the order of the key). The ID is always the last
//! key, so issues with the same values keep one order and a cursor that carries the values of the
//! last issue never skips or repeats one. Issues without a value come last, in either direction,
//! on both databases (they differ in where they put a `NULL`). Text is ordered by its bytes on both
//! (PostgreSQL's collation is set to `C`), so the same issues come in the same order everywhere.

use bb_app::search::{FieldTarget, SearchPosition, SortValue};
use bb_core::issue::Priority;
use bb_searchlang::Direction;
use sea_query::{
    Alias, Cond, DynIden, Expr, ExprTrait as _, Func, IntoIden as _, JoinType, Order, Query,
    SelectStatement,
};

use super::source::{Domain, Subject, subject_of};
use super::{CompileError, Compiler, nothing};
use crate::db::{Dialect, StoredId as _};
use crate::schema::{
    FieldChoiceRow, GroupRow, IssueRow, IssueTypeRow, LabelRow, PersonRow, ProjectRow, TagRow,
};

/// What a key holds, which says how it is kept in a cursor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Text,
    Whole,
}

/// How a field splits the issues into groups.
pub(super) enum Grouping {
    /// One value for each issue.
    One(Expr, Kind),
    /// Any number of values: the rows of the values of the issue, the value as the group is named,
    /// and the condition of an issue that has none.
    Many {
        rows: SelectStatement,
        value: Expr,
        kind: Kind,
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

    /// The value of a field that stands for an issue in the order, and what it holds.
    fn sort_value(&self, target: FieldTarget) -> Result<(Expr, Kind), CompileError> {
        let (subject, domain) = subject_of(target);
        let first = |value: Expr| match &subject {
            Subject::Column(_) => value,
            Subject::Linked(rows, _) => {
                let mut rows = (**rows).clone();
                rows.clear_selects().expr(Func::min(value));
                super::scalar(rows)
            }
        };
        match domain {
            Domain::Priority | Domain::StatusGroup => {
                Ok((bigint(self.rank(&subject, domain)), Kind::Whole))
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
                    self.bytes(Self::named(&subject, table, id, name)),
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
    fn named(subject: &Subject, table: DynIden, id: DynIden, name: DynIden) -> Expr {
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
                rows.clear_selects().expr(Func::min(name_of)).join_as(
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
    fn rank(&self, subject: &Subject, domain: Domain) -> Expr {
        let Subject::Column(value) = subject else {
            return Expr::null();
        };
        let levels: Vec<Vec<sea_query::Value>> = match domain {
            Domain::Priority => Priority::ALL
                .iter()
                .map(|each| vec![sea_query::Value::from(each.code())])
                .collect(),
            _ => self
                .request
                .status_groups
                .iter()
                .map(|(_, keys)| keys.iter().cloned().map(sea_query::Value::from).collect())
                .collect(),
        };
        let mut ranked: Option<sea_query::CaseStatement> = None;
        for (at, level) in levels.into_iter().enumerate() {
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
        ranked.map_or_else(Expr::null, |case| case.finally(Expr::null()).into())
    }

    /// How a field groups the issues. A field with one value for an issue is a value of the
    /// issue; a field with many (tags, labels, assigned people, an own field with several
    /// values) puts the issue in the group of each of its values, and an issue with none in the
    /// group of no value.
    pub(super) fn grouping(&self, word: &str) -> Result<Grouping, CompileError> {
        let target = self.target(word)?;
        let (subject, domain) = subject_of(target);
        let Subject::Linked(rows, value) = subject.clone() else {
            let (value, kind) = self.sort_value(target)?;
            return Ok(Grouping::One(value, kind));
        };
        let looked_up = Alias::new("named");
        let (value, kind, name_table) = match domain {
            Domain::Text => (self.bytes(value), Kind::Text, None),
            Domain::Number(_) | Domain::YesNo | Domain::Time(_) => {
                (bigint(value), Kind::Whole, None)
            }
            named => {
                let (table, id, name) =
                    names_of(named).ok_or(CompileError::Unsupported("grouping by this field"))?;
                (
                    self.bytes(Expr::col((looked_up.clone(), name))),
                    Kind::Text,
                    Some((table, Expr::col((looked_up.clone(), id)).eq(value))),
                )
            }
        };
        let mut rows = *rows;
        rows.clear_selects();
        if let Some((table, on)) = name_table {
            rows.join_as(JoinType::InnerJoin, table, looked_up, on);
        }
        Ok(Grouping::Many {
            rows,
            value,
            kind,
            without: subject.empty(),
        })
    }

    /// The keys of the order of this search, the ID last.
    pub(super) fn keys(&self) -> Result<Vec<Key>, CompileError> {
        let sort = &self.request.search.search().sort;
        if sort.is_empty() {
            return Ok(vec![Key {
                value: Expr::col((IssueRow::Table, IssueRow::CreatedAt)),
                kind: Kind::Whole,
                order: Order::Desc,
            }]);
        }
        sort.iter()
            .map(|key| {
                let (value, kind) = self.sort_value(self.target(&key.field.word)?)?;
                let order = match key.direction {
                    Some(Direction::HighestFirst | Direction::NewestFirst | Direction::ZToA) => {
                        Order::Desc
                    }
                    Some(Direction::LowestFirst | Direction::OldestFirst | Direction::AToZ) => {
                        Order::Asc
                    }
                    // What reads first without a word: the newest, the highest, A to Z.
                    None => match kind {
                        Kind::Text => Order::Asc,
                        Kind::Whole => Order::Desc,
                    },
                };
                Ok(Key { value, kind, order })
            })
            .collect()
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

    /// The issues after the last one of the page before.
    pub(super) fn after(keys: &[Key], position: &SearchPosition) -> Result<Cond, CompileError> {
        if position.values.len() != keys.len() {
            return Err(CompileError::Unsupported("a cursor of another order"));
        }
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
        for (at, (key, last)) in keys.iter().zip(&position.values).enumerate() {
            let mut all = Cond::all();
            for (before, seen) in keys.iter().zip(&position.values).take(at) {
                all = all.add(same(before, seen));
            }
            let after = match last {
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
        for (key, last) in keys.iter().zip(&position.values) {
            all = all.add(same(key, last));
        }
        let id = Expr::col((IssueRow::Table, IssueRow::Id));
        let bytes = position.id.stored();
        all = all.add(if matches!(Self::id_order(keys), Order::Desc) {
            id.lt(bytes)
        } else {
            id.gt(bytes)
        });
        Ok(any.add(all))
    }
}
