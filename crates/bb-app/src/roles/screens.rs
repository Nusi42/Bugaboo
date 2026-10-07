//! What the screens of *Roles & permissions* read besides the lists (SEC-03, SEC-08, UX §5.2): one
//! role with what it is used for, the permission catalogue the role page is made from, and the
//! choices the form of *Who has which role* offers.

use bb_core::ErrorCode;
use bb_core::id::RoleId;
use bb_policy::guards::ensure_no_escalation;
use bb_policy::{Authorized, Counts, Masked, Permission, Place, Topic};
use serde::Serialize;

use super::types::RoleRef;
use super::{RoleView, view};
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::people::{
    AssignmentChoicesQuery, ChoiceKind, ChoicePage, ChoiceView, GroupView, RoleUsedBy,
};

/// One role, with what it is used for.
pub struct RoleQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The role.
    pub role: RoleId,
}

/// The permission catalogue, for the role page.
pub struct PermissionsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// A role as the page of a role shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RoleDetail {
    /// The role.
    pub role: RoleView,
    /// What it is used for.
    pub used_by: RoleUsedBy,
}

impl Masked for RoleDetail {}

/// One permission of the catalogue, with what the role page needs to list it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct PermissionView {
    /// The permission; its sentence is the message `permission.<key>`.
    pub permission: Permission,
    /// What it is about; the name of the topic is the message `permission-topic.<topic>`.
    pub topic: Topic,
    /// Where it counts: a permission that counts only for the whole organization gives nothing
    /// in a role that is given on a project, and the role page says so.
    pub counts: Counts,
}

/// The whole catalogue, in its order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PermissionCatalogue {
    /// The permissions, topic by topic.
    pub permissions: Vec<PermissionView>,
}

impl Masked for PermissionCatalogue {}

/// One role with its permissions and what it is used for.
///
/// # Errors
/// `access.forbidden` without *Manage roles*, `resource.not_found` for an unknown role.
pub async fn role<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: RoleQuery,
) -> Result<Authorized<RoleDetail>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(&actor, Permission::ManageRoles, Place::Organization)
        .await?;
    let record = ctx.role_or_missing(input.role).await?;
    let rows = ctx
        .store()
        .assignments_of_role(record.role.id)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let seen = ctx.rows_seen_by(&actor, &rows).await?;
    Ok(access.authorize(RoleDetail {
        role: view(&record),
        used_by: RoleUsedBy {
            whole_organization: seen.whole_organization,
            projects: u64::try_from(seen.projects.len()).unwrap_or(u64::MAX),
            people: u64::try_from(seen.people.len()).unwrap_or(u64::MAX),
            groups: u64::try_from(seen.groups.len()).unwrap_or(u64::MAX),
        },
    }))
}

/// The permission catalogue, for the role page.
///
/// # Errors
/// `access.forbidden` without *Manage roles*.
pub async fn permissions<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: PermissionsQuery,
) -> Result<Authorized<PermissionCatalogue>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(&actor, Permission::ManageRoles, Place::Organization)
        .await?;
    Ok(access.authorize(PermissionCatalogue {
        permissions: Permission::ALL
            .iter()
            .map(|permission| PermissionView {
                permission: *permission,
                topic: permission.topic(),
                counts: permission.counts(),
            })
            .collect(),
    }))
}

/// What the form of *Who has which role* offers to choose: people, groups or roles. Choosing is
/// for those who decide who has which role at the place, who see names there (M2-Q3).
///
/// # Errors
/// `access.forbidden` without *Decide who has which role* there (`resource.not_found` for a
/// project the person may not know about), `request.invalid` for a cursor that is not one.
pub async fn assignment_choices<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AssignmentChoicesQuery,
) -> Result<Authorized<ChoicePage>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let place = input.project.map_or(Place::Organization, Place::Project);
    let access = ctx.require(&actor, Permission::DecideRoles, place).await?;
    let page = match input.kind {
        ChoiceKind::Person => {
            ctx.people_choices(
                &access,
                input.search.as_deref(),
                input.cursor.as_deref(),
                input.limit,
                |_| None,
            )
            .await?
        }
        ChoiceKind::Group => {
            let groups = ctx
                .store()
                .groups()
                .await
                .map_err(|error| internal::<S>(&error))?;
            let named: Vec<ChoiceView> = groups
                .iter()
                .map(GroupView::from)
                .filter(|group| matches_words(&group.name, input.search.as_deref()))
                .map(|group| ChoiceView {
                    id: group.id.to_string(),
                    name: group.name,
                    already_in: None,
                })
                .collect();
            in_pages(named, input.cursor.as_deref(), input.limit)?
        }
        ChoiceKind::Role => {
            let records = ctx
                .store()
                .roles()
                .await
                .map_err(|error| internal::<S>(&error))?;
            // Only the roles the person may give here: they hold every permission of them.
            let named: Vec<ChoiceView> = records
                .iter()
                .filter(|record| ensure_no_escalation(&access, &record.permissions).is_ok())
                .map(|record| RoleRef {
                    id: record.role.id,
                    name: record.role.name.clone(),
                })
                .filter(|role| matches_words(&role.name, input.search.as_deref()))
                .map(|role| ChoiceView {
                    id: role.id.to_string(),
                    name: role.name,
                    already_in: None,
                })
                .collect();
            in_pages(named, input.cursor.as_deref(), input.limit)?
        }
    };
    Ok(access.authorize(page))
}

/// Whether a name holds the words of a search, without regard to capitals.
fn matches_words(name: &str, search: Option<&str>) -> bool {
    search
        .map(str::trim)
        .filter(|words| !words.is_empty())
        .is_none_or(|words| name.to_lowercase().contains(&words.to_lowercase()))
}

/// One page of choices that were made whole.
fn in_pages(
    mut all: Vec<ChoiceView>,
    cursor: Option<&str>,
    limit: u32,
) -> Result<ChoicePage, AppError> {
    let page = bb_core::paging::window(all.len(), cursor, limit)
        .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
    let items = all.drain(page.start..page.end).collect();
    Ok(ChoicePage {
        items,
        next_cursor: page.next,
    })
}
