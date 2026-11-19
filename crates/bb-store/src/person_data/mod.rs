//! The walk of a person's data (ADR-0044, decision 1): the rows of every table that name one
//! person, found by the links of the entity stored in it (`bb_core::personal::PersonLinks`) and
//! nothing else. It is the only code that searches rows by person; it works the same on both
//! databases, because every condition is built from the names in `schema` with the query builder.

mod cells;

use bb_app::ports::PersonData;
use bb_app::privacy::walk::{FoundRow, KindOfRows, ROWS_PAGE, RowsPage};
use bb_core::event::RecordKind;
use bb_core::id::UserId;
use bb_core::personal::{EntityMarks, Link, LinkScope, PersonLink};
use bb_core::whose::{FieldOf, OWN_FIELD_TYPE, PERSON_ADDRESS, Whose, whose};
use sea_query::{Alias, Cond, Expr, ExprTrait as _, Order, Query, SelectStatement};

use crate::AuthStorage;
use crate::db::StoredId as _;
use crate::error::StoreError;
use crate::like;
use crate::schema::{Computed, StoredTable, tables};

impl PersonData for AuthStorage {
    async fn kinds_of(&self, person: UserId) -> Result<Vec<KindOfRows>, StoreError> {
        let all = tables();
        let mut kinds = Vec::new();
        for table in &all {
            let Some(condition) = names_the_person(table, &all, person)? else {
                continue;
            };
            let mut select = Query::select();
            select
                .expr_as(Expr::cust("COUNT(*)"), Computed::Total)
                .from(Alias::new(table.name.as_str()))
                .cond_where(condition);
            let rows: i64 = match self.database().fetch_optional(&select).await? {
                Some(row) => row.get(Computed::Total)?,
                None => 0,
            };
            if rows > 0 {
                kinds.push(KindOfRows {
                    kind: table.name.clone(),
                    marks: table.marks,
                    rows: u64::try_from(rows).map_err(|_| StoreError::Corrupt("a count"))?,
                });
            }
        }
        Ok(kinds)
    }

    async fn rows_linked(
        &self,
        person: UserId,
        kind: &str,
        links: &[Link],
    ) -> Result<u64, StoreError> {
        let all = tables();
        let Some(table) = all.iter().find(|table| table.name == kind) else {
            return Ok(0);
        };
        let mut any = Cond::any();
        let mut some = false;
        for link in table.marks.person_links().links() {
            if link.link().is_some_and(|what| links.contains(&what)) {
                any = any.add(link_names(table, &all, link, person)?);
                some = true;
            }
        }
        if !some {
            return Ok(0);
        }
        let mut select = Query::select();
        select
            .expr_as(Expr::cust("COUNT(*)"), Computed::Total)
            .from(Alias::new(table.name.as_str()))
            .cond_where(any);
        let rows: i64 = match self.database().fetch_optional(&select).await? {
            Some(row) => row.get(Computed::Total)?,
            None => 0,
        };
        u64::try_from(rows).map_err(|_| StoreError::Corrupt("a count"))
    }

    async fn person_rows(
        &self,
        person: UserId,
        kind: &str,
        from: u64,
    ) -> Result<RowsPage, StoreError> {
        let all = tables();
        let Some(table) = all.iter().find(|table| table.name == kind) else {
            return Ok(RowsPage {
                rows: Vec::new(),
                next: None,
            });
        };
        let Some(condition) = names_the_person(table, &all, person)? else {
            return Ok(RowsPage {
                rows: Vec::new(),
                next: None,
            });
        };
        let links = table.marks.person_links().links();
        let mut select = Query::select();
        select.from(Alias::new(table.name.as_str()));
        for (column, _) in &table.columns {
            select.column(Alias::new(column.as_str()));
        }
        // Which link names the person in each row, as a flag of its own.
        for (at, link) in links.iter().enumerate() {
            let names = link_names(table, &all, link, person)?;
            select.expr_as(
                Expr::case(names, Expr::val(1_i64)).finally(Expr::val(0_i64)),
                Alias::new(cells::flag(at)),
            );
        }
        // A fixed order, so the pages neither repeat nor skip a row of a snapshot.
        for (column, _) in &table.columns {
            select.order_by(Alias::new(column.as_str()), Order::Asc);
        }
        select
            .cond_where(condition)
            .limit(ROWS_PAGE + 1)
            .offset(from);
        let mut found = self.database().fetch_all(&select).await?;
        let more = u64::try_from(found.len()).unwrap_or(u64::MAX) > ROWS_PAGE;
        found.truncate(usize::try_from(ROWS_PAGE).unwrap_or(usize::MAX));
        let rows = found
            .iter()
            .map(|row| cells::read_row(row, table, links))
            .collect::<Result<Vec<FoundRow>, StoreError>>()?;
        Ok(RowsPage {
            next: more.then_some(from + ROWS_PAGE),
            rows,
        })
    }
}

/// The condition for the rows of `table` that name `person` by any link; none when the entity
/// stored in it names nobody.
///
/// # Errors
/// [`StoreError::Corrupt`] when a link cannot be turned into a condition (a field it names is not
/// stored): such a link would make the walk miss rows without a word.
fn names_the_person(
    table: &StoredTable,
    all: &[StoredTable],
    person: UserId,
) -> Result<Option<Cond>, StoreError> {
    let links = table.marks.person_links().links();
    if links.is_empty() {
        return Ok(None);
    }
    let mut any = Cond::any();
    for link in links {
        any = any.add(link_names(table, all, link, person)?);
    }
    Ok(Some(any))
}

/// What a link that cannot be turned into a condition reports.
const UNSTORED_LINK: StoreError =
    StoreError::Corrupt("a link of a person to a field that is not stored");

/// The first or the last column of `field` in `table`.
fn column(table: &StoredTable, field: &str, last: bool) -> Result<Alias, StoreError> {
    let columns = columns_of(table, field);
    let found = if last {
        columns.last()
    } else {
        columns.first()
    };
    found.map(|name| Alias::new(*name)).ok_or(UNSTORED_LINK)
}

/// The condition for the rows of `table` in which the field of `link` names `person`.
fn link_names(
    table: &StoredTable,
    all: &[StoredTable],
    link: &PersonLink,
    person: UserId,
) -> Result<Cond, StoreError> {
    let field = link.field();
    let id = person.stored();
    Ok(match link.scope() {
        // The ID is in the last column: the only one, or the one after the kind (an actor).
        LinkScope::Every => Cond::all().add(Expr::col(column(table, field, true)?).eq(id)),
        LinkScope::Record { kind } => {
            let (kind_column, id_column) = match kind {
                None => (column(table, field, false)?, column(table, field, true)?),
                Some(kind) => (column(table, kind, false)?, column(table, field, false)?),
            };
            record_names(all, &kind_column, &id_column, person)?
        }
        LinkScope::FieldType { key, field_type } => Cond::all()
            .add(Expr::col(column(table, field, false)?).eq(id))
            .add(Expr::col(column(table, key, false)?).in_subquery(ids_where(
                all,
                OWN_FIELD_TYPE,
                field_type.code(),
            )?)),
        LinkScope::Email => {
            let (people, of) = (table_of(all, PERSON_ADDRESS.marks)?, PERSON_ADDRESS);
            let mut address = Query::select();
            address
                .column(column(people, of.value, false)?)
                .from(Alias::new(people.name.as_str()))
                .and_where(Expr::col(column(people, of.id, false)?).eq(id));
            Cond::all().add(Expr::col(column(table, field, false)?).in_subquery(address))
        }
        // The ID of a person is written into changes and JSON as its text, which no other value
        // has.
        LinkScope::Changes | LinkScope::Json => Cond::all().add(
            Expr::col(column(table, field, false)?).like(like::containing(&person.to_string())),
        ),
    })
}

/// The rows about a record that is `person` or belongs to them, as `whose` says for each kind.
fn record_names(
    all: &[StoredTable],
    kind_column: &Alias,
    id_column: &Alias,
    person: UserId,
) -> Result<Cond, StoreError> {
    let mut any = Cond::any();
    for kind in RecordKind::ALL {
        let of_the_kind = Expr::col(kind_column.clone()).eq(kind.code());
        let the_record = match whose(*kind) {
            Whose::ThePerson => Expr::col(id_column.clone()).eq(person.stored()),
            Whose::Owner { of } => {
                Expr::col(id_column.clone()).in_subquery(ids_where(all, of, person.stored())?)
            }
            Whose::Nobody => continue,
        };
        any = any.add(Cond::all().add(of_the_kind).add(the_record));
    }
    Ok(any)
}

/// The IDs (the field `of.id`) of the rows of the entity of `of` whose field `of.value` is
/// `value`.
fn ids_where(
    all: &[StoredTable],
    of: FieldOf,
    value: impl Into<sea_query::Value>,
) -> Result<SelectStatement, StoreError> {
    let table = table_of(all, of.marks)?;
    let mut select = Query::select();
    select
        .column(column(table, of.id, false)?)
        .from(Alias::new(table.name.as_str()))
        .and_where(Expr::col(column(table, of.value, true)?).eq(value));
    Ok(select)
}

/// The table the entity of `marks` is stored in.
fn table_of(all: &[StoredTable], marks: EntityMarks) -> Result<&StoredTable, StoreError> {
    all.iter()
        .find(|table| table.marks.name() == marks.name())
        .ok_or(UNSTORED_LINK)
}

/// The columns of `field` in `table`, in the order of the list.
fn columns_of<'t>(table: &'t StoredTable, field: &str) -> Vec<&'t str> {
    table
        .columns
        .iter()
        .filter(|(_, of)| *of == field)
        .map(|(column, _)| column.as_str())
        .collect()
}

#[cfg(test)]
mod tests;
