//! The part of the in-memory store for *Download my data* (PRIV-03): the records of the files, and
//! the walk of a person's data as a test sets it up (the real walk is the store's, tested on both
//! databases in `bb-store`).

use bb_core::data_export::{DataExport, ExportState};
use bb_core::id::{DataExportId, UserId};
use bb_core::personal::EntityMarks;
use jiff::Timestamp;

use super::fake::{FakeError, FakeStore, Tx};
use crate::ports::PersonData;
use crate::privacy::export::ExportStore;
use crate::privacy::walk::{FoundRow, KindOfRows, ROWS_PAGE, RowsPage};

/// One row of the walk a test set up: whose it is, of which kind, and the row.
#[derive(Clone, Debug)]
pub(crate) struct WalkedRow {
    pub(crate) person: UserId,
    pub(crate) kind: String,
    pub(crate) marks: EntityMarks,
    pub(crate) row: FoundRow,
}

fn newest_first(exports: &mut [DataExport]) {
    exports.sort_by(|a, b| b.asked_at.cmp(&a.asked_at).then(b.id.cmp(&a.id)));
}

fn count(rows: usize) -> u64 {
    u64::try_from(rows).unwrap_or(u64::MAX)
}

impl ExportStore for FakeStore {
    async fn data_export_in(
        tx: &mut Tx,
        id: DataExportId,
    ) -> Result<Option<DataExport>, FakeError> {
        Ok(tx
            .state
            .data_exports
            .iter()
            .find(|export| export.id == id)
            .cloned())
    }

    async fn waiting_export_of_in(
        tx: &mut Tx,
        person: UserId,
    ) -> Result<Option<DataExport>, FakeError> {
        Ok(tx
            .state
            .data_exports
            .iter()
            .find(|export| export.person == person && export.state == ExportState::Waiting)
            .cloned())
    }

    async fn latest_export_of(
        &self,
        person: UserId,
        asked_by: UserId,
    ) -> Result<Option<DataExport>, FakeError> {
        Ok(self.read(|state| {
            let mut found: Vec<DataExport> = state
                .data_exports
                .iter()
                .filter(|export| export.person == person && export.asked_by == asked_by)
                .cloned()
                .collect();
            newest_first(&mut found);
            found.into_iter().next()
        }))
    }

    async fn data_exports_page(&self, skip: u64, limit: u64) -> Result<Vec<DataExport>, FakeError> {
        Ok(self.read(|state| {
            let mut found = state.data_exports.clone();
            newest_first(&mut found);
            found
                .into_iter()
                .skip(usize::try_from(skip).unwrap_or(usize::MAX))
                .take(usize::try_from(limit).unwrap_or(usize::MAX))
                .collect()
        }))
    }

    async fn exports_asked_before(
        &self,
        before: Timestamp,
        limit: u64,
    ) -> Result<Vec<DataExport>, FakeError> {
        Ok(self.read(|state| {
            let mut found: Vec<DataExport> = state
                .data_exports
                .iter()
                .filter(|export| export.asked_at < before)
                .cloned()
                .collect();
            found.sort_by(|a, b| a.asked_at.cmp(&b.asked_at).then(a.id.cmp(&b.id)));
            found.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
            found
        }))
    }

    async fn lock_exports_of(_tx: &mut Tx, _person: UserId) -> Result<(), FakeError> {
        Ok(())
    }

    async fn insert_data_export(tx: &mut Tx, export: &DataExport) -> Result<(), FakeError> {
        tx.state.data_exports.push(export.clone());
        Ok(())
    }

    async fn finish_data_export(
        tx: &mut Tx,
        id: DataExportId,
        state: ExportState,
        ready_at: Option<Timestamp>,
        bytes: Option<i64>,
    ) -> Result<bool, FakeError> {
        let Some(found) = tx
            .state
            .data_exports
            .iter_mut()
            .find(|export| export.id == id && export.state == ExportState::Waiting)
        else {
            return Ok(false);
        };
        found.state = state;
        found.ready_at = ready_at;
        found.bytes = bytes;
        Ok(true)
    }

    async fn delete_data_export(tx: &mut Tx, id: DataExportId) -> Result<bool, FakeError> {
        let before = tx.state.data_exports.len();
        tx.state.data_exports.retain(|export| export.id != id);
        Ok(tx.state.data_exports.len() != before)
    }
}

impl PersonData for FakeStore {
    async fn kinds_of(&self, person: UserId) -> Result<Vec<KindOfRows>, FakeError> {
        // A real store waits for the database here: two tries of one export run side by side in a
        // test only if each lets the other go on at this point.
        tokio::task::yield_now().await;
        Ok(self.read(|state| {
            let mut kinds: Vec<KindOfRows> = Vec::new();
            for walked in state.walk.iter().filter(|walked| walked.person == person) {
                match kinds.iter_mut().find(|kind| kind.kind == walked.kind) {
                    Some(kind) => kind.rows += 1,
                    None => kinds.push(KindOfRows {
                        kind: walked.kind.clone(),
                        marks: walked.marks,
                        rows: 1,
                    }),
                }
            }
            kinds
        }))
    }

    async fn person_rows(
        &self,
        person: UserId,
        kind: &str,
        from: u64,
    ) -> Result<RowsPage, FakeError> {
        Ok(self.read(|state| {
            let all: Vec<FoundRow> = state
                .walk
                .iter()
                .filter(|walked| walked.person == person && walked.kind == kind)
                .map(|walked| walked.row.clone())
                .collect();
            let start = usize::try_from(from).unwrap_or(usize::MAX);
            let page = usize::try_from(ROWS_PAGE).unwrap_or(usize::MAX);
            let rows: Vec<FoundRow> = all.iter().skip(start).take(page).cloned().collect();
            let next = (count(all.len()) > from + ROWS_PAGE).then_some(from + ROWS_PAGE);
            RowsPage { rows, next }
        }))
    }
}

/// What the files of exports in memory hold.
#[derive(Default)]
struct Held {
    /// The finished files by their export.
    done: std::collections::HashMap<DataExportId, Vec<u8>>,
    /// The files tries are writing or left half-written, by export and try. Unlike the disk, a try
    /// that stops leaves its file here until it is deleted, as a server that stopped would.
    writing: std::collections::BTreeSet<(DataExportId, String)>,
    /// How often a finished file was kept.
    kept: usize,
    /// Whether writing into a file fails, as on a full disk.
    broken: bool,
}

/// The files of exports in memory, readable by the tests.
#[derive(Clone, Default)]
pub(crate) struct FakeExports(std::sync::Arc<std::sync::Mutex<Held>>);

impl FakeExports {
    fn held(&self) -> std::sync::MutexGuard<'_, Held> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The bytes of the finished file of `export`.
    pub(crate) fn bytes(&self, export: DataExportId) -> Option<Vec<u8>> {
        self.held().done.get(&export).cloned()
    }

    /// How many finished files there are.
    pub(crate) fn count(&self) -> usize {
        self.held().done.len()
    }

    /// How many files of `export` are half-written.
    pub(crate) fn half_written(&self, export: DataExportId) -> usize {
        self.held()
            .writing
            .iter()
            .filter(|(of, _)| *of == export)
            .count()
    }

    /// How often a finished file was kept, by every try.
    pub(crate) fn kept(&self) -> usize {
        self.held().kept
    }

    /// From now on, writing into a file fails.
    pub(crate) fn break_writing(&self) {
        self.held().broken = true;
    }
}

/// A file being written in memory by one try.
struct FakeSink {
    bytes: std::io::Cursor<Vec<u8>>,
    export: DataExportId,
    attempt: String,
    files: FakeExports,
}

impl std::io::Write for FakeSink {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.files.held().broken {
            return Err(std::io::Error::from(std::io::ErrorKind::StorageFull));
        }
        self.bytes.write(bytes)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl std::io::Seek for FakeSink {
    fn seek(&mut self, to: std::io::SeekFrom) -> std::io::Result<u64> {
        self.bytes.seek(to)
    }
}

impl crate::privacy::export::ExportSink for FakeSink {
    fn finish(&mut self) -> std::io::Result<u64> {
        Ok(u64::try_from(self.bytes.get_ref().len()).unwrap_or(u64::MAX))
    }

    fn keep(self: Box<Self>) -> std::io::Result<()> {
        let FakeSink {
            bytes,
            export,
            attempt,
            files,
        } = *self;
        let mut held = files.held();
        held.writing.remove(&(export, attempt));
        held.done.insert(export, bytes.into_inner());
        held.kept += 1;
        Ok(())
    }
}

impl crate::privacy::export::ExportFiles for FakeExports {
    fn create(
        &self,
        export: DataExportId,
        attempt: &str,
    ) -> std::io::Result<Box<dyn crate::privacy::export::ExportSink>> {
        if !self.held().writing.insert((export, attempt.to_owned())) {
            return Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists));
        }
        Ok(Box::new(FakeSink {
            bytes: std::io::Cursor::new(Vec::new()),
            export,
            attempt: attempt.to_owned(),
            files: self.clone(),
        }))
    }

    fn open(
        &self,
        export: DataExportId,
    ) -> crate::ports::BoxFuture<'_, std::io::Result<Option<crate::files::FileBody>>> {
        let found = self.bytes(export);
        Box::pin(async move {
            Ok(found.map(|bytes| crate::files::FileBody {
                size: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
                reader: Box::new(super::fake_files::InMemory(Some(bytes))),
            }))
        })
    }

    fn delete(&self, export: DataExportId) -> crate::ports::BoxFuture<'_, std::io::Result<()>> {
        let mut held = self.held();
        held.done.remove(&export);
        held.writing.retain(|(of, _)| *of != export);
        Box::pin(async { Ok(()) })
    }
}
