//! Putting the copy of a PostgreSQL database back (ADR-0046, decision 4). The copy is read line by
//! line, never whole: once to convert every row before anything changes, then again inside one
//! transaction that empties the database, makes its tables again with the backup's own
//! migrations and fills them. PostgreSQL changes its tables inside a transaction too, so a copy
//! that fails half way leaves the database as it was.

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead as _, BufReader, Lines};
use std::path::Path;

use bb_core::backup::manifest::Manifest;
use bb_core::secret::StoredSecret;
use bb_core::technical::TechnicalName;
use jiff::Timestamp;
use sea_query::{Alias, Expr, Query, Table};
use serde_json::Value as Json;

use super::value::{ColumnKind, bytes_of, json_to_values, text_of};
use super::{catalog, io};
use crate::db::{Database, Transaction, from_micros, rows_per_insert};
use crate::error::StoreError;
use crate::migrations::migrate_only_in;
use crate::schema::{Migration, Secret};
use crate::secrets::all_columns;

/// The head of a table in the copy.
pub(super) struct TableHead {
    pub(super) name: String,
    pub(super) columns: Vec<(String, ColumnKind)>,
}

impl TableHead {
    fn kinds(&self) -> Vec<ColumnKind> {
        self.columns.iter().map(|(_, kind)| *kind).collect()
    }

    fn at(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|(column, _)| column == name)
    }
}

fn read_head(line: &Json) -> Option<TableHead> {
    let name = line.get("table")?.as_str()?.to_owned();
    let columns = line
        .get("columns")?
        .as_array()?
        .iter()
        .map(|column| {
            Some((
                column.get("name")?.as_str()?.to_owned(),
                ColumnKind::from_word(column.get("kind")?.as_str()?)?,
            ))
        })
        .collect::<Option<_>>()?;
    Some(TableHead { name, columns })
}

fn unreadable() -> StoreError {
    StoreError::Corrupt("the copy of a database")
}

/// A line of the copy: the head of the next table, or a row of the table before it.
enum Entry {
    Head(TableHead),
    Row(Json),
}

/// The lines of a copy, read one at a time.
struct Entries(Lines<BufReader<File>>);

impl Entries {
    fn open(copy: &Path) -> Result<Self, StoreError> {
        Ok(Self(BufReader::new(File::open(copy).map_err(io)?).lines()))
    }

    fn next_entry(&mut self) -> Result<Option<Entry>, StoreError> {
        let Some(line) = self.0.next() else {
            return Ok(None);
        };
        let line: Json = serde_json::from_str(&line.map_err(io)?)?;
        Ok(Some(if line.is_object() {
            Entry::Head(read_head(&line).ok_or_else(unreadable)?)
        } else {
            Entry::Row(line)
        }))
    }
}

/// Reads every table of the copy: `visit` sees each head with no row, then each of its rows.
fn each_row(
    copy: &Path,
    mut visit: impl FnMut(&TableHead, Option<Json>) -> Result<(), StoreError>,
) -> Result<(), StoreError> {
    let mut entries = Entries::open(copy)?;
    let mut current: Option<TableHead> = None;
    while let Some(entry) = entries.next_entry()? {
        match entry {
            Entry::Head(next) => {
                visit(&next, None)?;
                current = Some(next);
            }
            Entry::Row(values) => visit(current.as_ref().ok_or_else(unreadable)?, Some(values))?,
        }
    }
    Ok(())
}

/// Converts every row of the copy into the values it is written with, keeping none: a copy with
/// a row that cannot be read is refused before anything changes.
pub(super) fn check_rows(copy: &Path) -> Result<(), StoreError> {
    let mut kinds = Vec::new();
    each_row(copy, |head, row| match row {
        None => {
            kinds = head.kinds();
            Ok(())
        }
        Some(row) => json_to_values(&row, &kinds).map(|_| ()),
    })
}

/// The stored secrets in the copy.
pub(super) fn secrets_in_rows(copy: &Path) -> Result<Vec<StoredSecret>, StoreError> {
    let wanted = sea_query::Iden::to_string(&Secret::Table);
    let mut secrets = Vec::new();
    each_row(copy, |head, row| {
        if let (Some(row), true) = (row, head.name == wanted) {
            secrets.push(secret_of(head, &row)?);
        }
        Ok(())
    })?;
    Ok(secrets)
}

fn secret_of(head: &TableHead, row: &Json) -> Result<StoredSecret, StoreError> {
    let [
        name,
        key_id,
        wrapped_key,
        wrapped_nonce,
        nonce,
        ciphertext,
        updated_at,
    ] = all_columns().map(|column| {
        head.at(&sea_query::Iden::to_string(&column))
            .ok_or_else(unreadable)
    });
    let cell = |index: Result<usize, StoreError>| row.get(index?).ok_or_else(unreadable);
    let word = |index| {
        text_of(cell(index)?)?
            .and_then(|text| TechnicalName::new(&text).ok())
            .ok_or_else(unreadable)
    };
    let bytes = |index| bytes_of(cell(index)?)?.ok_or_else(unreadable);
    Ok(StoredSecret {
        name: word(name)?,
        key_id: word(key_id)?,
        wrapped_key: bytes(wrapped_key)?,
        wrapped_nonce: bytes(wrapped_nonce)?,
        nonce: bytes(nonce)?,
        ciphertext: bytes(ciphertext)?,
        updated_at: from_micros(cell(updated_at)?.as_i64().ok_or_else(unreadable)?)?,
    })
}

/// Empties the database, makes its tables again with the backup's migrations and fills them from
/// the copy, all in one transaction (see the module).
pub(super) async fn fill(
    database: &Database,
    manifest: &Manifest,
    copy: &Path,
    now: Timestamp,
) -> Result<(), StoreError> {
    check_rows(copy)?;
    let mut transaction = database.begin().await?;
    for table in catalog::tables(&mut transaction).await?.iter().rev() {
        let mut drop = Table::drop();
        drop.table(Alias::new(table)).cascade();
        transaction.execute_schema(&drop).await?;
    }
    let mut drop = Table::drop();
    drop.table(Migration::Table).if_exists();
    transaction.execute_schema(&drop).await?;
    let versions: Vec<i64> = manifest
        .migrations
        .iter()
        .map(|applied| applied.version)
        .collect();
    migrate_only_in(&mut transaction, &versions, now).await?;
    // What the migrations seeded is in the copy too.
    for table in catalog::tables(&mut transaction).await?.iter().rev() {
        let mut delete = Query::delete();
        delete.from_table(Alias::new(table));
        transaction.execute(&delete).await?;
    }
    let mut entries = Entries::open(copy)?;
    let mut table: Option<TableFill> = None;
    while let Some(entry) = entries.next_entry()? {
        match entry {
            Entry::Head(head) => {
                if let Some(done) = table.take() {
                    done.finish(&mut transaction).await?;
                }
                table = Some(TableFill::new(&mut transaction, head).await?);
            }
            Entry::Row(row) => {
                table
                    .as_mut()
                    .ok_or_else(unreadable)?
                    .push(&mut transaction, row)
                    .await?;
            }
        }
    }
    if let Some(done) = table {
        done.finish(&mut transaction).await?;
    }
    transaction.commit().await
}

/// The rows of one table on their way in. A table whose rows point into the table itself (the
/// parent of a project) keeps its rows until its head is done, so the rows they point to go first;
/// any other table writes them as they come, a batch at a time.
struct TableFill {
    head: TableHead,
    kinds: Vec<ColumnKind>,
    /// The pairs (column, column it points to) that point into the same table, by their places.
    pairs: Vec<(usize, usize)>,
    rows: Vec<Json>,
}

impl TableFill {
    async fn new(transaction: &mut Transaction, head: TableHead) -> Result<Self, StoreError> {
        let pairs = catalog::self_references(transaction, &head.name)
            .await?
            .iter()
            .filter_map(|(child, parent)| Some((head.at(child)?, head.at(parent)?)))
            .collect();
        Ok(Self {
            kinds: head.kinds(),
            head,
            pairs,
            rows: Vec::new(),
        })
    }

    fn batch(&self) -> usize {
        rows_per_insert(self.kinds.len().max(1))
    }

    async fn push(&mut self, transaction: &mut Transaction, row: Json) -> Result<(), StoreError> {
        self.rows.push(row);
        if self.pairs.is_empty() && self.rows.len() >= self.batch() {
            let rows = std::mem::take(&mut self.rows);
            self.write(transaction, &rows).await?;
        }
        Ok(())
    }

    async fn finish(mut self, transaction: &mut Transaction) -> Result<(), StoreError> {
        let rows = std::mem::take(&mut self.rows);
        let rows = if self.pairs.is_empty() {
            rows
        } else {
            parents_first(rows, &self.pairs)
        };
        for chunk in rows.chunks(self.batch()) {
            self.write(transaction, chunk).await?;
        }
        for column in catalog::columns(transaction, &self.head.name).await? {
            if column.counts_up {
                catalog::continue_count(transaction, &self.head.name, &column.name).await?;
            }
        }
        Ok(())
    }

    async fn write(&self, transaction: &mut Transaction, rows: &[Json]) -> Result<(), StoreError> {
        if rows.is_empty() {
            return Ok(());
        }
        let mut insert = Query::insert();
        insert
            .into_table(Alias::new(&self.head.name))
            .columns(self.head.columns.iter().map(|(name, _)| Alias::new(name)));
        for row in rows {
            insert
                .values(json_to_values(row, &self.kinds)?.into_iter().map(Expr::val))
                .map_err(|_| unreadable())?;
        }
        transaction.execute(&insert).await.map(|_| ())
    }
}

/// The rows in an order where every row comes after the rows of the table it points to. A row
/// that points to a value no row has goes in as it is; rows that point at each other in a ring
/// (which the database would refuse anyway) go last.
fn parents_first(rows: Vec<Json>, pairs: &[(usize, usize)]) -> Vec<Json> {
    // For each pair, which row holds each value of the column pointed to.
    let holders: Vec<HashMap<String, usize>> = pairs
        .iter()
        .map(|(_, parent)| {
            rows.iter()
                .enumerate()
                .filter_map(|(index, row)| {
                    let value = row.get(*parent)?;
                    (!value.is_null()).then(|| (value.to_string(), index))
                })
                .collect()
        })
        .collect();
    let mut waiting = vec![0_usize; rows.len()];
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); rows.len()];
    for (index, row) in rows.iter().enumerate() {
        let parents: HashSet<usize> = pairs
            .iter()
            .zip(&holders)
            .filter_map(|((child, _), holders)| {
                let value = row.get(*child)?;
                holders.get(&value.to_string()).copied()
            })
            .filter(|parent| *parent != index)
            .collect();
        waiting[index] = parents.len();
        for parent in parents {
            children[parent].push(index);
        }
    }
    let mut order: Vec<usize> = (0..rows.len())
        .filter(|index| waiting[*index] == 0)
        .collect();
    let mut next = 0;
    while let Some(&done) = order.get(next) {
        next += 1;
        for &child in &children[done] {
            waiting[child] -= 1;
            if waiting[child] == 0 {
                order.push(child);
            }
        }
    }
    let placed: HashSet<usize> = order.iter().copied().collect();
    order.extend((0..rows.len()).filter(|index| !placed.contains(index)));
    let mut slots: Vec<Option<Json>> = rows.into_iter().map(Some).collect();
    order
        .into_iter()
        .filter_map(|index| slots.get_mut(index).and_then(Option::take))
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::parents_first;

    #[test]
    fn a_row_comes_after_the_row_it_points_to() {
        // (id, parent): 3 points to 2, 2 to 1, 1 to nothing, 4 to a row that is not there.
        let rows = vec![
            json!([3, 2]),
            json!([2, 1]),
            json!([4, 9]),
            json!([1, null]),
        ];
        let ordered = parents_first(rows, &[(1, 0)]);
        let ids: Vec<i64> = ordered.iter().filter_map(|row| row[0].as_i64()).collect();
        assert_eq!(ids, [4, 1, 2, 3]);
    }

    #[test]
    fn a_long_chain_is_put_in_order_at_once() {
        // Every row points to the next one, the last to nothing: the deepest order there is.
        let count = 5_000_i64;
        let rows: Vec<_> = (0..count)
            .map(|id| {
                json!([
                    id,
                    if id + 1 < count {
                        json!(id + 1)
                    } else {
                        json!(null)
                    }
                ])
            })
            .collect();
        let ordered = parents_first(rows, &[(1, 0)]);
        let ids: Vec<i64> = ordered.iter().filter_map(|row| row[0].as_i64()).collect();
        assert_eq!(ids, (0..count).rev().collect::<Vec<_>>());
    }
}
