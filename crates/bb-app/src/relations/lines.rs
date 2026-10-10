//! Issues as lines of a list, for a viewer: the sub-issues, the related issues, the issues named in
//! the history and in the *Mentions* block.
//!
//! The one place that decides whether a viewer sees an issue in such a list (the issue is not in
//! the trash and `Evaluation::sees_issue` says yes in the project of that issue) and which fields
//! of it they see there. A hidden issue is not a line: it is left out of the list, so a list never
//! tells that something is hidden.

use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::fields::Field;
use bb_core::id::{IssueId, IssueTypeId, ProjectId};
use bb_core::issue::Issue;
use bb_core::issue_key::IssueKey;
use bb_core::project::Project;
use bb_policy::{Evaluation, Place};

use super::types::IssueLine;
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::issues::IssueTypeRef;
use crate::projects::ProjectRef;

/// What turns issues into lines for one viewer: the issue types and the projects are read once
/// each.
pub(crate) struct LineMaker {
    types: BTreeMap<IssueTypeId, IssueTypeRef>,
    projects: HashMap<ProjectId, Option<Project>>,
}

impl LineMaker {
    /// What an issue type is called.
    pub(crate) fn type_name(&self, id: IssueTypeId) -> Option<&str> {
        self.types.get(&id).map(|found| found.name.as_str())
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// An issue the actor may see, by its readable or internal ID, with what they may do in its
    /// project. An issue that does not exist, is in the trash or that they may not see answers
    /// `resource.not_found`: the three cannot be told apart.
    pub(crate) async fn visible_issue(
        &self,
        actor: &Actor,
        address: &str,
    ) -> Result<(Issue, Arc<Evaluation>), AppError> {
        let not_found = || AppError::Refused(ErrorCode::NotFound);
        let found = self.find_issue(address).await?;
        let place = found.as_ref().map_or(Place::Organization, |found| {
            Place::Project(found.issue.project)
        });
        let held = self.evaluation_of(actor, place).await?;
        let issue = found
            .map(|found| found.issue)
            .filter(|issue| issue.deleted_at.is_none() && held.sees_issue(issue.reported_by))
            .ok_or_else(not_found)?;
        Ok((issue, held))
    }

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
        })
    }

    /// The line of `issue` for the actor, or `None` when they may not see it.
    pub(crate) async fn line_of(
        &self,
        actor: &Actor,
        maker: &mut LineMaker,
        issue: &Issue,
    ) -> Result<Option<IssueLine>, AppError> {
        if issue.deleted_at.is_some() {
            return Ok(None);
        }
        let held = match self
            .evaluation_of(actor, Place::Project(issue.project))
            .await
        {
            Ok(held) => held,
            Err(AppError::Refused(ErrorCode::NotFound)) => return Ok(None),
            Err(error) => return Err(error),
        };
        if !held.sees_issue(issue.reported_by) {
            return Ok(None);
        }
        if let Entry::Vacant(slot) = maker.projects.entry(issue.project) {
            let project = self
                .store()
                .project(issue.project)
                .await
                .map_err(|error| internal::<S>(&error))?;
            slot.insert(project);
        }
        let Some(Some(project)) = maker.projects.get(&issue.project) else {
            return Ok(None);
        };
        let Some(issue_type) = maker.types.get(&issue.issue_type).cloned() else {
            return Err(AppError::Internal(
                "an issue has an issue type that is gone".into(),
            ));
        };
        let mask = held.field_mask();
        let status_seen = mask.can_see(Field::Status);
        Ok(Some(IssueLine {
            id: issue.id,
            key: IssueKey::new(project.short_name.clone(), issue.code.clone()).to_string(),
            project: ProjectRef::from(project),
            issue_type,
            title: mask.can_see(Field::Title).then(|| issue.title.clone()),
            status: status_seen.then_some(issue.status),
            status_group: status_seen.then(|| issue.status.group()),
        }))
    }

    /// The issues with these IDs, in the order of the IDs that exist.
    pub(crate) async fn issues_named(&self, ids: &[IssueId]) -> Result<Vec<Issue>, AppError> {
        self.store()
            .issues_by_ids(ids)
            .await
            .map_err(|error| internal::<S>(&error))
    }
}
