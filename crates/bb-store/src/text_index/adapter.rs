//! The adapter of the index port (`bb_app::text_index::TextIndexStore`): the events the consumer
//! has not read, the update of the rows and the place the consumer has come to (ISS-13,
//! ADR-0007), and the rebuild from the stored texts.
//!
//! The rows of an issue are made from its texts as they are stored when the consumer runs, in the
//! same transaction that records the place, so a consumer that stops half way reads the same events
//! again and makes the same rows.

use std::collections::BTreeSet;

use bb_app::text_index::{TextIndexStore, Touched};
use bb_core::event::DomainEvent;
use bb_core::id::IssueId;
use bb_core::technical::TechnicalName;
use bb_core::text_index::CONSUMER;
use jiff::Timestamp;
use sea_query::{Expr, ExprTrait as _, Order, Query};

use super::write::reindex_issues;
use crate::auth::AuthStorage;
use crate::db::{StoredId as _, Transaction};
use crate::error::StoreError;
use crate::schema::{AttachmentRow, CommentRow, IssueRow};

/// How many issues the rebuild makes in one transaction.
const REBUILD_BATCH: u64 = 500;

fn consumer() -> Result<TechnicalName, StoreError> {
    TechnicalName::new(CONSUMER).map_err(|_| StoreError::Corrupt("the name of the text index"))
}

/// The issue of a comment or of a file, if there is one.
async fn issue_of(
    transaction: &mut Transaction,
    touched: Touched,
) -> Result<Option<IssueId>, StoreError> {
    let mut select = Query::select();
    match touched {
        Touched::Issue(issue) => Ok(Some(issue)),
        Touched::Comment(comment) => {
            select
                .column(CommentRow::Issue)
                .from(CommentRow::Table)
                .and_where(Expr::col(CommentRow::Id).eq(comment.stored()));
            transaction
                .fetch_optional(&select)
                .await?
                .map(|row| row.id::<IssueId>(CommentRow::Issue))
                .transpose()
        }
        Touched::File(file) => {
            select
                .column(AttachmentRow::Issue)
                .from(AttachmentRow::Table)
                .and_where(Expr::col(AttachmentRow::Id).eq(file.stored()));
            transaction
                .fetch_optional(&select)
                .await?
                .map(|row| row.id::<IssueId>(AttachmentRow::Issue))
                .transpose()
        }
    }
}

impl TextIndexStore for AuthStorage {
    async fn text_index_events(&self, limit: u64) -> Result<Vec<(i64, DomainEvent)>, StoreError> {
        Ok(self
            .database()
            .outbox_after(&consumer()?, limit)
            .await?
            .into_iter()
            .map(|(entry, event)| (entry.position, event))
            .collect())
    }

    async fn store_text_index(
        &self,
        touched: &[Touched],
        position: i64,
        now: Timestamp,
    ) -> Result<(), StoreError> {
        let database = self.database();
        if !touched.is_empty() {
            let mut transaction = database.begin().await?;
            let mut issues = BTreeSet::new();
            for one in touched {
                if let Some(issue) = issue_of(&mut transaction, *one).await? {
                    issues.insert(issue.to_stored());
                }
            }
            let issues = issues
                .into_iter()
                .map(|bytes| {
                    IssueId::from_stored(bytes)
                        .map_err(|_| StoreError::Corrupt("an issue of the text index"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            reindex_issues(&mut transaction, &issues).await?;
            transaction.commit().await?;
        }
        database.outbox_done(&consumer()?, position, now).await
    }

    async fn rebuild_text_index(&self) -> Result<u64, StoreError> {
        let database = self.database();
        let mut after: Option<Vec<u8>> = None;
        let mut read = 0_u64;
        loop {
            let mut transaction = database.begin().await?;
            let mut select = Query::select();
            select
                .column(IssueRow::Id)
                .from(IssueRow::Table)
                .order_by(IssueRow::Id, Order::Asc)
                .limit(REBUILD_BATCH);
            if let Some(after) = &after {
                select.and_where(Expr::col(IssueRow::Id).gt(after.clone()));
            }
            let ids: Vec<Vec<u8>> = transaction
                .fetch_all(&select)
                .await?
                .iter()
                .map(|row| row.get(IssueRow::Id))
                .collect::<Result<_, _>>()?;
            let issues = ids
                .iter()
                .map(|bytes| IssueId::read(bytes.clone()))
                .collect::<Result<Vec<_>, _>>()?;
            reindex_issues(&mut transaction, &issues).await?;
            transaction.commit().await?;
            read = read.saturating_add(u64::try_from(ids.len()).unwrap_or(u64::MAX));
            match ids.last() {
                Some(last) if u64::try_from(ids.len()).unwrap_or(0) == REBUILD_BATCH => {
                    after = Some(last.clone());
                }
                _ => return Ok(read),
            }
        }
    }
}
