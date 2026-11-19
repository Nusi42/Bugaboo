//! The adapter of the port of corrections (`bb_app::people::CorrectionStore`, PRIV-06): a person's
//! name and address, and the changes of addresses that wait for their link.

use bb_app::people::CorrectionStore;
use bb_core::e_mail_change::EmailChange;
use bb_core::id::UserId;
use bb_core::names::name_key;
use bb_core::security_log::Digest;
use jiff::Timestamp;
use sea_query::{Cond, Expr, ExprTrait as _, Query};

use crate::AuthStorage;
use crate::auth::digest;
use crate::db::{Row, StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::keeping::delete_limited;
use crate::schema::{EmailChangeRow as Columns, PersonRow};

fn columns() -> [Columns; 5] {
    [
        Columns::Person,
        Columns::TokenDigest,
        Columns::NewEmail,
        Columns::AskedAt,
        Columns::ExpiresAt,
    ]
}

fn read(row: &Row) -> Result<EmailChange, StoreError> {
    Ok(EmailChange {
        person: row.id::<UserId>(Columns::Person)?,
        token_digest: digest(&row.get::<Vec<u8>>(Columns::TokenDigest)?)?,
        new_email: row.get(Columns::NewEmail)?,
        asked_at: row.time(Columns::AskedAt)?,
        expires_at: row.time(Columns::ExpiresAt)?,
    })
}

impl CorrectionStore for AuthStorage {
    async fn e_mail_change_by_digest(
        &self,
        digest: &Digest,
    ) -> Result<Option<EmailChange>, StoreError> {
        let mut select = Query::select();
        select
            .columns(columns())
            .from(Columns::Table)
            .and_where(Expr::col(Columns::TokenDigest).eq(digest.as_bytes().to_vec()));
        self.database()
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read)
            .transpose()
    }

    async fn set_person_name(
        transaction: &mut Transaction,
        person: UserId,
        name: &str,
    ) -> Result<(), StoreError> {
        let mut update = Query::update();
        update
            .table(PersonRow::Table)
            .value(PersonRow::DisplayName, name)
            .value(PersonRow::NameKey, name_key(name))
            .and_where(Expr::col(PersonRow::Id).eq(person.stored()));
        transaction.execute(&update).await.map(|_| ())
    }

    async fn set_person_email(
        transaction: &mut Transaction,
        person: UserId,
        email: &str,
    ) -> Result<(), StoreError> {
        let mut update = Query::update();
        update
            .table(PersonRow::Table)
            .value(PersonRow::Email, email)
            .and_where(Expr::col(PersonRow::Id).eq(person.stored()));
        transaction.execute(&update).await.map(|_| ())
    }

    async fn put_e_mail_change(
        transaction: &mut Transaction,
        change: &EmailChange,
    ) -> Result<(), StoreError> {
        Self::drop_e_mail_change(transaction, change.person).await?;
        let mut insert = Query::insert();
        insert
            .into_table(Columns::Table)
            .columns(columns())
            .values([
                Expr::val(change.person.stored()),
                Expr::val(change.token_digest.as_bytes().to_vec()),
                Expr::val(change.new_email.as_str()),
                Expr::val(to_micros(change.asked_at)),
                Expr::val(to_micros(change.expires_at)),
            ])
            .map_err(|_| StoreError::Corrupt("a change of an address"))?;
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn take_e_mail_change(
        transaction: &mut Transaction,
        digest: &Digest,
    ) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(Columns::Table)
            .and_where(Expr::col(Columns::TokenDigest).eq(digest.as_bytes().to_vec()));
        Ok(transaction.execute(&delete).await? > 0)
    }

    async fn drop_e_mail_change(
        transaction: &mut Transaction,
        person: UserId,
    ) -> Result<(), StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(Columns::Table)
            .and_where(Expr::col(Columns::Person).eq(person.stored()));
        transaction.execute(&delete).await.map(|_| ())
    }

    async fn delete_old_e_mail_changes(
        transaction: &mut Transaction,
        now: Timestamp,
        limit: u64,
    ) -> Result<u64, StoreError> {
        let delete = delete_limited(
            Columns::Table,
            &[Columns::Person],
            Cond::all().add(Expr::col(Columns::ExpiresAt).lte(to_micros(now))),
            limit,
        );
        transaction.execute(&delete).await
    }
}

#[cfg(test)]
#[path = "tests/corrections.rs"]
mod tests;
