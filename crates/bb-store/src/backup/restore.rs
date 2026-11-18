//! The restore of a backup, with the server stopped (ADR-0046, decision 4). In this order:
//!
//! 1. the backup is read and checked whole: every name, size and checksum, its kind of database,
//!    its migrations, and that the master key opens its stored secrets; a refused backup changes
//!    nothing;
//! 2. a backup of the present is made, which the rotation keeps for the keep period of the
//!    security log;
//! 3. the database is put back: a SQLite file replaces the file; on PostgreSQL the tables are
//!    made again by the backup's own migrations and filled table by table;
//! 4. the files come back, the migrations the backup lacks run, every session ends, and the lines
//!    of the replay list newer than the backup run again;
//! 5. the anchor of the security log starts again at the restored log's head with the range that
//!    is gone, the restore is recorded with its line in the log, and the log is verified.
//!
//! A failure after step 2 leaves the backup of the present in place and names it, so it can be
//! restored the same way.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use bb_app::auth::{OAuthStore as _, SecondStepStore as _};
use bb_app::backups::record_backup_run;
use bb_core::backup::DatabaseKind;
use bb_core::backup::manifest::{BackupProblem, KnownMigration, Manifest};
use bb_core::backup::member::{self, MemberName};
use bb_core::backup::name::{BackupName, Purpose};
use bb_core::backup::replay::{self, ReplayKind, ReplayLine};
use bb_core::backup::run::{BackupEvent, BackupMaker, BackupRun, CreatedFile};
use bb_core::event::{Actor, Channel, DomainEvent, EntityKind, EntityRef, FieldChange};
use bb_core::id::{AccessKeyId, BackupId, EntityId, EventId, GrantId, IdSource, PasskeyId, UserId};
use bb_core::oauth::GrantEnd;
use bb_core::secret::StoredSecret;
use bb_core::technical::TechnicalName;
use jiff::Timestamp;
use sea_query::{Alias, Expr, ExprTrait as _, Query, Table};
use serde_json::Value as Json;

use super::value::{ColumnKind, bytes_of, json_to_values, text_of};
use super::{BackupPlaces, CreatedBackup, archive, catalog, io, work_folder};
use crate::anchor::{Anchored, LogSeal};
use crate::auth::AuthStorage;
use crate::chain::Verification;
use crate::db::{Database, StoredId as _, Transaction, from_micros, rows_per_insert};
use crate::error::StoreError;
use crate::keys::KeyRing;
use crate::migrations::known_migrations;
use crate::schema::{AccessKeyRow, Migration, PasskeyRow, Secret, SignInSession};
use crate::secrets::{all_columns, keys_open};
use crate::security_log::head_in;

/// The database a backup is restored into.
#[derive(Clone, Debug)]
pub enum RestoreTarget {
    /// The SQLite file of the installation, which the restore replaces.
    File(PathBuf),
    /// A PostgreSQL database, by its open pool.
    Postgres(Database),
}

impl RestoreTarget {
    /// The target that the opened `database` of the installation is: a SQLite database by its
    /// file (closed here, because the restore replaces the file), PostgreSQL by its pool.
    pub async fn of(database: Database) -> Self {
        match database.file() {
            Some(file) => {
                database.close().await;
                Self::File(file)
            }
            None => Self::Postgres(database),
        }
    }

    const fn kind(&self) -> DatabaseKind {
        match self {
            Self::File(_) => DatabaseKind::Sqlite,
            Self::Postgres(_) => DatabaseKind::Postgres,
        }
    }

    async fn connect(&self) -> Result<Database, StoreError> {
        match self {
            Self::File(file) => Database::connect_file(file).await,
            Self::Postgres(database) => Ok(database.clone()),
        }
    }

    /// Lets a database [`Self::connect`] gave go: a SQLite file is closed, so it can be replaced;
    /// the pool of PostgreSQL stays open for the next step and the caller.
    async fn release(&self, database: &Database) {
        if let Self::File(_) = self {
            database.close().await;
        }
    }
}

/// What a restore needs.
pub struct RestoreInput<'a> {
    /// The backup file.
    pub archive: &'a Path,
    /// The database it goes into.
    pub target: &'a RestoreTarget,
    /// The backups folder, the files folder and the replay list of the installation.
    pub places: &'a BackupPlaces,
    /// The keys of the installation.
    pub keys: &'a Arc<KeyRing>,
    /// The seal of the security log of the installation.
    pub seal: &'a LogSeal,
    /// Copies of the replay list given besides the data folder's and the backup's (the one
    /// `bugaboo keys export` wrote).
    pub more_replay_lists: &'a [PathBuf],
    /// The moment of the restore.
    pub now: Timestamp,
}

/// What a restore did.
#[derive(Debug)]
pub struct RestoreReport {
    /// The moment of the restored backup: every change after it is gone, but what the replay did.
    pub moment: Timestamp,
    /// The backup of what was there before the restore.
    pub before: CreatedBackup,
    /// The migrations that ran because the backup was older.
    pub migrations_run: Vec<i64>,
    /// How many sessions ended.
    pub sessions_ended: u64,
    /// The lines of the replay list that ran again.
    pub replayed: Vec<ReplayLine>,
    /// The lines newer than the backup that this program cannot run yet.
    pub not_replayed: Vec<ReplayLine>,
    /// The lines of the copies of the list that could not be read.
    pub unreadable_lines: usize,
    /// The entries of the security log that are gone, first and last.
    pub lost: Option<(i64, i64)>,
    /// What checking the restored security log found.
    pub log: Verification,
}

/// Why a restore failed, and the backup of the present it made, if it got that far: restoring
/// that backup puts back what was there.
#[derive(Debug)]
pub struct RestoreFailure {
    /// What went wrong.
    pub error: StoreError,
    /// The backup of the present, when it was made before the failure.
    pub before: Option<PathBuf>,
}

fn early(error: StoreError) -> RestoreFailure {
    RestoreFailure {
        error,
        before: None,
    }
}

/// Restores a backup (see the module). The server must be stopped; the caller checks that.
///
/// # Errors
/// A [`RestoreFailure`]: a refused backup or wrong keys change nothing; a later failure names the
/// backup of the present.
pub async fn restore(
    input: RestoreInput<'_>,
    ids: &mut impl IdSource,
) -> Result<RestoreReport, RestoreFailure> {
    let work = work_folder(&input.places.folder, "restore").map_err(early)?;
    let result = run(&input, ids, &work).await;
    let _ = std::fs::remove_dir_all(&work);
    result
}

async fn run(
    input: &RestoreInput<'_>,
    ids: &mut impl IdSource,
    work: &Path,
) -> Result<RestoreReport, RestoreFailure> {
    let manifest = check(input, work).await.map_err(early)?;
    let present = input.target.connect().await.map_err(early)?;
    let created = present
        .create_backup(input.places, Purpose::BeforeRestore, input.now)
        .await;
    let present_head = match &created {
        Ok(_) => newest_known(&present, input.seal).await,
        Err(_) => Ok(0),
    };
    input.target.release(&present).await;
    let before = created.map_err(early)?;
    let failed = |error: StoreError| RestoreFailure {
        error,
        before: Some(before.path.clone()),
    };
    let present_head = present_head.map_err(failed)?;
    put_back(input, &manifest, work).await.map_err(failed)?;
    let restored = input.target.connect().await.map_err(failed)?;
    let done = finish(
        input,
        ids,
        &restored,
        &manifest,
        work,
        &before,
        present_head,
    )
    .await;
    input.target.release(&restored).await;
    done.map_err(failed)
}

/// Reads and checks the backup into `work`; nothing else is touched.
async fn check(input: &RestoreInput<'_>, work: &Path) -> Result<Manifest, StoreError> {
    let manifest = archive::read(input.archive, work)?;
    manifest
        .check_database(input.target.kind())
        .map_err(StoreError::BackupRefused)?;
    let known = known_migrations()?;
    let known: Vec<KnownMigration<'_>> = known
        .iter()
        .map(|(version, name, checksum)| KnownMigration {
            version: *version,
            name,
            checksum: *checksum,
        })
        .collect();
    manifest
        .check_migrations(&known)
        .map_err(StoreError::BackupRefused)?;
    let secrets = match manifest.database {
        DatabaseKind::Sqlite => {
            let copy = Database::open_copy(&database_member(work, &manifest)?).await?;
            let secrets = copy.stored_secrets().await;
            copy.close().await;
            secrets?
        }
        DatabaseKind::Postgres => secrets_in_rows(&database_member(work, &manifest)?)?,
    };
    if keys_open(input.keys, &secrets) {
        Ok(manifest)
    } else {
        Err(StoreError::KeysDoNotOpen)
    }
}

fn database_member(work: &Path, manifest: &Manifest) -> Result<PathBuf, StoreError> {
    let name = MemberName::parse(manifest.database.member()).map_err(|problem| {
        StoreError::BackupRefused(BackupProblem::BadName(String::new(), problem))
    })?;
    Ok(archive::extracted(work, &name))
}

/// The newest place of the security log that the present installation knows: its database's or
/// its anchor's, whichever is further.
async fn newest_known(database: &Database, seal: &LogSeal) -> Result<i64, StoreError> {
    let mut transaction = database.begin_read().await?;
    let head = head_in(&mut transaction).await?;
    transaction.commit().await?;
    let anchored = match seal.latest()? {
        Anchored::At(seq, _) => seq,
        Anchored::Nothing | Anchored::Forged => 0,
    };
    Ok(Ord::max(head.map_or(0, |(seq, _)| seq), anchored))
}

/// Puts the database and the files of the backup in place of the present ones.
async fn put_back(
    input: &RestoreInput<'_>,
    manifest: &Manifest,
    work: &Path,
) -> Result<(), StoreError> {
    let copy = database_member(work, manifest)?;
    match input.target {
        RestoreTarget::File(file) => replace_file(&copy, file)?,
        RestoreTarget::Postgres(database) => fill(database, manifest, &copy, input.now).await?,
    }
    put_back_files(&work.join(member::FILES), &input.places.files)
}

/// Replaces the SQLite file with the copy. The write-ahead files of the old database go first,
/// so SQLite cannot lay them over the copy.
fn replace_file(copy: &Path, file: &Path) -> Result<(), StoreError> {
    for suffix in ["-wal", "-shm", "-journal"] {
        let mut companion = file.as_os_str().to_owned();
        companion.push(suffix);
        match std::fs::remove_file(PathBuf::from(companion)) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => return Err(io(error)),
            _ => {}
        }
    }
    let mut staged = file.as_os_str().to_owned();
    staged.push(".restoring");
    let staged = PathBuf::from(staged);
    std::fs::copy(copy, &staged).map_err(io)?;
    std::fs::rename(&staged, file).map_err(io)
}

/// Moves the files of the backup into the files folder; a file that is there already is the same
/// file (it is named by its content) and stays.
fn put_back_files(from: &Path, files: &Path) -> Result<(), StoreError> {
    let Ok(entries) = std::fs::read_dir(from) else {
        return Ok(());
    };
    for entry in entries {
        let entry = entry.map_err(io)?;
        let target = files.join(entry.file_name());
        if entry.file_type().map_err(io)?.is_dir() {
            put_back_files(&entry.path(), &target)?;
        } else if !target.exists() {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(io)?;
            }
            if std::fs::rename(entry.path(), &target).is_err() {
                std::fs::copy(entry.path(), &target).map_err(io)?;
            }
        }
    }
    Ok(())
}

/// The head of a table in the copy of a PostgreSQL database.
struct TableHead {
    name: String,
    columns: Vec<(String, ColumnKind)>,
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

/// Every table of the copy with its rows, in the order of the copy.
fn tables_of(copy: &Path) -> Result<Vec<(TableHead, Vec<Json>)>, StoreError> {
    let text = std::fs::read_to_string(copy).map_err(io)?;
    let mut tables: Vec<(TableHead, Vec<Json>)> = Vec::new();
    for line in text.lines() {
        let line: Json = serde_json::from_str(line)?;
        if line.is_object() {
            tables.push((read_head(&line).ok_or_else(unreadable)?, Vec::new()));
        } else {
            tables.last_mut().ok_or_else(unreadable)?.1.push(line);
        }
    }
    Ok(tables)
}

/// The stored secrets in the copy of a PostgreSQL database.
fn secrets_in_rows(copy: &Path) -> Result<Vec<StoredSecret>, StoreError> {
    let name = sea_query::Iden::to_string(&Secret::Table);
    let Some((head, rows)) = tables_of(copy)?
        .into_iter()
        .find(|(head, _)| head.name == name)
    else {
        return Ok(Vec::new());
    };
    let at = |column: Secret| {
        let wanted = sea_query::Iden::to_string(&column);
        head.columns
            .iter()
            .position(|(name, _)| *name == wanted)
            .ok_or_else(unreadable)
    };
    let [
        name,
        key_id,
        wrapped_key,
        wrapped_nonce,
        nonce,
        ciphertext,
        updated_at,
    ] = all_columns().map(at);
    let (name, key_id, wrapped_key, wrapped_nonce, nonce, ciphertext, updated_at) = (
        name?,
        key_id?,
        wrapped_key?,
        wrapped_nonce?,
        nonce?,
        ciphertext?,
        updated_at?,
    );
    rows.iter()
        .map(|row| {
            let cell = |index: usize| row.get(index).ok_or_else(unreadable);
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
        })
        .collect()
}

/// Empties the PostgreSQL database, makes its tables again with the backup's migrations and
/// fills them from the copy.
async fn fill(
    database: &Database,
    manifest: &Manifest,
    copy: &Path,
    now: Timestamp,
) -> Result<(), StoreError> {
    let tables = tables_of(copy)?;
    let mut transaction = database.begin().await?;
    for table in catalog::tables(&mut transaction).await?.iter().rev() {
        let mut drop = Table::drop();
        drop.table(Alias::new(table)).cascade();
        transaction.execute_schema(&drop).await?;
    }
    let mut drop = Table::drop();
    drop.table(Migration::Table).if_exists();
    transaction.execute_schema(&drop).await?;
    transaction.commit().await?;
    let versions: Vec<i64> = manifest
        .migrations
        .iter()
        .map(|applied| applied.version)
        .collect();
    database.migrate_only(&versions, now).await?;
    let mut transaction = database.begin().await?;
    // What the migrations seeded is in the copy too.
    for table in catalog::tables(&mut transaction).await?.iter().rev() {
        let mut delete = Query::delete();
        delete.from_table(Alias::new(table));
        transaction.execute(&delete).await?;
    }
    for (head, rows) in tables {
        insert_rows(&mut transaction, &head, rows).await?;
    }
    transaction.commit().await
}

/// Writes the rows of one table; the rows that others of the same table point to go first.
async fn insert_rows(
    transaction: &mut Transaction,
    head: &TableHead,
    rows: Vec<Json>,
) -> Result<(), StoreError> {
    let references = catalog::self_references(transaction, &head.name).await?;
    let index = |name: &str| head.columns.iter().position(|(column, _)| column == name);
    let pairs: Vec<(usize, usize)> = references
        .iter()
        .filter_map(|(child, parent)| Some((index(child)?, index(parent)?)))
        .collect();
    let kinds: Vec<ColumnKind> = head.columns.iter().map(|(_, kind)| *kind).collect();
    let mut pending = rows;
    let mut written: Vec<String> = Vec::new();
    while !pending.is_empty() {
        let (ready, rest): (Vec<Json>, Vec<Json>) = pending.into_iter().partition(|row| {
            pairs.iter().all(|(child, _)| {
                row.get(*child)
                    .is_none_or(|value| value.is_null() || written.contains(&value.to_string()))
            })
        });
        // Rows whose parents are not in the table at all go in as they are.
        let ready = if ready.is_empty() {
            rest.clone()
        } else {
            ready
        };
        let rest = if ready.len() == rest.len() && ready == rest {
            Vec::new()
        } else {
            rest
        };
        for (_, parent) in &pairs {
            written.extend(
                ready
                    .iter()
                    .filter_map(|row| Some(row.get(*parent)?.to_string())),
            );
        }
        for chunk in ready.chunks(rows_per_insert(kinds.len().max(1))) {
            let mut insert = Query::insert();
            insert
                .into_table(Alias::new(&head.name))
                .columns(head.columns.iter().map(|(name, _)| Alias::new(name)));
            for row in chunk {
                insert
                    .values(json_to_values(row, &kinds)?.into_iter().map(Expr::val))
                    .map_err(|_| unreadable())?;
            }
            transaction.execute(&insert).await?;
        }
        pending = rest;
    }
    for column in catalog::columns(transaction, &head.name).await? {
        if column.counts_up {
            catalog::continue_count(transaction, &head.name, &column.name).await?;
        }
    }
    Ok(())
}

/// Everything after the database is back: migrations, sessions, the replay list, the anchor, the
/// records and the check of the log.
async fn finish(
    input: &RestoreInput<'_>,
    ids: &mut impl IdSource,
    restored: &Database,
    manifest: &Manifest,
    work: &Path,
    before: &CreatedBackup,
    present_head: i64,
) -> Result<RestoreReport, StoreError> {
    let migrations_run = restored.migrate(input.now).await?;
    let mut sessions = Query::delete();
    sessions.from_table(SignInSession::Table);
    let sessions_ended = restored.execute(&sessions).await?;
    let (replayed, not_replayed, unreadable_lines) =
        replay_newer(input, ids, restored, manifest, work).await?;
    let mut transaction = restored.begin_read().await?;
    let head = head_in(&mut transaction).await?;
    transaction.commit().await?;
    let backup_seq = head.map_or(0, |(seq, _)| seq);
    let lost = (present_head > backup_seq).then_some((backup_seq + 1, present_head));
    input.seal.restart(head, lost.unwrap_or((0, 0)))?;
    record(input, ids, restored, manifest, before, lost).await?;
    let log = restored.verify_security_log(input.seal).await?;
    Ok(RestoreReport {
        moment: manifest.moment,
        before: before.clone(),
        migrations_run,
        sessions_ended,
        replayed,
        not_replayed,
        unreadable_lines,
        lost,
        log,
    })
}

/// Writes the records of the backup of the present and of the restore, each with its event and
/// its line in the security log.
async fn record(
    input: &RestoreInput<'_>,
    ids: &mut impl IdSource,
    restored: &Database,
    manifest: &Manifest,
    before: &CreatedBackup,
    lost: Option<(i64, i64)>,
) -> Result<(), StoreError> {
    let (size, checksum) = archive::measure(input.archive)?;
    let restored_file = CreatedFile {
        name: BackupName::new(Purpose::Regular, manifest.moment, &manifest.version),
        size,
        checksum,
    };
    let runs = [
        BackupRun::new(
            BackupId::generate(ids).map_err(|_| unreadable())?,
            BackupEvent::Made,
            &before.file,
            BackupMaker::CommandLine,
            None,
            input.now,
        ),
        BackupRun::new(
            BackupId::generate(ids).map_err(|_| unreadable())?,
            BackupEvent::Restored,
            &restored_file,
            BackupMaker::CommandLine,
            None,
            input.now,
        )
        .with_lost(lost),
    ];
    let storage = AuthStorage::new(restored.clone(), Arc::clone(input.keys), input.seal.clone());
    for run in &runs {
        let event = run
            .domain_event(
                EventId::generate(ids).map_err(|_| unreadable())?,
                Actor::System,
                Channel::System,
            )
            .map_err(|_| unreadable())?;
        record_backup_run(&storage, run, &event, None)
            .await?
            .written?;
    }
    Ok(())
}

/// Merges the copies of the replay list (the data folder's, the backup's and the ones given),
/// keeps the merged list in the data folder, and runs every line newer than the backup.
async fn replay_newer(
    input: &RestoreInput<'_>,
    ids: &mut impl IdSource,
    restored: &Database,
    manifest: &Manifest,
    work: &Path,
) -> Result<(Vec<ReplayLine>, Vec<ReplayLine>, usize), StoreError> {
    let mut copies = vec![
        input.places.replay_list.clone(),
        work.join(member::REPLAY_LIST),
    ];
    copies.extend(input.more_replay_lists.iter().cloned());
    let mut unreadable_lines = 0;
    let mut lists = Vec::new();
    for copy in copies {
        let text = match std::fs::read_to_string(&copy) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(io(error)),
        };
        let (lines, unreadable) = replay::read(&text);
        unreadable_lines += unreadable;
        lists.push(lines);
    }
    let merged = replay::merge(lists);
    if !merged.is_empty() {
        if let Some(folder) = input.places.replay_list.parent() {
            std::fs::create_dir_all(folder).map_err(io)?;
        }
        std::fs::write(&input.places.replay_list, replay::write(&merged)).map_err(io)?;
    }
    let mut replayed = Vec::new();
    let mut not_replayed = Vec::new();
    for line in merged
        .into_iter()
        .filter(|line| line.moment > manifest.moment)
    {
        if replay_line(restored, ids, &line, input.now).await? {
            replayed.push(line);
        } else {
            not_replayed.push(line);
        }
    }
    Ok((replayed, not_replayed, unreadable_lines))
}

/// Runs one line again; `false` for a kind this program cannot run yet (a removal and a switch-off
/// come with *Remove a person*, a support access with support access).
async fn replay_line(
    database: &Database,
    ids: &mut impl IdSource,
    line: &ReplayLine,
    now: Timestamp,
) -> Result<bool, StoreError> {
    let bad = || StoreError::Corrupt("the ID of a line of the replay list");
    let (kind, entity) = match line.kind {
        ReplayKind::AccessKey => (
            "access_key",
            EntityId::from(line.id.parse::<AccessKeyId>().map_err(|_| bad())?),
        ),
        ReplayKind::AiAppGrant => (
            "oauth_grant",
            EntityId::from(line.id.parse::<GrantId>().map_err(|_| bad())?),
        ),
        ReplayKind::Passkey => (
            "passkey",
            EntityId::from(line.id.parse::<PasskeyId>().map_err(|_| bad())?),
        ),
        ReplayKind::Removal | ReplayKind::SwitchOff | ReplayKind::SupportAccess => {
            return Ok(false);
        }
    };
    let event = DomainEvent {
        id: EventId::generate(ids).map_err(|_| bad())?,
        time: now,
        actor: Actor::System,
        channel: Channel::System,
        entity: EntityRef {
            kind: EntityKind::new(kind).map_err(|_| bad())?,
            id: entity,
        },
        changes: vec![FieldChange {
            field: "replayed".to_owned(),
            before: None,
            after: Some(line.to_string().into()),
        }],
        reason: None,
        run: None,
    };
    let line = line.clone();
    database
        .record_change(&event, async move |transaction| {
            take_back(transaction, &line, now).await
        })
        .await?;
    Ok(true)
}

/// Takes back what a line names, through the function that takes it back in the program.
async fn take_back(
    transaction: &mut Transaction,
    line: &ReplayLine,
    now: Timestamp,
) -> Result<(), StoreError> {
    let bad = || StoreError::Corrupt("the ID of a line of the replay list");
    match line.kind {
        ReplayKind::AccessKey => {
            let id: AccessKeyId = line.id.parse().map_err(|_| bad())?;
            let mut owner = Query::select();
            owner
                .column(AccessKeyRow::Owner)
                .from(AccessKeyRow::Table)
                .and_where(Expr::col(AccessKeyRow::Id).eq(id.stored()));
            if let Some(row) = transaction.fetch_optional(&owner).await? {
                let owner: UserId = row.id(AccessKeyRow::Owner)?;
                crate::access_keys::delete_access_key(transaction, owner, id).await?;
            }
        }
        ReplayKind::AiAppGrant => {
            let id: GrantId = line.id.parse().map_err(|_| bad())?;
            AuthStorage::end_grant(transaction, id, GrantEnd::TakenBack, now).await?;
        }
        ReplayKind::Passkey => {
            let id: PasskeyId = line.id.parse().map_err(|_| bad())?;
            let mut person = Query::select();
            person
                .column(PasskeyRow::Person)
                .from(PasskeyRow::Table)
                .and_where(Expr::col(PasskeyRow::Id).eq(id.stored()));
            if let Some(row) = transaction.fetch_optional(&person).await? {
                let person: UserId = row.id(PasskeyRow::Person)?;
                AuthStorage::delete_passkey(transaction, person, id).await?;
            }
        }
        ReplayKind::Removal | ReplayKind::SwitchOff | ReplayKind::SupportAccess => {}
    }
    Ok(())
}
