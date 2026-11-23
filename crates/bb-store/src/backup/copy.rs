//! The copy of the database (ADR-0046, decision 2), one routine per database, each reading one
//! consistent state while people go on saving:
//!
//! - SQLite: `VACUUM INTO` a new file; what the backup needs to know (the migrations, the head of
//!   the security log, the files) is then read from that copy, so it matches it exactly;
//! - PostgreSQL: one transaction at `REPEATABLE READ` reads all of it: the same facts and the rows
//!   of every table in the order the tables were made, each table in one statement whose rows are
//!   taken as the database sends them, written as JSON lines (`value.rs`): a head
//!   `{"table": …, "columns": […]}` for each table, then one array per row. The lines are gathered
//!   into pieces on the runtime and written by a blocking thread, which counts the size and the
//!   SHA-256 of the file as it goes, so the archive does not read it once more for them.

use std::io::Write as _;
use std::path::Path;
use std::sync::Arc;

use bb_core::migration::MigrationRecord;
use bb_core::security_log::Digest;
use futures_util::TryStreamExt as _;
use sea_query::{Alias, Query, SelectStatement};
use serde_json::{Value as Json, json};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::db::{Database, Transaction};
use crate::error::StoreError;
use crate::migrations::read_applied;
use crate::schema::StoredFileRow;
use crate::security_log::head_in;

use super::archive::Counting;
use super::catalog;
use super::value::row_to_json;
use super::work::WorkFolder;

/// What a backup needs to know about the copy it holds.
#[derive(Clone, Debug)]
pub(crate) struct Copied {
    /// The migrations applied to the copied database.
    pub(crate) migrations: Vec<MigrationRecord>,
    /// The newest place and hash of its security log.
    pub(crate) log_head: Option<(i64, Digest)>,
    /// The files it points to.
    pub(crate) files: Vec<Digest>,
    /// The size and SHA-256 of the copy, when its writer counted them.
    pub(crate) measured: Option<(u64, Digest)>,
}

/// How many bytes of lines go to the disk at a time.
const PIECE: usize = 1 << 20;

/// How many pieces may wait for the disk: the database is read while the disk writes, and a slow
/// disk holds the reading back instead of filling the memory.
const PIECES_ON_THE_WAY: usize = 8;

/// Every row of `table`, with its data columns, in whatever order the database keeps them.
fn whole_table(table: &str, columns: &[catalog::Column]) -> SelectStatement {
    let mut select = Query::select();
    select
        .columns(columns.iter().map(|column| Alias::new(&column.name)))
        .from(Alias::new(table));
    select
}

/// The line that starts the rows of `table` in the copy.
pub(super) fn table_head(table: &str, columns: &[catalog::Column]) -> Json {
    json!({
        "table": table,
        "columns": columns
            .iter()
            .map(|column| json!({ "name": column.name, "kind": column.kind.word() }))
            .collect::<Vec<_>>(),
    })
}

fn write_failed(error: std::io::Error) -> StoreError {
    StoreError::Backup(error)
}

/// The lines of the copy on their way to its file.
struct Lines {
    piece: Vec<u8>,
    to_disk: Option<mpsc::Sender<Vec<u8>>>,
    writer: Option<JoinHandle<Result<(u64, Digest), StoreError>>>,
}

impl Lines {
    /// Starts the blocking thread that writes the new file `file`; it holds the work folder, so
    /// the folder goes only after the thread let the file go.
    fn open(file: &Path, work: &Arc<WorkFolder>) -> Self {
        let (to_disk, pieces) = mpsc::channel(PIECES_ON_THE_WAY);
        let (file, work) = (file.to_owned(), Arc::clone(work));
        let writer = tokio::task::spawn_blocking(move || {
            let written = write_pieces(&file, pieces);
            drop(work);
            written
        });
        Self {
            piece: Vec::with_capacity(PIECE),
            to_disk: Some(to_disk),
            writer: Some(writer),
        }
    }

    /// Adds one line.
    async fn line(&mut self, value: &Json) -> Result<(), StoreError> {
        serde_json::to_writer(&mut self.piece, value)
            .map_err(|error| write_failed(error.into()))?;
        self.piece.push(b'\n');
        if self.piece.len() >= PIECE {
            self.send().await?;
        }
        Ok(())
    }

    async fn send(&mut self) -> Result<(), StoreError> {
        let piece = std::mem::replace(&mut self.piece, Vec::with_capacity(PIECE));
        let sent = match &self.to_disk {
            Some(to_disk) => to_disk.send(piece).await.is_ok(),
            None => false,
        };
        if sent {
            return Ok(());
        }
        // The thread stopped taking pieces: its error says why.
        self.to_disk = None;
        Err(self.written().await.err().unwrap_or_else(|| {
            write_failed(std::io::Error::other("the copy stopped being written"))
        }))
    }

    /// What the thread wrote, once it is done.
    async fn written(&mut self) -> Result<(u64, Digest), StoreError> {
        let writer = self
            .writer
            .take()
            .ok_or_else(|| write_failed(std::io::Error::other("the copy was written already")))?;
        writer
            .await
            .map_err(|error| write_failed(std::io::Error::other(error)))?
    }

    /// Writes the rest, lets the file go, and says its size and SHA-256.
    async fn finish(mut self) -> Result<(u64, Digest), StoreError> {
        if !self.piece.is_empty() {
            self.send().await?;
        }
        self.to_disk = None;
        self.written().await
    }
}

/// Writes the pieces into the new file `file` until no more come, waits until it is on disk, and
/// says its size and SHA-256. Blocking.
fn write_pieces(
    file: &Path,
    mut pieces: mpsc::Receiver<Vec<u8>>,
) -> Result<(u64, Digest), StoreError> {
    let mut out = Counting::new(std::fs::File::create_new(file).map_err(write_failed)?);
    while let Some(piece) = pieces.blocking_recv() {
        out.write_all(&piece).map_err(write_failed)?;
    }
    let (file, size, digest) = out.finish();
    file.sync_all().map_err(write_failed)?;
    Ok((size, digest))
}

impl Database {
    /// Copies the database into the new file `file` in `work` and says what the copy holds.
    pub(crate) async fn copy_into(
        &self,
        file: &Path,
        work: &Arc<WorkFolder>,
    ) -> Result<Copied, StoreError> {
        if self.vacuum_into(file).await? {
            let copy = Self::open_copy(file).await?;
            let copied = copy.facts().await;
            copy.close().await;
            return copied;
        }
        let mut transaction = self.begin_read().await?;
        transaction.without_time_limits().await?;
        let mut copied = facts_in(&mut transaction).await?;
        let mut out = Lines::open(file, work);
        for table in catalog::tables(&mut transaction).await? {
            let columns = catalog::columns(&mut transaction, &table).await?;
            out.line(&table_head(&table, &columns)).await?;
            let kinds: Vec<_> = columns.iter().map(|column| column.kind).collect();
            let mut rows = transaction.stream_rows(&whole_table(&table, &columns))?;
            while let Some(row) = rows.try_next().await? {
                out.line(&row_to_json(&row, &kinds)?).await?;
            }
        }
        copied.measured = Some(out.finish().await?);
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
        measured: None,
    })
}
