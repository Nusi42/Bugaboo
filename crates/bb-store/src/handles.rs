//! The adapter of the handles port (`bb_app::notifications::HandleStore`): the short unique name of
//! each person for mentions, and the ones they had before (M3-F70, ADR-0033 §6).
//!
//! A handle names one person for good: the primary key is the handle, so two people cannot have it,
//! and the one a person gives up stays in its row, no longer the current one.

use bb_app::notifications::HandleStore;
use bb_core::id::UserId;
use bb_core::person::Person;
use jiff::Timestamp;
use sea_query::{Expr, ExprTrait as _, Order, Query};

use crate::auth::AuthStorage;
use crate::auth::{person_columns, read_person};
use crate::db::{LockClass, StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::schema::{PersonHandleRow, PersonRow};

impl HandleStore for AuthStorage {
    async fn handle_taken_in(
        transaction: &mut Transaction,
        handle: &str,
    ) -> Result<bool, StoreError> {
        let mut select = Query::select();
        select
            .column(PersonHandleRow::Handle)
            .from(PersonHandleRow::Table)
            .and_where(Expr::col(PersonHandleRow::Handle).eq(handle));
        Ok(transaction.fetch_optional(&select).await?.is_some())
    }

    async fn handle_of_person(&self, person: UserId) -> Result<Option<String>, StoreError> {
        let mut select = Query::select();
        select
            .column(PersonHandleRow::Handle)
            .from(PersonHandleRow::Table)
            .and_where(Expr::col(PersonHandleRow::Person).eq(person.stored()))
            .and_where(Expr::col(PersonHandleRow::Current).eq(true));
        self.database()
            .fetch_optional(&select)
            .await?
            .map(|row| row.get(PersonHandleRow::Handle))
            .transpose()
    }

    async fn person_with_handle(&self, handle: &str) -> Result<Option<UserId>, StoreError> {
        let mut select = Query::select();
        select
            .column(PersonHandleRow::Person)
            .from(PersonHandleRow::Table)
            .and_where(Expr::col(PersonHandleRow::Handle).eq(handle));
        self.database()
            .fetch_optional(&select)
            .await?
            .map(|row| row.id::<UserId>(PersonHandleRow::Person))
            .transpose()
    }

    async fn person_with_current_handle(&self, handle: &str) -> Result<Option<UserId>, StoreError> {
        let mut select = Query::select();
        select
            .column(PersonHandleRow::Person)
            .from(PersonHandleRow::Table)
            .and_where(Expr::col(PersonHandleRow::Handle).eq(handle))
            .and_where(Expr::col(PersonHandleRow::Current).eq(true));
        self.database()
            .fetch_optional(&select)
            .await?
            .map(|row| row.id::<UserId>(PersonHandleRow::Person))
            .transpose()
    }

    async fn handle_changed_in(
        transaction: &mut Transaction,
        person: UserId,
    ) -> Result<Option<Timestamp>, StoreError> {
        // The lock is named by the person: four random bytes of the ID.
        let bytes = person.to_bytes();
        let mut tail = [0_u8; 4];
        tail.copy_from_slice(&bytes[12..]);
        transaction
            .advisory_lock(LockClass::Handles, i32::from_be_bytes(tail))
            .await?;
        let mut select = Query::select();
        select
            .columns([PersonHandleRow::ChangedAt, PersonHandleRow::Current])
            .from(PersonHandleRow::Table)
            .and_where(Expr::col(PersonHandleRow::Person).eq(person.stored()));
        let rows = transaction.fetch_all(&select).await?;
        // One row is the handle the account was given; a second one is a change.
        if rows.len() < 2 {
            return Ok(None);
        }
        for row in &rows {
            if row.get::<bool>(PersonHandleRow::Current)? {
                return row.time(PersonHandleRow::ChangedAt).map(Some);
            }
        }
        Ok(None)
    }

    async fn people_without_handle(&self, limit: u64) -> Result<Vec<Person>, StoreError> {
        let mut select = Query::select();
        select
            .columns(person_columns().map(|column| (PersonRow::Table, column)))
            .from(PersonRow::Table)
            .left_join(
                PersonHandleRow::Table,
                Expr::col((PersonHandleRow::Table, PersonHandleRow::Person))
                    .equals((PersonRow::Table, PersonRow::Id)),
            )
            .and_where(Expr::col((PersonHandleRow::Table, PersonHandleRow::Person)).is_null())
            .order_by((PersonRow::Table, PersonRow::CreatedAt), Order::Asc)
            .order_by((PersonRow::Table, PersonRow::Id), Order::Asc)
            .limit(limit);
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_person)
            .collect()
    }

    async fn handle_choices(
        &self,
        start: &str,
        limit: u64,
    ) -> Result<Vec<(Person, String)>, StoreError> {
        let mut select = Query::select();
        select
            .columns(person_columns().map(|column| (PersonRow::Table, column)))
            .column((PersonHandleRow::Table, PersonHandleRow::Handle))
            .from(PersonHandleRow::Table)
            .inner_join(
                PersonRow::Table,
                Expr::col((PersonHandleRow::Table, PersonHandleRow::Person))
                    .equals((PersonRow::Table, PersonRow::Id)),
            )
            .and_where(Expr::col((PersonHandleRow::Table, PersonHandleRow::Current)).eq(true))
            .order_by(
                (PersonHandleRow::Table, PersonHandleRow::Handle),
                Order::Asc,
            )
            .limit(limit);
        // A handle holds only letters, digits, `_`, `.` and `-`: the start has none of the
        // characters a pattern gives a meaning to, except `_`, which is escaped.
        if !start.is_empty() {
            select.and_where(
                Expr::col((PersonHandleRow::Table, PersonHandleRow::Handle)).like(
                    sea_query::LikeExpr::new(format!("{}%", start.replace('_', "\\_")))
                        .escape('\\'),
                ),
            );
        }
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(|row| Ok((read_person(row)?, row.get(PersonHandleRow::Handle)?)))
            .collect()
    }

    async fn put_handle(
        transaction: &mut Transaction,
        person: UserId,
        handle: &str,
        at: Timestamp,
    ) -> Result<(), StoreError> {
        let mut retire = Query::update();
        retire
            .table(PersonHandleRow::Table)
            .value(PersonHandleRow::Current, false)
            .and_where(Expr::col(PersonHandleRow::Person).eq(person.stored()));
        transaction.execute(&retire).await?;
        // A handle the person had before is theirs again; anybody else's makes the insert fail.
        let mut take_up = Query::update();
        take_up
            .table(PersonHandleRow::Table)
            .value(PersonHandleRow::Current, true)
            .value(PersonHandleRow::ChangedAt, to_micros(at))
            .and_where(Expr::col(PersonHandleRow::Person).eq(person.stored()))
            .and_where(Expr::col(PersonHandleRow::Handle).eq(handle));
        if transaction.execute(&take_up).await? > 0 {
            return Ok(());
        }
        let mut insert = Query::insert();
        insert
            .into_table(PersonHandleRow::Table)
            .columns([
                PersonHandleRow::Person,
                PersonHandleRow::Handle,
                PersonHandleRow::ChangedAt,
                PersonHandleRow::Current,
            ])
            .values([
                Expr::val(person.stored()),
                Expr::val(handle),
                Expr::val(to_micros(at)),
                Expr::val(true),
            ])
            .map_err(|_| StoreError::Corrupt("a handle"))?;
        transaction.execute(&insert).await?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/handles.rs"]
mod tests;
