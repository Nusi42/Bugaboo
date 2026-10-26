//! The workflows part of the in-memory store.

use bb_core::id::{IssueId, IssueTypeId, ProjectId, WorkflowId};
use bb_core::issue::Issue;
use bb_core::version::Version;
use bb_core::workflow::{StoredWorkflow, Workflow, WorkflowContent, WorkflowPlace, WorkflowUse};

use super::fake::{FakeError, FakeStore, Tx};
use crate::workflows::{DirectoryRows, StatusFilter, WorkflowStore};

impl WorkflowStore for FakeStore {
    async fn workflow_uses_in(tx: &mut Tx) -> Result<Vec<WorkflowUse>, FakeError> {
        Ok(tx.state.workflow_uses.clone())
    }

    async fn lock_workflows(_tx: &mut Tx) -> Result<(), FakeError> {
        Ok(())
    }

    async fn lock_workflows_shared(_tx: &mut Tx) -> Result<(), FakeError> {
        Ok(())
    }

    async fn workflow_directory_in(tx: &mut Tx) -> Result<DirectoryRows, FakeError> {
        let uses = tx.state.workflow_uses.clone();
        let workflows = tx
            .state
            .workflows
            .iter()
            .filter(|stored| uses.iter().any(|used| used.workflow == stored.workflow.id))
            .cloned()
            .collect();
        Ok(DirectoryRows { uses, workflows })
    }

    async fn workflow_headers_in(tx: &mut Tx) -> Result<Vec<Workflow>, FakeError> {
        let mut found: Vec<Workflow> = tx
            .state
            .workflows
            .iter()
            .map(|stored| stored.workflow.clone())
            .collect();
        found.sort_by(|a, b| (&a.name, a.id).cmp(&(&b.name, b.id)));
        Ok(found)
    }

    async fn stored_workflow_in(
        tx: &mut Tx,
        id: WorkflowId,
    ) -> Result<Option<StoredWorkflow>, FakeError> {
        Ok(tx
            .state
            .workflows
            .iter()
            .find(|stored| stored.workflow.id == id)
            .cloned())
    }

    async fn stored_workflows_in(
        tx: &mut Tx,
        ids: Vec<WorkflowId>,
    ) -> Result<Vec<StoredWorkflow>, FakeError> {
        let mut found: Vec<StoredWorkflow> = tx
            .state
            .workflows
            .iter()
            .filter(|stored| ids.contains(&stored.workflow.id))
            .cloned()
            .collect();
        found.sort_by_key(|stored| stored.workflow.id);
        Ok(found)
    }

    async fn has_issues_in(tx: &mut Tx) -> Result<bool, FakeError> {
        Ok(!tx.state.issues.is_empty())
    }

    async fn live_issues_in(
        tx: &mut Tx,
        projects: Option<Vec<ProjectId>>,
        issue_types: Option<Vec<IssueTypeId>>,
        statuses: StatusFilter,
    ) -> Result<Vec<Issue>, FakeError> {
        let mut found: Vec<Issue> = tx
            .state
            .issues
            .iter()
            .filter(|issue| {
                issue.deleted_at.is_none()
                    && projects
                        .as_ref()
                        .is_none_or(|projects| projects.contains(&issue.project))
                    && issue_types
                        .as_ref()
                        .is_none_or(|types| types.contains(&issue.issue_type))
                    && match &statuses {
                        StatusFilter::Any => true,
                        StatusFilter::In(keys) => keys.contains(&issue.status),
                        StatusFilter::NotIn(keys) => !keys.contains(&issue.status),
                    }
            })
            .cloned()
            .collect();
        found.sort_by_key(|issue| issue.id);
        Ok(found)
    }

    async fn live_issues_after_in(
        tx: &mut Tx,
        projects: Option<Vec<ProjectId>>,
        issue_types: Option<Vec<IssueTypeId>>,
        statuses: StatusFilter,
        after: Option<IssueId>,
        limit: usize,
    ) -> Result<Vec<Issue>, FakeError> {
        let all = Self::live_issues_in(tx, projects, issue_types, statuses).await?;
        Ok(all
            .into_iter()
            .filter(|issue| after.is_none_or(|after| issue.id > after))
            .take(limit)
            .collect())
    }

    async fn insert_workflow(
        tx: &mut Tx,
        workflow: &Workflow,
        content: &WorkflowContent,
    ) -> Result<(), FakeError> {
        tx.state.workflows.push(StoredWorkflow {
            workflow: workflow.clone(),
            content: content.clone(),
        });
        Ok(())
    }

    async fn replace_workflow(
        tx: &mut Tx,
        id: WorkflowId,
        name: &str,
        content: &WorkflowContent,
        version: Version,
    ) -> Result<bool, FakeError> {
        let Some(found) =
            tx.state.workflows.iter_mut().find(|stored| {
                stored.workflow.id == id && stored.workflow.version == version.get()
            })
        else {
            return Ok(false);
        };
        name.clone_into(&mut found.workflow.name);
        found.content = content.clone();
        found.workflow.version += 1;
        Ok(true)
    }

    async fn delete_workflow(
        tx: &mut Tx,
        id: WorkflowId,
        version: Version,
    ) -> Result<bool, FakeError> {
        let before = tx.state.workflows.len();
        tx.state.workflows.retain(|stored| {
            !(stored.workflow.id == id && stored.workflow.version == version.get())
        });
        let deleted = tx.state.workflows.len() != before;
        if deleted {
            tx.state.workflow_uses.retain(|used| used.workflow != id);
        }
        Ok(deleted)
    }

    async fn put_workflow_use(tx: &mut Tx, used: &WorkflowUse) -> Result<(), FakeError> {
        tx.state
            .workflow_uses
            .retain(|each| each.place() != used.place());
        tx.state.workflow_uses.push(*used);
        Ok(())
    }

    async fn remove_workflow_use(tx: &mut Tx, place: WorkflowPlace) -> Result<(), FakeError> {
        tx.state.workflow_uses.retain(|each| each.place() != place);
        Ok(())
    }
}
