//! The place an editor is asked for (ADR-0027, ADR-0031): the whole organization, or a project with
//! the chain of projects above it, and the issue type when it is for one type only. The editors of
//! layouts and of workflows ask for their rows the same way, so they share this; a layout adds the
//! kind of page it is (`K`), a workflow nothing.

use bb_core::ErrorCode;
use bb_core::id::{IssueTypeId, ProjectId};
use bb_core::project::Project;
use bb_policy::Place;

use crate::auth::{AppError, AuthStore, Ctx};
use crate::projects::ProjectRef;

/// The place an editor is asked for.
pub(crate) struct EditorPlace<K = ()> {
    /// The project; none for the whole organization.
    pub(crate) project: Option<ProjectRef>,
    /// The projects from the top down to the place; empty for the organization.
    pub(crate) chain: Vec<ProjectId>,
    /// The issue type; none for all types.
    pub(crate) issue_type: Option<IssueTypeId>,
    /// What else the editor is about (the issue page or the new-issue form for a layout).
    pub(crate) kind: K,
}

impl<K> EditorPlace<K> {
    /// The place for the permissions.
    pub(crate) fn place(&self) -> Place {
        self.project
            .as_ref()
            .map_or(Place::Organization, |project| Place::Project(project.id))
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The place of a request for an editor: the project must exist for the person.
    pub(crate) async fn editor_place<K>(
        &self,
        project: Option<&str>,
        issue_type: Option<IssueTypeId>,
        kind: K,
    ) -> Result<EditorPlace<K>, AppError> {
        let (found, chain) = self.place_of_request(project).await?;
        Ok(EditorPlace {
            project: found.as_ref().map(ProjectRef::from),
            chain,
            issue_type,
            kind,
        })
    }

    /// The project a request for an editor names (it must exist for the person) and the projects
    /// from the top down to it: both empty for the organization.
    pub(crate) async fn place_of_request(
        &self,
        project: Option<&str>,
    ) -> Result<(Option<Project>, Vec<ProjectId>), AppError> {
        let found = match project {
            Some(address) => Some(
                self.find_project(address)
                    .await?
                    .map(|found| found.project)
                    .ok_or(AppError::Refused(ErrorCode::NotFound))?,
            ),
            None => None,
        };
        let chain = match &found {
            Some(project) => self.chain_of(project.id).await?,
            None => Vec::new(),
        };
        Ok((found, chain))
    }
}
