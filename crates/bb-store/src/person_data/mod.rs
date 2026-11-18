//! The walk of a person's data (ADR-0044, decision 1): the rows of every table that name one
//! person, found by the links of the entity stored in it (`bb_core::personal::PersonLinks`) and
//! nothing else. It is the only code that searches rows by person; it works the same on both
//! databases, because every condition is built from the names in `schema` with the query builder.

use bb_core::event::{RecordKind, Whose, whose};
use bb_core::id::UserId;
use bb_core::personal::{Entity, EntityMarks, LinkScope, PersonLink};
use sea_query::{Alias, Cond, Expr, ExprTrait as _, Query, SelectStatement};

use crate::AuthStorage;
use crate::db::StoredId as _;
use crate::error::StoreError;
use crate::like;
use crate::schema::{Computed, StoredTable, tables};

/// How many rows of one table name a person.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowCount {
    /// The table.
    pub table: String,
    /// The entity stored in it.
    pub entity: &'static str,
    /// How many of its rows name the person.
    pub rows: u64,
}

impl AuthStorage {
    /// How many rows of each table name `person`, by any of the links of its entity; only the
    /// tables with at least one such row, in the order of the list of tables. What the removal
    /// plan counts (ADR-0044, decision 6).
    ///
    /// # Errors
    /// Fails when the database fails.
    pub async fn person_row_counts(&self, person: UserId) -> Result<Vec<RowCount>, StoreError> {
        let all = tables();
        let mut counts = Vec::new();
        for table in &all {
            let Some(condition) = names_the_person(table, &all, person) else {
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
                counts.push(RowCount {
                    table: table.name.clone(),
                    entity: table.marks.name(),
                    rows: u64::try_from(rows).map_err(|_| StoreError::Corrupt("a count"))?,
                });
            }
        }
        Ok(counts)
    }
}

/// The condition for the rows of `table` that name `person` by any link; none when the entity
/// stored in it names nobody.
fn names_the_person(table: &StoredTable, all: &[StoredTable], person: UserId) -> Option<Cond> {
    let links = table.marks.person_links().links();
    if links.is_empty() {
        return None;
    }
    let mut any = Cond::any();
    for link in links {
        if let Some(condition) = link_names(table, all, link, person) {
            any = any.add(condition);
        }
    }
    Some(any)
}

/// The condition for the rows of `table` in which the field of `link` names `person`.
fn link_names(
    table: &StoredTable,
    all: &[StoredTable],
    link: &PersonLink,
    person: UserId,
) -> Option<Cond> {
    let columns = columns_of(table, link.field());
    let id = person.stored();
    match link.scope() {
        // The ID is in the last column: the only one, or the one after the kind (an actor).
        LinkScope::Every => Some(Cond::all().add(Expr::col(Alias::new(*columns.last()?)).eq(id))),
        LinkScope::Record { kind } => {
            let (kind_column, id_column) = match kind {
                None => (*columns.first()?, *columns.last()?),
                Some(kind) => (*columns_of(table, kind).first()?, *columns.first()?),
            };
            Some(record_names(all, kind_column, id_column, person))
        }
        LinkScope::FieldType { key, field_type } => {
            let own_fields = table_of(all, <bb_core::own_fields::OwnField as Entity>::MARKS)?;
            let mut fields_of_the_type = Query::select();
            fields_of_the_type
                .column(Alias::new(*columns_of(own_fields, ID).first()?))
                .from(Alias::new(own_fields.name.as_str()))
                .and_where(
                    Expr::col(Alias::new(*columns_of(own_fields, OWN_FIELD_TYPE).first()?))
                        .eq(field_type.code()),
                );
            Some(
                Cond::all()
                    .add(Expr::col(Alias::new(*columns.first()?)).eq(id))
                    .add(
                        Expr::col(Alias::new(*columns_of(table, key).first()?))
                            .in_subquery(fields_of_the_type),
                    ),
            )
        }
        LinkScope::Email => {
            let people = table_of(all, <bb_core::person::Person as Entity>::MARKS)?;
            let mut address = Query::select();
            address
                .column(Alias::new(*columns_of(people, PERSON_EMAIL).first()?))
                .from(Alias::new(people.name.as_str()))
                .and_where(Expr::col(Alias::new(*columns_of(people, ID).first()?)).eq(id));
            Some(Cond::all().add(Expr::col(Alias::new(*columns.first()?)).in_subquery(address)))
        }
        // The ID of a person is written into the changes as its text, which no other value has.
        LinkScope::Changes => Some(Cond::all().add(
            Expr::col(Alias::new(*columns.first()?)).like(like::containing(&person.to_string())),
        )),
    }
}

/// The rows about a record that is `person` or belongs to them, as `whose` says for each kind.
fn record_names(all: &[StoredTable], kind_column: &str, id_column: &str, person: UserId) -> Cond {
    let mut any = Cond::any();
    for kind in RecordKind::ALL {
        let of_the_kind = Expr::col(Alias::new(kind_column)).eq(kind.code());
        let the_record = match whose(*kind) {
            Whose::ThePerson => Expr::col(Alias::new(id_column)).eq(person.stored()),
            Whose::Owner { marks, owner } => {
                let Some(records) = owned_by(all, marks, owner, person) else {
                    continue;
                };
                Expr::col(Alias::new(id_column)).in_subquery(records)
            }
            Whose::Nobody => continue,
        };
        any = any.add(Cond::all().add(of_the_kind).add(the_record));
    }
    any
}

/// The IDs of the records of the entity of `marks` whose field `owner` names `person`.
fn owned_by(
    all: &[StoredTable],
    marks: EntityMarks,
    owner: &str,
    person: UserId,
) -> Option<SelectStatement> {
    let table = table_of(all, marks)?;
    let mut select = Query::select();
    select
        .column(Alias::new(ID))
        .from(Alias::new(table.name.as_str()))
        .and_where(Expr::col(Alias::new(*columns_of(table, owner).last()?)).eq(person.stored()));
    Some(select)
}

/// The field (and, for the records of the second hop, the column) that holds the ID of a record.
const ID: &str = "id";
/// The field of an own field that holds its type.
const OWN_FIELD_TYPE: &str = "kind";
/// The field of a person that holds their e-mail address.
const PERSON_EMAIL: &str = "email";

/// The table the entity of `marks` is stored in.
fn table_of(all: &[StoredTable], marks: EntityMarks) -> Option<&StoredTable> {
    all.iter().find(|table| table.marks.name() == marks.name())
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
