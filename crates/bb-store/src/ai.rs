//! The adapter of the AI setup port (`bb_app::ai::AiStore`): the key of the provider, a stored
//! secret (SEC-16), and the agreement an administrator confirmed (PRIV-08).

use bb_app::ai::AiStore;
use bb_core::ai::{Agreement, Destination, KEY_SECRET};
use bb_core::id::UserId;
use bb_core::technical::TechnicalName;
use jiff::Timestamp;
use sea_query::{Expr, Query};
use zeroize::Zeroizing;

use crate::auth::AuthStorage;
use crate::db::{Database, StoredId as _, Transaction, to_micros};
use crate::error::{SecretProblem, StoreError};
use crate::schema::AiAgreementRow as Columns;

fn key_name() -> Result<TechnicalName, StoreError> {
    TechnicalName::new(KEY_SECRET).map_err(|_| StoreError::Corrupt("the name of a secret"))
}

impl AiStore for AuthStorage {
    async fn ai_key(&self) -> Result<Option<Zeroizing<Vec<u8>>>, StoreError> {
        match self
            .database()
            .get_secret(self.key_ring(), &key_name()?)
            .await
        {
            Ok(secret) => Ok(Some(secret)),
            Err(StoreError::Secret(SecretProblem::Missing)) => Ok(None),
            Err(error) => Err(error),
        }
    }

    async fn put_ai_key(&self, secret: &[u8], at: Timestamp) -> Result<(), StoreError> {
        self.database()
            .put_secret(self.key_ring(), &key_name()?, secret, at)
            .await
    }

    async fn delete_ai_key(&self) -> Result<(), StoreError> {
        self.database()
            .delete_secret(&key_name()?)
            .await
            .map(|_| ())
    }

    async fn ai_agreement(&self) -> Result<Option<Agreement>, StoreError> {
        let mut select = Query::select();
        select
            .columns([
                Columns::Destination,
                Columns::ConfirmedBy,
                Columns::ConfirmedAt,
            ])
            .from(Columns::Table);
        let Some(row) = self.database().fetch_optional(&select).await? else {
            return Ok(None);
        };
        let destination: String = row.get(Columns::Destination)?;
        Ok(Some(Agreement {
            destination: Destination::from_code(&destination)
                .ok_or(StoreError::Corrupt("the destination of an agreement"))?,
            confirmed_by: row.id::<UserId>(Columns::ConfirmedBy)?,
            confirmed_at: row.time(Columns::ConfirmedAt)?,
        }))
    }

    async fn put_ai_agreement(
        transaction: &mut Transaction,
        agreement: &Agreement,
    ) -> Result<(), StoreError> {
        // The table holds one agreement: the old one goes before the new one is written.
        Self::delete_ai_agreement(transaction).await?;
        let mut insert = Query::insert();
        insert
            .into_table(Columns::Table)
            .columns([
                Columns::Destination,
                Columns::ConfirmedBy,
                Columns::ConfirmedAt,
            ])
            .values([
                Expr::val(agreement.destination.code()),
                Expr::val(agreement.confirmed_by.stored()),
                Expr::val(to_micros(agreement.confirmed_at)),
            ])
            .map_err(|_| StoreError::Corrupt("an agreement"))?;
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn delete_ai_agreement(transaction: &mut Transaction) -> Result<(), StoreError> {
        let mut delete = Query::delete();
        delete.from_table(Columns::Table);
        transaction.execute(&delete).await.map(|_| ())
    }

    async fn forget_ai_key(transaction: &mut Transaction) -> Result<(), StoreError> {
        Database::delete_secret_in(transaction, &key_name()?)
            .await
            .map(|_| ())
    }
}
