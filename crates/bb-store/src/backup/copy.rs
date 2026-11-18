//! The copy of the database (ADR-0046, decision 2), one routine per database, each reading one
//! consistent state while people go on saving:
//!
//! - SQLite: `VACUUM INTO` a new file; what the backup needs to know (the migrations, the head of
//!   the security log, the files) is then read from that copy, so it matches it exactly;
//! - PostgreSQL: one transaction at `REPEATABLE READ` reads all of it: the same facts and the rows
//!   of every table in the order the tables were made, written as JSON lines (`value.rs`): a head
//!   `{"table": …, "columns": […]}` for each table, then one array per row.

use std::io::{BufWriter, Write as _};
use std::path::Path;

use bb_core::migration::MigrationRecord;
use bb_core::security_log::Digest;
use sea_query::{Alias, Query};
use serde_json::json;

use crate::db::{Database, Transaction};
use crate::error::StoreError;
use crate::migrations::read_applied;
use crate::schema::StoredFileRow;
use crate::security_log::head_in;

use super::catalog;
use super::value::row_to_json;

/// What a backup needs to know about the copy it holds.
#[derive(Clone, Debug)]
pub(crate) struct Copied {
    /// The migrations applied to the copied database.
    pub(crate) migrations: Vec<MigrationRecord>,
    /// The newest place and hash of its security log.
    pub(crate) log_head: Option<(i64, Digest)>,
    /// The files it points to.
    pub(crate) files: Vec<Digest>,
}

fn write_failed(error: std::io::Error) -> StoreError {
    StoreError::Backup(error)
}

impl Database {
    /// Copies the database into the new file `file` and says what the copy holds.
    pub(crate) async fn copy_into(&self, file: &Path) -> Result<Copied, StoreError> {
        if self.vacuum_into(file).await? {
            let copy = Self::open_copy(file).await?;
            let copied = copy.facts().await;
            copy.close().await;
            return copied;
        }
        let mut transaction = self.begin_read().await?;
        let copied = facts_in(&mut transaction).await?;
        let mut out = BufWriter::new(std::fs::File::create_new(file).map_err(write_failed)?);
        for table in catalog::tables(&mut transaction).await? {
            let columns = catalog::columns(&mut transaction, &table).await?;
            let head = json!({
                "table": table,
                "columns": columns
                    .iter()
                    .map(|column| json!({ "name": column.name, "kind": column.kind.word() }))
                    .collect::<Vec<_>>(),
            });
            writeln!(out, "{head}").map_err(write_failed)?;
            let mut select = Query::select();
            select
                .columns(columns.iter().map(|column| Alias::new(&column.name)))
                .from(Alias::new(&table));
            let kinds: Vec<_> = columns.iter().map(|column| column.kind).collect();
            for row in transaction.fetch_all(&select).await? {
                writeln!(out, "{}", row_to_json(&row, &kinds)?).map_err(write_failed)?;
            }
        }
        out.flush().map_err(write_failed)?;
        out.get_ref().sync_all().map_err(write_failed)?;
        transaction.commit().await?;
        Ok(copied)
    }

    /// What a backup needs to know, read from this database in one snapshot.
    async fn facts(&self) -> Result<Copied, StoreError> {
        let mut transaction = self.begin_read().await?;
        let copied = facts_in(&mut transaction).await?;
        transaction.commit().await?;
        Ok(copied)
    }
}

async fn facts_in(transaction: &mut Transaction) -> Result<Copied, StoreError> {
    let migrations = read_applied(transaction).await?;
    let log_head = head_in(transaction).await?;
    let mut select = Query::select();
    select
        .column(StoredFileRow::Hash)
        .from(StoredFileRow::Table);
    let files = transaction
        .fetch_all(&select)
        .await?
        .iter()
        .map(|row| {
            let bytes: Vec<u8> = row.get(StoredFileRow::Hash)?;
            Digest::from_slice(&bytes).ok_or(StoreError::Corrupt("the hash of a file"))
        })
        .collect::<Result<_, _>>()?;
    Ok(Copied {
        migrations,
        log_head,
        files,
    })
}
