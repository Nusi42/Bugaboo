//! The page of one group: its name, what it is used for and its roles (SEC-02, UX §5.1).

use bb_core::id::GroupId;
use bb_policy::{Authorized, Permission, Place};

use super::GroupView;
use super::used_by::GroupDetail;
use crate::auth::{AppError, AuthStore, Ctx, internal};

/// The most rows of *Who has which role* the page of a group lists. A group with more is cut off
/// and the answer says so (`truncated`); *Who has which role* has them all.
pub const GROUP_ROLES_MAX: usize = 100;

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
/// `access.forbidden` without *Manage groups*, `resource.not-found` for an unknown group.
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
    let rows = ctx
        .store()
        .assignments_of_group(found.id)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let mut roles = ctx.held_roles_seen_by(&actor, &rows).await?;
    let truncated = roles.len() > GROUP_ROLES_MAX;
    roles.truncate(GROUP_ROLES_MAX);
    Ok(access.authorize(GroupDetail {
        group: GroupView::from(&found),
        used_by: ctx.group_used_by(&actor, &rows, members).await?,
        roles,
        truncated,
    }))
}
