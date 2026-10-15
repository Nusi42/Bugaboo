//! The one way to find an issue a person may see by its ID. An issue that does not exist, is in the
//! trash (or, when asked for the trash, is not in it) or that the person may not see answers
//! `resource.not_found`: the three cannot be told apart.

use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::id::IssueId;
use bb_core::issue::Issue;
use bb_policy::{Evaluation, Place};

use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};

impl<S: AuthStore> Ctx<'_, S> {
    /// An issue the actor may see, by its readable or internal ID, with what they may do in its
    /// project. An issue that does not exist, is in the trash or that they may not see answers
    /// `resource.not_found`.
    pub(crate) async fn visible_issue(
        &self,
        actor: &Actor,
        address: &str,
    ) -> Result<(Issue, Arc<Evaluation>), AppError> {
        self.seen_issue(actor, address, Where::Live).await
    }

    /// An issue in the trash that the actor may see: the same as `visible_issue` for the issues
    /// that were deleted (*Undo*).
    pub(crate) async fn trashed_issue(
        &self,
        actor: &Actor,
        address: &str,
    ) -> Result<(Issue, Arc<Evaluation>), AppError> {
        self.seen_issue(actor, address, Where::Trash).await
    }

    /// The issue with this internal ID, when the actor may see it (it is not in the trash and
    /// they see it in its project), with what they may do there. `None` for an issue that does
    /// not exist, is in the trash or that they may not see, and for a project that is gone.
    pub(crate) async fn seen_issue_by_id(
        &self,
        actor: &Actor,
        id: IssueId,
    ) -> Result<Option<(Issue, Arc<Evaluation>)>, AppError> {
        let found = self
            .store()
            .issue(id)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let Some(issue) = found else {
            return Ok(None);
        };
        Ok(self
            .seen_in(actor, &issue, Where::Live)
            .await?
            .map(|held| (issue, held)))
    }

    /// What the actor may do in the project of `issue` when they may see it there, in the place
    /// `place` says; `None` when they may not, or the project is gone.
    pub(crate) async fn seen_in(
        &self,
        actor: &Actor,
        issue: &Issue,
        place: Where,
    ) -> Result<Option<Arc<Evaluation>>, AppError> {
        if !is_in_place(issue, place) {
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
        Ok(seen_by(&held, issue, place).then_some(held))
    }

    /// The answer for a comment or a relation that is not there: it does the work that one whose
    /// issue the person cannot see costs (a read that stands for the issue, and the evaluation of the
    /// rights in a project), so the time it takes does not tell the two apart.
    pub(crate) async fn not_found_like_a_hidden_issue(&self, actor: &Actor) -> AppError {
        let tree = match self.store().project_tree().await {
            Ok(tree) => tree,
            Err(error) => return internal::<S>(&error),
        };
        if let Some(project) = tree.first() {
            // Only the work counts, not the answer.
            self.evaluation_of(actor, Place::Project(project.id))
                .await
                .ok();
        }
        AppError::Refused(ErrorCode::NotFound)
    }

    async fn seen_issue(
        &self,
        actor: &Actor,
        address: &str,
        place: Where,
    ) -> Result<(Issue, Arc<Evaluation>), AppError> {
        let found = self.find_issue(address).await?;
        let Some(found) = found else {
            // The same work as for an issue that exists, so the answers cannot be told apart.
            self.evaluation_of(actor, Place::Organization).await?;
            return Err(AppError::Refused(ErrorCode::NotFound));
        };
        let issue = found.issue;
        match self.seen_in(actor, &issue, place).await? {
            Some(held) => Ok((issue, held)),
            None => Err(AppError::Refused(ErrorCode::NotFound)),
        }
    }
}

/// Whether the issue is where it is looked for: among the issues that are there, or in the trash.
fn is_in_place(issue: &Issue, place: Where) -> bool {
    issue.deleted_at.is_some() == (place == Where::Trash)
}

/// Whether somebody with `held` in the project of the issue sees it there: it is where it is looked
/// for and the person sees it. The one statement of it, for `seen_in` and for the kinds of record
/// that sit under an issue (a comment).
pub(crate) fn seen_by(held: &Evaluation, issue: &Issue, place: Where) -> bool {
    is_in_place(issue, place) && held.sees_issue(issue.reported_by)
}

/// Where an issue is looked for: among the issues that are there, or in the trash.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Where {
    /// The issues that were not deleted.
    Live,
    /// The issues that were deleted.
    Trash,
}
