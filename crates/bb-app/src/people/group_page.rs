//! The page of one group: its name and what it is used for (SEC-02, UX §5.1).

use bb_core::id::GroupId;
use bb_policy::{Authorized, Permission, Place};

use super::GroupView;
use super::used_by::GroupDetail;
use crate::auth::{AppError, AuthStore, Ctx, internal};

/// One group, with what it is used for.
pub struct GroupQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The group.
    pub group: GroupId,
}

/// One group with what it is used for, counting only what the viewer may see.
///
/// # Errors
/// `access.forbidden` without *Manage groups*, `resource.not_found` for an unknown group.
pub async fn group<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: GroupQuery,
) -> Result<Authorized<GroupDetail>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(&actor, Permission::ManageGroups, Place::Organization)
        .await?;
    let found = ctx.group_or_missing(input.group).await?;
    let members = ctx
        .store()
        .group_members(found.id)
        .await
        .map_err(|error| internal::<S>(&error))?
        .len();
    Ok(access.authorize(GroupDetail {
        group: GroupView::from(&found),
        used_by: ctx.group_used_by(&actor, &found, members).await?,
    }))
}
