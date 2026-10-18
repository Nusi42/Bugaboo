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
use bb_core::tags::{Label, LabelGroup};

use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::offers::Here;
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

/// Whether a label is known, given the groups of labels the person may know: it is in no group, or
/// in one of them. The one definition of it, for the lists, the changes that name a label and the
/// history.
pub(crate) fn label_is_known(group: Option<LabelGroupId>, known: &[LabelGroup]) -> bool {
    group.is_none_or(|group| known.iter().any(|each| each.id == group))
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

    /// Refuses (`resource.not_found`) a label that is in a group of labels the person may not
    /// know: a label of a group nobody told them about is as good as not there.
    pub(crate) async fn ensure_label_known(
        &self,
        actor: &Actor,
        label: &Label,
    ) -> Result<(), AppError> {
        match label.group {
            Some(group) => self.known_group(actor, group).await.map(|_| ()),
            None => Ok(()),
        }
    }

    /// The issue types a list holds: those the person may know, and, in a list about a project, only
    /// those that can reach it (`Here::reached_by`).
    pub(crate) async fn listed_types(
        &self,
        shown: &Shown,
        here: Option<&Here>,
        types: Vec<IssueType>,
    ) -> Result<Vec<IssueType>, AppError> {
        Ok(self
            .known_types(shown, types)
            .await?
            .into_iter()
            .filter(|found| here.is_none_or(|here| here.reached_by(found.project)))
            .collect())
    }

    /// The groups of labels a list holds, by the same rule as [`Self::listed_types`].
    pub(crate) async fn listed_group_rows(
        &self,
        shown: &Shown,
        here: Option<&Here>,
        groups: Vec<LabelGroup>,
    ) -> Result<Vec<LabelGroup>, AppError> {
        Ok(self
            .known_groups(shown, groups)
            .await?
            .into_iter()
            .filter(|found| here.is_none_or(|here| here.reached_by(found.project)))
            .collect())
    }

    /// The issue type, or `resource.not_found` for one that is not there or that the person may
    /// not know. A type of the organization is known to everybody, so only one created in a project
    /// costs a look at what the person knows.
    pub(crate) async fn known_type(
        &self,
        actor: &Actor,
        id: IssueTypeId,
    ) -> Result<IssueType, AppError> {
        let found = self
            .store()
            .issue_type(id)
            .await
            .map_err(|error| internal::<S>(&error))?
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        if found.project.is_none() {
            return Ok(found);
        }
        let shown = self.shown_to(actor).await?;
        self.known_types(&shown, vec![found])
            .await?
            .pop()
            .ok_or(AppError::Refused(ErrorCode::NotFound))
    }

    /// Refuses (`resource.not_found`) an issue type a request names that the person may not know:
    /// a screen that is for one issue type does not open as if it were for all of them.
    pub(crate) async fn ensure_type_known(
        &self,
        actor: &Actor,
        issue_type: Option<IssueTypeId>,
    ) -> Result<(), AppError> {
        match issue_type {
            Some(id) => self.known_type(actor, id).await.map(|_| ()),
            None => Ok(()),
        }
    }

    /// The group of labels, or `resource.not_found` for one that is not there or that the person
    /// may not know.
    pub(crate) async fn known_group(
        &self,
        actor: &Actor,
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
        if found.project.is_none() {
            return Ok(found);
        }
        let shown = self.shown_to(actor).await?;
        self.known_groups(&shown, vec![found])
            .await?
            .pop()
            .ok_or(AppError::Refused(ErrorCode::NotFound))
    }
}
