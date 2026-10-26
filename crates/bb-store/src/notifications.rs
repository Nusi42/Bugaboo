//! The adapter of the notifications port (`bb_app::notifications::NotificationStore`): the rows the
//! reader builds from the events, the place it has come to, and the e-mail that waits (NTF-01,
//! NTF-02, ADR-0007, ADR-0033).
//!
//! A row is stored once for each person and key (`ON CONFLICT DO NOTHING`), the jobs that send
//! their e-mail are queued in the same transaction, and the place of the reader moves on after
//! that. A reader that stops between the two reads the same events again and finds its rows made,
//! so nothing is doubled and nothing is lost.

use bb_app::jobs::{JOB_ATTEMPTS_MAX, QueuedJob};
use bb_app::notifications::NotificationStore;
use bb_core::event::{DomainEvent, FieldChange};
use bb_core::id::{EntityId, EventId, IssueId, NotificationId, UserId};
use bb_core::mail::PASSWORD_SECRET;
use bb_core::notification::{
    CONSUMER, MailState, Notification, NotificationFilter, NotificationKind, Once,
};
use bb_core::technical::TechnicalName;
use jiff::Timestamp;
use sea_query::{Expr, ExprTrait as _, OnConflict, Order, Query};
use zeroize::Zeroizing;

use crate::auth::AuthStorage;
use crate::db::{Database, Row, StoredId, Transaction, to_micros};
use crate::error::{SecretProblem, StoreError};
use crate::events::{consumer_name, outbox_move};
use crate::jobs::{NewJob, enqueue};
use crate::schema::NotificationRow;

fn password_name() -> Result<TechnicalName, StoreError> {
    TechnicalName::new(PASSWORD_SECRET).map_err(|_| StoreError::Corrupt("the name of a secret"))
}

fn columns() -> [NotificationRow; 12] {
    [
        NotificationRow::Id,
        NotificationRow::Recipient,
        NotificationRow::Kind,
        NotificationRow::Issue,
        NotificationRow::Event,
        NotificationRow::Once,
        NotificationRow::Record,
        NotificationRow::By,
        NotificationRow::At,
        NotificationRow::Changes,
        NotificationRow::InApp,
        NotificationRow::Mail,
    ]
}

fn select() -> sea_query::SelectStatement {
    let mut select = Query::select();
    select
        .columns(columns())
        .column(NotificationRow::ReadAt)
        .from(NotificationRow::Table);
    select
}

fn read_notification(row: &Row) -> Result<Notification, StoreError> {
    let changes: Vec<FieldChange> =
        serde_json::from_str(&row.get::<String>(NotificationRow::Changes)?)
            .map_err(|_| StoreError::Corrupt("the changes of a notification"))?;
    Ok(Notification {
        id: row.id::<NotificationId>(NotificationRow::Id)?,
        recipient: row.id::<UserId>(NotificationRow::Recipient)?,
        kind: NotificationKind::from_code(&row.get::<String>(NotificationRow::Kind)?)
            .ok_or(StoreError::Corrupt("a kind of notification"))?,
        issue: row.optional_id::<IssueId>(NotificationRow::Issue)?,
        event: row.id::<EventId>(NotificationRow::Event)?,
        once: Once::of_key(&row.get::<String>(NotificationRow::Once)?)
            .ok_or(StoreError::Corrupt("the key of a notification"))?,
        record: row.optional_id::<EntityId>(NotificationRow::Record)?,
        by: row.optional_id::<UserId>(NotificationRow::By)?,
        at: row.time(NotificationRow::At)?,
        changes,
        in_app: row.get(NotificationRow::InApp)?,
        mail: MailState::from_code(&row.get::<String>(NotificationRow::Mail)?)
            .ok_or(StoreError::Corrupt("where an e-mail is"))?,
        read_at: row.optional_time(NotificationRow::ReadAt)?,
    })
}

fn ids_of(ids: &[NotificationId]) -> Vec<Vec<u8>> {
    ids.iter().map(|id| id.stored()).collect()
}

/// Stores one notification unless the person has it already. Returns whether it was stored.
async fn insert(transaction: &mut Transaction, row: &Notification) -> Result<bool, StoreError> {
    let mut insert = Query::insert();
    insert
        .into_table(NotificationRow::Table)
        .columns(columns())
        .values([
            Expr::val(row.id.stored()),
            Expr::val(row.recipient.stored()),
            Expr::val(row.kind.code()),
            Expr::val(row.issue.map(StoredId::stored)),
            Expr::val(row.event.stored()),
            Expr::val(row.once.key()),
            Expr::val(row.record.map(StoredId::stored)),
            Expr::val(row.by.map(StoredId::stored)),
            Expr::val(to_micros(row.at)),
            Expr::val(serde_json::to_string(&row.changes)?),
            Expr::val(row.in_app),
            Expr::val(row.mail.code()),
        ])
        .map_err(|_| StoreError::Corrupt("a notification"))?
        .on_conflict(
            OnConflict::columns([NotificationRow::Recipient, NotificationRow::Once])
                .do_nothing()
                .to_owned(),
        );
    Ok(transaction.execute(&insert).await? > 0)
}

impl Database {
    /// Every notification a person has, whatever the filter: what a check of a run of many people
    /// compares with what they may see.
    ///
    /// # Errors
    /// Fails when the database fails or a row is unreadable.
    #[cfg(any(test, feature = "testing"))]
    pub async fn stored_notifications(
        &self,
        recipient: UserId,
    ) -> Result<Vec<Notification>, StoreError> {
        let mut select = select();
        select
            .and_where(Expr::col(NotificationRow::Recipient).eq(recipient.stored()))
            .order_by(NotificationRow::Id, Order::Asc);
        self.fetch_all(&select)
            .await?
            .iter()
            .map(read_notification)
            .collect()
    }
}

impl NotificationStore for AuthStorage {
    async fn notification_events(&self, limit: u64) -> Result<Vec<(i64, DomainEvent)>, StoreError> {
        self.database().outbox_events(CONSUMER, limit).await
    }

    async fn store_notifications(
        &self,
        rows: &[Notification],
        jobs: &[QueuedJob],
        position: i64,
        now: Timestamp,
    ) -> Result<(), StoreError> {
        // The rows, the jobs and the place of the reader are one step: a reader that stops reads
        // the same events again, and the rows it made once are not made twice.
        let mut transaction = self.database().begin().await?;
        for row in rows {
            insert(&mut transaction, row).await?;
        }
        for queued in jobs {
            let job = NewJob {
                kind: queued.kind.clone(),
                payload: queued.payload.clone(),
                run_at: queued.run_at,
                max_attempts: JOB_ATTEMPTS_MAX,
            };
            enqueue(&mut transaction, queued.id, &job, now).await?;
        }
        outbox_move(&mut transaction, &consumer_name(CONSUMER)?, position, now).await?;
        transaction.commit().await
    }

    async fn notifications_of(
        &self,
        recipient: UserId,
        filter: NotificationFilter,
        before: Option<NotificationId>,
        limit: u64,
    ) -> Result<Vec<Notification>, StoreError> {
        let mut select = select();
        select
            .and_where(Expr::col(NotificationRow::Recipient).eq(recipient.stored()))
            .and_where(Expr::col(NotificationRow::InApp).eq(true))
            .order_by(NotificationRow::Id, Order::Desc)
            .limit(limit);
        match filter {
            NotificationFilter::All => {}
            NotificationFilter::Unread => {
                select.and_where(Expr::col(NotificationRow::ReadAt).is_null());
            }
            NotificationFilter::Mentions => {
                select.and_where(
                    Expr::col(NotificationRow::Kind).eq(NotificationKind::Mentioned.code()),
                );
            }
        }
        if let Some(before) = before {
            select.and_where(Expr::col(NotificationRow::Id).lt(before.stored()));
        }
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_notification)
            .collect()
    }

    async fn waiting_mail(
        &self,
        recipient: UserId,
        issue: Option<IssueId>,
        until: Timestamp,
        limit: u64,
    ) -> Result<Vec<Notification>, StoreError> {
        let mut select = select();
        select
            .and_where(Expr::col(NotificationRow::Recipient).eq(recipient.stored()))
            .and_where(Expr::col(NotificationRow::Mail).eq(MailState::Waiting.code()))
            .and_where(Expr::col(NotificationRow::At).lte(to_micros(until)))
            .order_by(NotificationRow::At, Order::Asc)
            .order_by(NotificationRow::Id, Order::Asc)
            .limit(limit);
        match issue {
            Some(issue) => select.and_where(Expr::col(NotificationRow::Issue).eq(issue.stored())),
            None => select.and_where(Expr::col(NotificationRow::Issue).is_null()),
        };
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_notification)
            .collect()
    }

    async fn set_mail_state(
        &self,
        ids: &[NotificationId],
        state: MailState,
    ) -> Result<(), StoreError> {
        if ids.is_empty() {
            return Ok(());
        }
        let mut update = Query::update();
        update
            .table(NotificationRow::Table)
            .value(NotificationRow::Mail, state.code())
            .and_where(Expr::col(NotificationRow::Id).is_in(ids_of(ids)));
        self.database().execute(&update).await?;
        Ok(())
    }

    async fn finish_mail(
        &self,
        recipient: UserId,
        issue: Option<IssueId>,
        until: Timestamp,
        state: MailState,
    ) -> Result<u64, StoreError> {
        let mut update = Query::update();
        update
            .table(NotificationRow::Table)
            .value(NotificationRow::Mail, state.code())
            .and_where(Expr::col(NotificationRow::Recipient).eq(recipient.stored()))
            .and_where(Expr::col(NotificationRow::Mail).eq(MailState::Waiting.code()))
            .and_where(Expr::col(NotificationRow::At).lte(to_micros(until)));
        match issue {
            Some(issue) => update.and_where(Expr::col(NotificationRow::Issue).eq(issue.stored())),
            None => update.and_where(Expr::col(NotificationRow::Issue).is_null()),
        };
        self.database().execute(&update).await
    }

    async fn mail_password(&self) -> Result<Option<Zeroizing<Vec<u8>>>, StoreError> {
        match self
            .database()
            .get_secret(self.key_ring(), &password_name()?)
            .await
        {
            Ok(secret) => Ok(Some(secret)),
            Err(StoreError::Secret(SecretProblem::Missing)) => Ok(None),
            Err(error) => Err(error),
        }
    }

    async fn put_mail_password(&self, secret: &[u8], at: Timestamp) -> Result<(), StoreError> {
        self.database()
            .put_secret(self.key_ring(), &password_name()?, secret, at)
            .await
    }

    async fn forget_mail_password(transaction: &mut Transaction) -> Result<(), StoreError> {
        Database::delete_secret_in(transaction, &password_name()?)
            .await
            .map(|_| ())
    }

    async fn delete_mail_password(&self) -> Result<(), StoreError> {
        self.database()
            .delete_secret(&password_name()?)
            .await
            .map(|_| ())
    }

    async fn purge_notifications(&self, before: Timestamp) -> Result<u64, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(NotificationRow::Table)
            .and_where(Expr::col(NotificationRow::At).lt(to_micros(before)));
        self.database().execute(&delete).await
    }

    async fn mark_notifications_read(
        transaction: &mut Transaction,
        recipient: UserId,
        ids: &[NotificationId],
        at: Timestamp,
    ) -> Result<u64, StoreError> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut update = Query::update();
        update
            .table(NotificationRow::Table)
            .value(NotificationRow::ReadAt, to_micros(at))
            .and_where(Expr::col(NotificationRow::Recipient).eq(recipient.stored()))
            .and_where(Expr::col(NotificationRow::Id).is_in(ids_of(ids)))
            .and_where(Expr::col(NotificationRow::ReadAt).is_null());
        transaction.execute(&update).await
    }

    async fn mark_notifications_read_up_to(
        transaction: &mut Transaction,
        recipient: UserId,
        up_to: NotificationId,
        at: Timestamp,
    ) -> Result<u64, StoreError> {
        let mut update = Query::update();
        update
            .table(NotificationRow::Table)
            .value(NotificationRow::ReadAt, to_micros(at))
            .and_where(Expr::col(NotificationRow::Recipient).eq(recipient.stored()))
            .and_where(Expr::col(NotificationRow::Id).lte(up_to.stored()))
            .and_where(Expr::col(NotificationRow::InApp).eq(true))
            .and_where(Expr::col(NotificationRow::ReadAt).is_null());
        transaction.execute(&update).await
    }
}

#[cfg(test)]
#[path = "tests/notifications.rs"]
mod tests;
