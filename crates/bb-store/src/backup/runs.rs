//! The records of the backups made and restored (migration 54), each with its domain event and
//! its line in the security log (ADR-0046, decisions 3 and 4).

use bb_core::backup::name::BackupName;
use bb_core::backup::run::{BackupEvent, BackupMaker, BackupRun};
use bb_core::event::DomainEvent;
use bb_core::id::UserId;
use bb_core::security_log::Digest;
use bb_core::technical::TechnicalName;
use sea_query::{Expr, Order, Query};

use crate::anchor::LogSeal;
use crate::auth::AuthStorage;
use crate::db::{Database, Row, StoredId as _, Transaction, to_micros};
use crate::error::StoreError;
use crate::schema::BackupRunRow as Columns;
use crate::security_log::NewSecurityLogEntry;

const COLUMNS: [Columns; 10] = [
    Columns::Id,
    Columns::Event,
    Columns::Name,
    Columns::Size,
    Columns::Checksum,
    Columns::Maker,
    Columns::Person,
    Columns::At,
    Columns::LostFrom,
    Columns::LostTo,
];

async fn insert(transaction: &mut Transaction, run: &BackupRun) -> Result<(), StoreError> {
    let mut insert = Query::insert();
    insert
        .into_table(Columns::Table)
        .columns(COLUMNS)
        .values([
            Expr::val(run.id.stored()),
            Expr::val(run.event.code()),
            Expr::val(run.name.to_string()),
            Expr::val(run.size),
            Expr::val(run.checksum.as_bytes().to_vec()),
            Expr::val(run.maker.code()),
            Expr::val(run.person.map(UserId::stored)),
            Expr::val(to_micros(run.at)),
            Expr::val(run.lost_from),
            Expr::val(run.lost_to),
        ])
        .map_err(|_| StoreError::Corrupt("a backup record"))?;
    transaction.execute(&insert).await.map(|_| ())
}

fn read(row: &Row) -> Result<BackupRun, StoreError> {
    let word = |column| -> Result<String, StoreError> { row.get(column) };
    let checksum: Vec<u8> = row.get(Columns::Checksum)?;
    Ok(BackupRun {
        id: row.id(Columns::Id)?,
        event: BackupEvent::from_code(&word(Columns::Event)?)
            .ok_or(StoreError::Corrupt("what happened to a backup"))?,
        name: BackupName::parse(&word(Columns::Name)?)
            .ok_or(StoreError::Corrupt("the name of a backup"))?,
        size: row.get(Columns::Size)?,
        checksum: Digest::from_slice(&checksum)
            .ok_or(StoreError::Corrupt("the checksum of a backup"))?,
        maker: BackupMaker::from_code(&word(Columns::Maker)?)
            .ok_or(StoreError::Corrupt("who made a backup"))?,
        person: row.optional_id(Columns::Person)?,
        at: row.time(Columns::At)?,
        lost_from: row.get(Columns::LostFrom)?,
        lost_to: row.get(Columns::LostTo)?,
    })
}

impl Database {
    /// Writes the record of a backup made or restored together with its event, and then its line
    /// in the security log, which points to the record.
    ///
    /// # Errors
    /// Fails when the database fails, or with [`StoreError::AnchorFile`] when the line is stored
    /// but the anchor file could not be written.
    pub async fn record_backup(
        &self,
        seal: &LogSeal,
        run: &BackupRun,
        event: &DomainEvent,
    ) -> Result<(), StoreError> {
        self.record_change(event, async |transaction| insert(transaction, run).await)
            .await?;
        let action = TechnicalName::new(run.security_action().code())
            .map_err(|_| StoreError::Corrupt("an action of the security log"))?;
        self.append_security_log(
            seal,
            &NewSecurityLogEntry {
                time: run.at,
                person: run.person,
                action,
                subject: Some(event.entity.clone()),
                network: None,
            },
        )
        .await
        .map(|_| ())
    }

    /// The newest `limit` records of backups made and restored, newest first.
    ///
    /// # Errors
    /// Fails when the database fails or a record cannot be read.
    pub async fn backup_runs(&self, limit: u64) -> Result<Vec<BackupRun>, StoreError> {
        let mut select = Query::select();
        select
            .columns(COLUMNS)
            .from(Columns::Table)
            .order_by(Columns::At, Order::Desc)
            .limit(limit);
        self.fetch_all(&select).await?.iter().map(read).collect()
    }
}

impl bb_app::backups::BackupStore for AuthStorage {
    async fn backup_runs(&self, limit: u64) -> Result<Vec<BackupRun>, StoreError> {
        self.database().backup_runs(limit).await
    }

    async fn insert_backup_run(
        transaction: &mut Transaction,
        run: &BackupRun,
    ) -> Result<(), StoreError> {
        insert(transaction, run).await
    }
}
