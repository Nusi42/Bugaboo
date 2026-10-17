//! The adapter of the workflows port (`bb_app::workflows::WorkflowStore`): the workflows, their
//! statuses and status changes, and where each is used (WF-01, ADR-0031).
//!
//! A workflow is saved as a whole: its statuses and status changes are deleted and written again in
//! the one transaction that checks the version, so a reader never sees half a workflow.

use bb_app::workflows::WorkflowStore;
use bb_core::id::WorkflowId;
use bb_core::issue::{Issue, StatusGroup};
use bb_core::workflow::{
    StatusKey, StatusRules, StoredChange, StoredStatus, StoredWorkflow, Workflow, WorkflowContent,
    WorkflowPlace, WorkflowUse,
};
use sea_query::{Expr, ExprTrait as _, Order, Query};

use crate::auth::AuthStorage;
use crate::db::{Row, StoredId, Transaction};
use crate::error::StoreError;
use crate::issues::{issue_select, read_issue};
use crate::schema::{IssueRow, WorkflowChangeRow, WorkflowRow, WorkflowStatusRow, WorkflowUseRow};

fn read_key(text: &str) -> Result<StatusKey, StoreError> {
    StatusKey::new(text).map_err(|_| StoreError::Corrupt("a status key"))
}

fn header_select() -> sea_query::SelectStatement {
    let mut select = Query::select();
    select
        .columns([WorkflowRow::Id, WorkflowRow::Name, WorkflowRow::Version])
        .from(WorkflowRow::Table);
    select
}

fn read_header(row: &Row) -> Result<Workflow, StoreError> {
    Ok(Workflow {
        id: row.id::<WorkflowId>(WorkflowRow::Id)?,
        name: row.get(WorkflowRow::Name)?,
        version: row.get(WorkflowRow::Version)?,
    })
}

fn read_status(row: &Row) -> Result<StoredStatus, StoreError> {
    Ok(StoredStatus {
        workflow: row.id(WorkflowStatusRow::Workflow)?,
        key: read_key(&row.get::<String>(WorkflowStatusRow::Key)?)?,
        name: row.get(WorkflowStatusRow::Name)?,
        group: StatusGroup::from_code(&row.get::<String>(WorkflowStatusRow::StatusGroup)?)
            .ok_or(StoreError::Corrupt("a status group"))?,
        position: row.get(WorkflowStatusRow::Position)?,
    })
}

fn read_change(row: &Row) -> Result<StoredChange, StoreError> {
    Ok(StoredChange {
        workflow: row.id(WorkflowChangeRow::Workflow)?,
        from: read_key(&row.get::<String>(WorkflowChangeRow::FromStatus)?)?,
        to: read_key(&row.get::<String>(WorkflowChangeRow::ToStatus)?)?,
        button: row.get(WorkflowChangeRow::Button)?,
        rules: serde_json::from_str::<StatusRules>(&row.get::<String>(WorkflowChangeRow::Rules)?)
            .map_err(|_| StoreError::Corrupt("the rules of a status change"))?,
        position: row.get(WorkflowChangeRow::Position)?,
    })
}

/// Writes the statuses and status changes of a workflow.
async fn insert_content(
    transaction: &mut Transaction,
    id: WorkflowId,
    content: &WorkflowContent,
) -> Result<(), StoreError> {
    let (statuses, changes) = content.rows(id);
    for status in &statuses {
        let mut insert = Query::insert();
        insert
            .into_table(WorkflowStatusRow::Table)
            .columns([
                WorkflowStatusRow::Workflow,
                WorkflowStatusRow::Key,
                WorkflowStatusRow::Name,
                WorkflowStatusRow::StatusGroup,
                WorkflowStatusRow::Position,
            ])
            .values([
                Expr::val(status.workflow.stored()),
                Expr::val(status.key.as_str()),
                Expr::val(status.name.as_str()),
                Expr::val(status.group.code()),
                Expr::val(status.position),
            ])
            .map_err(|_| StoreError::Corrupt("a status of a workflow"))?;
        transaction.execute(&insert).await?;
    }
    for change in &changes {
        let rules = serde_json::to_string(&change.rules)
            .map_err(|_| StoreError::Corrupt("the rules of a status change"))?;
        let mut insert = Query::insert();
        insert
            .into_table(WorkflowChangeRow::Table)
            .columns([
                WorkflowChangeRow::Workflow,
                WorkflowChangeRow::FromStatus,
                WorkflowChangeRow::ToStatus,
                WorkflowChangeRow::Button,
                WorkflowChangeRow::Rules,
                WorkflowChangeRow::Position,
            ])
            .values([
                Expr::val(change.workflow.stored()),
                Expr::val(change.from.as_str()),
                Expr::val(change.to.as_str()),
                Expr::val(change.button.as_str()),
                Expr::val(rules),
                Expr::val(change.position),
            ])
            .map_err(|_| StoreError::Corrupt("a status change of a workflow"))?;
        transaction.execute(&insert).await?;
    }
    Ok(())
}

/// Takes the statuses and status changes of a workflow away.
async fn delete_content(transaction: &mut Transaction, id: WorkflowId) -> Result<(), StoreError> {
    let mut changes = Query::delete();
    changes
        .from_table(WorkflowChangeRow::Table)
        .and_where(Expr::col(WorkflowChangeRow::Workflow).eq(id.stored()));
    transaction.execute(&changes).await?;
    let mut statuses = Query::delete();
    statuses
        .from_table(WorkflowStatusRow::Table)
        .and_where(Expr::col(WorkflowStatusRow::Workflow).eq(id.stored()));
    transaction.execute(&statuses).await?;
    Ok(())
}

/// The statuses and status changes of these workflows, three reads for any number of them.
async fn read_contents(
    transaction: &mut Transaction,
    ids: &[WorkflowId],
) -> Result<Vec<(WorkflowId, WorkflowContent)>, StoreError> {
    let wanted: Vec<Vec<u8>> = ids.iter().map(|id| id.stored()).collect();
    let mut statuses = Query::select();
    statuses
        .columns([
            WorkflowStatusRow::Workflow,
            WorkflowStatusRow::Key,
            WorkflowStatusRow::Name,
            WorkflowStatusRow::StatusGroup,
            WorkflowStatusRow::Position,
        ])
        .from(WorkflowStatusRow::Table)
        .and_where(Expr::col(WorkflowStatusRow::Workflow).is_in(wanted.clone()))
        .order_by(WorkflowStatusRow::Position, Order::Asc);
    let statuses = transaction
        .fetch_all(&statuses)
        .await?
        .iter()
        .map(read_status)
        .collect::<Result<Vec<_>, StoreError>>()?;
    let mut changes = Query::select();
    changes
        .columns([
            WorkflowChangeRow::Workflow,
            WorkflowChangeRow::FromStatus,
            WorkflowChangeRow::ToStatus,
            WorkflowChangeRow::Button,
            WorkflowChangeRow::Rules,
            WorkflowChangeRow::Position,
        ])
        .from(WorkflowChangeRow::Table)
        .and_where(Expr::col(WorkflowChangeRow::Workflow).is_in(wanted))
        .order_by(WorkflowChangeRow::Position, Order::Asc);
    let changes = transaction
        .fetch_all(&changes)
        .await?
        .iter()
        .map(read_change)
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok(ids
        .iter()
        .map(|id| {
            let own_statuses: Vec<StoredStatus> = statuses
                .iter()
                .filter(|status| status.workflow == *id)
                .cloned()
                .collect();
            let own_changes: Vec<StoredChange> = changes
                .iter()
                .filter(|change| change.workflow == *id)
                .cloned()
                .collect();
            (*id, WorkflowContent::from_rows(&own_statuses, &own_changes))
        })
        .collect())
}

async fn read_content(
    transaction: &mut Transaction,
    id: WorkflowId,
) -> Result<WorkflowContent, StoreError> {
    Ok(read_contents(transaction, &[id])
        .await?
        .into_iter()
        .next()
        .map(|(_, content)| content)
        .unwrap_or_default())
}

/// The condition that picks the use at exactly this place: a missing project or issue type is
/// `NULL`, which `=` never matches.
fn at_place(place: WorkflowPlace) -> sea_query::Condition {
    let project = match place.project {
        Some(project) => Expr::col(WorkflowUseRow::Project).eq(project.stored()),
        None => Expr::col(WorkflowUseRow::Project).is_null(),
    };
    let issue_type = match place.issue_type {
        Some(issue_type) => Expr::col(WorkflowUseRow::IssueType).eq(issue_type.stored()),
        None => Expr::col(WorkflowUseRow::IssueType).is_null(),
    };
    sea_query::Condition::all().add(project).add(issue_type)
}

impl WorkflowStore for AuthStorage {
    async fn workflow_uses_in(
        transaction: &mut Transaction,
    ) -> Result<Vec<WorkflowUse>, StoreError> {
        let mut select = Query::select();
        select
            .columns([
                WorkflowUseRow::Project,
                WorkflowUseRow::IssueType,
                WorkflowUseRow::Workflow,
            ])
            .from(WorkflowUseRow::Table)
            .order_by(WorkflowUseRow::Workflow, Order::Asc);
        transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(|row| {
                Ok(WorkflowUse {
                    project: row.optional_id(WorkflowUseRow::Project)?,
                    issue_type: row.optional_id(WorkflowUseRow::IssueType)?,
                    workflow: row.id(WorkflowUseRow::Workflow)?,
                })
            })
            .collect()
    }

    async fn workflow_headers_in(
        transaction: &mut Transaction,
    ) -> Result<Vec<Workflow>, StoreError> {
        let mut select = header_select();
        select
            .order_by(WorkflowRow::Name, Order::Asc)
            .order_by(WorkflowRow::Id, Order::Asc);
        transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_header)
            .collect()
    }

    async fn stored_workflow_in(
        transaction: &mut Transaction,
        id: WorkflowId,
    ) -> Result<Option<StoredWorkflow>, StoreError> {
        let mut select = header_select();
        select.and_where(Expr::col(WorkflowRow::Id).eq(id.stored()));
        let Some(workflow) = transaction
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_header)
            .transpose()?
        else {
            return Ok(None);
        };
        let content = read_content(transaction, id).await?;
        Ok(Some(StoredWorkflow { workflow, content }))
    }

    async fn stored_workflows_in(
        transaction: &mut Transaction,
        ids: Vec<WorkflowId>,
    ) -> Result<Vec<StoredWorkflow>, StoreError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut select = header_select();
        select
            .and_where(Expr::col(WorkflowRow::Id).is_in(ids.iter().map(|id| id.stored())))
            .order_by(WorkflowRow::Id, Order::Asc);
        let headers = transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_header)
            .collect::<Result<Vec<_>, StoreError>>()?;
        let found: Vec<WorkflowId> = headers.iter().map(|header| header.id).collect();
        let mut contents = read_contents(transaction, &found).await?;
        Ok(headers
            .into_iter()
            .zip(contents.drain(..))
            .map(|(workflow, (_, content))| StoredWorkflow { workflow, content })
            .collect())
    }

    async fn issues_in_statuses_in(
        transaction: &mut Transaction,
        keys: Vec<StatusKey>,
    ) -> Result<Vec<Issue>, StoreError> {
        if keys.is_empty() {
            return Ok(Vec::new());
        }
        let mut select = issue_select();
        select
            .and_where(Expr::col(IssueRow::Status).is_in(keys.iter().map(StatusKey::as_str)))
            .and_where(Expr::col(IssueRow::DeletedAt).is_null())
            .order_by(IssueRow::Id, Order::Asc);
        transaction
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_issue)
            .collect()
    }

    async fn insert_workflow(
        transaction: &mut Transaction,
        workflow: &Workflow,
        content: &WorkflowContent,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(WorkflowRow::Table)
            .columns([WorkflowRow::Id, WorkflowRow::Name, WorkflowRow::Version])
            .values([
                Expr::val(workflow.id.stored()),
                Expr::val(workflow.name.as_str()),
                Expr::val(workflow.version),
            ])
            .map_err(|_| StoreError::Corrupt("a workflow"))?;
        transaction.execute(&insert).await?;
        insert_content(transaction, workflow.id, content).await
    }

    async fn replace_workflow(
        transaction: &mut Transaction,
        id: WorkflowId,
        name: &str,
        content: &WorkflowContent,
        version: i64,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(WorkflowRow::Table)
            .value(WorkflowRow::Name, name)
            .value(WorkflowRow::Version, Expr::col(WorkflowRow::Version).add(1))
            .and_where(Expr::col(WorkflowRow::Id).eq(id.stored()))
            .and_where(Expr::col(WorkflowRow::Version).eq(version));
        if transaction.execute(&update).await? == 0 {
            return Ok(false);
        }
        delete_content(transaction, id).await?;
        insert_content(transaction, id, content).await?;
        Ok(true)
    }

    async fn delete_workflow(
        transaction: &mut Transaction,
        id: WorkflowId,
        version: i64,
    ) -> Result<bool, StoreError> {
        let mut select = header_select();
        select
            .and_where(Expr::col(WorkflowRow::Id).eq(id.stored()))
            .and_where(Expr::col(WorkflowRow::Version).eq(version));
        if transaction.fetch_optional(&select).await?.is_none() {
            return Ok(false);
        }
        let mut uses = Query::delete();
        uses.from_table(WorkflowUseRow::Table)
            .and_where(Expr::col(WorkflowUseRow::Workflow).eq(id.stored()));
        transaction.execute(&uses).await?;
        delete_content(transaction, id).await?;
        let mut delete = Query::delete();
        delete
            .from_table(WorkflowRow::Table)
            .and_where(Expr::col(WorkflowRow::Id).eq(id.stored()));
        Ok(transaction.execute(&delete).await? > 0)
    }

    async fn put_workflow_use(
        transaction: &mut Transaction,
        used: &WorkflowUse,
    ) -> Result<(), StoreError> {
        Self::remove_workflow_use(transaction, used.place()).await?;
        let mut insert = Query::insert();
        insert
            .into_table(WorkflowUseRow::Table)
            .columns([
                WorkflowUseRow::Project,
                WorkflowUseRow::IssueType,
                WorkflowUseRow::Workflow,
            ])
            .values([
                Expr::val(used.project.map(StoredId::stored)),
                Expr::val(used.issue_type.map(StoredId::stored)),
                Expr::val(used.workflow.stored()),
            ])
            .map_err(|_| StoreError::Corrupt("the use of a workflow"))?;
        transaction.execute(&insert).await?;
        Ok(())
    }

    async fn remove_workflow_use(
        transaction: &mut Transaction,
        place: WorkflowPlace,
    ) -> Result<(), StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(WorkflowUseRow::Table)
            .cond_where(at_place(place));
        transaction.execute(&delete).await?;
        Ok(())
    }
}
