//! How many records of a kind Bugaboo deletes by itself a cutoff reaches (ADR-0045, decision 2):
//! the page of the rules says what a changed period would delete before it is saved. Each count
//! asks the condition that the purge of its kind deletes by, from the module of its tables (the dead
//! jobs' here, which keeps the module of jobs small), so the count and the deleting cannot drift
//! apart; the purges add only their limit.

use bb_core::job::JobState;
use bb_core::keeping::KeepKind;
use jiff::Timestamp;
use sea_query::{Asterisk, Condition, Expr, ExprTrait as _, IntoTableRef, Query};

use super::trash::may_go;
use super::trashed_before;
use crate::auth::{past_life, past_networks};
use crate::db::{Database, count_of, to_micros};
use crate::error::StoreError;
use crate::notifications::past_notifications;
use crate::schema::{
    AttachmentRow, Computed, IssueRow, Job, NotificationRow, RemovalPendingRow, SignInSession,
};

/// The jobs that gave up before the cutoff `before`: what the purge deletes, and what the count of
/// a changed period counts.
pub(crate) fn past_dead_jobs(before: Timestamp) -> Condition {
    // The dead job of a removal that is not finished stays: it is what the status page and the
    // job that resumes the removal look at (ADR-0044, decision 6).
    let mut removals = Query::select();
    removals
        .column(RemovalPendingRow::Job)
        .from(RemovalPendingRow::Table);
    Condition::all()
        .add(Expr::col(Job::State).eq(JobState::Dead.code()))
        .add(Expr::col(Job::RunAt).lt(to_micros(before)))
        .add(Expr::col(Job::Id).not_in_subquery(removals))
}

/// How many rows of `table` match `condition`.
async fn count_where(
    database: &Database,
    table: impl IntoTableRef,
    condition: Condition,
) -> Result<u64, StoreError> {
    let mut select = Query::select();
    select
        .expr_as(Expr::col(Asterisk).count(), Computed::Total)
        .from(table)
        .cond_where(condition);
    count_of(&mut &*database, &select).await
}

/// How many records of `kind` the purge of its job would delete with the cutoff `before`, without
/// its limit. Only the kinds whose period is one setting are counted: a fixed period is never
/// changed, and the rotation of backups is the work of two settings.
pub(crate) async fn count_past(
    database: &Database,
    kind: KeepKind,
    before: Timestamp,
) -> Result<u64, StoreError> {
    match kind {
        KeepKind::IpAddresses => {
            count_where(database, SignInSession::Table, past_networks(before)).await
        }
        KeepKind::Sessions => count_where(database, SignInSession::Table, past_life(before)).await,
        KeepKind::Notifications => {
            count_where(database, NotificationRow::Table, past_notifications(before)).await
        }
        KeepKind::Trash => {
            let issues = count_where(
                database,
                IssueRow::Table,
                trashed_before(IssueRow::DeletedAt, before).add(may_go()),
            )
            .await?;
            let attachments = count_where(
                database,
                AttachmentRow::Table,
                trashed_before(AttachmentRow::DeletedAt, before),
            )
            .await?;
            Ok(issues.saturating_add(attachments))
        }
        KeepKind::DeadJobs => count_where(database, Job::Table, past_dead_jobs(before)).await,
        KeepKind::SecurityLog => database.count_prunable(before).await,
        KeepKind::SignUpLinks
        | KeepKind::EmailChangeLinks
        | KeepKind::AgentRequests
        | KeepKind::Drafts
        | KeepKind::Uploads
        | KeepKind::AppGrants
        | KeepKind::ToolCalls
        | KeepKind::AwayTimes
        | KeepKind::Backups
        | KeepKind::DataExports
        | KeepKind::WorkDays
        | KeepKind::TimeEntries => Err(StoreError::Corrupt(
            "a kind whose period no single setting changes",
        )),
    }
}
