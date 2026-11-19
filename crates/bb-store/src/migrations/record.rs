//! The record of the migrations applied to a database: the table `migration`, one row for each
//! migration with its name, checksum and the moment it ran.

use bb_core::migration::MigrationRecord;
use bb_core::security_log::Digest;
use bb_core::technical::TechnicalName;
use jiff::Timestamp;
use sea_query::{ColumnDef, Expr, Order, Query, Table, TableCreateStatement};

use super::Definition;
use crate::db::{Database, Row, Transaction, from_micros, to_micros};
use crate::error::StoreError;
use crate::schema::Migration;

/// The table of the record, made when a database has none yet.
pub(super) fn record_table() -> TableCreateStatement {
    Table::create()
        .table(Migration::Table)
        .if_not_exists()
        .col(
            ColumnDef::new(Migration::Version)
                .big_integer()
                .not_null()
                .primary_key(),
        )
        .col(ColumnDef::new(Migration::Name).text().not_null())
        .col(ColumnDef::new(Migration::Checksum).blob().not_null())
        .col(
            ColumnDef::new(Migration::AppliedAt)
                .big_integer()
                .not_null(),
        )
        .to_owned()
}

impl Database {
    /// The migrations the database has recorded, oldest first.
    ///
    /// # Errors
    /// Fails when the database fails or a record is unreadable.
    pub async fn applied_migrations(&self) -> Result<Vec<MigrationRecord>, StoreError> {
        self.fetch_all(&applied_query())
            .await?
            .iter()
            .map(read_record)
            .collect()
    }
}

fn applied_query() -> sea_query::SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            Migration::Version,
            Migration::Name,
            Migration::Checksum,
            Migration::AppliedAt,
        ])
        .from(Migration::Table)
        .order_by(Migration::Version, Order::Asc);
    select
}

pub(crate) async fn read_applied(
    transaction: &mut Transaction,
) -> Result<Vec<MigrationRecord>, StoreError> {
    transaction
        .fetch_all(&applied_query())
        .await?
        .iter()
        .map(read_record)
        .collect()
}

pub(super) async fn record(
    transaction: &mut Transaction,
    definition: &Definition,
    now: Timestamp,
) -> Result<(), StoreError> {
    let mut insert = Query::insert();
    insert
        .into_table(Migration::Table)
        .columns([
            Migration::Version,
            Migration::Name,
            Migration::Checksum,
            Migration::AppliedAt,
        ])
        .values([
            Expr::val(definition.version),
            Expr::val(definition.name),
            Expr::val(definition.checksum()?.as_bytes().to_vec()),
            Expr::val(to_micros(now)),
        ])
        .map_err(|_| StoreError::Corrupt("a migration record"))?;
    transaction.execute(&insert).await?;
    Ok(())
}

fn read_record(row: &Row) -> Result<MigrationRecord, StoreError> {
    let checksum: Vec<u8> = row.get(Migration::Checksum)?;
    Ok(MigrationRecord {
        version: row.get(Migration::Version)?,
        name: TechnicalName::new(&row.get::<String>(Migration::Name)?)
            .map_err(|_| StoreError::Corrupt("a migration name"))?,
        checksum: Digest::from_slice(&checksum).ok_or(StoreError::Corrupt("a checksum"))?,
        applied_at: from_micros(row.get(Migration::AppliedAt)?)?,
    })
}

impl Database {
    /// Records a migration this program does not have, as a newer program would have: the database
    /// then stands [`SchemaState::Newer`], for the tests of what refuses to work on it.
    ///
    /// # Errors
    /// When the database fails.
    #[cfg(any(test, feature = "testing"))]
    pub async fn record_newer_migration(&self, now: Timestamp) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(Migration::Table)
            .columns([
                Migration::Version,
                Migration::Name,
                Migration::Checksum,
                Migration::AppliedAt,
            ])
            .values([
                Expr::val(super::latest_version() + 1),
                Expr::val("from-the-future"),
                Expr::val(vec![0_u8; 32]),
                Expr::val(to_micros(now)),
            ])
            .map_err(|_| StoreError::Corrupt("a migration"))?;
        self.execute(&insert).await?;
        Ok(())
    }

    /// Changes the checksum recorded for migration `version`, as another program with a migration
    /// of that number in another form would have left it: the database is then refused with
    /// [`StoreError::MigrationMismatch`], for the tests of what refuses to work on it.
    ///
    /// # Errors
    /// When the database fails.
    #[cfg(any(test, feature = "testing"))]
    pub async fn change_recorded_migration(&self, version: i64) -> Result<(), StoreError> {
        let mut update = Query::update();
        update
            .table(Migration::Table)
            .value(Migration::Checksum, vec![1_u8; 32])
            .and_where(sea_query::ExprTrait::eq(
                Expr::col(Migration::Version),
                version,
            ));
        self.execute(&update).await?;
        Ok(())
    }
}
