//! Keeping the index: the texts of some issues as they are stored now are prepared and written in
//! place of the rows the issues had. Reading the same event again, a reader that stops half way and
//! the rebuild all do exactly this, so each is idempotent and they cannot disagree.
//!
//! What leaves the index: a trashed issue (all its rows), a trashed comment, a trashed file, and a
//! file on a trashed comment. A restored one comes back at the next run, because its issue is
//! read again.
//!
//! Many issues are read in one go (a few statements for the whole batch, not for each issue), so
//! the rebuild of a large database and a reader that has fallen behind do not cost a statement
//! for every text.

use std::collections::BTreeMap;

use bb_core::event::RecordKind;
use bb_core::id::{CommentId, EntityId, FieldId, IssueId};
use bb_core::own_fields::FieldType;
use bb_core::text_index::TextSource;
use bb_core::text_words::prepare_joined;
use sea_query::{Expr, ExprTrait as _, Order, Query};

use crate::db::{Reader, STATEMENT_ITEMS, StoredId as _, Transaction, rows_per_insert};
use crate::error::StoreError;
use crate::schema::{AttachmentRow, CommentRow, FieldRow, FieldValueRow, IssueRow, TextIndexRow};

/// How many columns a row of the index binds when it is written.
const WRITTEN_COLUMNS: usize = 4;

/// One text to index, as a person typed it.
struct Text {
    source: TextSource,
    source_ref: EntityId,
    raw: String,
}

/// The ids as the values a statement binds.
fn bound(issues: &[IssueId]) -> Vec<Vec<u8>> {
    issues.iter().map(|issue| issue.stored()).collect()
}

/// The texts of these issues as they are stored now, by issue. An issue that is not there or is
/// in the trash has no entry.
async fn texts_of(
    reader: &mut impl Reader,
    issues: &[IssueId],
) -> Result<BTreeMap<[u8; 16], Vec<Text>>, StoreError> {
    let mut texts: BTreeMap<[u8; 16], Vec<Text>> = BTreeMap::new();
    let mut select = Query::select();
    select
        .columns([IssueRow::Id, IssueRow::Title, IssueRow::Description])
        .from(IssueRow::Table)
        .and_where(Expr::col(IssueRow::Id).is_in(bound(issues)))
        .and_where(Expr::col(IssueRow::DeletedAt).is_null());
    for row in reader.fetch_all(&select).await? {
        let issue = row.id::<IssueId>(IssueRow::Id)?;
        let own = EntityId::from(issue);
        texts.insert(
            issue.to_stored(),
            vec![
                Text {
                    source: TextSource::Title,
                    source_ref: own,
                    raw: row.get(IssueRow::Title)?,
                },
                Text {
                    source: TextSource::Description,
                    source_ref: own,
                    raw: row.get(IssueRow::Description)?,
                },
            ],
        );
    }
    // Only the issues that are there and not in the trash, from here on.
    let live: Vec<Vec<u8>> = texts.keys().map(|issue| issue.to_vec()).collect();
    if live.is_empty() {
        return Ok(texts);
    }

    let mut select = Query::select();
    select
        .columns([CommentRow::Issue, CommentRow::Id, CommentRow::Text])
        .from(CommentRow::Table)
        .and_where(Expr::col(CommentRow::Issue).is_in(live.clone()))
        .and_where(Expr::col(CommentRow::DeletedAt).is_null())
        .order_by(CommentRow::Id, Order::Asc);
    for row in reader.fetch_all(&select).await? {
        let issue = row.id::<IssueId>(CommentRow::Issue)?;
        push(
            &mut texts,
            issue,
            Text {
                source: TextSource::Comment,
                source_ref: row.id::<CommentId>(CommentRow::Id)?.into(),
                raw: row.get(CommentRow::Text)?,
            },
        );
    }

    // A file on a comment in the trash is gone with the comment.
    let mut select = Query::select();
    select
        .columns([
            (AttachmentRow::Table, AttachmentRow::Issue),
            (AttachmentRow::Table, AttachmentRow::Id),
            (AttachmentRow::Table, AttachmentRow::Name),
        ])
        .from(AttachmentRow::Table)
        .left_join(
            CommentRow::Table,
            Expr::col((CommentRow::Table, CommentRow::Id))
                .equals((AttachmentRow::Table, AttachmentRow::Comment)),
        )
        .and_where(Expr::col((AttachmentRow::Table, AttachmentRow::Issue)).is_in(live.clone()))
        .and_where(Expr::col((AttachmentRow::Table, AttachmentRow::DeletedAt)).is_null())
        .and_where(Expr::col((CommentRow::Table, CommentRow::DeletedAt)).is_null())
        .order_by((AttachmentRow::Table, AttachmentRow::Id), Order::Asc);
    for row in reader.fetch_all(&select).await? {
        push(
            &mut texts,
            row.id::<IssueId>(AttachmentRow::Issue)?,
            Text {
                source: TextSource::File,
                source_ref: row.id(AttachmentRow::Id)?,
                raw: row.get(AttachmentRow::Name)?,
            },
        );
    }

    let mut select = Query::select();
    select
        .columns([
            (FieldValueRow::Table, FieldValueRow::EntityId),
            (FieldValueRow::Table, FieldValueRow::Field),
            (FieldValueRow::Table, FieldValueRow::ValueText),
        ])
        .from(FieldValueRow::Table)
        .inner_join(
            FieldRow::Table,
            Expr::col((FieldRow::Table, FieldRow::Id))
                .equals((FieldValueRow::Table, FieldValueRow::Field)),
        )
        .and_where(
            Expr::col((FieldValueRow::Table, FieldValueRow::EntityKind))
                .eq(RecordKind::Issue.code()),
        )
        .and_where(Expr::col((FieldValueRow::Table, FieldValueRow::EntityId)).is_in(live))
        .and_where(Expr::col((FieldRow::Table, FieldRow::Kind)).eq(FieldType::LongText.code()))
        .and_where(Expr::col((FieldValueRow::Table, FieldValueRow::ValueText)).is_not_null())
        .order_by((FieldValueRow::Table, FieldValueRow::Field), Order::Asc);
    for row in reader.fetch_all(&select).await? {
        push(
            &mut texts,
            row.id::<IssueId>(FieldValueRow::EntityId)?,
            Text {
                source: TextSource::Field,
                source_ref: row.id::<FieldId>(FieldValueRow::Field)?.into(),
                raw: row.get(FieldValueRow::ValueText)?,
            },
        );
    }
    Ok(texts)
}

fn push(texts: &mut BTreeMap<[u8; 16], Vec<Text>>, issue: IssueId, text: Text) {
    if let Some(list) = texts.get_mut(&issue.to_stored()) {
        list.push(text);
    }
}

/// Makes the rows of these issues what their stored texts say now. An issue in the trash, or one
/// that is not there, has no rows afterwards.
///
/// # Errors
/// Fails when the database fails.
pub(crate) async fn reindex_issues(
    transaction: &mut Transaction,
    issues: &[IssueId],
) -> Result<(), StoreError> {
    for chunk in issues.chunks(STATEMENT_ITEMS / 64) {
        let texts = texts_of(transaction, chunk).await?;
        let mut delete = Query::delete();
        delete
            .from_table(TextIndexRow::Table)
            .and_where(Expr::col(TextIndexRow::Issue).is_in(bound(chunk)));
        transaction.execute(&delete).await?;
        let mut rows: Vec<(IssueId, TextSource, EntityId, String)> = Vec::new();
        for (issue, list) in texts {
            let issue = IssueId::from_stored(issue)
                .map_err(|_| StoreError::Corrupt("an issue of the text index"))?;
            for text in list {
                let words = prepare_joined(&text.raw);
                if !words.is_empty() {
                    rows.push((issue, text.source, text.source_ref, words));
                }
            }
        }
        insert_rows(transaction, &rows).await?;
    }
    Ok(())
}

/// Adds rows, as many to a statement as one may bind.
async fn insert_rows(
    transaction: &mut Transaction,
    rows: &[(IssueId, TextSource, EntityId, String)],
) -> Result<(), StoreError> {
    for chunk in rows.chunks(rows_per_insert(WRITTEN_COLUMNS)) {
        let mut insert = Query::insert();
        insert.into_table(TextIndexRow::Table).columns([
            TextIndexRow::Issue,
            TextIndexRow::Source,
            TextIndexRow::SourceRef,
            TextIndexRow::Words,
        ]);
        for (issue, source, source_ref, words) in chunk {
            insert
                .values([
                    Expr::val(issue.stored()),
                    Expr::val(source.code()),
                    Expr::val(source_ref.stored()),
                    Expr::val(words.as_str()),
                ])
                .map_err(|_| StoreError::Corrupt("a row of the text index"))?;
        }
        transaction.execute(&insert).await?;
    }
    Ok(())
}

/// Adds the row of one text to the index, as a test of the search needs it: the text is prepared
/// the way the real reader prepares it. For a test that needs a row without the stored text
/// behind it; the real index is made by [`reindex_issues`].
///
/// # Errors
/// Fails when the database fails.
#[cfg(any(test, feature = "testing"))]
pub async fn index_text(
    transaction: &mut Transaction,
    issue: IssueId,
    source: TextSource,
    source_ref: EntityId,
    raw: &str,
) -> Result<(), StoreError> {
    insert_rows(
        transaction,
        &[(issue, source, source_ref, prepare_joined(raw))],
    )
    .await
}
