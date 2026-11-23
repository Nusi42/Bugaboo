//! The adapter of the comments port (`bb_app::comments::CommentStore`): comments, their earlier
//! texts and the mentions they make (ISS-05).
//!
//! A comment keeps its earlier texts in `comment_version`; the text in `comment` is the current
//! one. A mention row exists for each comment and person, and a person who stays mentioned when
//! the text is edited keeps the date they read it.

use bb_app::comments::CommentStore;
use bb_core::comment::{Comment, CommentVersion, Mention, MentionTarget};
use bb_core::event::RecordKind;
use bb_core::id::{CommentId, IssueId, UserId};
use bb_core::version::Version;
use jiff::Timestamp;
use sea_query::{Expr, ExprTrait as _, Order, Query, SelectStatement};

use crate::auth::AuthStorage;
use crate::db::{Row, StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::keeping::{delete_events, events_about};
use crate::schema::{
    AttachmentRow, CommentRow, CommentVersionRow, MentionRow, NotificationRow, RemovalPendingRow,
    TextMarkRow, UploadRow,
};

/// The IDs of the comments in the trash: no search finds one, also while the full-text index
/// still holds its rows (the change of a removal puts every comment that goes into the trash at
/// once, [`hide_comments_of`], and the index forgets them as its job deletes them). The index of
/// migration 64 answers it.
pub(crate) fn trashed_comments() -> SelectStatement {
    let mut select = Query::select();
    select
        .column(CommentRow::Id)
        .from(CommentRow::Table)
        .and_where(Expr::col(CommentRow::DeletedAt).is_not_null());
    select
}

/// The IDs of the comments that go with a removal that is not finished (ADR-0044, decision 6):
/// they stay in the trash, where the change of the removal put them, until its job deletes them.
fn going_comments() -> SelectStatement {
    let mut select = Query::select();
    select
        .column((CommentRow::Table, CommentRow::Id))
        .from(CommentRow::Table)
        .inner_join(
            RemovalPendingRow::Table,
            Expr::col((RemovalPendingRow::Table, RemovalPendingRow::Person))
                .equals((CommentRow::Table, CommentRow::Author)),
        )
        .and_where(Expr::col((RemovalPendingRow::Table, RemovalPendingRow::CommentsGo)).eq(true));
    select
}

/// Puts every comment of `author` that is not in the trash into it at `at` (the change of a
/// removal whose comments go); how many. One statement through the index of the author.
///
/// # Errors
/// Fails when the database fails.
pub(crate) async fn hide_comments_of(
    transaction: &mut Transaction,
    author: UserId,
    at: Timestamp,
) -> Result<u64, StoreError> {
    let mut update = Query::update();
    update
        .table(CommentRow::Table)
        .value(CommentRow::DeletedAt, to_micros(at))
        .and_where(Expr::col(CommentRow::Author).eq(author.stored()))
        .and_where(Expr::col(CommentRow::DeletedAt).is_null());
    transaction.execute(&update).await
}

fn comment_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            CommentRow::Id,
            CommentRow::Issue,
            CommentRow::Author,
            CommentRow::Text,
            CommentRow::CreatedAt,
            CommentRow::EditedAt,
            CommentRow::DeletedAt,
            CommentRow::Version,
        ])
        .from(CommentRow::Table);
    select
}

fn read_comment(row: &Row) -> Result<Comment, StoreError> {
    Ok(Comment {
        id: row.id::<CommentId>(CommentRow::Id)?,
        issue: row.id::<IssueId>(CommentRow::Issue)?,
        author: row.id::<UserId>(CommentRow::Author)?,
        text: row.get(CommentRow::Text)?,
        created_at: row.time(CommentRow::CreatedAt)?,
        edited_at: row.optional_time(CommentRow::EditedAt)?,
        deleted_at: row.optional_time(CommentRow::DeletedAt)?,
        version: row.get(CommentRow::Version)?,
    })
}

fn mention_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([MentionRow::Comment, MentionRow::Recipient, MentionRow::Via])
        .from(MentionRow::Table);
    select
}

fn read_mention(row: &Row) -> Result<Mention, StoreError> {
    Ok(Mention {
        comment: row.id::<CommentId>(MentionRow::Comment)?,
        recipient: row.id::<UserId>(MentionRow::Recipient)?,
        via: MentionTarget::from_code(&row.get::<String>(MentionRow::Via)?)
            .ok_or(StoreError::Corrupt("a kind of mention"))?,
    })
}

impl CommentStore for AuthStorage {
    async fn comment_in(
        transaction: &mut Transaction,
        id: CommentId,
    ) -> Result<Option<Comment>, StoreError> {
        let mut select = comment_select();
        select.and_where(Expr::col(CommentRow::Id).eq(id.stored()));
        transaction
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_comment)
            .transpose()
    }

    async fn comment_versions_in(
        transaction: &mut Transaction,
        id: CommentId,
    ) -> Result<Vec<CommentVersion>, StoreError> {
        let mut select = Query::select();
        select
            .columns([
                CommentVersionRow::Comment,
                CommentVersionRow::Version,
                CommentVersionRow::Text,
                CommentVersionRow::SavedAt,
            ])
            .from(CommentVersionRow::Table)
            .and_where(Expr::col(CommentVersionRow::Comment).eq(id.stored()))
            .order_by(CommentVersionRow::Version, Order::Asc);
        transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(|row| {
                Ok(CommentVersion {
                    comment: row.id::<CommentId>(CommentVersionRow::Comment)?,
                    version: row.get(CommentVersionRow::Version)?,
                    text: row.get(CommentVersionRow::Text)?,
                    saved_at: row.time(CommentVersionRow::SavedAt)?,
                })
            })
            .collect()
    }

    async fn comments_by_ids(&self, ids: &[CommentId]) -> Result<Vec<Comment>, StoreError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut select = comment_select();
        select.and_where(
            Expr::col(CommentRow::Id).is_in(ids.iter().map(|id| id.stored()).collect::<Vec<_>>()),
        );
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_comment)
            .collect()
    }

    async fn mentions_in_comment(&self, comment: CommentId) -> Result<Vec<Mention>, StoreError> {
        let mut select = mention_select();
        select
            .and_where(Expr::col(MentionRow::Comment).eq(comment.stored()))
            .order_by(MentionRow::Recipient, Order::Asc);
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_mention)
            .collect()
    }

    async fn insert_comment(
        transaction: &mut Transaction,
        comment: &Comment,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(CommentRow::Table)
            .columns([
                CommentRow::Id,
                CommentRow::Issue,
                CommentRow::Author,
                CommentRow::Text,
                CommentRow::CreatedAt,
                CommentRow::EditedAt,
                CommentRow::DeletedAt,
                CommentRow::Version,
            ])
            .values([
                Expr::val(comment.id.stored()),
                Expr::val(comment.issue.stored()),
                Expr::val(comment.author.stored()),
                Expr::val(comment.text.as_str()),
                Expr::val(to_micros(comment.created_at)),
                Expr::val(comment.edited_at.map(to_micros)),
                Expr::val(comment.deleted_at.map(to_micros)),
                Expr::val(comment.version),
            ])
            .map_err(|_| StoreError::Corrupt("a comment"))?;
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn update_comment_text(
        transaction: &mut Transaction,
        id: CommentId,
        text: &str,
        edited_at: Timestamp,
        version: Version,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(CommentRow::Table)
            .value(CommentRow::Text, text)
            .value(CommentRow::EditedAt, to_micros(edited_at))
            .value(CommentRow::Version, Expr::col(CommentRow::Version).add(1))
            .and_where(Expr::col(CommentRow::Id).eq(id.stored()))
            .and_where(Expr::col(CommentRow::Version).eq(version.get()));
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn insert_comment_version(
        transaction: &mut Transaction,
        version: &CommentVersion,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(CommentVersionRow::Table)
            .columns([
                CommentVersionRow::Comment,
                CommentVersionRow::Version,
                CommentVersionRow::Text,
                CommentVersionRow::SavedAt,
            ])
            .values([
                Expr::val(version.comment.stored()),
                Expr::val(version.version),
                Expr::val(version.text.as_str()),
                Expr::val(to_micros(version.saved_at)),
            ])
            .map_err(|_| StoreError::Corrupt("an earlier text of a comment"))?;
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn set_comment_deleted(
        transaction: &mut Transaction,
        id: CommentId,
        deleted_at: Option<Timestamp>,
        version: Option<Version>,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(CommentRow::Table)
            .value(CommentRow::DeletedAt, deleted_at.map(to_micros))
            .value(CommentRow::Version, Expr::col(CommentRow::Version).add(1))
            .and_where(Expr::col(CommentRow::Id).eq(id.stored()));
        if let Some(version) = version {
            update.and_where(Expr::col(CommentRow::Version).eq(version.get()));
        }
        // Only a comment that is where the change expects it: not in the trash to put it there,
        // in the trash to take it out.
        update.and_where(if deleted_at.is_some() {
            Expr::col(CommentRow::DeletedAt).is_null()
        } else {
            Expr::col(CommentRow::DeletedAt).is_not_null()
        });
        // A comment that goes with a removal stays in the trash until the removal deletes it.
        if deleted_at.is_none() {
            update.and_where(Expr::col(CommentRow::Id).not_in_subquery(going_comments()));
        }
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn throw_away_comments(
        transaction: &mut Transaction,
        ids: &[CommentId],
    ) -> Result<(), StoreError> {
        crate::keeping::trash::throw_away_comments(
            transaction,
            ids.iter().map(|id| id.stored()).collect(),
        )
        .await
    }

    async fn replace_mentions(
        transaction: &mut Transaction,
        comment: CommentId,
        mentions: &[Mention],
    ) -> Result<(), StoreError> {
        let mut select = mention_select();
        select.and_where(Expr::col(MentionRow::Comment).eq(comment.stored()));
        let known: Vec<Mention> = transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_mention)
            .collect::<Result<_, _>>()?;
        // The people no longer mentioned.
        for old in &known {
            if !mentions
                .iter()
                .any(|mention| mention.recipient == old.recipient)
            {
                let mut delete = Query::delete();
                delete
                    .from_table(MentionRow::Table)
                    .and_where(Expr::col(MentionRow::Comment).eq(comment.stored()))
                    .and_where(Expr::col(MentionRow::Recipient).eq(old.recipient.stored()));
                transaction.execute(&delete).await?;
            }
        }
        // The people newly mentioned; the ones who stay keep what they read.
        for mention in mentions {
            if known.iter().any(|old| old.recipient == mention.recipient) {
                continue;
            }
            let mut insert = Query::insert();
            insert
                .into_table(MentionRow::Table)
                .columns([MentionRow::Comment, MentionRow::Recipient, MentionRow::Via])
                .values([
                    Expr::val(mention.comment.stored()),
                    Expr::val(mention.recipient.stored()),
                    Expr::val(mention.via.code()),
                ])
                .map_err(|_| StoreError::Corrupt("a mention"))?;
            transaction.execute(&insert).await?;
        }
        Ok(())
    }

    async fn delete_comments_of(
        transaction: &mut Transaction,
        author: UserId,
        limit: u64,
    ) -> Result<Vec<(CommentId, IssueId)>, StoreError> {
        let mut held = Query::select();
        held.columns([CommentRow::Id, CommentRow::Issue])
            .from(CommentRow::Table)
            .and_where(Expr::col(CommentRow::Author).eq(author.stored()))
            .order_by(CommentRow::Id, Order::Asc)
            .limit(limit);
        let mut gone = Vec::new();
        for row in transaction.fetch_all(&held).await? {
            gone.push((
                row.id::<CommentId>(CommentRow::Id)?,
                row.id::<IssueId>(CommentRow::Issue)?,
            ));
        }
        if gone.is_empty() {
            return Ok(gone);
        }
        // The comments of this call, by their IDs: the author may have more than `limit`.
        let ids: Vec<Vec<u8>> = gone.iter().map(|(comment, _)| comment.stored()).collect();
        let comments = || {
            let mut select = Query::select();
            select
                .column(CommentRow::Id)
                .from(CommentRow::Table)
                .and_where(Expr::col(CommentRow::Id).is_in(ids.clone()));
            select
        };
        // A file stays with its issue when the comment it came with goes.
        let mut attachments = Query::update();
        attachments
            .table(AttachmentRow::Table)
            .value(AttachmentRow::Comment, Option::<Vec<u8>>::None)
            .and_where(Expr::col(AttachmentRow::Comment).in_subquery(comments()));
        transaction.execute(&attachments).await?;
        let mut uploads = Query::update();
        uploads
            .table(UploadRow::Table)
            .value(UploadRow::Comment, Option::<Vec<u8>>::None)
            .and_where(Expr::col(UploadRow::Comment).in_subquery(comments()));
        transaction.execute(&uploads).await?;
        // A notification of a comment carries its text in its changes.
        let mut told = Query::delete();
        told.from_table(NotificationRow::Table).and_where(
            Expr::col(NotificationRow::Event)
                .in_subquery(events_about(RecordKind::Comment, comments())),
        );
        transaction.execute(&told).await?;
        delete_comment_rows(transaction, &comments()).await?;
        Ok(gone)
    }
}

/// Deletes the comments that `comments` selects (their IDs) with what hangs under them: their
/// versions, mentions and marks, then their events with what was made of them, then the comments.
/// The one deletion of comments, which the purge of the trash and the removal of a person call;
/// what points to a comment from outside (files, notifications) is the caller's.
///
/// # Errors
/// Fails when the database fails.
pub(crate) async fn delete_comment_rows(
    transaction: &mut Transaction,
    comments: &SelectStatement,
) -> Result<(), StoreError> {
    let mut versions = Query::delete();
    versions
        .from_table(CommentVersionRow::Table)
        .and_where(Expr::col(CommentVersionRow::Comment).in_subquery(comments.clone()));
    transaction.execute(&versions).await?;
    let mut mentions = Query::delete();
    mentions
        .from_table(MentionRow::Table)
        .and_where(Expr::col(MentionRow::Comment).in_subquery(comments.clone()));
    transaction.execute(&mentions).await?;
    let mut marks = Query::delete();
    marks
        .from_table(TextMarkRow::Table)
        .and_where(Expr::col(TextMarkRow::EntityKind).eq(RecordKind::Comment.code()))
        .and_where(Expr::col(TextMarkRow::EntityId).in_subquery(comments.clone()));
    transaction.execute(&marks).await?;
    delete_events(
        transaction,
        &events_about(RecordKind::Comment, comments.clone()),
    )
    .await?;
    // The IDs first: the select may name the comments by a column of their own rows.
    let ids: Vec<Vec<u8>> = transaction
        .fetch_all(comments)
        .await?
        .iter()
        .map(|row| row.get(CommentRow::Id))
        .collect::<Result<_, _>>()?;
    for chunk in ids.chunks(crate::db::STATEMENT_ITEMS) {
        let mut delete = Query::delete();
        delete
            .from_table(CommentRow::Table)
            .and_where(Expr::col(CommentRow::Id).is_in(chunk.to_vec()));
        transaction.execute(&delete).await?;
    }
    Ok(())
}
