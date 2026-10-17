//! Which workflow counts for a project and an issue type (ADR-0031 decision 2).
//!
//! The rows of the places a workflow is used at are read once for a request, with the workflows
//! they name; the nearest place wins (`bb_core::workflow::find`), and where nobody set a workflow
//! the starting workflow *Simple* counts.

use std::collections::HashMap;
use std::sync::Arc;

use bb_core::id::{IssueTypeId, ProjectId, WorkflowId};
use bb_core::workflow::starter::SIMPLE;
use bb_core::workflow::{WorkflowContent, WorkflowUse, find};

use crate::auth::{AppError, AuthStore, Ctx, internal};

/// The places workflows are used at, and the workflows they name.
pub(crate) struct WorkflowDirectory {
    uses: Vec<WorkflowUse>,
    workflows: HashMap<WorkflowId, Arc<WorkflowContent>>,
    starting: Arc<WorkflowContent>,
}

impl WorkflowDirectory {
    /// The workflow that counts in a place whose projects from the top down are `chain`, for an
    /// issue of this type (`None`: all types).
    pub(crate) fn in_force(
        &self,
        chain: &[ProjectId],
        issue_type: Option<IssueTypeId>,
    ) -> Arc<WorkflowContent> {
        find(chain, issue_type, &self.uses)
            .and_then(|used| self.workflows.get(&used.workflow))
            .map_or_else(|| Arc::clone(&self.starting), Arc::clone)
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The places workflows are used at, read once for the request.
    async fn workflow_directory(&self) -> Result<Arc<WorkflowDirectory>, AppError> {
        let known = self
            .known
            .workflows
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        if let Some(directory) = known {
            return Ok(directory);
        }
        let uses = self
            .store()
            .workflow_uses()
            .await
            .map_err(|error| internal::<S>(&error))?;
        let mut named: Vec<WorkflowId> = uses.iter().map(|used| used.workflow).collect();
        named.sort_unstable();
        named.dedup();
        let workflows = self
            .store()
            .stored_workflows(named)
            .await
            .map_err(|error| internal::<S>(&error))?
            .into_iter()
            .map(|stored| (stored.workflow.id, Arc::new(stored.content)))
            .collect();
        let directory = Arc::new(WorkflowDirectory {
            uses,
            workflows,
            starting: Arc::new(SIMPLE.content()),
        });
        *self
            .known
            .workflows
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Arc::clone(&directory));
        Ok(directory)
    }

    /// The workflow that counts in a place whose projects from the top down are `chain`, for an
    /// issue of this type (`None`: all types).
    pub(crate) async fn workflow_in_force(
        &self,
        chain: &[ProjectId],
        issue_type: Option<IssueTypeId>,
    ) -> Result<Arc<WorkflowContent>, AppError> {
        Ok(self.workflow_directory().await?.in_force(chain, issue_type))
    }
}
