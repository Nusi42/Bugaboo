//! The second step: the code app, backup codes and passkeys.

use bb_app::auth::SecondStepStore;
use bb_core::id::{PasskeyId, RoleId, UserId};
use bb_core::security_log::Digest;
use bb_core::sign_in::{StoredPasskey, TotpStep, TwoStep};
use jiff::Timestamp;
use sea_query::{Expr, ExprTrait as _, Order, Query};
use zeroize::Zeroizing;

use crate::db::{StoredId, Transaction, to_micros};
use crate::error::{SecretProblem, StoreError};
use crate::schema::{BackupCode, Computed, PasskeyRow, TwoStep as TwoStepColumns};

use super::AuthStorage;
use crate::db::Row;
use bb_core::technical::TechnicalName;

fn passkey_columns() -> [PasskeyRow; 7] {
    [
        PasskeyRow::Id,
        PasskeyRow::Person,
        PasskeyRow::CredentialId,
        PasskeyRow::Data,
        PasskeyRow::Label,
        PasskeyRow::CreatedAt,
        PasskeyRow::LastUsedAt,
    ]
}

fn read_passkey(row: &Row) -> Result<StoredPasskey, StoreError> {
    Ok(StoredPasskey {
        id: row.id::<PasskeyId>(PasskeyRow::Id)?,
        person: row.id::<UserId>(PasskeyRow::Person)?,
        credential_id: row.get(PasskeyRow::CredentialId)?,
        data: row.get(PasskeyRow::Data)?,
        label: row.get(PasskeyRow::Label)?,
        created_at: row.time(PasskeyRow::CreatedAt)?,
        last_used_at: row.optional_time(PasskeyRow::LastUsedAt)?,
    })
}

fn totp_name(person: UserId) -> Result<TechnicalName, StoreError> {
    TechnicalName::new(&format!("{TOTP_SECRET_PREFIX}{person}"))
        .map_err(|_| StoreError::Corrupt("a secret name"))
}

/// The prefix of the name of a code app secret in the secrets table.
const TOTP_SECRET_PREFIX: &str = "totp.";

impl AuthStorage {
    async fn count(&self, select: &sea_query::SelectStatement) -> Result<i64, StoreError> {
        match self.database.fetch_optional(select).await? {
            Some(row) => row.get(Computed::Total),
            None => Ok(0),
        }
    }
}

impl SecondStepStore for AuthStorage {
    async fn two_step(&self, person: UserId) -> Result<Option<TwoStep>, StoreError> {
        let mut select = Query::select();
        select
            .columns([
                TwoStepColumns::Person,
                TwoStepColumns::Confirmed,
                TwoStepColumns::LastStep,
                TwoStepColumns::CreatedAt,
            ])
            .from(TwoStepColumns::Table)
            .and_where(Expr::col(TwoStepColumns::Person).eq(person.stored()));
        let Some(row) = self.database.fetch_optional(&select).await? else {
            return Ok(None);
        };
        Ok(Some(TwoStep {
            person,
            confirmed: row.get(TwoStepColumns::Confirmed)?,
            last_step: TotpStep::new(row.get(TwoStepColumns::LastStep)?),
            created_at: row.time(TwoStepColumns::CreatedAt)?,
        }))
    }

    async fn totp_secret(&self, person: UserId) -> Result<Option<Zeroizing<Vec<u8>>>, StoreError> {
        match self
            .database
            .get_secret(&self.keys, &totp_name(person)?)
            .await
        {
            Ok(secret) => Ok(Some(secret)),
            Err(StoreError::Secret(SecretProblem::Missing)) => Ok(None),
            Err(error) => Err(error),
        }
    }

    async fn backup_codes_left(&self, person: UserId) -> Result<i64, StoreError> {
        let mut select = Query::select();
        select
            .expr_as(Expr::col(BackupCode::CodeHash).count(), Computed::Total)
            .from(BackupCode::Table)
            .and_where(Expr::col(BackupCode::Person).eq(person.stored()))
            .and_where(Expr::col(BackupCode::UsedAt).is_null());
        self.count(&select).await
    }

    async fn passkeys_of(&self, person: UserId) -> Result<Vec<StoredPasskey>, StoreError> {
        let mut select = Query::select();
        select
            .columns(passkey_columns())
            .from(PasskeyRow::Table)
            .and_where(Expr::col(PasskeyRow::Person).eq(person.stored()))
            .order_by(PasskeyRow::CreatedAt, Order::Asc)
            .order_by(PasskeyRow::Id, Order::Asc);
        self.database
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_passkey)
            .collect()
    }

    async fn second_step_required(&self, person: UserId) -> Result<bool, StoreError> {
        self.database.second_step_required(person).await
    }

    async fn has_second_step(&self, person: UserId) -> Result<bool, StoreError> {
        self.database.has_second_step(person).await
    }

    async fn people_without_second_step(&self, role: RoleId) -> Result<Vec<UserId>, StoreError> {
        self.database.people_without_second_step(role).await
    }

    async fn put_totp_secret(
        &self,
        person: UserId,
        secret: &[u8],
        at: Timestamp,
    ) -> Result<(), StoreError> {
        self.database
            .put_secret(&self.keys, &totp_name(person)?, secret, at)
            .await
    }

    async fn delete_totp_secret(&self, person: UserId) -> Result<(), StoreError> {
        self.database
            .delete_secret(&totp_name(person)?)
            .await
            .map(|_| ())
    }

    async fn upsert_two_step(
        transaction: &mut Transaction,
        step: &TwoStep,
    ) -> Result<(), StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(TwoStepColumns::Table)
            .and_where(Expr::col(TwoStepColumns::Person).eq(step.person.stored()));
        transaction.execute(&delete).await?;
        let mut insert = Query::insert();
        insert
            .into_table(TwoStepColumns::Table)
            .columns([
                TwoStepColumns::Person,
                TwoStepColumns::Confirmed,
                TwoStepColumns::LastStep,
                TwoStepColumns::CreatedAt,
            ])
            .values([
                Expr::val(step.person.stored()),
                Expr::val(step.confirmed),
                Expr::val(step.last_step.get()),
                Expr::val(to_micros(step.created_at)),
            ])
            .map_err(|_| StoreError::Corrupt("a second step"))?;
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn confirm_two_step(
        transaction: &mut Transaction,
        person: UserId,
    ) -> Result<(), StoreError> {
        let mut update = Query::update();
        update
            .table(TwoStepColumns::Table)
            .value(TwoStepColumns::Confirmed, true)
            .and_where(Expr::col(TwoStepColumns::Person).eq(person.stored()));
        transaction.execute(&update).await.map(|_| ())
    }

    async fn delete_two_step(
        transaction: &mut Transaction,
        person: UserId,
    ) -> Result<(), StoreError> {
        let mut codes = Query::delete();
        codes
            .from_table(BackupCode::Table)
            .and_where(Expr::col(BackupCode::Person).eq(person.stored()));
        transaction.execute(&codes).await?;
        let mut delete = Query::delete();
        delete
            .from_table(TwoStepColumns::Table)
            .and_where(Expr::col(TwoStepColumns::Person).eq(person.stored()));
        transaction.execute(&delete).await.map(|_| ())
    }

    async fn accept_totp_step(
        transaction: &mut Transaction,
        person: UserId,
        step: TotpStep,
    ) -> Result<bool, StoreError> {
        // One statement checks and writes, so two requests with the same code cannot both pass.
        let mut update = Query::update();
        update
            .table(TwoStepColumns::Table)
            .value(TwoStepColumns::LastStep, step.get())
            .and_where(Expr::col(TwoStepColumns::Person).eq(person.stored()))
            .and_where(Expr::col(TwoStepColumns::LastStep).lt(step.get()));
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn replace_backup_codes(
        transaction: &mut Transaction,
        person: UserId,
        hashes: &[Digest],
    ) -> Result<(), StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(BackupCode::Table)
            .and_where(Expr::col(BackupCode::Person).eq(person.stored()));
        transaction.execute(&delete).await?;
        for hash in hashes {
            let mut insert = Query::insert();
            insert
                .into_table(BackupCode::Table)
                .columns([BackupCode::Person, BackupCode::CodeHash, BackupCode::UsedAt])
                .values([
                    Expr::val(person.stored()),
                    Expr::val(hash.as_bytes().to_vec()),
                    Expr::val(Option::<i64>::None),
                ])
                .map_err(|_| StoreError::Corrupt("a backup code"))?;
            transaction.execute(&insert).await?;
        }
        Ok(())
    }

    async fn use_backup_code(
        transaction: &mut Transaction,
        person: UserId,
        hash: &Digest,
        at: Timestamp,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(BackupCode::Table)
            .value(BackupCode::UsedAt, to_micros(at))
            .and_where(Expr::col(BackupCode::Person).eq(person.stored()))
            .and_where(Expr::col(BackupCode::CodeHash).eq(hash.as_bytes().to_vec()))
            .and_where(Expr::col(BackupCode::UsedAt).is_null());
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn insert_passkey(
        transaction: &mut Transaction,
        passkey: &StoredPasskey,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(PasskeyRow::Table)
            .columns(passkey_columns())
            .values([
                Expr::val(passkey.id.stored()),
                Expr::val(passkey.person.stored()),
                Expr::val(passkey.credential_id.clone()),
                Expr::val(passkey.data.as_str()),
                Expr::val(passkey.label.as_str()),
                Expr::val(to_micros(passkey.created_at)),
                Expr::val(passkey.last_used_at.map(to_micros)),
            ])
            .map_err(|_| StoreError::Corrupt("a passkey"))?;
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn use_passkey(
        transaction: &mut Transaction,
        id: PasskeyId,
        data: Option<&str>,
        at: Timestamp,
    ) -> Result<(), StoreError> {
        let mut update = Query::update();
        update
            .table(PasskeyRow::Table)
            .value(PasskeyRow::LastUsedAt, to_micros(at))
            .and_where(Expr::col(PasskeyRow::Id).eq(id.stored()));
        if let Some(data) = data {
            update.value(PasskeyRow::Data, data);
        }
        transaction.execute(&update).await.map(|_| ())
    }

    async fn delete_passkey(
        transaction: &mut Transaction,
        person: UserId,
        id: PasskeyId,
    ) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(PasskeyRow::Table)
            .and_where(Expr::col(PasskeyRow::Id).eq(id.stored()))
            .and_where(Expr::col(PasskeyRow::Person).eq(person.stored()));
        Ok(transaction.execute(&delete).await? > 0)
    }
}
