//! Which issue types and groups of labels a person may know of (SEC-03, SEC-06).
//!
//! The one rule, for the lists, the rows of the offers, *Check access* and every use case that
//! takes a subject by its ID: a subject is known when it was made for the whole organization, or
//! when the person sees the project it was made in, or when it is used on an issue they read
//! (as for a tag). Anything else is as good as not there (`resource.not_found`), so the name of a
//! type made in a project nobody told the person about is not leaked (ADR-0025 decision 13).

use bb_core::ErrorCode;
use bb_core::id::{IssueTypeId, LabelGroupId, ProjectId};
use bb_core::issue::IssueType;
use bb_core::tags::LabelGroup;

use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::projects::Shown;

/// Whether the person has to be told by an issue: the subject was made in a project they do not
/// see.
fn needs_use(shown: &Shown, owner: Option<ProjectId>) -> bool {
    owner.is_some_and(|owner| !shown.sees(owner))
}

/// Whether a subject made in `owner` is known; `used` says whether an issue the person reads uses
/// it.
fn is_known(shown: &Shown, owner: Option<ProjectId>, used: bool) -> bool {
    !needs_use(shown, owner) || used
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The issue types of the list that the person may know. One more read only when some type was
    /// made in a project they do not see.
    pub(crate) async fn known_types(
        &self,
        shown: &Shown,
        types: Vec<IssueType>,
    ) -> Result<Vec<IssueType>, AppError> {
        let used: Vec<IssueTypeId> = if types.iter().any(|found| needs_use(shown, found.project)) {
            self.store()
                .used_issue_types(&shown.issue_sight())
                .await
                .map_err(|error| internal::<S>(&error))?
        } else {
            Vec::new()
        };
        Ok(types
            .into_iter()
            .filter(|found| is_known(shown, found.project, used.contains(&found.id)))
            .collect())
    }

    /// The groups of labels of the list that the person may know.
    pub(crate) async fn known_groups(
        &self,
        shown: &Shown,
        groups: Vec<LabelGroup>,
    ) -> Result<Vec<LabelGroup>, AppError> {
        let used: Vec<LabelGroupId> = if groups.iter().any(|found| needs_use(shown, found.project))
        {
            self.store()
                .used_label_groups(&shown.issue_sight())
                .await
                .map_err(|error| internal::<S>(&error))?
        } else {
            Vec::new()
        };
        Ok(groups
            .into_iter()
            .filter(|found| is_known(shown, found.project, used.contains(&found.id)))
            .collect())
    }

    /// The issue type, or `resource.not_found` for one that is not there or that the person may
    /// not know.
    pub(crate) async fn known_type(
        &self,
        shown: &Shown,
        id: IssueTypeId,
    ) -> Result<IssueType, AppError> {
        let found = self
            .store()
            .issue_type(id)
            .await
            .map_err(|error| internal::<S>(&error))?
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        self.known_types(shown, vec![found])
            .await?
            .pop()
            .ok_or(AppError::Refused(ErrorCode::NotFound))
    }

    /// The group of labels, or `resource.not_found` for one that is not there or that the person
    /// may not know.
    pub(crate) async fn known_group(
        &self,
        shown: &Shown,
        id: LabelGroupId,
    ) -> Result<LabelGroup, AppError> {
        let found = self
            .store()
            .label_groups()
            .await
            .map_err(|error| internal::<S>(&error))?
            .into_iter()
            .find(|each| each.id == id)
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        self.known_groups(shown, vec![found])
            .await?
            .pop()
            .ok_or(AppError::Refused(ErrorCode::NotFound))
    }
}
