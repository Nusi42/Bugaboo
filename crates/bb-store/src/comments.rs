//! The adapter of the comments port (`bb_app::comments::CommentStore`): comments, their earlier
//! texts and the mentions they make (ISS-05).
//!
//! A comment keeps its earlier texts in `comment_version`; the text in `comment` is the current
//! one. A mention row exists for each comment and person, and a person who stays mentioned when
//! the text is edited keeps the date they read it.

use bb_app::comments::CommentStore;
use bb_core::comment::{Comment, CommentVersion, Mention, MentionTarget};
use bb_core::id::{CommentId, IssueId, UserId};
use jiff::Timestamp;
use sea_query::{Expr, ExprTrait as _, Order, Query, SelectStatement};

use crate::auth::AuthStorage;
use crate::db::{Row, StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::schema::{CommentRow, CommentVersionRow, MentionRow};

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
        .columns([
            MentionRow::Comment,
            MentionRow::Recipient,
            MentionRow::Via,
            MentionRow::ReadAt,
        ])
        .from(MentionRow::Table);
    select
}

fn read_mention(row: &Row) -> Result<Mention, StoreError> {
    Ok(Mention {
        comment: row.id::<CommentId>(MentionRow::Comment)?,
        recipient: row.id::<UserId>(MentionRow::Recipient)?,
        via: MentionTarget::from_code(&row.get::<String>(MentionRow::Via)?)
            .ok_or(StoreError::Corrupt("a kind of mention"))?,
        read_at: row.optional_time(MentionRow::ReadAt)?,
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

    async fn mentions_of(
        &self,
        recipient: UserId,
        unread: bool,
        before: Option<CommentId>,
        limit: u64,
    ) -> Result<Vec<Mention>, StoreError> {
        let mut select = mention_select();
        select
            .and_where(Expr::col(MentionRow::Recipient).eq(recipient.stored()))
            .order_by(MentionRow::Comment, Order::Desc)
            .limit(limit);
        select.and_where(if unread {
            Expr::col(MentionRow::ReadAt).is_null()
        } else {
            Expr::col(MentionRow::ReadAt).is_not_null()
        });
        if let Some(before) = before {
            select.and_where(Expr::col(MentionRow::Comment).lt(before.stored()));
        }
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_mention)
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
        version: i64,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(CommentRow::Table)
            .value(CommentRow::Text, text)
            .value(CommentRow::EditedAt, to_micros(edited_at))
            .value(CommentRow::Version, Expr::col(CommentRow::Version).add(1))
            .and_where(Expr::col(CommentRow::Id).eq(id.stored()))
            .and_where(Expr::col(CommentRow::Version).eq(version));
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
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(CommentRow::Table)
            .value(CommentRow::DeletedAt, deleted_at.map(to_micros))
            .value(CommentRow::Version, Expr::col(CommentRow::Version).add(1))
            .and_where(Expr::col(CommentRow::Id).eq(id.stored()));
        // Only a comment that is where the change expects it: not in the trash to put it there,
        // in the trash to take it out.
        update.and_where(if deleted_at.is_some() {
            Expr::col(CommentRow::DeletedAt).is_null()
        } else {
            Expr::col(CommentRow::DeletedAt).is_not_null()
        });
        Ok(transaction.execute(&update).await? > 0)
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
                .columns([
                    MentionRow::Comment,
                    MentionRow::Recipient,
                    MentionRow::Via,
                    MentionRow::ReadAt,
                ])
                .values([
                    Expr::val(mention.comment.stored()),
                    Expr::val(mention.recipient.stored()),
                    Expr::val(mention.via.code()),
                    Expr::val(mention.read_at.map(to_micros)),
                ])
                .map_err(|_| StoreError::Corrupt("a mention"))?;
            transaction.execute(&insert).await?;
        }
        Ok(())
    }

    async fn mark_mentions_read(
        transaction: &mut Transaction,
        recipient: UserId,
        comments: &[CommentId],
        at: Timestamp,
    ) -> Result<(), StoreError> {
        if comments.is_empty() {
            return Ok(());
        }
        let mut update = Query::update();
        update
            .table(MentionRow::Table)
            .value(MentionRow::ReadAt, to_micros(at))
            .and_where(Expr::col(MentionRow::Recipient).eq(recipient.stored()))
            .and_where(Expr::col(MentionRow::ReadAt).is_null())
            .and_where(
                Expr::col(MentionRow::Comment)
                    .is_in(comments.iter().map(|id| id.stored()).collect::<Vec<_>>()),
            );
        transaction.execute(&update).await.map(|_| ())
    }
}
