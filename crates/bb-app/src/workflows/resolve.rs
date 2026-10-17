//! Which workflow counts for a project and an issue type (ADR-0031 decision 2).
//!
//! The rows of the places a workflow is used at are read once for a request, with the workflows
//! they name; the nearest place wins (`bb_core::workflow::find`), and where nobody set a workflow
//! the starting workflow *Simple* counts.

use std::collections::HashMap;
use std::sync::Arc;

use bb_core::id::{IssueTypeId, ProjectId, WorkflowId};
use bb_core::workflow::starter::{SIMPLE, STARTERS};
use bb_core::workflow::{StatusKey, StoredWorkflow, Workflow, WorkflowContent, WorkflowUse, find};

use super::types::StatusView;
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
    /// The places workflows are used at, with the workflows they name.
    pub(crate) fn build(uses: Vec<WorkflowUse>, stored: Vec<StoredWorkflow>) -> Self {
        Self {
            uses,
            workflows: stored
                .into_iter()
                .map(|each| {
                    (
                        each.workflow.id,
                        Stored {
                            workflow: each.workflow,
                            content: Arc::new(each.content),
                        },
                    )
                })
                .collect(),
            starting: Arc::new(SIMPLE.content()),
        }
    }

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

    /// The status with this key as a starter workflow or, failing that, another workflow has it:
    /// what an issue in a status of no workflow it counts under is shown as.
    pub(crate) fn named(&self, key: &StatusKey) -> Option<StatusView> {
        STARTERS
            .iter()
            .find_map(|starter| starter.content().status(key).map(StatusView::from))
            .or_else(|| {
                let mut ids: Vec<&WorkflowId> = self.workflows.keys().collect();
                ids.sort_unstable();
                ids.into_iter().find_map(|id| {
                    self.workflows
                        .get(id)
                        .and_then(|stored| stored.content.status(key).map(StatusView::from))
                })
            })
    }

    /// The keys of the statuses that every workflow here has, the starting workflow too: an issue
    /// in any other status may be in a status that the workflow governing it lacks.
    pub(crate) fn statuses_of_every_workflow(&self) -> Vec<StatusKey> {
        self.starting
            .statuses
            .iter()
            .map(|status| status.key.clone())
            .filter(|key| {
                self.workflows
                    .values()
                    .all(|stored| stored.content.status(key).is_some())
            })
            .collect()
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
        let rows = self
            .store()
            .workflow_directory()
            .await
            .map_err(|error| internal::<S>(&error))?;
        let directory = Arc::new(WorkflowDirectory::build(rows.uses, rows.workflows));
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

#[cfg(test)]
mod tests {
    use bb_core::issue::StatusGroup;

    use super::*;

    #[test]
    fn wf_01_an_issue_outside_its_workflow_shows_the_name_a_starter_gives_its_key() {
        let directory = WorkflowDirectory::build(Vec::new(), Vec::new());
        let testing = directory.named(&StatusKey::new("testing").unwrap());
        assert_eq!(
            testing.map(|each| (each.name, each.group)),
            Some(("Testing".to_owned(), StatusGroup::InProgress))
        );
        // A key that no workflow has is the only one shown as it is.
        assert!(
            directory
                .named(&StatusKey::new("nowhere").unwrap())
                .is_none()
        );
    }
}
