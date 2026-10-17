//! No issue is left in a status its workflow does not have (ADR-0031 amendment 5).
//!
//! One check, "every live issue that is governed after the change has a status of its workflow",
//! runs on the places and workflows as they will be after the change, inside the transaction that
//! makes the change: *Publish*, a place that uses another workflow (or follows its parent again),
//! and taking a workflow away all call it, and all take the one lock of the workflows. The places
//! the check looks at are read in the transaction too, after the lock, so what it looks at is what
//! is stored when it runs. An issue that is written with a status takes the same lock shared and
//! looks at the workflow again ([`ensure_status_in_force_in`]), so the two never miss each other.
//! Issues in the trash keep their status; when one comes back it starts again in the starting
//! status.

use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::event::RecordKind;
use bb_core::id::{IssueId, IssueTypeId, ProjectId, WorkflowId};
use bb_core::issue::Issue;
use bb_core::version::counted_up;
use bb_core::workflow::{StatusKey, WorkflowContent, WorkflowPlace, WorkflowUse};
use bb_policy::Tree;
use jiff::Timestamp;

use super::resolve::WorkflowDirectory;
use super::store::StatusFilter;
use crate::auth::{AppError, AuthStore, Ctx, EventMaker, entity, internal};
use crate::issues::member as issue_member;
use crate::projects::Why;

/// The issues a change to the workflows of some places can reach: the projects (with their
/// sub-projects) and issue types of those places. A part that is `None` reaches everything, so the
/// reach is never smaller than the truth.
pub(crate) struct Reach {
    projects: Option<Vec<ProjectId>>,
    issue_types: Option<Vec<IssueTypeId>>,
}

impl Reach {
    /// What these places reach, given the tree of the projects.
    pub(crate) fn of(places: &[WorkflowPlace], tree: &Tree) -> Self {
        let projects = if places.iter().any(|place| place.project.is_none()) {
            None
        } else {
            Some(
                tree.projects()
                    .filter(|project| {
                        places.iter().any(|place| {
                            place
                                .project
                                .is_some_and(|inside| tree.is_within(*project, inside))
                        })
                    })
                    .collect(),
            )
        };
        let issue_types = if places.iter().any(|place| place.issue_type.is_none()) {
            None
        } else {
            Some(places.iter().filter_map(|place| place.issue_type).collect())
        };
        Self {
            projects,
            issue_types,
        }
    }
}

/// The places a workflow is used at.
pub(crate) fn places_of(uses: &[WorkflowUse], workflow: WorkflowId) -> Vec<WorkflowPlace> {
    uses.iter()
        .filter(|used| used.workflow == workflow)
        .map(WorkflowUse::place)
        .collect()
}

/// What a change to the workflows can reach, read in the transaction: the places that this
/// workflow is used at now.
///
/// # Errors
/// The error of the store.
pub(crate) async fn reach_of_workflow_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    workflow: WorkflowId,
) -> Result<Reach, S::Error> {
    let tree = Tree::of(&S::project_tree_in(transaction).await?);
    let uses = S::workflow_uses_in(transaction).await?;
    Ok(Reach::of(&places_of(&uses, workflow), &tree))
}

/// What a change to the workflow of one place can reach, read in the transaction.
///
/// # Errors
/// The error of the store.
pub(crate) async fn reach_of_place_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    place: WorkflowPlace,
) -> Result<Reach, S::Error> {
    let tree = Tree::of(&S::project_tree_in(transaction).await?);
    Ok(Reach::of(&[place], &tree))
}

/// How many live issues in the reach are in a status that the workflow governing them does not
/// have, on the uses and workflows stored in this transaction (so, after the change it made).
///
/// # Errors
/// The error of the store.
pub(crate) async fn count_stranded_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    reach: &Reach,
) -> Result<usize, S::Error> {
    let tree = Tree::of(&S::project_tree_in(transaction).await?);
    let rows = S::workflow_directory_in(transaction).await?;
    let directory = WorkflowDirectory::build(rows.uses, rows.workflows);
    // Only the issues in a status that not every workflow has can be stranded.
    let issues = S::live_issues_in(
        transaction,
        reach.projects.clone(),
        reach.issue_types.clone(),
        StatusFilter::NotIn(directory.statuses_of_every_workflow()),
    )
    .await?;
    Ok(issues
        .iter()
        .filter(|issue| {
            let chain = tree.chain(issue.project).unwrap_or_default();
            directory
                .in_force(&chain, Some(issue.issue_type))
                .status(&issue.status)
                .is_none()
        })
        .count())
}

/// The workflow that counts for an issue of this type in this project, on what is stored in this
/// transaction.
///
/// # Errors
/// The error of the store.
pub(crate) async fn content_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    project: ProjectId,
    issue_type: IssueTypeId,
) -> Result<Arc<WorkflowContent>, S::Error> {
    let tree = Tree::of(&S::project_tree_in(transaction).await?);
    let rows = S::workflow_directory_in(transaction).await?;
    let directory = WorkflowDirectory::build(rows.uses, rows.workflows);
    let chain = tree.chain(project).unwrap_or_default();
    Ok(directory.in_force(&chain, Some(issue_type)))
}

/// Inside the transaction that writes an issue with this status: takes the lock of the workflows
/// shared and checks that the workflow that counts now still has the status, which the request was
/// judged on. If a workflow was published meanwhile and dropped it, the write is stopped with
/// `edit.version_changed`, and the person looks again.
///
/// # Errors
/// The error of the store, or the stop of the change.
pub(crate) async fn ensure_status_in_force_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    project: ProjectId,
    issue_type: IssueTypeId,
    status: &StatusKey,
    stopped: &Why,
) -> Result<(), S::Error> {
    S::lock_workflows_shared(transaction).await?;
    let content = content_in::<S>(transaction, project, issue_type).await?;
    if content.status(status).is_none() {
        return Err(stopped.stop::<S>(ErrorCode::VersionChanged));
    }
    Ok(())
}

/// [`ensure_status_in_force_in`] for an issue that is written: only when its status or its type
/// differs from what is stored, since only then a workflow was looked at.
///
/// # Errors
/// The error of the store, or the stop of the change.
pub(crate) async fn ensure_written_status_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    written: &Issue,
    stored: Option<&Issue>,
    stopped: &Why,
) -> Result<(), S::Error> {
    if stored
        .is_some_and(|each| each.status == written.status && each.issue_type == written.issue_type)
    {
        return Ok(());
    }
    ensure_status_in_force_in::<S>(
        transaction,
        written.project,
        written.issue_type,
        &written.status,
        stopped,
    )
    .await
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The live issues that this workflow governs and that are in one of these statuses, found
    /// from the places the workflow is used at and no others.
    pub(crate) async fn governed_issues(
        &self,
        directory: &WorkflowDirectory,
        workflow: WorkflowId,
        keys: &[StatusKey],
    ) -> Result<Vec<Issue>, AppError> {
        let failed = |error| internal::<S>(&error);
        let tree = Tree::of(&self.store().project_tree().await.map_err(failed)?);
        let reach = Reach::of(&places_of(directory.uses(), workflow), &tree);
        let found = self
            .store()
            .live_issues(
                reach.projects,
                reach.issue_types,
                StatusFilter::In(keys.to_vec()),
            )
            .await
            .map_err(failed)?;
        Ok(found
            .into_iter()
            .filter(|issue| {
                let chain = tree.chain(issue.project).unwrap_or_default();
                directory.id_in_force(&chain, Some(issue.issue_type)) == Some(workflow)
            })
            .collect())
    }
}

/// Puts an issue that is outside its workflow back into the starting status of that workflow, with
/// its history, inside the transaction. Says whether it did. The status and group come from the
/// workflow on the stored places, which the caller read after taking the lock.
async fn restart_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    events: &EventMaker,
    (directory, tree): (&WorkflowDirectory, &Tree),
    issue: &Issue,
    now: Timestamp,
) -> Result<bool, S::Error> {
    let chain = tree.chain(issue.project).unwrap_or_default();
    let content = directory.in_force(&chain, Some(issue.issue_type));
    if content.status(&issue.status).is_some() {
        return Ok(false);
    }
    let Some(start) = content.initial().and_then(|key| content.status(key)) else {
        return Ok(false);
    };
    let next = Issue {
        status: start.key.clone(),
        closed_at: start.group.closed_after(issue.closed_at, now),
        updated_at: now,
        ..issue.clone()
    };
    if !S::update_issue(transaction, &next, issue.version).await? {
        return Err(S::aborted());
    }
    let followed = events
        .create(
            entity(RecordKind::Issue, issue.id),
            vec![
                issue_member::status_change(&issue.status, &start.key),
                counted_up(issue.version),
            ],
        )
        .map_err(|_| S::aborted())?;
    S::append_follow_up_event(transaction, &followed).await?;
    Ok(true)
}

/// The places and workflows as they are stored in this transaction, after the shared lock.
async fn stored_places_in<S: AuthStore>(
    transaction: &mut S::Transaction,
) -> Result<(WorkflowDirectory, Tree), S::Error> {
    S::lock_workflows_shared(transaction).await?;
    let tree = Tree::of(&S::project_tree_in(transaction).await?);
    let rows = S::workflow_directory_in(transaction).await?;
    Ok((WorkflowDirectory::build(rows.uses, rows.workflows), tree))
}

/// An issue that comes back from the trash starts again in the starting status when its workflow
/// has no such status as it has, with history. Inside the change that took it out.
///
/// # Errors
/// The error of the store.
pub(crate) async fn restart_restored_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    events: &EventMaker,
    issue: IssueId,
    now: Timestamp,
) -> Result<(), S::Error> {
    let Some(stored) = S::issue_in(transaction, issue).await? else {
        return Ok(());
    };
    let (directory, tree) = stored_places_in::<S>(transaction).await?;
    restart_in::<S>(transaction, events, (&directory, &tree), &stored, now).await?;
    Ok(())
}

/// After a project moved: the live issues of it and of everything inside it whose workflow lacks
/// the status they have start again in the starting status, with history. Inside the move.
///
/// # Errors
/// The error of the store.
pub(crate) async fn restart_moved_in<S: AuthStore>(
    transaction: &mut S::Transaction,
    events: &EventMaker,
    project: ProjectId,
    now: Timestamp,
) -> Result<(), S::Error> {
    let (directory, tree) = stored_places_in::<S>(transaction).await?;
    let inside = tree.subtree(project);
    let issues = S::live_issues_in(
        transaction,
        Some(inside),
        None,
        StatusFilter::NotIn(directory.statuses_of_every_workflow()),
    )
    .await?;
    for issue in &issues {
        restart_in::<S>(transaction, events, (&directory, &tree), issue, now).await?;
    }
    Ok(())
}
