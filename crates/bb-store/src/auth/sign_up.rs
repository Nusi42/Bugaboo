//! The adapter of the sign-up port (`bb_app::auth::SignUpStore`): the links that signing up on
//! one's own makes (M1-F38, ADR-0033 §3). A link is an invitation with nobody who invited, kept in
//! a table of its own; the invitation use cases read both tables (`invitations.rs`).

use bb_app::auth::SignUpStore;
use bb_core::id::InvitationId;
use bb_core::security_log::Digest;
use bb_core::sign_in::Invitation;
use bb_core::sign_up_invitation::SignUpInvitation;
use jiff::Timestamp;
use sea_query::{Cond, Expr, ExprTrait as _, OnConflict, Query};

use super::AuthStorage;
use super::digest;
use crate::db::{Row, StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::schema::SignUpInvitationRow as Columns;
use crate::schema::SignUpMailRow as Mail;

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
    async fn sign_up_mail_since(&self, since: Timestamp) -> Result<u64, StoreError> {
        let mut select = Query::select();
        select
            .expr_as(
                Expr::col(Mail::Address).count(),
                crate::schema::Computed::Total,
            )
            .from(Mail::Table)
            .and_where(Expr::col(Mail::SentAt).gt(to_micros(since)));
        match self.database.fetch_optional(&select).await? {
            Some(row) => u64::try_from(row.get::<i64>(crate::schema::Computed::Total)?)
                .map_err(|_| StoreError::Corrupt("a count")),
            None => Ok(0),
        }
    }

    async fn claim_sign_up_mail(
        transaction: &mut Transaction,
        address: &Digest,
        invitation: InvitationId,
        since: Timestamp,
        now: Timestamp,
    ) -> Result<bool, StoreError> {
        // One statement decides, so two jobs for one address cannot both pass: the update takes
        // the row only when the last e-mail is old enough (or is this very sign-up's), and the
        // insert takes the address only when no row has it.
        let key = address.as_bytes().to_vec();
        let mut update = Query::update();
        update
            .table(Mail::Table)
            .value(Mail::Invitation, invitation.stored())
            .value(Mail::SentAt, to_micros(now))
            .and_where(Expr::col(Mail::Address).eq(key.clone()))
            .and_where(
                Cond::any()
                    .add(Expr::col(Mail::SentAt).lte(to_micros(since)))
                    .add(Expr::col(Mail::Invitation).eq(invitation.stored()))
                    .into(),
            );
        if transaction.execute(&update).await? > 0 {
            return Ok(true);
        }
        let mut insert = Query::insert();
        insert
            .into_table(Mail::Table)
            .columns([Mail::Address, Mail::Invitation, Mail::SentAt])
            .values([
                Expr::val(key),
                Expr::val(invitation.stored()),
                Expr::val(to_micros(now)),
            ])
            .map_err(|_| StoreError::Corrupt("a sign-up e-mail"))?
            .on_conflict(OnConflict::column(Mail::Address).do_nothing().to_owned());
        Ok(transaction.execute(&insert).await? > 0)
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

    async fn forget_old_sign_up_mail(
        transaction: &mut Transaction,
        before: Timestamp,
    ) -> Result<u64, StoreError> {
        let mut forget = Query::delete();
        forget
            .from_table(Mail::Table)
            .and_where(Expr::col(Mail::SentAt).lt(to_micros(before)));
        transaction.execute(&forget).await
    }
}

#[cfg(test)]
#[path = "../tests/sign_up_invitations.rs"]
mod tests;
