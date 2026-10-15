//! The adapter of the editing port (`bb_app::editing::EditStore`): the lease on a thing being
//! edited, the private drafts of people, and the notes for the person who was editing (ADR-0019).
//!
//! The lease, the heartbeat and the draft are bookkeeping, so they run on their own, each in one
//! transaction. Taking a lock applies the decision of `bb_core::edit::Taking` with a conditional
//! write (insert if no row, update only the row that was read), so two editors that ask at the same
//! moment on PostgreSQL never both get it. What other people can learn from (*Edit anyway*, a
//! note, a released lock) runs inside the transaction of a change, with its event.

use bb_app::editing::EditStore;
use bb_core::edit::{Draft, DraftTarget, EditLock, EditNotice, EditWhat, Hold, NoticeKind, Taking};
use bb_core::event::{DomainEvent, EntityKind, EntityRef};
use bb_core::id::{EditorId, EntityId, LockToken, UserId};
use bb_core::technical::TechnicalName;
use jiff::Timestamp;
use sea_query::{Condition, Expr, ExprTrait as _, OnConflict, Order, Query};

use crate::auth::AuthStorage;
use crate::db::{Database, Reader, Row, StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::schema::{EditDraftRow as Drafts, EditLockRow as Locks, EditNoticeRow as Notices};

/// How often a take starts over when another writer was faster; then the caller may try again.
const TAKE_ATTEMPTS: usize = 3;

/// The columns of a lock row, in the order `insert_lock` writes them: one list for reading and
/// writing.
const LOCK_COLUMNS: [Locks; 8] = [
    Locks::RecordKind,
    Locks::RecordId,
    Locks::Part,
    Locks::Holder,
    Locks::Editor,
    Locks::Token,
    Locks::TakenAt,
    Locks::ExpiresAt,
];

/// The rows of the one thing a lock is on.
fn lock_of(what: &EditWhat) -> Condition {
    Condition::all()
        .add(Expr::col(Locks::RecordKind).eq(what.record.kind.as_str()))
        .add(Expr::col(Locks::RecordId).eq(what.record.id.stored()))
        .add(Expr::col(Locks::Part).eq(what.part.as_str()))
}

fn read_lock(row: &Row) -> Result<EditLock, StoreError> {
    Ok(EditLock {
        record: EntityRef {
            kind: EntityKind::new(&row.get::<String>(Locks::RecordKind)?)
                .map_err(|_| StoreError::Corrupt("a kind of record"))?,
            id: row.id::<EntityId>(Locks::RecordId)?,
        },
        part: TechnicalName::new(&row.get::<String>(Locks::Part)?)
            .map_err(|_| StoreError::Corrupt("the part of a record"))?,
        holder: row.id::<UserId>(Locks::Holder)?,
        editor: row.id::<EditorId>(Locks::Editor)?,
        token: row.id::<LockToken>(Locks::Token)?,
        taken_at: row.time(Locks::TakenAt)?,
        expires_at: row.time(Locks::ExpiresAt)?,
    })
}

fn lock_select() -> sea_query::SelectStatement {
    let mut select = Query::select();
    select.columns(LOCK_COLUMNS).from(Locks::Table);
    select
}

/// The lock on a thing, if a row is stored (it may have expired).
async fn lock_in(
    reader: &mut impl Reader,
    what: &EditWhat,
) -> Result<Option<EditLock>, StoreError> {
    let mut select = lock_select();
    select.cond_where(lock_of(what));
    reader
        .fetch_optional(&select)
        .await?
        .as_ref()
        .map(read_lock)
        .transpose()
}

impl Database {
    /// Every lock row that is stored, an expired one too: the check after a run looks at what the
    /// leases left (TEST-CONCURRENT).
    ///
    /// # Errors
    /// Fails when the database fails or a row is unreadable.
    #[cfg(any(test, feature = "testing"))]
    pub async fn edit_lock_rows(&self) -> Result<Vec<EditLock>, StoreError> {
        self.fetch_all(&lock_select())
            .await?
            .iter()
            .map(read_lock)
            .collect()
    }

    /// The draft of a person for a target, however old it is.
    ///
    /// # Errors
    /// Fails when the database fails or the row is unreadable.
    #[cfg(any(test, feature = "testing"))]
    pub async fn stored_draft(
        &self,
        author: UserId,
        target: DraftTarget,
    ) -> Result<Option<Draft>, StoreError> {
        draft_in(&mut &*self, author, target, Timestamp::UNIX_EPOCH).await
    }
}

/// Stores the lock if the thing has no row; `false` when somebody was faster.
async fn insert_lock(transaction: &mut Transaction, lock: &EditLock) -> Result<bool, StoreError> {
    let mut insert = Query::insert();
    insert
        .into_table(Locks::Table)
        .columns(LOCK_COLUMNS)
        .values([
            Expr::val(lock.record.kind.as_str()),
            Expr::val(lock.record.id.stored()),
            Expr::val(lock.part.as_str()),
            Expr::val(lock.holder.stored()),
            Expr::val(lock.editor.stored()),
            Expr::val(lock.token.stored()),
            Expr::val(to_micros(lock.taken_at)),
            Expr::val(to_micros(lock.expires_at)),
        ])
        .map_err(|_| StoreError::Corrupt("a lock"))?
        .on_conflict(
            OnConflict::columns([Locks::RecordKind, Locks::RecordId, Locks::Part])
                .do_nothing()
                .to_owned(),
        );
    Ok(transaction.execute(&insert).await? > 0)
}

/// Gives the row of the thing to `lock`, only if the row still has the token that was read.
async fn update_lock(
    transaction: &mut Transaction,
    lock: &EditLock,
    read_token: LockToken,
) -> Result<bool, StoreError> {
    let mut update = Query::update();
    update
        .table(Locks::Table)
        .value(Locks::Holder, lock.holder.stored())
        .value(Locks::Editor, lock.editor.stored())
        .value(Locks::Token, lock.token.stored())
        .value(Locks::TakenAt, to_micros(lock.taken_at))
        .value(Locks::ExpiresAt, to_micros(lock.expires_at))
        .cond_where(lock_of(&lock.what()))
        .and_where(Expr::col(Locks::Token).eq(read_token.stored()));
    Ok(transaction.execute(&update).await? > 0)
}

async fn delete_lock(
    transaction: &mut Transaction,
    what: &EditWhat,
    hold: Option<Hold>,
) -> Result<bool, StoreError> {
    let mut delete = Query::delete();
    delete.from_table(Locks::Table).cond_where(lock_of(what));
    if let Some(hold) = hold {
        delete
            .and_where(Expr::col(Locks::Token).eq(hold.token.stored()))
            .and_where(Expr::col(Locks::Holder).eq(hold.holder.stored()));
    }
    Ok(transaction.execute(&delete).await? > 0)
}

/// Forgets the leases that ran out: no lock is kept after it ends.
async fn delete_expired_locks(
    transaction: &mut Transaction,
    now: Timestamp,
) -> Result<(), StoreError> {
    let mut delete = Query::delete();
    delete
        .from_table(Locks::Table)
        .and_where(Expr::col(Locks::ExpiresAt).lte(to_micros(now)));
    transaction.execute(&delete).await.map(|_| ())
}

fn draft_of(author: UserId, target: &DraftTarget) -> Condition {
    Condition::all()
        .add(Expr::col(Drafts::Author).eq(author.stored()))
        .add(Expr::col(Drafts::Target).eq(target.key()))
}

async fn draft_in(
    reader: &mut impl Reader,
    author: UserId,
    target: DraftTarget,
    not_before: Timestamp,
) -> Result<Option<Draft>, StoreError> {
    let mut select = Query::select();
    select
        .columns([Drafts::Text, Drafts::BaseVersion, Drafts::SavedAt])
        .from(Drafts::Table)
        .cond_where(draft_of(author, &target))
        .and_where(Expr::col(Drafts::SavedAt).gte(to_micros(not_before)));
    reader
        .fetch_optional(&select)
        .await?
        .map(|row| {
            Ok(Draft {
                author,
                target: target.clone(),
                text: row.get(Drafts::Text)?,
                base_version: row.get(Drafts::BaseVersion)?,
                saved_at: row.time(Drafts::SavedAt)?,
            })
        })
        .transpose()
}

async fn delete_draft(
    transaction: &mut Transaction,
    author: UserId,
    target: &DraftTarget,
) -> Result<bool, StoreError> {
    let mut delete = Query::delete();
    delete
        .from_table(Drafts::Table)
        .cond_where(draft_of(author, target));
    Ok(transaction.execute(&delete).await? > 0)
}

fn notice_of(recipient: UserId, what: &EditWhat) -> Condition {
    Condition::all()
        .add(Expr::col(Notices::Recipient).eq(recipient.stored()))
        .add(Expr::col(Notices::RecordKind).eq(what.record.kind.as_str()))
        .add(Expr::col(Notices::RecordId).eq(what.record.id.stored()))
        .add(Expr::col(Notices::Part).eq(what.part.as_str()))
}

fn read_notice(row: &Row, recipient: UserId, what: &EditWhat) -> Result<EditNotice, StoreError> {
    Ok(EditNotice {
        recipient,
        record: what.record.clone(),
        part: what.part.clone(),
        kind: NoticeKind::from_code(&row.get::<String>(Notices::Kind)?)
            .ok_or(StoreError::Corrupt("a kind of note"))?,
        from: row.id::<UserId>(Notices::FromPerson)?,
        at: row.time(Notices::At)?,
    })
}

impl EditStore for AuthStorage {
    async fn take_edit_lock(
        &self,
        wanted: &EditLock,
        now: Timestamp,
    ) -> Result<Taking, StoreError> {
        let what = wanted.what();
        for _ in 0..TAKE_ATTEMPTS {
            let mut transaction = self.database().begin().await?;
            delete_expired_locks(&mut transaction, now).await?;
            let existing = lock_in(&mut transaction, &what).await?;
            let taking = Taking::decide(existing.as_ref(), wanted, now);
            let won = match (&taking, &existing) {
                (Taking::Held(_), _) => true,
                (Taking::Granted(lock), None) => insert_lock(&mut transaction, lock).await?,
                (Taking::Granted(lock), Some(old)) => {
                    update_lock(&mut transaction, lock, old.token).await?
                }
            };
            if won {
                transaction.commit().await?;
                return Ok(taking);
            }
            // Another writer was faster: look again.
        }
        Err(StoreError::Contended)
    }

    async fn renew_edit_lock(
        &self,
        what: &EditWhat,
        hold: Hold,
        now: Timestamp,
        ends: Timestamp,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(Locks::Table)
            .value(Locks::ExpiresAt, to_micros(ends))
            .cond_where(lock_of(what))
            .and_where(Expr::col(Locks::Token).eq(hold.token.stored()))
            .and_where(Expr::col(Locks::Holder).eq(hold.holder.stored()))
            .and_where(Expr::col(Locks::ExpiresAt).gt(to_micros(now)));
        Ok(self.database().execute(&update).await? > 0)
    }

    async fn release_edit_lock(&self, what: &EditWhat, hold: Hold) -> Result<bool, StoreError> {
        let mut transaction = self.database().begin().await?;
        let released = delete_lock(&mut transaction, what, Some(hold)).await?;
        transaction.commit().await?;
        Ok(released)
    }

    async fn save_draft(&self, draft: &Draft) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(Drafts::Table)
            .columns([
                Drafts::Author,
                Drafts::Target,
                Drafts::Text,
                Drafts::BaseVersion,
                Drafts::SavedAt,
            ])
            .values([
                Expr::val(draft.author.stored()),
                Expr::val(draft.target.key()),
                Expr::val(draft.text.as_str()),
                Expr::val(draft.base_version),
                Expr::val(to_micros(draft.saved_at)),
            ])
            .map_err(|_| StoreError::Corrupt("a draft"))?
            .on_conflict(
                OnConflict::columns([Drafts::Author, Drafts::Target])
                    .update_columns([Drafts::Text, Drafts::BaseVersion, Drafts::SavedAt])
                    .to_owned(),
            );
        self.database().execute(&insert).await.map(|_| ())
    }

    async fn count_drafts(
        &self,
        author: UserId,
        not_before: Timestamp,
        limit: u64,
    ) -> Result<u64, StoreError> {
        let mut select = Query::select();
        select
            .column(Drafts::SavedAt)
            .from(Drafts::Table)
            .and_where(Expr::col(Drafts::Author).eq(author.stored()))
            .and_where(Expr::col(Drafts::SavedAt).gte(to_micros(not_before)))
            .limit(limit);
        Ok(self.database().fetch_all(&select).await?.len() as u64)
    }

    async fn delete_draft(&self, author: UserId, target: &DraftTarget) -> Result<bool, StoreError> {
        let mut transaction = self.database().begin().await?;
        let deleted = delete_draft(&mut transaction, author, target).await?;
        transaction.commit().await?;
        Ok(deleted)
    }

    async fn purge_drafts(&self, before: Timestamp, now: Timestamp) -> Result<u64, StoreError> {
        let mut transaction = self.database().begin().await?;
        let mut drafts = Query::delete();
        drafts
            .from_table(Drafts::Table)
            .and_where(Expr::col(Drafts::SavedAt).lt(to_micros(before)));
        let gone = transaction.execute(&drafts).await?;
        let mut notices = Query::delete();
        notices
            .from_table(Notices::Table)
            .and_where(Expr::col(Notices::At).lt(to_micros(before)));
        transaction.execute(&notices).await?;
        // A lock row that ran out is no lock; it stays only until somebody takes the thing again.
        let mut locks = Query::delete();
        locks
            .from_table(Locks::Table)
            .and_where(Expr::col(Locks::ExpiresAt).lt(to_micros(now)));
        transaction.execute(&locks).await?;
        transaction.commit().await?;
        Ok(gone)
    }

    async fn latest_version_event_about(
        &self,
        record: &EntityRef,
    ) -> Result<Option<DomainEvent>, StoreError> {
        self.database()
            .latest_event_about_where(record, DomainEvent::counted_version)
            .await
    }

    async fn field_changed_since(
        &self,
        record: &EntityRef,
        field: &str,
        version: i64,
    ) -> Result<bool, StoreError> {
        self.database()
            .field_changed_after(record, field, version)
            .await
    }

    async fn take_edit_notices(
        &self,
        recipient: UserId,
        what: &EditWhat,
    ) -> Result<Vec<EditNotice>, StoreError> {
        let mut transaction = self.database().begin().await?;
        let mut select = Query::select();
        select
            .columns([Notices::Kind, Notices::FromPerson, Notices::At])
            .from(Notices::Table)
            .cond_where(notice_of(recipient, what))
            .order_by(Notices::At, Order::Asc);
        let notices = transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(|row| read_notice(row, recipient, what))
            .collect::<Result<Vec<_>, _>>()?;
        let mut delete = Query::delete();
        delete
            .from_table(Notices::Table)
            .cond_where(notice_of(recipient, what));
        transaction.execute(&delete).await?;
        transaction.commit().await?;
        Ok(notices)
    }

    async fn edit_lock_in(
        transaction: &mut Transaction,
        what: EditWhat,
    ) -> Result<Option<EditLock>, StoreError> {
        lock_in(transaction, &what).await
    }

    async fn draft_in(
        transaction: &mut Transaction,
        author: UserId,
        target: DraftTarget,
        not_before: Timestamp,
    ) -> Result<Option<Draft>, StoreError> {
        draft_in(transaction, author, target, not_before).await
    }

    async fn replace_edit_lock(
        transaction: &mut Transaction,
        wanted: &EditLock,
    ) -> Result<Option<EditLock>, StoreError> {
        let former = lock_in(transaction, &wanted.what()).await?;
        match &former {
            Some(old) => {
                if !update_lock(transaction, wanted, old.token).await? {
                    return Err(StoreError::Contended);
                }
            }
            None => {
                if !insert_lock(transaction, wanted).await? {
                    return Err(StoreError::Contended);
                }
            }
        }
        Ok(former)
    }

    async fn remove_edit_lock(
        transaction: &mut Transaction,
        what: &EditWhat,
    ) -> Result<Option<EditLock>, StoreError> {
        let former = lock_in(transaction, what).await?;
        if former.is_some() {
            delete_lock(transaction, what, None).await?;
        }
        Ok(former)
    }

    async fn remove_edit_lock_held(
        transaction: &mut Transaction,
        what: &EditWhat,
        hold: Hold,
    ) -> Result<bool, StoreError> {
        delete_lock(transaction, what, Some(hold)).await
    }

    async fn delete_draft_in(
        transaction: &mut Transaction,
        author: UserId,
        target: &DraftTarget,
    ) -> Result<bool, StoreError> {
        delete_draft(transaction, author, target).await
    }

    async fn put_edit_notice(
        transaction: &mut Transaction,
        notice: &EditNotice,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(Notices::Table)
            .columns([
                Notices::Recipient,
                Notices::RecordKind,
                Notices::RecordId,
                Notices::Part,
                Notices::Kind,
                Notices::FromPerson,
                Notices::At,
            ])
            .values([
                Expr::val(notice.recipient.stored()),
                Expr::val(notice.record.kind.as_str()),
                Expr::val(notice.record.id.stored()),
                Expr::val(notice.part.as_str()),
                Expr::val(notice.kind.code()),
                Expr::val(notice.from.stored()),
                Expr::val(to_micros(notice.at)),
            ])
            .map_err(|_| StoreError::Corrupt("a note"))?
            .on_conflict(
                OnConflict::columns([
                    Notices::Recipient,
                    Notices::RecordKind,
                    Notices::RecordId,
                    Notices::Part,
                    Notices::Kind,
                ])
                .update_columns([Notices::FromPerson, Notices::At])
                .to_owned(),
            );
        transaction.execute(&insert).await.map(|_| ())
    }
}
