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

use bb_app::access::PolicyStore as _;
use bb_app::auth::{AuthStore as _, OAuthStore as _, SecondStepStore as _, SecurityEvent};
use bb_app::backups::record_backup_run;
use bb_app::people::switch_off_in;
use bb_app::ports::ChangeStore;
use bb_app::privacy::removal::{NotPrepared, prepare_removal, remove_in};
use bb_app::privacy::replay::Asked;
use bb_core::backup::DatabaseKind;
use bb_core::backup::manifest::{BackupProblem, KnownMigration, Manifest};
use bb_core::backup::member::{self, MemberName};
use bb_core::backup::name::{BackupName, Purpose};
use bb_core::backup::replay::{self, ReplayKind, ReplayLine};
use bb_core::backup::run::{BackupEvent, BackupMaker, BackupRun, CreatedFile};
use bb_core::event::{Actor, Channel, DomainEvent, EntityKind, EntityRef, FieldChange, RecordKind};
use bb_core::hex;
use bb_core::id::{AccessKeyId, BackupId, EntityId, EventId, GrantId, IdSource, PasskeyId, UserId};
use bb_core::oauth::GrantEnd;
use bb_core::settings::privacy::RemovedTexts;
use bb_core::sign_in::SecurityAction;
use bb_policy::guards::ensure_a_manager_remains;
use jiff::Timestamp;
use sea_query::{Expr, ExprTrait as _, Query};

use super::{BackupPlaces, CreatedBackup, archive, fill, io, seal, work_folder};
use crate::anchor::{Anchored, LogSeal};
use crate::auth::AuthStorage;
use crate::auth::replay_waiting::waiting_lines_of_any_version;
use crate::chain::Verification;
use crate::db::{Database, StoredId as _, Transaction};
use crate::error::StoreError;
use crate::keys::KeyRing;
use crate::migrations::known_migrations;
use crate::schema::{AccessKeyRow, PasskeyRow, SignInSession};
use crate::secrets::keys_open;
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
    /// Whether the lines that ran again left nobody who manages roles and decides who has which
    /// role, where somebody did before them: the replay does not check it (ADR-0044, decision 8),
    /// so it says so here and in the security log, and setup is open again.
    pub nobody_decides_roles: bool,
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
    let manifest = check(input.archive, input.target, &input.places.keys, work)
        .await
        .map_err(early)?;
    let present = input.target.connect().await.map_err(early)?;
    // The lines a change wrote and the file of the list does not hold yet are part of the list:
    // read before the database that holds them is replaced (ADR-0044, decision 8).
    let waiting = waiting_lines_of_any_version(&present).await;
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
    let present = Present {
        head: present_head.map_err(failed)?,
        waiting: waiting.map_err(failed)?,
        before: before.clone(),
    };
    put_back(
        input.target,
        &input.places.files,
        input.now,
        &manifest,
        work,
    )
    .await
    .map_err(failed)?;
    let restored = input.target.connect().await.map_err(failed)?;
    let done = finish(input, ids, &restored, &manifest, work, &present).await;
    input.target.release(&restored).await;
    done.map_err(failed)
}

/// Reads and checks the backup `archive` into `work` for `target`; nothing else is touched.
async fn check(
    archive: &Path,
    target: &RestoreTarget,
    keys: &KeyRing,
    work: &Path,
) -> Result<Manifest, StoreError> {
    let manifest = archive::read(archive, work, keys)?;
    manifest
        .check_database(target.kind())
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
        DatabaseKind::Postgres => fill::secrets_in_rows(&database_member(work, &manifest)?)?,
    };
    if seal::keys_fit(keys, &manifest.keys)? && keys_open(keys, &secrets) {
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
    target: &RestoreTarget,
    files: &Path,
    now: Timestamp,
    manifest: &Manifest,
    work: &Path,
) -> Result<(), StoreError> {
    let copy = database_member(work, manifest)?;
    match target {
        RestoreTarget::File(file) => replace_file(&copy, file)?,
        RestoreTarget::Postgres(database) => {
            fill::fill(database, manifest, &copy, now).await?;
        }
    }
    put_back_files(&work.join(member::FILES), files)
}

/// Puts the backup `archive`, made just before, back in place of `database` after an upgrade
/// whose migration failed (ADR-0046, decision 5), and checks that the database has the migrations
/// the backup names again. Nothing else of a restore is done: nothing was written in between, so
/// the sessions, the replay list and the anchor of the security log are as the backup has them.
/// A SQLite database is closed, because its file is replaced.
pub(crate) async fn put_back_before_upgrade(
    archive: &Path,
    database: &Database,
    places: &BackupPlaces,
    now: Timestamp,
) -> Result<(), StoreError> {
    let target = RestoreTarget::of(database.clone()).await;
    let work = work_folder(&places.folder, "restore")?;
    let result = put_back_checked(archive, &target, places, now, &work).await;
    let _ = std::fs::remove_dir_all(&work);
    result
}

async fn put_back_checked(
    archive: &Path,
    target: &RestoreTarget,
    places: &BackupPlaces,
    now: Timestamp,
    work: &Path,
) -> Result<(), StoreError> {
    let manifest = check(archive, target, &places.keys, work).await?;
    put_back(target, &places.files, now, &manifest, work).await?;
    let restored = target.connect().await?;
    let applied = restored.applied_migrations().await;
    target.release(&restored).await;
    let applied: Vec<(i64, String)> = applied?
        .iter()
        .map(|record| (record.version, hex::encode(record.checksum.as_bytes())))
        .collect();
    let named: Vec<(i64, String)> = manifest
        .migrations
        .iter()
        .map(|migration| (migration.version, migration.checksum.clone()))
        .collect();
    if applied == named {
        Ok(())
    } else {
        Err(StoreError::Corrupt(
            "the migrations of the database put back after an upgrade",
        ))
    }
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

fn unreadable() -> StoreError {
    StoreError::Corrupt("the copy of a database")
}

/// What the restore read from the present database before it replaced it.
struct Present {
    /// The backup of the present.
    before: CreatedBackup,
    /// The newest entry of the security log it knew.
    head: i64,
    /// The lines of the replay list that waited in it for the file.
    waiting: Vec<String>,
}

/// Everything after the database is back: migrations, sessions, the replay list, the anchor, the
/// records and the check of the log.
async fn finish(
    input: &RestoreInput<'_>,
    ids: &mut impl IdSource,
    restored: &Database,
    manifest: &Manifest,
    work: &Path,
    present: &Present,
) -> Result<RestoreReport, StoreError> {
    let (before, present_head) = (&present.before, present.head);
    let migrations_run = restored.migrate(input.now).await?;
    let mut sessions = Query::delete();
    sessions.from_table(SignInSession::Table);
    let sessions_ended = restored.execute(&sessions).await?;
    let replay = replay_newer(input, ids, restored, manifest, work, &present.waiting).await?;
    let mut transaction = restored.begin_read().await?;
    let head = head_in(&mut transaction).await?;
    transaction.commit().await?;
    // What is gone is what came after the backup's own head, whatever the steps after the
    // replace wrote since.
    let backup_seq = manifest
        .security_log_head
        .as_ref()
        .map_or(0, |head| head.seq);
    let lost = (present_head > backup_seq).then_some((backup_seq + 1, present_head));
    input.seal.restart(head, lost.unwrap_or((0, 0)))?;
    record(input, ids, restored, manifest, before, lost).await?;
    if replay.nobody_decides_roles {
        let storage = AuthStorage::new(
            restored.clone(),
            Arc::clone(&input.places.keys),
            input.seal.clone(),
        );
        storage
            .append_security_log(&SecurityEvent {
                time: input.now,
                person: None,
                action: SecurityAction::RestoreLeftNobodyForRoles,
                subject: None,
                network: None,
            })
            .await?;
    }
    let log = restored.verify_security_log(input.seal).await?;
    // The merged list replaces the data folder's only now that the restore went through.
    keep_merged(input, &replay.merged)?;
    Ok(RestoreReport {
        moment: manifest.moment,
        before: before.clone(),
        migrations_run,
        sessions_ended,
        replayed: replay.replayed,
        not_replayed: replay.not_replayed,
        unreadable_lines: replay.unreadable,
        nobody_decides_roles: replay.nobody_decides_roles,
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
    // The restored backup by the name of its file, which says what it was made for (a backup made
    // before an earlier restore keeps that kind); a file renamed by hand is named from its manifest.
    let name = input
        .archive
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(BackupName::parse)
        .unwrap_or_else(|| BackupName::new(Purpose::Regular, manifest.moment, &manifest.version));
    let restored_file = CreatedFile {
        name,
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
    let storage = AuthStorage::new(
        restored.clone(),
        Arc::clone(&input.places.keys),
        input.seal.clone(),
    );
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

/// What the replay list of a restore did.
struct Replay {
    /// Every readable line of every copy, each once.
    merged: Vec<ReplayLine>,
    /// The lines newer than the backup that ran again.
    replayed: Vec<ReplayLine>,
    /// The lines newer than the backup that this program cannot run yet.
    not_replayed: Vec<ReplayLine>,
    /// How many lines of the copies could not be read.
    unreadable: usize,
    /// Whether the lines that ran again left nobody who decides roles.
    nobody_decides_roles: bool,
}

/// Keeps the merged replay list in the data folder.
fn keep_merged(input: &RestoreInput<'_>, merged: &[ReplayLine]) -> Result<(), StoreError> {
    if merged.is_empty() {
        return Ok(());
    }
    (input.places.write_replay_list)(&input.places.replay_list, replay::write(merged).as_bytes())
        .map_err(io)
}

/// Merges the copies of the replay list (the data folder's, the backup's, the ones given and the
/// lines that waited in the present database) and runs every line newer than the backup.
async fn replay_newer(
    input: &RestoreInput<'_>,
    ids: &mut impl IdSource,
    restored: &Database,
    manifest: &Manifest,
    work: &Path,
    waiting: &[String],
) -> Result<Replay, StoreError> {
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
    let (lines, unreadable) = replay::read(&waiting.join("\n"));
    unreadable_lines += unreadable;
    lists.push(lines);
    let merged = replay::merge(lists);
    let storage = AuthStorage::new(
        restored.clone(),
        Arc::clone(&input.places.keys),
        input.seal.clone(),
    );
    let mut replayed = Vec::new();
    let mut not_replayed = Vec::new();
    let roles_before = storage.organization_state().await?;
    for line in merged.iter().filter(|line| line.moment > manifest.moment) {
        if replay_line(&storage, ids, line, input.now).await? {
            replayed.push(line.clone());
        } else {
            not_replayed.push(line.clone());
        }
    }
    let roles_after = storage.organization_state().await?;
    Ok(Replay {
        merged,
        replayed,
        not_replayed,
        unreadable: unreadable_lines,
        nobody_decides_roles: ensure_a_manager_remains(&roles_before, &roles_after).is_err(),
    })
}

/// What a line of the replay list takes back, when this program can take it back.
#[derive(Clone, Copy)]
enum TakeBack {
    AccessKey(AccessKeyId),
    Grant(GrantId),
    Passkey(PasskeyId),
}

impl TakeBack {
    /// What `line` takes back; `None` for a removal and a switch-off (`replay_person`), for a kind
    /// this program cannot run yet (a support access comes with support access), and for an ID of
    /// another kind, which reading the line refused already.
    fn of(line: &ReplayLine) -> Option<Self> {
        match line.kind {
            ReplayKind::AccessKey => line.id.parse().ok().map(Self::AccessKey),
            ReplayKind::AiAppGrant => line.id.parse().ok().map(Self::Grant),
            ReplayKind::Passkey => line.id.parse().ok().map(Self::Passkey),
            ReplayKind::Removal | ReplayKind::SwitchOff | ReplayKind::SupportAccess => None,
        }
    }

    /// The record it is about, for its event.
    fn entity(self) -> (&'static str, EntityId) {
        match self {
            Self::AccessKey(id) => ("access_key", EntityId::from(id)),
            Self::Grant(id) => ("oauth_grant", EntityId::from(id)),
            Self::Passkey(id) => ("passkey", EntityId::from(id)),
        }
    }
}

/// The event of a line that runs again: about `entity`, by the system, with the line as its change.
fn replay_event(
    ids: &mut impl IdSource,
    entity: EntityRef,
    line: &ReplayLine,
    now: Timestamp,
) -> Result<DomainEvent, StoreError> {
    Ok(DomainEvent {
        id: EventId::generate(ids)
            .map_err(|_| StoreError::Corrupt("the event of a line of the replay list"))?,
        time: now,
        actor: Actor::System,
        channel: Channel::System,
        entity,
        changes: vec![FieldChange {
            field: "replayed".to_owned(),
            before: None,
            after: Some(line.to_string().into()),
        }],
        reason: None,
        run: None,
    })
}

/// Runs one line again; `false` for a line this program cannot run ([`TakeBack::of`]).
async fn replay_line(
    storage: &AuthStorage,
    ids: &mut impl IdSource,
    line: &ReplayLine,
    now: Timestamp,
) -> Result<bool, StoreError> {
    if matches!(line.kind, ReplayKind::Removal | ReplayKind::SwitchOff) {
        return replay_person(storage, ids, line, now).await;
    }
    let Some(target) = TakeBack::of(line) else {
        return Ok(false);
    };
    let bad = || StoreError::Corrupt("the event of a line of the replay list");
    let (kind, entity) = target.entity();
    let event = replay_event(
        ids,
        EntityRef {
            kind: EntityKind::new(kind).map_err(|_| bad())?,
            id: entity,
        },
        line,
        now,
    )?;
    storage
        .database()
        .record_change(&event, async move |transaction| {
            take_back(transaction, target, now).await
        })
        .await?;
    Ok(true)
}

/// Switches off or removes again the person a line names, through the one change of the program
/// (`switch_off_in`, `remove_in`) as the replay list asks: at the moment of the line, with its
/// comments choice, no switch-off before a removal and nothing done twice. The line names no
/// administrator, so the standing names the person as who did it.
async fn replay_person(
    storage: &AuthStorage,
    ids: &mut impl IdSource,
    line: &ReplayLine,
    now: Timestamp,
) -> Result<bool, StoreError> {
    let Ok(person) = line.id.parse::<UserId>() else {
        return Ok(false);
    };
    let event = replay_event(
        ids,
        EntityRef {
            kind: EntityKind::from(RecordKind::Person),
            id: EntityId::from(person),
        },
        line,
        now,
    )?;
    let at = line.moment;
    match line.kind {
        ReplayKind::SwitchOff => {
            ChangeStore::record_change(storage, &event, move |transaction| {
                Box::pin(async move {
                    switch_off_in::<AuthStorage>(transaction, person, at, Asked::ByReplay)
                        .await
                        .map(|_| ())
                })
            })
            .await?;
        }
        ReplayKind::Removal => {
            let comments = line.choice.as_deref().and_then(RemovedTexts::from_code);
            let removal = prepare_removal(storage, ids, person, at, comments, Asked::ByReplay)
                .await
                .map_err(|failure| match failure {
                    NotPrepared::Store(error) => error,
                    NotPrepared::NoFreeLabel | NotPrepared::NoChoice | NotPrepared::NoId(_) => {
                        StoreError::Corrupt("the removal of a line of the replay list")
                    }
                })?;
            ChangeStore::record_change(storage, &event, move |transaction| {
                Box::pin(async move {
                    remove_in::<AuthStorage, _>(transaction, &removal)
                        .await
                        .map(|_| ())
                })
            })
            .await?;
        }
        ReplayKind::AccessKey
        | ReplayKind::AiAppGrant
        | ReplayKind::Passkey
        | ReplayKind::SupportAccess => return Ok(false),
    }
    Ok(true)
}

/// Takes back what a line names, through the function that takes it back in the program.
async fn take_back(
    transaction: &mut Transaction,
    target: TakeBack,
    now: Timestamp,
) -> Result<(), StoreError> {
    match target {
        TakeBack::AccessKey(id) => {
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
        TakeBack::Grant(id) => {
            AuthStorage::end_grant(transaction, id, GrantEnd::TakenBack, now).await?;
        }
        TakeBack::Passkey(id) => {
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
    }
    Ok(())
}
