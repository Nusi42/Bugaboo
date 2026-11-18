//! The issues of the trash past its keep period (ADR-0045, decision 1, M9-Q1, M3-F25, M4-F131):
//! each goes for good with everything that belongs to it, in one change: its comments with their
//! versions, mentions and marks, the rows of its files (the files go when nothing points to them
//! any more), its relations and its place under or over other issues, its fields, tags, labels,
//! followers and people, its words for the search, its plan, its agent work, its notifications, its
//! history, and the events of all of them with their entries in the outbox.
//!
//! An issue with a time entry stays in the trash: a time entry is a business record that only an
//! administrator deletes (§147 AO, §257 HGB), so the purge never takes one along. When the last of
//! its entries has been deleted after its keep period, the next purge takes the issue. An issue on
//! which a timer runs stays too, so nobody loses the time it is counting.

use bb_core::event::RecordKind;
use jiff::Timestamp;
use sea_query::{
    ColumnRef, Cond, Condition, Expr, ExprTrait as _, IntoColumnRef, IntoTableRef, Order, Query,
    SelectStatement, TableRef,
};

use super::{delete_events, events_about};
use crate::db::{Database, Transaction, to_micros};
use crate::error::StoreError;
use crate::files::delete_attachments;
use crate::schema::{
    AcceptedWarningRow, AssignedPersonRow, AttachmentRow, AttemptResultRow, AttemptRow, CommentRow,
    CommentVersionRow, FieldValueRow, FollowerRow, HistoryEntryRow, IssueAliasRow, IssueLabelRow,
    IssueParentRow, IssueRow, IssueTagRow, IssueWorkRow, MentionRow, NotificationRow, ProposalRow,
    QuestionRow, RelationRow, ReviewRow, RuleMatchRow, TextIndexRow, TextMarkRow, TimeEntryRow,
    TimeFactorRow, TimerRow, UploadRow,
};

/// Deletes at most `limit` issues that went into the trash before `before` and have no time entry
/// and no running timer, with everything that belongs to them; says how many went.
pub(crate) async fn purge(
    database: &Database,
    before: Timestamp,
    limit: u64,
) -> Result<u64, StoreError> {
    let mut transaction = database.begin().await?;
    let issues = hold(&mut transaction, before, limit).await?;
    if issues.is_empty() {
        return Ok(0);
    }
    let count = u64::try_from(issues.len()).map_err(|_| StoreError::Corrupt("a count"))?;
    let doomed = Doomed::of(issues);
    // The events first, while the records they are about can still be found.
    delete_events_and_files(&mut transaction, &doomed).await?;
    delete_under(&mut transaction, &doomed).await?;
    delete_named(&mut transaction, &doomed).await?;
    transaction.commit().await?;
    Ok(count)
}

/// The issues a step purges, and the records under them, each as a select of their IDs.
struct Doomed {
    /// The IDs of the issues.
    issues: Vec<Vec<u8>>,
    /// The issues.
    chosen: SelectStatement,
    /// Their comments.
    comments: SelectStatement,
    /// Their agent work.
    works: SelectStatement,
    /// The relations from or to them.
    relations: SelectStatement,
    /// Their files.
    attachments: SelectStatement,
}

impl Doomed {
    fn of(issues: Vec<Vec<u8>>) -> Self {
        let of_issues = |column: ColumnRef| Expr::col(column).is_in(issues.clone());
        Self {
            chosen: ids_where(
                IssueRow::Table,
                IssueRow::Id,
                of_issues(IssueRow::Id.into_column_ref()),
            ),
            comments: ids_where(
                CommentRow::Table,
                CommentRow::Id,
                of_issues(CommentRow::Issue.into_column_ref()),
            ),
            works: ids_where(
                IssueWorkRow::Table,
                IssueWorkRow::Id,
                of_issues(IssueWorkRow::Issue.into_column_ref()),
            ),
            relations: ids_where(
                RelationRow::Table,
                RelationRow::Id,
                Cond::any()
                    .add(of_issues(RelationRow::FromIssue.into_column_ref()))
                    .add(of_issues(RelationRow::ToIssue.into_column_ref())),
            ),
            attachments: ids_where(
                AttachmentRow::Table,
                AttachmentRow::Id,
                of_issues(AttachmentRow::Issue.into_column_ref()),
            ),
            issues,
        }
    }

    /// The condition that `column` names one of the issues.
    fn names(&self, column: ColumnRef) -> Expr {
        Expr::col(column).is_in(self.issues.clone())
    }
}

/// Deletes the events about the issues and the records under them, with the rows of their files.
async fn delete_events_and_files(
    transaction: &mut Transaction,
    doomed: &Doomed,
) -> Result<(), StoreError> {
    for (kind, records) in [
        (RecordKind::Issue, &doomed.chosen),
        (RecordKind::Follower, &doomed.chosen),
        (RecordKind::Comment, &doomed.comments),
        (RecordKind::Relation, &doomed.relations),
        (RecordKind::AgentWork, &doomed.works),
    ] {
        delete_events(transaction, &events_about(kind, records.clone())).await?;
    }
    delete_attachments(transaction, &doomed.attachments).await?;
    Ok(())
}

/// Deletes what hangs under the agent work, the comments and the issues.
async fn delete_under(transaction: &mut Transaction, doomed: &Doomed) -> Result<(), StoreError> {
    let attempts = ids_where(
        AttemptRow::Table,
        AttemptRow::Id,
        Expr::col(AttemptRow::Work).in_subquery(doomed.works.clone()),
    );
    delete_where(
        transaction,
        AttemptResultRow::Table,
        Expr::col(AttemptResultRow::Attempt).in_subquery(attempts),
    )
    .await?;
    let under: [(TableRef, ColumnRef, &SelectStatement); 5] = [
        (
            AttemptRow::Table.into_table_ref(),
            AttemptRow::Work.into_column_ref(),
            &doomed.works,
        ),
        (
            QuestionRow::Table.into_table_ref(),
            QuestionRow::Work.into_column_ref(),
            &doomed.works,
        ),
        (
            ReviewRow::Table.into_table_ref(),
            ReviewRow::Work.into_column_ref(),
            &doomed.works,
        ),
        (
            CommentVersionRow::Table.into_table_ref(),
            CommentVersionRow::Comment.into_column_ref(),
            &doomed.comments,
        ),
        (
            MentionRow::Table.into_table_ref(),
            MentionRow::Comment.into_column_ref(),
            &doomed.comments,
        ),
    ];
    for (table, column, parents) in under {
        delete_where(
            transaction,
            table,
            Expr::col(column).in_subquery(parents.clone()),
        )
        .await?;
    }
    let of_records: [(TableRef, ColumnRef, ColumnRef, RecordKind, &SelectStatement); 3] = [
        (
            TextMarkRow::Table.into_table_ref(),
            TextMarkRow::EntityKind.into_column_ref(),
            TextMarkRow::EntityId.into_column_ref(),
            RecordKind::Comment,
            &doomed.comments,
        ),
        (
            TextMarkRow::Table.into_table_ref(),
            TextMarkRow::EntityKind.into_column_ref(),
            TextMarkRow::EntityId.into_column_ref(),
            RecordKind::Issue,
            &doomed.chosen,
        ),
        (
            FieldValueRow::Table.into_table_ref(),
            FieldValueRow::EntityKind.into_column_ref(),
            FieldValueRow::EntityId.into_column_ref(),
            RecordKind::Issue,
            &doomed.chosen,
        ),
    ];
    for (table, kind_column, id_column, kind, records) in of_records {
        delete_where(
            transaction,
            table,
            Cond::all()
                .add(Expr::col(kind_column).eq(kind.code()))
                .add(Expr::col(id_column).in_subquery(records.clone())),
        )
        .await?;
    }
    Ok(())
}

/// Deletes the rows that name the issues themselves, then the issues.
async fn delete_named(transaction: &mut Transaction, doomed: &Doomed) -> Result<(), StoreError> {
    let named: [(TableRef, ColumnRef); 20] = [
        (
            UploadRow::Table.into_table_ref(),
            UploadRow::Issue.into_column_ref(),
        ),
        (
            CommentRow::Table.into_table_ref(),
            CommentRow::Issue.into_column_ref(),
        ),
        (
            IssueWorkRow::Table.into_table_ref(),
            IssueWorkRow::Issue.into_column_ref(),
        ),
        (
            ProposalRow::Table.into_table_ref(),
            ProposalRow::Issue.into_column_ref(),
        ),
        (
            RuleMatchRow::Table.into_table_ref(),
            RuleMatchRow::Issue.into_column_ref(),
        ),
        (
            HistoryEntryRow::Table.into_table_ref(),
            HistoryEntryRow::Issue.into_column_ref(),
        ),
        (
            NotificationRow::Table.into_table_ref(),
            NotificationRow::Issue.into_column_ref(),
        ),
        (
            FollowerRow::Table.into_table_ref(),
            FollowerRow::Issue.into_column_ref(),
        ),
        (
            AssignedPersonRow::Table.into_table_ref(),
            AssignedPersonRow::Issue.into_column_ref(),
        ),
        (
            IssueAliasRow::Table.into_table_ref(),
            IssueAliasRow::Issue.into_column_ref(),
        ),
        (
            IssueTagRow::Table.into_table_ref(),
            IssueTagRow::Issue.into_column_ref(),
        ),
        (
            IssueLabelRow::Table.into_table_ref(),
            IssueLabelRow::Issue.into_column_ref(),
        ),
        (
            TextIndexRow::Table.into_table_ref(),
            TextIndexRow::Issue.into_column_ref(),
        ),
        (
            TimeFactorRow::Table.into_table_ref(),
            TimeFactorRow::Issue.into_column_ref(),
        ),
        (
            AcceptedWarningRow::Table.into_table_ref(),
            AcceptedWarningRow::Issue.into_column_ref(),
        ),
        (
            RelationRow::Table.into_table_ref(),
            RelationRow::FromIssue.into_column_ref(),
        ),
        (
            RelationRow::Table.into_table_ref(),
            RelationRow::ToIssue.into_column_ref(),
        ),
        (
            IssueParentRow::Table.into_table_ref(),
            IssueParentRow::Child.into_column_ref(),
        ),
        (
            IssueParentRow::Table.into_table_ref(),
            IssueParentRow::Parent.into_column_ref(),
        ),
        (
            IssueRow::Table.into_table_ref(),
            IssueRow::Id.into_column_ref(),
        ),
    ];
    for (table, column) in named {
        delete_where(transaction, table, doomed.names(column)).await?;
    }
    Ok(())
}

/// Chooses the issues to purge and holds them for the change, in one statement: an issue that
/// somebody takes out of the trash at the same moment is either taken out first, and not chosen,
/// or waits until the purge is done and finds it gone. The IDs of the chosen issues.
async fn hold(
    transaction: &mut Transaction,
    before: Timestamp,
    limit: u64,
) -> Result<Vec<Vec<u8>>, StoreError> {
    let booked = ids_where(
        TimeEntryRow::Table,
        TimeEntryRow::Issue,
        Expr::col(TimeEntryRow::Issue).is_not_null(),
    );
    let timed = ids_where(
        TimerRow::Table,
        TimerRow::Issue,
        Expr::col(TimerRow::Issue).is_not_null(),
    );
    let mut chosen = Query::select();
    chosen
        .column(IssueRow::Id)
        .from(IssueRow::Table)
        .and_where(Expr::col(IssueRow::DeletedAt).lt(to_micros(before)))
        .and_where(Expr::col(IssueRow::Id).not_in_subquery(booked))
        .and_where(Expr::col(IssueRow::Id).not_in_subquery(timed))
        .order_by(IssueRow::Id, Order::Asc)
        .limit(limit);
    let mut held = Query::update();
    held.table(IssueRow::Table)
        .value(IssueRow::DeletedAt, Expr::col(IssueRow::DeletedAt))
        .and_where(Expr::col(IssueRow::Id).in_subquery(chosen))
        .returning_col(IssueRow::Id);
    transaction
        .fetch_all(&held)
        .await?
        .iter()
        .map(|row| row.get(IssueRow::Id))
        .collect()
}

/// The column `id` of the rows of `table` that match `condition`.
fn ids_where(
    table: impl IntoTableRef,
    id: impl IntoColumnRef,
    condition: impl Into<Condition>,
) -> SelectStatement {
    let mut select = Query::select();
    select.column(id).from(table).cond_where(condition);
    select
}

/// Deletes the rows of `table` that match `condition`.
async fn delete_where(
    transaction: &mut Transaction,
    table: impl IntoTableRef,
    condition: impl Into<Condition>,
) -> Result<u64, StoreError> {
    let mut delete = Query::delete();
    delete.from_table(table).cond_where(condition);
    transaction.execute(&delete).await
}
