//! Sign-in sessions.

use bb_app::auth::{SessionRecord, SessionStore};
use bb_core::id::{SessionId, UserId};
use bb_core::network::NetworkPrefix;
use bb_core::security_log::Digest;
use bb_core::sign_in::{SessionStage, SignInSession};
use jiff::Timestamp;
use sea_query::{Expr, ExprTrait as _, Order, Query};

use crate::db::{StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::schema::SignInSession as SessionColumns;

use std::collections::HashSet;
use std::sync::{Arc, Mutex, PoisonError};

use super::AuthStorage;
use super::digest;
use crate::db::Row;

/// The sessions that are being marked as used right now.
#[derive(Debug, Default)]
pub(super) struct Touches(Mutex<HashSet<SessionId>>);

impl Touches {
    /// Takes the place of the marking of this session; `false` when one is on its way already.
    fn begin(&self, id: SessionId) -> bool {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(id)
    }

    fn end(&self, id: SessionId) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&id);
    }

    /// How many sessions are being marked.
    #[cfg(any(test, feature = "testing"))]
    fn pending(&self) -> usize {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).len()
    }
}

impl AuthStorage {
    /// How many sessions are being marked as used (the tests wait for none, so that a clock they
    /// move on does not pass a mark that is still on its way).
    #[cfg(any(test, feature = "testing"))]
    #[must_use]
    pub fn touches_pending(&self) -> usize {
        self.touches.pending()
    }
}

fn session_columns() -> [SessionColumns; 8] {
    [
        SessionColumns::Id,
        SessionColumns::Person,
        SessionColumns::TokenHash,
        SessionColumns::Stage,
        SessionColumns::CreatedAt,
        SessionColumns::LastSeenAt,
        SessionColumns::Network,
        SessionColumns::Device,
    ]
}

fn read_session(row: &Row) -> Result<SignInSession, StoreError> {
    Ok(SignInSession {
        id: row.id::<SessionId>(SessionColumns::Id)?,
        person: row.id::<UserId>(SessionColumns::Person)?,
        token_hash: digest(&row.get::<Vec<u8>>(SessionColumns::TokenHash)?)?,
        stage: SessionStage::from_code(&row.get::<String>(SessionColumns::Stage)?)
            .ok_or(StoreError::Corrupt("a session stage"))?,
        created_at: row.time(SessionColumns::CreatedAt)?,
        last_seen_at: row.time(SessionColumns::LastSeenAt)?,
        network: row
            .get::<Option<String>>(SessionColumns::Network)?
            .map(|text| NetworkPrefix::from_stored(&text).ok_or(StoreError::Corrupt("a network")))
            .transpose()?,
        device: row.get(SessionColumns::Device)?,
    })
}

impl SessionStore for AuthStorage {
    async fn session_by_hash(&self, hash: &Digest) -> Result<Option<SessionRecord>, StoreError> {
        let mut select = Query::select();
        select
            .columns(session_columns())
            .from(SessionColumns::Table)
            .and_where(Expr::col(SessionColumns::TokenHash).eq(hash.as_bytes().to_vec()));
        let Some(row) = self.database.fetch_optional(&select).await? else {
            return Ok(None);
        };
        let session = read_session(&row)?;
        let Some(person) = self.person_by_id(session.person).await? else {
            return Ok(None);
        };
        Ok(Some(SessionRecord { session, person }))
    }

    async fn sessions_of(&self, person: UserId) -> Result<Vec<SignInSession>, StoreError> {
        let mut select = Query::select();
        select
            .columns(session_columns())
            .from(SessionColumns::Table)
            .and_where(Expr::col(SessionColumns::Person).eq(person.stored()))
            .order_by(SessionColumns::CreatedAt, Order::Desc)
            .order_by(SessionColumns::Id, Order::Asc);
        self.database
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_session)
            .collect()
    }

    /// Marks the session as used without making the request wait for it: the write runs on its own,
    /// and only one runs for a session at a time. A mark that is not made is lost without harm, the
    /// next request makes it (M7-F222).
    async fn touch_session(&self, id: SessionId, at: Timestamp) -> Result<(), StoreError> {
        if !self.touches.begin(id) {
            return Ok(());
        }
        let database = self.database.clone();
        let touches = Arc::clone(&self.touches);
        tokio::spawn(async move {
            let mut update = Query::update();
            update
                .table(SessionColumns::Table)
                .value(SessionColumns::LastSeenAt, to_micros(at))
                .and_where(Expr::col(SessionColumns::Id).eq(id.stored()))
                .and_where(Expr::col(SessionColumns::LastSeenAt).lt(to_micros(at)));
            database.execute(&update).await.ok();
            touches.end(id);
        });
        Ok(())
    }

    async fn insert_session(
        transaction: &mut Transaction,
        session: &SignInSession,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(SessionColumns::Table)
            .columns(session_columns())
            .values([
                Expr::val(session.id.stored()),
                Expr::val(session.person.stored()),
                Expr::val(session.token_hash.as_bytes().to_vec()),
                Expr::val(session.stage.code()),
                Expr::val(to_micros(session.created_at)),
                Expr::val(to_micros(session.last_seen_at)),
                Expr::val(session.network.as_ref().map(NetworkPrefix::as_str)),
                Expr::val(session.device.as_str()),
            ])
            .map_err(|_| StoreError::Corrupt("a session"))?;
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn delete_session(
        transaction: &mut Transaction,
        id: SessionId,
    ) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(SessionColumns::Table)
            .and_where(Expr::col(SessionColumns::Id).eq(id.stored()));
        Ok(transaction.execute(&delete).await? > 0)
    }

    async fn delete_sessions_of(
        transaction: &mut Transaction,
        person: UserId,
        except: Option<SessionId>,
    ) -> Result<u64, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(SessionColumns::Table)
            .and_where(Expr::col(SessionColumns::Person).eq(person.stored()));
        if let Some(except) = except {
            delete.and_where(Expr::col(SessionColumns::Id).ne(except.stored()));
        }
        transaction.execute(&delete).await
    }

    async fn purge_sessions(
        transaction: &mut Transaction,
        idle_before: Timestamp,
        absolute_before: Timestamp,
        pending_before: Timestamp,
    ) -> Result<u64, StoreError> {
        let active = Expr::col(SessionColumns::Stage)
            .eq(SessionStage::Active.code())
            .and(
                Expr::col(SessionColumns::LastSeenAt)
                    .lt(to_micros(idle_before))
                    .or(Expr::col(SessionColumns::CreatedAt).lt(to_micros(absolute_before))),
            );
        let pending = Expr::col(SessionColumns::Stage)
            .eq(SessionStage::SecondStepPending.code())
            .and(Expr::col(SessionColumns::CreatedAt).lt(to_micros(pending_before)));
        let mut delete = Query::delete();
        delete
            .from_table(SessionColumns::Table)
            .and_where(active.or(pending));
        transaction.execute(&delete).await
    }
}
