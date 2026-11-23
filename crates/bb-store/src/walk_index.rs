//! The index of the walk of a person's data (ADR-0044, decision 1, amendment *the index of the
//! walk*): where the changes and the JSON of a table name a person by ID. The one writer
//! ([`note`]) runs in the change that writes the row, so the index is never behind; the walk asks
//! it which rows may name the person ([`may_name`]) and decides by the text as before, so it finds
//! exactly what a search of the whole table found. [`fill`] writes the index of the rows that are
//! older than it (migration 64).

use std::collections::BTreeSet;

use bb_core::walk_index::ids_in;
use sea_query::{
    Alias, Cond, Expr, ExprTrait as _, Iden, OnConflict, Order, Query, SelectStatement,
    SubQueryStatement,
};

use crate::db::{Dialect, Row, Transaction, rows_per_insert};
use crate::error::StoreError;
use crate::schema::{
    ChangeRunNamesPerson, Event, EventNamesPerson, HistoryEntryRow, HistoryNamesPerson, Job,
    JobNamesPerson, ManyChangeRunRow, NotificationNamesPerson, NotificationRow, PersonRow,
    ProposalNamesPerson, ProposalRow,
};

/// A table whose changes or JSON can name a person, with its index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Indexed {
    /// The events, by the changes.
    Event,
    /// The history of issues, by the changes.
    History,
    /// The notifications, by the changes.
    Notification,
    /// The jobs, by the payload.
    Job,
    /// The runs of changes to many issues, by the action.
    ChangeRun,
    /// The proposals of agents, by the value before and the value proposed.
    Proposal,
}

/// The name of a column or a table, as the schema writes it: the one way the tables the walk and a
/// removal go through by key are named.
pub(crate) fn name(iden: &impl Iden) -> Alias {
    Alias::new(iden.to_string())
}

impl Indexed {
    /// Every table with an index.
    pub(crate) const ALL: [Self; 6] = [
        Self::Event,
        Self::History,
        Self::Notification,
        Self::Job,
        Self::ChangeRun,
        Self::Proposal,
    ];

    /// The table whose rows the index points to.
    pub(crate) fn table(self) -> Alias {
        match self {
            Self::Event => name(&Event::Table),
            Self::History => name(&HistoryEntryRow::Table),
            Self::Notification => name(&NotificationRow::Table),
            Self::Job => name(&Job::Table),
            Self::ChangeRun => name(&ManyChangeRunRow::Table),
            Self::Proposal => name(&ProposalRow::Table),
        }
    }

    /// The table of a name, when it has an index.
    pub(crate) fn of_table(table: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|indexed| indexed.table().to_string() == table)
    }

    /// The columns of the key of a row of the table, in the order of the index.
    pub(crate) fn key(self) -> Vec<Alias> {
        match self {
            Self::Event => vec![name(&Event::Id)],
            Self::History => vec![name(&HistoryEntryRow::Event), name(&HistoryEntryRow::Issue)],
            Self::Notification => vec![name(&NotificationRow::Id)],
            Self::Job => vec![name(&Job::Id)],
            Self::ChangeRun => vec![name(&ManyChangeRunRow::Id)],
            Self::Proposal => vec![name(&ProposalRow::Id)],
        }
    }

    /// The columns whose text can name a person.
    pub(crate) fn texts(self) -> Vec<Alias> {
        match self {
            Self::Event => vec![name(&Event::Changes)],
            Self::History => vec![name(&HistoryEntryRow::Changes)],
            Self::Notification => vec![name(&NotificationRow::Changes)],
            Self::Job => vec![name(&Job::Payload)],
            Self::ChangeRun => vec![name(&ManyChangeRunRow::Action)],
            Self::Proposal => vec![name(&ProposalRow::Before), name(&ProposalRow::After)],
        }
    }

    /// The table of the index.
    fn index(self) -> Alias {
        match self {
            Self::Event => name(&EventNamesPerson::Table),
            Self::History => name(&HistoryNamesPerson::Table),
            Self::Notification => name(&NotificationNamesPerson::Table),
            Self::Job => name(&JobNamesPerson::Table),
            Self::ChangeRun => name(&ChangeRunNamesPerson::Table),
            Self::Proposal => name(&ProposalNamesPerson::Table),
        }
    }

    /// The columns of the index that hold the key of the row, in the order of [`Self::key`].
    fn index_key(self) -> Vec<Alias> {
        match self {
            Self::History => vec![
                name(&HistoryNamesPerson::Event),
                name(&HistoryNamesPerson::Issue),
            ],
            Self::Event | Self::Notification | Self::Job | Self::ChangeRun | Self::Proposal => {
                vec![name(&EventNamesPerson::Row)]
            }
        }
    }

    /// The column of the index that holds the person.
    fn index_person(self) -> Alias {
        match self {
            Self::History => name(&HistoryNamesPerson::Person),
            Self::Event | Self::Notification | Self::Job | Self::ChangeRun | Self::Proposal => {
                name(&EventNamesPerson::Person)
            }
        }
    }
}

/// The names the parts of a key are read by: a key of a table [`Indexed`] has at most two columns
/// (`Indexed::key`).
#[derive(Clone, Copy, Iden)]
pub(crate) enum KeyPart {
    Key0,
    Key1,
}

impl KeyPart {
    const ALL: [Self; 2] = [Self::Key0, Self::Key1];

    /// Selects the columns of `key`, each by the name of its part.
    pub(crate) fn select(select: &mut SelectStatement, key: &[Alias]) {
        for (column, part) in key.iter().zip(Self::ALL) {
            select.expr_as(Expr::col(column.clone()), part);
        }
    }

    /// The key of `columns` columns that [`Self::select`] selected in `row`, each column as stored.
    ///
    /// # Errors
    /// Fails when a part is not stored bytes.
    pub(crate) fn read(row: &Row, columns: usize) -> Result<Vec<Vec<u8>>, StoreError> {
        Self::ALL
            .iter()
            .take(columns)
            .map(|part| row.get::<Vec<u8>>(*part))
            .collect()
    }
}

/// The rows whose key comes after `last` in the order of `key`: the one keyset pager of the tables
/// that are gone through by key a page at a time (the fill of the index, the scrub of a removal).
/// A key of several columns is compared column by column (`a > x OR (a = x AND b > y)`), which
/// both databases answer from the index of the key.
pub(crate) fn after_key(key: &[Alias], last: &[Vec<u8>]) -> Cond {
    let mut after = Cond::any();
    for (at, column) in key.iter().enumerate() {
        let mut this = Cond::all();
        for (before, value) in key.iter().zip(last).take(at) {
            this = this.add(Expr::col(before.clone()).eq(value.clone()));
        }
        if let Some(value) = last.get(at) {
            this = this.add(Expr::col(column.clone()).gt(value.clone()));
        }
        after = after.add(this);
    }
    after
}

/// The rows of `of` whose text may name `person` (`person` is the ID as stored): those whose key
/// begins with a key the index holds for them (for the history, every row of an event that names
/// them in one of its rows). The caller still decides by the text.
pub(crate) fn may_name(of: Indexed, person: Vec<u8>, dialect: Dialect) -> Cond {
    let (Some(key), Some(index_key)) = (
        of.key().into_iter().next(),
        of.index_key().into_iter().next(),
    ) else {
        return Cond::any();
    };
    let mut held = Query::select();
    held.column(index_key)
        .from(of.index())
        .and_where(Expr::col(of.index_person()).eq(person));
    Cond::all().add(member_of(key, held, dialect))
}

/// `column` is one of the values `values` selects. PostgreSQL is given them as an array, so it
/// looks them up in the index of `column` whatever it estimates of the rest of the condition;
/// SQLite always looks the values of a subquery up in an index it has.
pub(crate) fn member_of(column: Alias, values: SelectStatement, dialect: Dialect) -> Expr {
    match dialect {
        Dialect::Sqlite => Expr::col(column).in_subquery(values),
        Dialect::Postgres => Expr::cust_with_exprs(
            "$1 = ANY(ARRAY $2)",
            [
                Expr::col(column),
                Expr::SubQuery(None, Box::new(SubQueryStatement::SelectStatement(values))),
            ],
        ),
    }
}

/// A row that was written: its key (each column as stored) and the texts that can name a person.
pub(crate) struct Written<'t> {
    /// The key, in the order of [`Indexed::key`].
    pub(crate) key: Vec<Vec<u8>>,
    /// The texts of the row, as stored.
    pub(crate) texts: Vec<&'t str>,
}

/// Writes the index of `rows` of `of`, inside the change that wrote them: for each row, every
/// person whose ID its texts hold. A row that is in the index already stays as it is.
///
/// An ID that is no person's when the row is written is left out (most IDs in a change name
/// issues, projects or fields), so the rule of the order is: a person is written before any row
/// that names them. It holds because an ID is only known once its record is written: a person is
/// created before anything can name them, and a change that writes a person and a row naming them
/// writes the person first. A person is never deleted (a removal keeps the record as *Former
/// user*), so an ID that was a person's stays one.
///
/// # Errors
/// Fails when the database fails.
pub(crate) async fn index_rows(
    transaction: &mut Transaction,
    of: Indexed,
    rows: &[Written<'_>],
) -> Result<(), StoreError> {
    let named: Vec<(&Written<'_>, Vec<[u8; 16]>)> = rows
        .iter()
        .map(|row| {
            let mut ids: Vec<[u8; 16]> = row.texts.iter().flat_map(|text| ids_in(text)).collect();
            ids.sort_unstable();
            ids.dedup();
            (row, ids)
        })
        .filter(|(_, ids)| !ids.is_empty())
        .collect();
    if named.is_empty() {
        return Ok(());
    }
    let all: BTreeSet<[u8; 16]> = named
        .iter()
        .flat_map(|(_, ids)| ids.iter().copied())
        .collect();
    let people = people_among(transaction, &all).await?;
    let pairs: Vec<(&Written<'_>, [u8; 16])> = named
        .iter()
        .flat_map(|(row, ids)| {
            ids.iter()
                .filter(|id| people.contains(*id))
                .map(move |id| (*row, *id))
        })
        .collect();
    let mut columns = of.index_key();
    columns.push(of.index_person());
    for chunk in pairs.chunks(rows_per_insert(columns.len())) {
        let mut insert = Query::insert();
        insert.into_table(of.index()).columns(columns.clone());
        for (row, person) in chunk {
            let values = row
                .key
                .iter()
                .map(|part| Expr::val(part.clone()))
                .chain([Expr::val(person.to_vec())])
                .collect::<Vec<Expr>>();
            insert
                .values(values)
                .map_err(|_| StoreError::Corrupt("a row of the index of the walk"))?;
        }
        insert.on_conflict(OnConflict::columns(columns.clone()).do_nothing().to_owned());
        transaction.execute(&insert).await?;
    }
    Ok(())
}

/// Which of `ids` are the IDs of people.
async fn people_among(
    transaction: &mut Transaction,
    ids: &BTreeSet<[u8; 16]>,
) -> Result<BTreeSet<[u8; 16]>, StoreError> {
    let ids: Vec<[u8; 16]> = ids.iter().copied().collect();
    let mut people = BTreeSet::new();
    for chunk in ids.chunks(rows_per_insert(1)) {
        let mut select = Query::select();
        select
            .column(PersonRow::Id)
            .from(PersonRow::Table)
            .and_where(Expr::col(PersonRow::Id).is_in(chunk.iter().map(|id| id.to_vec())));
        for row in transaction.fetch_all(&select).await? {
            let id: Vec<u8> = row.get(PersonRow::Id)?;
            if let Ok(id) = <[u8; 16]>::try_from(id.as_slice()) {
                people.insert(id);
            }
        }
    }
    Ok(people)
}

/// The names [`fill`] reads the texts of a row by: a table has at most two (`Indexed::texts`).
#[derive(Clone, Copy, Iden)]
enum TextPart {
    Text0,
    Text1,
}

impl TextPart {
    const ALL: [Self; 2] = [Self::Text0, Self::Text1];
}

/// How many rows [`fill`] reads at a time.
const FILL_PAGE: u64 = 2_000;

/// Writes the index of every row that is there already (migration 64), a page of each table at a
/// time by key, so no page is held whole in memory.
///
/// # Errors
/// Fails when the database fails or a row is unreadable.
pub(crate) async fn fill(transaction: &mut Transaction) -> Result<(), StoreError> {
    for of in Indexed::ALL {
        let key = of.key();
        let texts = of.texts();
        let mut last: Option<Vec<Vec<u8>>> = None;
        loop {
            let mut select = Query::select();
            select.from(of.table()).limit(FILL_PAGE);
            KeyPart::select(&mut select, &key);
            for (column, part) in texts.iter().zip(TextPart::ALL) {
                select.expr_as(Expr::col(column.clone()), part);
            }
            for column in &key {
                select.order_by(column.clone(), Order::Asc);
            }
            if let Some(last) = &last {
                select.cond_where(after_key(&key, last));
            }
            let page = transaction.fetch_all(&select).await?;
            let mut stored = Vec::with_capacity(page.len());
            for row in &page {
                let key = KeyPart::read(row, key.len())?;
                let texts = TextPart::ALL
                    .iter()
                    .take(texts.len())
                    .map(|read| row.get::<String>(*read))
                    .collect::<Result<Vec<_>, _>>()?;
                stored.push((key, texts));
            }
            let written: Vec<Written<'_>> = stored
                .iter()
                .map(|(key, texts)| Written {
                    key: key.clone(),
                    texts: texts.iter().map(String::as_str).collect(),
                })
                .collect();
            index_rows(transaction, of, &written).await?;
            last = stored.last().map(|(key, _)| key.clone());
            if (page.len() as u64) < FILL_PAGE {
                break;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/walk_index.rs"]
mod tests;
