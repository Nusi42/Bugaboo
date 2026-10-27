//! The adapter of the index port (`bb_app::text_index::TextIndexStore`): the events the consumer
//! has not read, the update of the rows and the place the consumer has come to (ISS-13,
//! ADR-0007), and the rebuild from the stored texts.
//!
//! The rows of an issue are made from its texts as they are stored when the consumer runs, in the
//! same transaction that moves the place of the consumer, so a consumer that stops reads the same
//! events again and makes the same rows. Which issue a comment or a file belongs to is looked up
//! for a whole batch at once.

use std::collections::BTreeSet;

use bb_app::text_index::{TextIndexStore, Touched};
use bb_core::event::DomainEvent;
use bb_core::id::IssueId;
use bb_core::text_index::CONSUMER;
use jiff::Timestamp;
use sea_query::{Expr, ExprTrait as _, Order, Query};

use super::write::{ISSUES_PER_STEP, reindex_issues};
use crate::auth::AuthStorage;
use crate::db::{STATEMENT_ITEMS, StoredId as _, Transaction};
use crate::error::StoreError;
use crate::events::{consumer_name, outbox_move};
use crate::schema::{AttachmentRow, CommentRow, IssueRow};

/// The issues a batch of events touches: the ones named, and the issues of the comments and files
/// named (two statements for the batch, not one for each).
async fn issues_of(
    transaction: &mut Transaction,
    touched: &[Touched],
) -> Result<Vec<IssueId>, StoreError> {
    let mut issues: BTreeSet<[u8; 16]> = BTreeSet::new();
    let (mut comments, mut files) = (Vec::new(), Vec::new());
    for one in touched {
        match one {
            Touched::Issue(issue) => {
                issues.insert(issue.to_stored());
            }
            Touched::Comment(comment) => comments.push(comment.stored()),
            Touched::File(file) => files.push(file.stored()),
        }
    }
    let kinds = (CommentRow::Table, CommentRow::Id, CommentRow::Issue);
    issues_of_records(transaction, kinds, &comments, &mut issues).await?;
    let kinds = (
        AttachmentRow::Table,
        AttachmentRow::Id,
        AttachmentRow::Issue,
    );
    issues_of_records(transaction, kinds, &files, &mut issues).await?;
    issues
        .into_iter()
        .map(|bytes| {
            IssueId::from_stored(bytes)
                .map_err(|_| StoreError::Corrupt("an issue of the text index"))
        })
        .collect()
}

/// The issues of the records with these IDs, from the table of one kind of record (comments or
/// attachments; `columns` are its table, its ID and its issue): one statement for each chunk of
/// the list, added to `issues`.
async fn issues_of_records<C: sea_query::Iden + Copy + 'static>(
    transaction: &mut Transaction,
    columns: (C, C, C),
    ids: &[Vec<u8>],
    issues: &mut BTreeSet<[u8; 16]>,
) -> Result<(), StoreError> {
    let (table, id, issue) = columns;
    for chunk in ids.chunks(STATEMENT_ITEMS) {
        let mut select = Query::select();
        select
            .column(issue)
            .from(table)
            .and_where(Expr::col(id).is_in(chunk.to_vec()));
        for row in transaction.fetch_all(&select).await? {
            issues.insert(row.id::<IssueId>(issue)?.to_stored());
        }
    }
    Ok(())
}

impl TextIndexStore for AuthStorage {
    async fn text_index_events(&self, limit: u64) -> Result<Vec<(i64, DomainEvent)>, StoreError> {
        self.database().outbox_events(CONSUMER, limit).await
    }

    async fn store_text_index(
        &self,
        touched: &[Touched],
        position: i64,
        now: Timestamp,
    ) -> Result<(), StoreError> {
        let mut transaction = self.database().begin().await?;
        let issues = issues_of(&mut transaction, touched).await?;
        reindex_issues(&mut transaction, &issues).await?;
        outbox_move(&mut transaction, &consumer_name(CONSUMER)?, position, now).await?;
        transaction.commit().await
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
                .limit(u64::try_from(ISSUES_PER_STEP).unwrap_or(u64::MAX));
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
                Some(last) if ids.len() == ISSUES_PER_STEP => after = Some(last.clone()),
                _ => return Ok(read),
            }
        }
    }
}
