//! The adapter of the confirmation port (`bb_app::notifications::ConfirmationStore`): the accounts
//! made by signing up that wait for the link in an e-mail (M1-F38, ADR-0033 §3).
//!
//! A row exists only while an account waits. Opening the link deletes it; an account whose time ran
//! out is deleted with everything that belongs to it, which is only its password, its handle and
//! this row, because it could not sign in to make anything else.

use bb_app::notifications::ConfirmationStore;
use bb_core::confirmation::AddressConfirmation;
use bb_core::id::UserId;
use bb_core::security_log::Digest;
use jiff::Timestamp;
use sea_query::{Expr, ExprTrait as _, Order, Query};

use crate::auth::{AuthStorage, digest};
use crate::db::{Row, StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::schema::{AddressConfirmationRow, PasswordLogin, PersonHandleRow, PersonRow};

fn columns() -> [AddressConfirmationRow; 4] {
    [
        AddressConfirmationRow::Person,
        AddressConfirmationRow::TokenHash,
        AddressConfirmationRow::CreatedAt,
        AddressConfirmationRow::ExpiresAt,
    ]
}

fn read_confirmation(row: &Row) -> Result<AddressConfirmation, StoreError> {
    Ok(AddressConfirmation {
        person: row.id::<UserId>(AddressConfirmationRow::Person)?,
        token_hash: digest(&row.get::<Vec<u8>>(AddressConfirmationRow::TokenHash)?)?,
        created_at: row.time(AddressConfirmationRow::CreatedAt)?,
        expires_at: row.time(AddressConfirmationRow::ExpiresAt)?,
    })
}

impl ConfirmationStore for AuthStorage {
    async fn address_confirmation_by_hash(
        &self,
        hash: &Digest,
    ) -> Result<Option<AddressConfirmation>, StoreError> {
        let mut select = Query::select();
        select
            .columns(columns())
            .from(AddressConfirmationRow::Table)
            .and_where(Expr::col(AddressConfirmationRow::TokenHash).eq(hash.as_bytes().to_vec()));
        self.database()
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_confirmation)
            .transpose()
    }

    async fn unconfirmed(&self, person: UserId) -> Result<Option<AddressConfirmation>, StoreError> {
        let mut select = Query::select();
        select
            .columns(columns())
            .from(AddressConfirmationRow::Table)
            .and_where(Expr::col(AddressConfirmationRow::Person).eq(person.stored()));
        self.database()
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_confirmation)
            .transpose()
    }

    async fn renew_confirmation(
        &self,
        person: UserId,
        hash: &Digest,
        expires_at: Timestamp,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(AddressConfirmationRow::Table)
            .value(AddressConfirmationRow::TokenHash, hash.as_bytes().to_vec())
            .value(AddressConfirmationRow::ExpiresAt, to_micros(expires_at))
            .and_where(Expr::col(AddressConfirmationRow::Person).eq(person.stored()));
        Ok(self.database().execute(&update).await? > 0)
    }

    async fn stale_sign_ups(
        &self,
        before: Timestamp,
        limit: u64,
    ) -> Result<Vec<UserId>, StoreError> {
        let mut select = Query::select();
        select
            .column(AddressConfirmationRow::Person)
            .from(AddressConfirmationRow::Table)
            .and_where(Expr::col(AddressConfirmationRow::ExpiresAt).lt(to_micros(before)))
            .order_by(AddressConfirmationRow::ExpiresAt, Order::Asc)
            .limit(limit);
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(|row| row.id::<UserId>(AddressConfirmationRow::Person))
            .collect()
    }

    async fn put_address_confirmation(
        transaction: &mut Transaction,
        confirmation: &AddressConfirmation,
    ) -> Result<(), StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(AddressConfirmationRow::Table)
            .and_where(Expr::col(AddressConfirmationRow::Person).eq(confirmation.person.stored()));
        transaction.execute(&delete).await?;
        let mut insert = Query::insert();
        insert
            .into_table(AddressConfirmationRow::Table)
            .columns(columns())
            .values([
                Expr::val(confirmation.person.stored()),
                Expr::val(confirmation.token_hash.as_bytes().to_vec()),
                Expr::val(to_micros(confirmation.created_at)),
                Expr::val(to_micros(confirmation.expires_at)),
            ])
            .map_err(|_| StoreError::Corrupt("an address confirmation"))?;
        transaction.execute(&insert).await?;
        Ok(())
    }

    async fn use_address_confirmation(
        transaction: &mut Transaction,
        hash: &Digest,
        now: Timestamp,
    ) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(AddressConfirmationRow::Table)
            .and_where(Expr::col(AddressConfirmationRow::TokenHash).eq(hash.as_bytes().to_vec()))
            .and_where(Expr::col(AddressConfirmationRow::ExpiresAt).gt(to_micros(now)));
        Ok(transaction.execute(&delete).await? > 0)
    }

    async fn delete_unconfirmed_person(
        transaction: &mut Transaction,
        person: UserId,
    ) -> Result<bool, StoreError> {
        let mut waiting = Query::delete();
        waiting
            .from_table(AddressConfirmationRow::Table)
            .and_where(Expr::col(AddressConfirmationRow::Person).eq(person.stored()));
        // Only an account that waits is ever deleted: the row is taken first, and nothing else
        // goes when there was none.
        if transaction.execute(&waiting).await? == 0 {
            return Ok(false);
        }
        let mut handles = Query::delete();
        handles
            .from_table(PersonHandleRow::Table)
            .and_where(Expr::col(PersonHandleRow::Person).eq(person.stored()));
        transaction.execute(&handles).await?;
        let mut password = Query::delete();
        password
            .from_table(PasswordLogin::Table)
            .and_where(Expr::col(PasswordLogin::Person).eq(person.stored()));
        transaction.execute(&password).await?;
        let mut account = Query::delete();
        account
            .from_table(PersonRow::Table)
            .and_where(Expr::col(PersonRow::Id).eq(person.stored()));
        transaction.execute(&account).await?;
        Ok(true)
    }
}

#[cfg(test)]
#[path = "tests/confirmations.rs"]
mod tests;
