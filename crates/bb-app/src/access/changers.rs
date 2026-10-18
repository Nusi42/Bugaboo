//! Who may change a field at a place (M3-F53, ADR-0031): the roles that a person who sees the field
//! but cannot change it is told to ask, and that the workflow editor offers in *Who can do it*.
//!
//! One question, `bb_policy::roles_that_change`, answered over the roles of the organization. Three
//! rules keep it from telling more than the reader may know:
//! - **A field the reader cannot see is not there** (`resource.not_found`).
//! - **Only roles somebody holds at the place are named**: a role nobody has there is not part of
//!   the answer, so it tells nothing about roles of other projects.
//! - **A reader who is not shown the people of the project by role or by name is named no role**:
//!   the roles of the people there are exactly what such a reader may not know.

use std::collections::BTreeSet;

use bb_core::ErrorCode;
use bb_core::access::FieldLevel;
use bb_core::fields::FieldKey;
use bb_core::id::{IssueTypeId, RoleId};
use bb_policy::{Authorized, Masked, Permission, Place, RoleData, roles_that_change, row_reaches};
use serde::Serialize;

use crate::auth::{AppError, AuthStore, Ctx, internal};

/// Ask which roles may change a field in the issues of a project.
pub struct WhoMayChangeQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project, by its ID or its short name.
    pub project: String,
    /// The field, built-in or own.
    pub field: FieldKey,
    /// The issue type; none for all types.
    pub issue_type: Option<IssueTypeId>,
}

/// A role that may change the field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ChangingRole {
    /// The ID of the role.
    pub id: RoleId,
    /// Its name.
    pub name: String,
    /// Whether it may change the field only in the issues its holders reported.
    pub own_only: bool,
}

/// The roles that may change a field, as the reader may know them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WhoMayChange {
    /// The roles somebody holds in the project that may change the field, in the order of roles.
    pub roles: Vec<ChangingRole>,
    /// The reader is not shown the people of the project, so no role is named.
    pub roles_hidden: bool,
}

impl Masked for WhoMayChange {}

/// Which roles may change a field in the issues of a project.
///
/// # Errors
/// `resource.not_found` for a project the person may not see and for a field they may not see;
/// a refusal for a person who holds no right to see issues there.
pub async fn who_may_change<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: WhoMayChangeQuery,
) -> Result<Authorized<WhoMayChange>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let found = ctx.find_project(&input.project).await?;
    let place = Place::from(found.as_ref().map(|found| found.project.id));
    let held = ctx.evaluation_of(&actor, place).await?;
    let Some(found) = found.filter(|_| held.sees_project()) else {
        return Err(AppError::Refused(ErrorCode::NotFound));
    };
    // Fields are those of issues: a reader who holds no right to see issues there is refused.
    let held = ctx.require(&actor, Permission::SeeIssues, place).await?;
    ctx.ensure_type_known(&actor, input.issue_type).await?;
    if held.field_level_for(input.issue_type, input.field) < FieldLevel::CanSee {
        return Err(AppError::Refused(ErrorCode::NotFound));
    }
    if !held.may_know_roles_of_people() {
        return Ok(held.authorize(WhoMayChange {
            roles: Vec::new(),
            roles_hidden: true,
        }));
    }
    let failed = |error| internal::<S>(&error);
    let chain = ctx.chain_of(found.project.id).await?;
    let records = ctx.store().roles().await.map_err(failed)?;
    // The roles somebody holds in the project, in a row that reaches it.
    let mut held_roles: BTreeSet<RoleId> = BTreeSet::new();
    let mut places: Vec<Option<bb_core::id::ProjectId>> = vec![None];
    places.extend(chain.iter().copied().map(Some));
    for at in places {
        for row in ctx.store().assignments_on(at).await.map_err(failed)? {
            if let Some(role) = row
                .role
                .filter(|_| row_reaches(row.project, row.applies_to, &chain))
            {
                held_roles.insert(role);
            }
        }
    }
    let ids: Vec<RoleId> = records.iter().map(|record| record.role.id).collect();
    let levels = ctx
        .store()
        .level_rules(ids, Some(found.project.id))
        .await
        .map_err(failed)?
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    let roles: Vec<RoleData> = records.into_iter().map(RoleData::from).collect();
    let roles = roles_that_change(
        &roles,
        &levels.rules,
        &levels.chain,
        input.field,
        input.issue_type,
    )
    .into_iter()
    .filter(|changer| held_roles.contains(&changer.role))
    .filter_map(|changer| {
        roles
            .iter()
            .find(|role| role.id == changer.role)
            .map(|role| ChangingRole {
                id: role.id,
                name: role.name.clone(),
                own_only: changer.own_only,
            })
    })
    .collect();
    Ok(held.authorize(WhoMayChange {
        roles,
        roles_hidden: false,
    }))
}
