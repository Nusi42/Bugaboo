//! The adapter of the sign-up port (`bb_app::auth::SignUpStore`): the links that signing up on
//! one's own makes (M1-F38, ADR-0033 §3). A link is an invitation with nobody who invited, kept in
//! a table of its own; the invitation use cases read both tables (`invitations.rs`).

use bb_app::auth::SignUpStore;
use bb_core::id::InvitationId;
use bb_core::security_log::Digest;
use bb_core::sign_in::Invitation;
use bb_core::sign_up_invitation::SignUpInvitation;
use jiff::Timestamp;
use sea_query::{Cond, Expr, ExprTrait as _, Order, Query};

use super::AuthStorage;
use super::digest;
use crate::db::{Row, StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::schema::SignUpInvitationRow as Columns;

fn columns() -> [Columns; 6] {
    [
        Columns::Id,
        Columns::TokenHash,
        Columns::Email,
        Columns::CreatedAt,
        Columns::ExpiresAt,
        Columns::UsedAt,
    ]
}

fn read(row: &Row) -> Result<SignUpInvitation, StoreError> {
    Ok(SignUpInvitation {
        id: row.id::<InvitationId>(Columns::Id)?,
        token_hash: digest(&row.get::<Vec<u8>>(Columns::TokenHash)?)?,
        email: row.get(Columns::Email)?,
        created_at: row.time(Columns::CreatedAt)?,
        expires_at: row.time(Columns::ExpiresAt)?,
        used_at: row.optional_time(Columns::UsedAt)?,
    })
}

impl AuthStorage {
    /// The link with the secret that has this hash, as an invitation.
    pub(super) async fn sign_up_link_by_hash(
        &self,
        hash: &Digest,
    ) -> Result<Option<Invitation>, StoreError> {
        let mut select = Query::select();
        select
            .columns(columns())
            .from(Columns::Table)
            .and_where(Expr::col(Columns::TokenHash).eq(hash.as_bytes().to_vec()));
        Ok(self
            .database
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read)
            .transpose()?
            .map(Invitation::from))
    }

    /// Uses up the link with this ID if it is unused and has not run out; `false` otherwise.
    pub(super) async fn use_sign_up_link(
        transaction: &mut Transaction,
        id: InvitationId,
        now: Timestamp,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(Columns::Table)
            .value(Columns::UsedAt, to_micros(now))
            .and_where(Expr::col(Columns::Id).eq(id.stored()))
            .and_where(Expr::col(Columns::UsedAt).is_null())
            .and_where(Expr::col(Columns::ExpiresAt).gt(to_micros(now)));
        Ok(transaction.execute(&update).await? > 0)
    }
}

impl SignUpStore for AuthStorage {
    async fn open_sign_up_for(
        &self,
        email: &str,
        now: Timestamp,
    ) -> Result<Option<SignUpInvitation>, StoreError> {
        let mut select = Query::select();
        select
            .columns(columns())
            .from(Columns::Table)
            .and_where(Expr::col(Columns::Email).eq(email))
            .and_where(Expr::col(Columns::UsedAt).is_null())
            .and_where(Expr::col(Columns::ExpiresAt).gt(to_micros(now)))
            .order_by(Columns::CreatedAt, Order::Desc)
            .limit(1);
        self.database
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read)
            .transpose()
    }

    async fn put_sign_up_invitation(
        transaction: &mut Transaction,
        invitation: &SignUpInvitation,
    ) -> Result<(), StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(Columns::Table)
            .and_where(Expr::col(Columns::Id).eq(invitation.id.stored()));
        transaction.execute(&delete).await?;
        let mut insert = Query::insert();
        insert
            .into_table(Columns::Table)
            .columns(columns())
            .values([
                Expr::val(invitation.id.stored()),
                Expr::val(invitation.token_hash.as_bytes().to_vec()),
                Expr::val(invitation.email.as_str()),
                Expr::val(to_micros(invitation.created_at)),
                Expr::val(to_micros(invitation.expires_at)),
                Expr::val(invitation.used_at.map(to_micros)),
            ])
            .map_err(|_| StoreError::Corrupt("a sign-up invitation"))?;
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn delete_old_sign_up_invitations(
        transaction: &mut Transaction,
        before: Timestamp,
        now: Timestamp,
    ) -> Result<u64, StoreError> {
        let finished = Cond::any()
            .add(Expr::col(Columns::UsedAt).is_not_null())
            .add(Expr::col(Columns::ExpiresAt).lte(to_micros(now)));
        let mut delete = Query::delete();
        delete
            .from_table(Columns::Table)
            .and_where(Expr::col(Columns::CreatedAt).lt(to_micros(before)))
            .cond_where(finished);
        transaction.execute(&delete).await
    }
}

#[cfg(test)]
#[path = "../tests/sign_up_invitations.rs"]
mod tests;
