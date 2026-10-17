//! Issues as lines of a list, for a viewer: the sub-issues, the related issues, the issues named in
//! the history and in the *Mentions* block.
//!
//! The one place that decides whether a viewer sees an issue in such a list (the issue is not in
//! the trash and `Evaluation::sees_issue` says yes in the project of that issue) and which fields
//! of it they see there. A hidden issue is not a line: it is left out of the list, so a list never
//! tells that something is hidden.

use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap};

use bb_core::fields::Field;
use bb_core::id::{IssueId, IssueTypeId, ProjectId};
use bb_core::issue::Issue;
use bb_core::project::Project;
use bb_core::readable_id::ReadableId;
use bb_core::workflow::WorkflowContent;

use super::types::IssueLine;
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::issues::{IssueTypeRef, Where};
use crate::projects::ProjectRef;
use crate::workflows::StatusView;
use std::sync::Arc;

/// What turns issues into lines for one viewer: the issue types and the projects are read once
/// each.
#[derive(Default)]
pub(crate) struct LineMaker {
    types: BTreeMap<IssueTypeId, IssueTypeRef>,
    projects: HashMap<ProjectId, Option<Project>>,
    /// The workflow that counts for each project and issue type of the lines so far.
    workflows: HashMap<(ProjectId, IssueTypeId), Arc<WorkflowContent>>,
}

impl LineMaker {
    /// What an issue type is called.
    pub(crate) fn type_name(&self, id: IssueTypeId) -> Option<&str> {
        self.types.get(&id).map(|found| found.name.as_str())
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The maker of lines for a request.
    pub(crate) async fn line_maker(&self) -> Result<LineMaker, AppError> {
        let types = self
            .store()
            .issue_types()
            .await
            .map_err(|error| internal::<S>(&error))?
            .iter()
            .map(|found| (found.id, IssueTypeRef::from(found)))
            .collect();
        Ok(LineMaker {
            types,
            projects: HashMap::new(),
            workflows: HashMap::new(),
        })
    }

    /// The line of `issue` for the actor, or `None` when they may not see it.
    pub(crate) async fn line_of(
        &self,
        actor: &Actor,
        maker: &mut LineMaker,
        issue: &Issue,
    ) -> Result<Option<IssueLine>, AppError> {
        let Some(held) = self.seen_in(actor, issue, Where::Live).await? else {
            return Ok(None);
        };
        if let Entry::Vacant(slot) = maker.projects.entry(issue.project) {
            let project = self
                .store()
                .project(issue.project)
                .await
                .map_err(|error| internal::<S>(&error))?;
            slot.insert(project);
        }
        if !matches!(maker.projects.get(&issue.project), Some(Some(_))) {
            return Ok(None);
        }
        let Some(issue_type) = maker.types.get(&issue.issue_type).cloned() else {
            return Err(AppError::Internal(
                "an issue has an issue type that is gone".into(),
            ));
        };
        let mask = held.field_mask_for(Some(issue.issue_type));
        let status_seen = mask.can_see(Field::Status);
        let status = if status_seen {
            let workflow = self.line_workflow(maker, issue, held.chain()).await?;
            let directory = self.workflow_directory().await?;
            Some(StatusView::of_issue(&workflow, issue, &directory))
        } else {
            None
        };
        let Some(Some(project)) = maker.projects.get(&issue.project) else {
            return Ok(None);
        };
        Ok(Some(IssueLine {
            id: issue.id,
            key: ReadableId::new(project.short_name.clone(), issue.code.clone()).to_string(),
            project: ProjectRef::from(project),
            issue_type,
            title: mask.can_see(Field::Title).then(|| issue.title.clone()),
            status,
        }))
    }

    /// The workflow that counts for the project and the issue type of an issue.
    async fn line_workflow(
        &self,
        maker: &mut LineMaker,
        issue: &Issue,
        chain: &[ProjectId],
    ) -> Result<Arc<WorkflowContent>, AppError> {
        let place = (issue.project, issue.issue_type);
        if let Some(found) = maker.workflows.get(&place) {
            return Ok(Arc::clone(found));
        }
        let found = self
            .workflow_in_force(chain, Some(issue.issue_type))
            .await?;
        maker.workflows.insert(place, Arc::clone(&found));
        Ok(found)
    }

    /// The issues with these IDs, in the order of the IDs that exist.
    pub(crate) async fn issues_named(&self, ids: &[IssueId]) -> Result<Vec<Issue>, AppError> {
        self.store()
            .issues_by_ids(ids)
            .await
            .map_err(|error| internal::<S>(&error))
    }
}
