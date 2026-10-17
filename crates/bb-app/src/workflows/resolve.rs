//! Which workflow counts for a project and an issue type (ADR-0031 decision 2).
//!
//! The rows of the places a workflow is used at are read once for a request, with the workflows
//! they name; the nearest place wins (`bb_core::workflow::find`), and where nobody set a workflow
//! the starting workflow *Simple* counts.

use std::collections::HashMap;
use std::sync::Arc;

use bb_core::id::{IssueTypeId, ProjectId, WorkflowId};
use bb_core::workflow::starter::SIMPLE;
use bb_core::workflow::{Workflow, WorkflowContent, WorkflowUse, find};

use crate::auth::{AppError, AuthStore, Ctx, internal};

/// A workflow that is stored, with its content shared.
pub(crate) struct Stored {
    pub(crate) workflow: Workflow,
    pub(crate) content: Arc<WorkflowContent>,
}

/// The workflow that counts in a place, with where it is set.
pub(crate) struct InForce {
    /// The place it is set for; none while the starting workflow counts.
    pub(crate) used: Option<WorkflowUse>,
    /// The workflow, when one is stored.
    pub(crate) workflow: Option<Workflow>,
    /// What it holds.
    pub(crate) content: Arc<WorkflowContent>,
}

/// The places workflows are used at, and the workflows they name.
pub(crate) struct WorkflowDirectory {
    uses: Vec<WorkflowUse>,
    workflows: HashMap<WorkflowId, Stored>,
    starting: Arc<WorkflowContent>,
}

impl WorkflowDirectory {
    /// The workflow that counts in a place whose projects from the top down are `chain`, for an
    /// issue of this type (`None`: all types), and where it is set.
    pub(crate) fn detail(&self, chain: &[ProjectId], issue_type: Option<IssueTypeId>) -> InForce {
        let found = find(chain, issue_type, &self.uses).and_then(|used| {
            self.workflows
                .get(&used.workflow)
                .map(|stored| (used, stored))
        });
        match found {
            Some((used, stored)) => InForce {
                used: Some(*used),
                workflow: Some(stored.workflow.clone()),
                content: Arc::clone(&stored.content),
            },
            None => InForce {
                used: None,
                workflow: None,
                content: Arc::clone(&self.starting),
            },
        }
    }

    /// The workflow that counts, without where it is set.
    pub(crate) fn in_force(
        &self,
        chain: &[ProjectId],
        issue_type: Option<IssueTypeId>,
    ) -> Arc<WorkflowContent> {
        self.detail(chain, issue_type).content
    }

    /// The ID of the stored workflow that counts, if one is stored.
    pub(crate) fn id_in_force(
        &self,
        chain: &[ProjectId],
        issue_type: Option<IssueTypeId>,
    ) -> Option<WorkflowId> {
        self.detail(chain, issue_type)
            .workflow
            .map(|workflow| workflow.id)
    }

    /// The same places without one workflow: the places that used it follow their parent.
    pub(crate) fn without(&self, workflow: WorkflowId) -> Self {
        Self {
            uses: self
                .uses
                .iter()
                .filter(|used| used.workflow != workflow)
                .copied()
                .collect(),
            workflows: self
                .workflows
                .iter()
                .filter(|(id, _)| **id != workflow)
                .map(|(id, stored)| {
                    (
                        *id,
                        Stored {
                            workflow: stored.workflow.clone(),
                            content: Arc::clone(&stored.content),
                        },
                    )
                })
                .collect(),
            starting: Arc::clone(&self.starting),
        }
    }

    /// Every place a workflow is used at.
    pub(crate) fn uses(&self) -> &[WorkflowUse] {
        &self.uses
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The places workflows are used at, read once for the request.
    pub(crate) async fn workflow_directory(&self) -> Result<Arc<WorkflowDirectory>, AppError> {
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
            .map(|stored| {
                (
                    stored.workflow.id,
                    Stored {
                        workflow: stored.workflow,
                        content: Arc::new(stored.content),
                    },
                )
            })
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

    /// Forgets what this request read about workflows: a change was made, and the next question
    /// reads the store again.
    pub(crate) fn forget_workflows(&self) {
        *self
            .known
            .workflows
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
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
