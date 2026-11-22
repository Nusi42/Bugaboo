//! The adapter of the port of *Download my data* (`bb_app::privacy::export::ExportStore`): one row
//! for each file, on both databases (PRIV-03).

use bb_app::privacy::export::ExportStore;
use bb_core::data_export::{DataExport, ExportState};
use bb_core::id::{DataExportId, UserId};
use jiff::Timestamp;
use sea_query::{Expr, ExprTrait as _, Order, Query, SelectStatement};

use crate::auth::AuthStorage;
use crate::db::{Row, StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::locks::{LockClass, lock_key};
use crate::schema::DataExportRow;

fn select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            DataExportRow::Id,
            DataExportRow::Person,
            DataExportRow::AskedBy,
            DataExportRow::Reason,
            DataExportRow::State,
            DataExportRow::AskedAt,
            DataExportRow::ReadyAt,
            DataExportRow::Bytes,
        ])
        .from(DataExportRow::Table);
    select
}

fn read_export(row: &Row) -> Result<DataExport, StoreError> {
    Ok(DataExport {
        id: row.id::<DataExportId>(DataExportRow::Id)?,
        person: row.id::<UserId>(DataExportRow::Person)?,
        asked_by: row.id::<UserId>(DataExportRow::AskedBy)?,
        reason: row.get(DataExportRow::Reason)?,
        state: ExportState::from_code(&row.get::<String>(DataExportRow::State)?)
            .ok_or(StoreError::Corrupt("the state of a data export"))?,
        asked_at: row.time(DataExportRow::AskedAt)?,
        ready_at: row.optional_time(DataExportRow::ReadyAt)?,
        bytes: row.get(DataExportRow::Bytes)?,
    })
}

/// The newest first, and by ID among those of the same moment, so pages neither repeat nor skip.
fn newest_first(select: &mut SelectStatement) {
    select
        .order_by(DataExportRow::AskedAt, Order::Desc)
        .order_by(DataExportRow::Id, Order::Desc);
}

impl ExportStore for AuthStorage {
    async fn data_export_in(
        transaction: &mut Transaction,
        id: DataExportId,
    ) -> Result<Option<DataExport>, StoreError> {
        let mut select = select();
        select.and_where(Expr::col(DataExportRow::Id).eq(id.stored()));
        transaction
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_export)
            .transpose()
    }

    async fn waiting_export_of_in(
        transaction: &mut Transaction,
        person: UserId,
    ) -> Result<Option<DataExport>, StoreError> {
        let mut select = select();
        select
            .and_where(Expr::col(DataExportRow::Person).eq(person.stored()))
            .and_where(Expr::col(DataExportRow::State).eq(ExportState::Waiting.code()))
            .limit(1);
        transaction
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_export)
            .transpose()
    }

    async fn exports_of_in(
        transaction: &mut Transaction,
        person: UserId,
    ) -> Result<Vec<DataExportId>, StoreError> {
        let mut select = Query::select();
        select
            .column(DataExportRow::Id)
            .from(DataExportRow::Table)
            .and_where(Expr::col(DataExportRow::Person).eq(person.stored()));
        transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(|row| row.id::<DataExportId>(DataExportRow::Id))
            .collect()
    }

    async fn export_asks_since_in(
        transaction: &mut Transaction,
        person: UserId,
        since: Timestamp,
    ) -> Result<Vec<Timestamp>, StoreError> {
        let mut select = Query::select();
        select
            .column(DataExportRow::AskedAt)
            .from(DataExportRow::Table)
            .and_where(Expr::col(DataExportRow::Person).eq(person.stored()))
            .and_where(Expr::col(DataExportRow::AskedAt).gt(to_micros(since)));
        transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(|row| row.time(DataExportRow::AskedAt))
            .collect()
    }

    async fn latest_export_of(
        &self,
        person: UserId,
        asked_by: UserId,
    ) -> Result<Option<DataExport>, StoreError> {
        let mut select = select();
        select
            .and_where(Expr::col(DataExportRow::Person).eq(person.stored()))
            .and_where(Expr::col(DataExportRow::AskedBy).eq(asked_by.stored()))
            .limit(1);
        newest_first(&mut select);
        self.database()
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_export)
            .transpose()
    }

    async fn data_exports_page(
        &self,
        skip: u64,
        limit: u64,
    ) -> Result<Vec<DataExport>, StoreError> {
        let mut select = select();
        newest_first(&mut select);
        select.limit(limit).offset(skip);
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_export)
            .collect()
    }

    async fn exports_asked_before(
        &self,
        before: Timestamp,
        limit: u64,
    ) -> Result<Vec<DataExport>, StoreError> {
        let mut select = select();
        select
            .and_where(Expr::col(DataExportRow::AskedAt).lt(to_micros(before)))
            .order_by(DataExportRow::AskedAt, Order::Asc)
            .order_by(DataExportRow::Id, Order::Asc)
            .limit(limit);
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_export)
            .collect()
    }

    async fn lock_exports_of(
        transaction: &mut Transaction,
        person: UserId,
    ) -> Result<(), StoreError> {
        transaction
            .advisory_lock(LockClass::DataExports, lock_key(person.to_bytes()))
            .await
    }

    async fn insert_data_export(
        transaction: &mut Transaction,
        export: &DataExport,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(DataExportRow::Table)
            .columns([
                DataExportRow::Id,
                DataExportRow::Person,
                DataExportRow::AskedBy,
                DataExportRow::Reason,
                DataExportRow::State,
                DataExportRow::AskedAt,
                DataExportRow::ReadyAt,
                DataExportRow::Bytes,
            ])
            .values([
                Expr::val(export.id.stored()),
                Expr::val(export.person.stored()),
                Expr::val(export.asked_by.stored()),
                Expr::val(export.reason.clone()),
                Expr::val(export.state.code()),
                Expr::val(to_micros(export.asked_at)),
                Expr::val(export.ready_at.map(to_micros)),
                Expr::val(export.bytes),
            ])
            .map_err(|_| StoreError::Corrupt("a data export"))?;
        transaction.execute(&insert).await?;
        Ok(())
    }

    async fn finish_data_export(
        transaction: &mut Transaction,
        id: DataExportId,
        state: ExportState,
        ready_at: Option<Timestamp>,
        bytes: Option<i64>,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(DataExportRow::Table)
            .value(DataExportRow::State, state.code())
            .value(DataExportRow::ReadyAt, Expr::val(ready_at.map(to_micros)))
            .value(DataExportRow::Bytes, Expr::val(bytes))
            .and_where(Expr::col(DataExportRow::Id).eq(id.stored()))
            .and_where(Expr::col(DataExportRow::State).eq(ExportState::Waiting.code()));
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn delete_data_export(
        transaction: &mut Transaction,
        id: DataExportId,
    ) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(DataExportRow::Table)
            .and_where(Expr::col(DataExportRow::Id).eq(id.stored()));
        Ok(transaction.execute(&delete).await? > 0)
    }
}

#[cfg(test)]
#[path = "tests/data_exports.rs"]
mod tests;
