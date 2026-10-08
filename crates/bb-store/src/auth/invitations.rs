//! Invitations and the setup code of the first start.

use bb_app::auth::InvitationStore;
use bb_core::id::InvitationId;
use bb_core::security_log::Digest;
use bb_core::sign_in::{Invitation, SetupCode};
use jiff::Timestamp;
use sea_query::{Expr, ExprTrait as _, Order, Query};

use crate::db::{StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::paging::after_last;
use crate::schema::{Invitation as InvitationColumns, SetupCodeRow};

use super::AuthStorage;
use super::digest;
use crate::db::Row;
use bb_core::id::UserId;

fn invitation_columns() -> [InvitationColumns; 7] {
    [
        InvitationColumns::Id,
        InvitationColumns::TokenHash,
        InvitationColumns::Email,
        InvitationColumns::InvitedBy,
        InvitationColumns::CreatedAt,
        InvitationColumns::ExpiresAt,
        InvitationColumns::UsedAt,
    ]
}

fn read_invitation(row: &Row) -> Result<Invitation, StoreError> {
    Ok(Invitation {
        id: row.id::<InvitationId>(InvitationColumns::Id)?,
        token_hash: digest(&row.get::<Vec<u8>>(InvitationColumns::TokenHash)?)?,
        email: row.get(InvitationColumns::Email)?,
        invited_by: row.id::<UserId>(InvitationColumns::InvitedBy)?,
        created_at: row.time(InvitationColumns::CreatedAt)?,
        expires_at: row.time(InvitationColumns::ExpiresAt)?,
        used_at: row.optional_time(InvitationColumns::UsedAt)?,
    })
}

impl InvitationStore for AuthStorage {
    async fn setup_code(&self) -> Result<Option<SetupCode>, StoreError> {
        let mut select = Query::select();
        select
            .columns([SetupCodeRow::CodeHash, SetupCodeRow::CreatedAt])
            .from(SetupCodeRow::Table)
            .limit(1);
        let Some(row) = self.database.fetch_optional(&select).await? else {
            return Ok(None);
        };
        Ok(Some(SetupCode {
            code_hash: digest(&row.get::<Vec<u8>>(SetupCodeRow::CodeHash)?)?,
            created_at: row.time(SetupCodeRow::CreatedAt)?,
        }))
    }

    async fn invitation_by_hash(&self, hash: &Digest) -> Result<Option<Invitation>, StoreError> {
        let mut select = Query::select();
        select
            .columns(invitation_columns())
            .from(InvitationColumns::Table)
            .and_where(Expr::col(InvitationColumns::TokenHash).eq(hash.as_bytes().to_vec()));
        self.database
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_invitation)
            .transpose()
    }

    async fn open_invitations(
        &self,
        now: Timestamp,
        after: Option<(Timestamp, InvitationId)>,
        limit: u64,
    ) -> Result<Vec<Invitation>, StoreError> {
        let mut select = Query::select();
        select
            .columns(invitation_columns())
            .from(InvitationColumns::Table)
            .and_where(Expr::col(InvitationColumns::UsedAt).is_null())
            .and_where(Expr::col(InvitationColumns::ExpiresAt).gt(to_micros(now)))
            .order_by(InvitationColumns::CreatedAt, Order::Desc)
            .order_by(InvitationColumns::Id, Order::Asc)
            .limit(limit);
        if let Some((created, id)) = after {
            select.cond_where(after_last(
                || Expr::col(InvitationColumns::CreatedAt),
                || Expr::val(to_micros(created)),
                &Order::Desc,
                InvitationColumns::Id,
                id.stored(),
            ));
        }
        self.database
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_invitation)
            .collect()
    }

    async fn insert_invitation(
        transaction: &mut Transaction,
        invitation: &Invitation,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(InvitationColumns::Table)
            .columns(invitation_columns())
            .values([
                Expr::val(invitation.id.stored()),
                Expr::val(invitation.token_hash.as_bytes().to_vec()),
                Expr::val(invitation.email.as_str()),
                Expr::val(invitation.invited_by.stored()),
                Expr::val(to_micros(invitation.created_at)),
                Expr::val(to_micros(invitation.expires_at)),
                Expr::val(invitation.used_at.map(to_micros)),
            ])
            .map_err(|_| StoreError::Corrupt("an invitation"))?;
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn delete_invitation(
        transaction: &mut Transaction,
        id: InvitationId,
    ) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(InvitationColumns::Table)
            .and_where(Expr::col(InvitationColumns::Id).eq(id.stored()));
        Ok(transaction.execute(&delete).await? > 0)
    }

    async fn use_invitation(
        transaction: &mut Transaction,
        id: InvitationId,
        now: Timestamp,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(InvitationColumns::Table)
            .value(InvitationColumns::UsedAt, to_micros(now))
            .and_where(Expr::col(InvitationColumns::Id).eq(id.stored()))
            .and_where(Expr::col(InvitationColumns::UsedAt).is_null())
            .and_where(Expr::col(InvitationColumns::ExpiresAt).gt(to_micros(now)));
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn put_setup_code(
        transaction: &mut Transaction,
        code: &SetupCode,
    ) -> Result<(), StoreError> {
        let delete = Query::delete().from_table(SetupCodeRow::Table).to_owned();
        transaction.execute(&delete).await?;
        let mut insert = Query::insert();
        insert
            .into_table(SetupCodeRow::Table)
            .columns([SetupCodeRow::CodeHash, SetupCodeRow::CreatedAt])
            .values([
                Expr::val(code.code_hash.as_bytes().to_vec()),
                Expr::val(to_micros(code.created_at)),
            ])
            .map_err(|_| StoreError::Corrupt("a setup code"))?;
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn take_setup_code(
        transaction: &mut Transaction,
        hash: &Digest,
    ) -> Result<bool, StoreError> {
        // The hash is matched here, inside the transaction, so a code that was replaced while the
        // request was running is not used up by a request that typed the old one.
        let mut delete = Query::delete();
        delete
            .from_table(SetupCodeRow::Table)
            .and_where(Expr::col(SetupCodeRow::CodeHash).eq(hash.as_bytes().to_vec()));
        Ok(transaction.execute(&delete).await? > 0)
    }

    async fn clear_setup_code(transaction: &mut Transaction) -> Result<(), StoreError> {
        let delete = Query::delete().from_table(SetupCodeRow::Table).to_owned();
        transaction.execute(&delete).await.map(|_| ())
    }
}
