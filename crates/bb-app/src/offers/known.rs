//! Which issue types and groups of labels a person may know of (SEC-03, SEC-06).
//!
//! The one rule, for the lists, the rows of the offers, *Check access*, the editors, and every use
//! case that takes a subject by its ID: a subject is known when it was created for the whole
//! organization, or when the person may know the project it was created in (the policy's
//! `knows_project`) or works in a project inside it, or when it is used on an issue they read (for
//! a group of labels: on an issue whose labels they may see; as for a tag). Anything else is as
//! good as not there (`resource.not_found`), so the name of a type created in a project nobody
//! told the person about is not leaked (ADR-0025 decision 13).

use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::id::{IssueTypeId, LabelGroupId, ProjectId};
use bb_core::issue::IssueType;
use bb_core::tags::LabelGroup;

use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::projects::Shown;

/// Whether the person has to be told by an issue: the subject was created in a project they may
/// not know and in none they work inside of (the one definition of knowing a project is the
/// policy's, `Evaluation::knows_project`).
fn needs_use(shown: &Shown, owner: Option<ProjectId>) -> bool {
    owner.is_some_and(|owner| !shown.knows(owner) && !shown.sees_inside(owner))
}

/// Whether a subject made in `owner` is known; `used` says whether an issue the person reads uses
/// it.
fn is_known(shown: &Shown, owner: Option<ProjectId>, used: bool) -> bool {
    !needs_use(shown, owner) || used
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The issue types in use on the issues the person reads, read once for a request.
    async fn used_types(&self, shown: &Shown) -> Result<Arc<Vec<IssueTypeId>>, AppError> {
        if let Some(known) = self
            .known
            .used_types
            .lock()
            .ok()
            .and_then(|each| each.clone())
        {
            return Ok(known);
        }
        let found = Arc::new(
            self.store()
                .used_issue_types(&shown.issue_sight())
                .await
                .map_err(|error| internal::<S>(&error))?,
        );
        if let Ok(mut each) = self.known.used_types.lock() {
            *each = Some(Arc::clone(&found));
        }
        Ok(found)
    }

    /// The groups of labels in use on the issues whose labels the person may see, read once for a
    /// request.
    async fn used_groups(&self, shown: &Shown) -> Result<Arc<Vec<LabelGroupId>>, AppError> {
        if let Some(known) = self
            .known
            .used_groups
            .lock()
            .ok()
            .and_then(|each| each.clone())
        {
            return Ok(known);
        }
        let failed = |error| internal::<S>(&error);
        let types: Vec<IssueTypeId> = self
            .store()
            .issue_types()
            .await
            .map_err(failed)?
            .iter()
            .map(|found| found.id)
            .collect();
        let rules = self
            .store()
            .level_rules_of_roles(shown.role_ids())
            .await
            .map_err(failed)?;
        let found = Arc::new(
            self.store()
                .used_label_groups(&shown.label_sight(rules, &types))
                .await
                .map_err(failed)?,
        );
        if let Ok(mut each) = self.known.used_groups.lock() {
            *each = Some(Arc::clone(&found));
        }
        Ok(found)
    }

    /// The issue types of the list that the person may know. One more read only when some type was
    /// made in a project they do not see.
    pub(crate) async fn known_types(
        &self,
        shown: &Shown,
        types: Vec<IssueType>,
    ) -> Result<Vec<IssueType>, AppError> {
        let used: Arc<Vec<IssueTypeId>> =
            if types.iter().any(|found| needs_use(shown, found.project)) {
                self.used_types(shown).await?
            } else {
                Arc::default()
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
        let used: Arc<Vec<LabelGroupId>> =
            if groups.iter().any(|found| needs_use(shown, found.project)) {
                self.used_groups(shown).await?
            } else {
                Arc::default()
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
