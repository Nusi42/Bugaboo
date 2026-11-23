//! The walk of a person's data (ADR-0044, decision 1): the rows of every table that name one
//! person, found by the links of the entity stored in it (`bb_core::personal::PersonLinks`) and
//! nothing else. It is the only code that searches rows by person; it works the same on both
//! databases, because every condition is built from the names in `schema` with the query builder.
//!
//! The rows of a table are found in arms (`arms.rs`): each arm is one condition that one index
//! answers, and no row is in two arms, so the walk counts and pages without reading a table whole
//! (M9-F144).

mod arms;
mod cells;
mod removal;

use bb_app::ports::PersonData;
use bb_app::privacy::walk::{FoundRow, KindOfRows, ROWS_PAGE, RowsPage};
use bb_core::event::RecordKind;
use bb_core::id::UserId;
use bb_core::personal::{EntityMarks, Link, LinkScope, PersonLink};
use bb_core::walk_index::is_index_of_the_walk;
use bb_core::whose::{FieldOf, OWN_FIELD_TYPE, PERSON_ADDRESS, Whose, whose};
use sea_query::{Alias, Cond, Expr, ExprTrait as _, Query, SelectStatement};

use crate::AuthStorage;
use crate::db::{Dialect, StoredId as _, Transaction};
use crate::error::StoreError;
use crate::like;
use crate::schema::{Computed, StoredTable, tables};
use crate::walk_index::{Indexed, may_name, member_of};

impl PersonData for AuthStorage {
    async fn kinds_of(&self, person: UserId) -> Result<Vec<KindOfRows>, StoreError> {
        let all = tables();
        let dialect = self.database().dialect();
        let mut kinds = Vec::new();
        // One snapshot for every count, read without parallel workers.
        let mut reads = self.database().begin_read_serial().await?;
        for table in &all {
            let Some(arms) = names_the_person(table, &all, person, dialect)? else {
                continue;
            };
            let rows = walk_count(&mut reads, &arms::counted(table, arms)).await?;
            if rows > 0 {
                kinds.push(KindOfRows {
                    kind: table.name.clone(),
                    marks: table.marks,
                    rows,
                });
            }
        }
        reads.commit().await?;
        Ok(kinds)
    }

    async fn rows_linked(
        &self,
        person: UserId,
        kind: &str,
        links: &[Link],
    ) -> Result<u64, StoreError> {
        let all = tables();
        let dialect = self.database().dialect();
        let Some(table) = all.iter().find(|table| table.name == kind) else {
            return Ok(0);
        };
        let arms = arms::arms(table, &all, person, dialect, |link| {
            link.link().is_some_and(|what| links.contains(&what))
        })?;
        if arms.is_empty() {
            return Ok(0);
        }
        let mut reads = self.database().begin_read_serial().await?;
        let rows = walk_count(&mut reads, &arms::counted(table, arms)).await?;
        reads.commit().await?;
        Ok(rows)
    }

    async fn person_rows(
        &self,
        person: UserId,
        kind: &str,
        from: u64,
    ) -> Result<RowsPage, StoreError> {
        let all = tables();
        let dialect = self.database().dialect();
        let Some(table) = all.iter().find(|table| table.name == kind) else {
            return Ok(RowsPage {
                rows: Vec::new(),
                next: None,
            });
        };
        let Some(arms) = names_the_person(table, &all, person, dialect)? else {
            return Ok(RowsPage {
                rows: Vec::new(),
                next: None,
            });
        };
        let links = table.marks.person_links().links();
        // Which link names the person in each row, as a flag of its own; for a link that holds a
        // person only in the rows of an own field of its type, whether the row is one of them.
        let mut flags = Vec::new();
        for (at, link) in links.iter().enumerate() {
            let names = link_names(table, &all, link, person, dialect)?;
            flags.push((
                Expr::case(names, Expr::val(1_i64))
                    .finally(Expr::val(0_i64))
                    .into(),
                Alias::new(cells::flag(at)),
            ));
            if let Some(typed) = of_its_type(table, &all, link)? {
                flags.push((
                    Expr::case(typed, Expr::val(1_i64))
                        .finally(Expr::val(0_i64))
                        .into(),
                    Alias::new(cells::typed(at)),
                ));
            }
        }
        let mut select = arms::rows(table, arms, &flags);
        select.limit(ROWS_PAGE + 1).offset(from);
        let mut reads = self.database().begin_read_serial().await?;
        let mut found = reads.fetch_all(&select).await?;
        reads.commit().await?;
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

/// The number a statement of [`arms::counted`] counts.
async fn walk_count(reads: &mut Transaction, select: &SelectStatement) -> Result<u64, StoreError> {
    let rows: i64 = match reads.fetch_optional(select).await? {
        Some(row) => row.get(Computed::Total)?,
        None => 0,
    };
    u64::try_from(rows).map_err(|_| StoreError::Corrupt("a count"))
}

/// The condition for the rows of `table` that name `person` by a link that is one of `links`;
/// none when the table has none of the links.
fn linked(
    table: &StoredTable,
    all: &[StoredTable],
    person: UserId,
    links: &[Link],
    dialect: Dialect,
) -> Result<Option<Cond>, StoreError> {
    let mut any = Cond::any();
    let mut some = false;
    for link in table.marks.person_links().links() {
        if link.link().is_some_and(|what| links.contains(&what)) {
            any = any.add(link_names(table, all, link, person, dialect)?);
            some = true;
        }
    }
    Ok(some.then_some(any))
}

/// The arms of the rows of `table` that name `person` by any link; none when the entity stored in
/// it names nobody, or is the index of the walk itself.
///
/// # Errors
/// [`StoreError::Corrupt`] when a link cannot be turned into a condition (a field it names is not
/// stored): such a link would make the walk miss rows without a word.
fn names_the_person(
    table: &StoredTable,
    all: &[StoredTable],
    person: UserId,
    dialect: Dialect,
) -> Result<Option<Vec<Cond>>, StoreError> {
    if table.marks.person_links().links().is_empty() || is_index_of_the_walk(&table.marks) {
        return Ok(None);
    }
    arms::arms(table, all, person, dialect, |_| true).map(Some)
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

/// For a link that holds a person only in the rows of an own field of its type, the condition for
/// those rows; none for every other link.
fn of_its_type(
    table: &StoredTable,
    all: &[StoredTable],
    link: &PersonLink,
) -> Result<Option<Cond>, StoreError> {
    let LinkScope::FieldType { key, field_type } = link.scope() else {
        return Ok(None);
    };
    Ok(Some(Cond::all().add(
        Expr::col(column(table, key, false)?).in_subquery(ids_where(
            all,
            OWN_FIELD_TYPE,
            field_type.code(),
        )?),
    )))
}

/// The condition for the rows of `table` in which the field of `link` names `person`.
fn link_names(
    table: &StoredTable,
    all: &[StoredTable],
    link: &PersonLink,
    person: UserId,
    dialect: Dialect,
) -> Result<Cond, StoreError> {
    Ok(link_parts(table, all, link, person, dialect)?
        .into_iter()
        .fold(Cond::any(), Cond::add))
}

/// The rows of `table` in which the field of `link` names `person`, in parts that one index each
/// answers and that share no row: one part, or for a reference to a record one for each kind.
fn link_parts(
    table: &StoredTable,
    all: &[StoredTable],
    link: &PersonLink,
    person: UserId,
    dialect: Dialect,
) -> Result<Vec<Cond>, StoreError> {
    let field = link.field();
    let id = person.stored();
    Ok(match link.scope() {
        // The ID is in the last column: the only one, or the one after the kind (an actor).
        LinkScope::Every => vec![Cond::all().add(Expr::col(column(table, field, true)?).eq(id))],
        LinkScope::Record { kind } => {
            let (kind_column, id_column) = match kind {
                None => (column(table, field, false)?, column(table, field, true)?),
                Some(kind) => (column(table, kind, false)?, column(table, field, false)?),
            };
            record_parts(
                all,
                RecordKind::ALL,
                &kind_column,
                &id_column,
                (person, dialect),
            )?
        }
        LinkScope::FieldType { .. } => {
            let typed = of_its_type(table, all, link)?.ok_or(StoreError::Corrupt("a link"))?;
            vec![
                Cond::all()
                    .add(Expr::col(column(table, field, false)?).eq(id))
                    .add(typed),
            ]
        }
        LinkScope::Email => {
            let (people, of) = (table_of(all, PERSON_ADDRESS.marks)?, PERSON_ADDRESS);
            let mut address = Query::select();
            address
                .column(column(people, of.value, false)?)
                .from(Alias::new(people.name.as_str()))
                .and_where(Expr::col(column(people, of.id, false)?).eq(id));
            vec![Cond::all().add(member_of(column(table, field, false)?, address, dialect))]
        }
        // The ID of a person is written into changes and JSON as its text, which no other value
        // has. The index of the walk names the rows that may hold it, so no table is read whole;
        // the text decides, as it did without the index.
        LinkScope::Changes | LinkScope::Json => {
            let of = Indexed::of_table(&table.name).ok_or(UNINDEXED_LINK)?;
            vec![Cond::all().add(may_name(of, id, dialect)).add(
                Expr::col(column(table, field, false)?).like(like::containing(&person.to_string())),
            )]
        }
    })
}

/// What a link into changes or JSON of a table without the index of the walk reports: the walk
/// would have to read the whole table.
const UNINDEXED_LINK: StoreError =
    StoreError::Corrupt("a link of a person into changes or JSON of a table without an index");

/// The rows about a record of one of `kinds` that is `person` or belongs to them, as `whose` says
/// for each kind: a part for each kind, which the index of the kind and the ID answers.
fn record_parts(
    all: &[StoredTable],
    kinds: &[RecordKind],
    kind_column: &Alias,
    id_column: &Alias,
    (person, dialect): (UserId, Dialect),
) -> Result<Vec<Cond>, StoreError> {
    let mut parts = Vec::new();
    for kind in kinds {
        let of_the_kind = Expr::col(kind_column.clone()).eq(kind.code());
        let the_record = match whose(*kind) {
            Whose::ThePerson => Expr::col(id_column.clone()).eq(person.stored()),
            Whose::Owner { of } => member_of(
                id_column.clone(),
                ids_where(all, of, person.stored())?,
                dialect,
            ),
            Whose::Nobody => continue,
        };
        parts.push(Cond::all().add(of_the_kind).add(the_record));
    }
    Ok(parts)
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
