//! The one way to find an issue a person may see by its ID. An issue that does not exist, is in the
//! trash (or, when asked for the trash, is not in it) or that the person may not see answers
//! `resource.not_found`: the three cannot be told apart.

use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::issue::Issue;
use bb_policy::{Evaluation, Place};

use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx};

impl<S: AuthStore> Ctx<'_, S> {
    /// An issue the actor may see, by its readable or internal ID, with what they may do in its
    /// project. An issue that does not exist, is in the trash or that they may not see answers
    /// `resource.not_found`.
    pub(crate) async fn visible_issue(
        &self,
        actor: &Actor,
        address: &str,
    ) -> Result<(Issue, Arc<Evaluation>), AppError> {
        self.seen_issue(actor, address, false).await
    }

    /// An issue in the trash that the actor may see: the same as `visible_issue` for the issues
    /// that were deleted (*Undo*).
    pub(crate) async fn trashed_issue(
        &self,
        actor: &Actor,
        address: &str,
    ) -> Result<(Issue, Arc<Evaluation>), AppError> {
        self.seen_issue(actor, address, true).await
    }

    async fn seen_issue(
        &self,
        actor: &Actor,
        address: &str,
        in_trash: bool,
    ) -> Result<(Issue, Arc<Evaluation>), AppError> {
        let found = self.find_issue(address).await?;
        let place = found.as_ref().map_or(Place::Organization, |found| {
            Place::Project(found.issue.project)
        });
        let held = self.evaluation_of(actor, place).await?;
        let issue = found
            .map(|found| found.issue)
            .filter(|issue| {
                issue.deleted_at.is_some() == in_trash && held.sees_issue(issue.reported_by)
            })
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        Ok((issue, held))
    }
}
