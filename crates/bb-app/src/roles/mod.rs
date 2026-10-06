//! Roles, who has which role, field levels and *Check access* (SEC-02, SEC-03, SEC-04, SEC-08,
//! PRIV-12, ADR-0025).
//!
//! Every change asks the engine first, is held to the two rules of ADR-0025 decision 6 (nobody
//! locks the organization out, nobody gives what they do not have), writes its domain event and a
//! security-log entry, and every answer leaves as `Authorized<T>`.

mod assignments;
mod check;
mod copy;
mod levels;
mod replace;
mod screens;
mod types;

use std::collections::BTreeSet;

use bb_core::ErrorCode;
use bb_core::access::Role;
use bb_core::event::FieldChange;
use bb_core::id::RoleId;
use bb_core::sign_in::SecurityAction;
use bb_core::technical::TechnicalName;
use bb_policy::guards::{Refusal, ensure_a_manager_remains, ensure_no_escalation};
use bb_policy::{Authorized, Permission, Place, StartingRole};
use serde_json::json;

pub use assignments::{assign_role, assignments, remove_assignment};
pub use check::*;
pub use copy::{CopyRole, copy_role};
pub use levels::{role_levels, set_field_level, set_people_level};
pub use screens::{
    PermissionCatalogue, PermissionView, PermissionsQuery, RoleDetail, RoleQuery,
    assignment_choices, permissions, role,
};
pub use types::*;

use crate::access::{RoleRecord, held_in};
use crate::auth::{AppError, AuthStore, Ctx, Done, by, changed, entity, internal, set_to};
use crate::people::{clean_name, stopped};

/// The keys of the permissions in a set: what an event and a view say.
fn keys(permissions: &BTreeSet<Permission>) -> Vec<&'static str> {
    permissions
        .iter()
        .map(|permission| permission.code())
        .collect()
}

fn view(record: &RoleRecord) -> RoleView {
    RoleView {
        id: record.role.id,
        name: record.role.name.clone(),
        starting: record
            .role
            .starting
            .as_ref()
            .map(|key| key.as_str().to_owned()),
        position: record.role.position,
        two_step: record.role.two_step,
        permissions: record.permissions.iter().copied().collect(),
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The role, or `resource.not_found`.
    pub(crate) async fn role_or_missing(&self, id: RoleId) -> Result<RoleRecord, AppError> {
        self.store()
            .role(id)
            .await
            .map_err(|error| internal::<S>(&error))?
            .ok_or(AppError::Refused(ErrorCode::NotFound))
    }
}

/// The permissions as a set; a permission named twice counts once.
fn set_of(permissions: &[Permission]) -> BTreeSet<Permission> {
    permissions.iter().copied().collect()
}

/// Makes a role. A new role has no field levels and no people levels: everything is hidden until
/// somebody decides otherwise (default deny).
///
/// # Errors
/// `access.forbidden` without *Manage roles*, `request.invalid` for a name that cannot be one,
/// `name.taken` when another role has the name, `access.too_much` when the role holds a
/// permission the person who makes it does not hold.
pub async fn make_role<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: MakeRole,
) -> Result<Authorized<RoleView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(&actor, Permission::ManageRoles, Place::Organization)
        .await?;
    let name = clean_name(&input.name)?;
    let permissions = set_of(&input.permissions);
    let position = ctx
        .store()
        .roles()
        .await
        .map_err(|error| internal::<S>(&error))?
        .iter()
        .map(|record| record.role.position)
        .max()
        .map_or(0, |last| last.saturating_add(1));
    let role = Role {
        id: RoleId::generate(&mut ctx.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?,
        name,
        starting: None,
        position,
        two_step: input.two_step,
    };
    let event = ctx.event(
        by(actor.person.id),
        entity("role", role.id)?,
        vec![
            set_to("name", json!(role.name)),
            set_to("permissions", json!(keys(&permissions))),
            set_to("two_step", json!(role.two_step)),
        ],
    )?;
    let (stored, stored_permissions) = (
        role.clone(),
        permissions.iter().copied().collect::<Vec<_>>(),
    );
    let maker = actor.person.id;
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_policy(transaction).await?;
                // Nobody gives a role what they do not hold, as it is stored now.
                let held = held_in::<S>(transaction, maker, Place::Organization).await?;
                ensure_no_escalation(&held, &set_of(&stored_permissions)).map_err(S::refused)?;
                S::insert_role(transaction, &stored, &stored_permissions, &[], &[]).await
            })
        })
        .await
        .map_err(|error| stopped::<S>(&error, ErrorCode::NotFound, ErrorCode::NameTaken))?;
    ctx.note(
        SecurityAction::RoleMade,
        Some(actor.person.id),
        Some(entity("role", role.id)?),
    )
    .await;
    Ok(access.authorize(view(&RoleRecord { role, permissions })))
}

/// Changes a role: its name, its permissions, its two-step switch.
///
/// # Errors
/// `access.forbidden` without *Manage roles*, `resource.not_found` for an unknown role,
/// `request.invalid` for a name that cannot be one, `name.taken`, `access.too_much` when a
/// permission is added that the person who asks does not hold, `access.would_lock_out` when it
/// would leave nobody who can manage roles.
pub async fn change_role<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangeRole,
) -> Result<Authorized<RoleView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(&actor, Permission::ManageRoles, Place::Organization)
        .await?;
    let before = ctx.role_or_missing(input.role).await?;
    let name = match &input.name {
        Some(name) => clean_name(name)?,
        None => before.role.name.clone(),
    };
    let permissions = input
        .permissions
        .as_deref()
        .map_or_else(|| before.permissions.clone(), set_of);
    let two_step = input.two_step.unwrap_or(before.role.two_step);
    let id = before.role.id;
    let event = ctx.event(
        by(actor.person.id),
        entity("role", id)?,
        vec![
            FieldChange {
                field: "name".to_owned(),
                before: Some(json!(before.role.name)),
                after: Some(json!(name)),
            },
            FieldChange {
                field: "permissions".to_owned(),
                before: Some(json!(keys(&before.permissions))),
                after: Some(json!(keys(&permissions))),
            },
            FieldChange {
                field: "two_step".to_owned(),
                before: Some(json!(before.role.two_step)),
                after: Some(json!(two_step)),
            },
        ],
    )?;
    let (new_name, new_permissions) = (
        name.clone(),
        permissions.iter().copied().collect::<Vec<_>>(),
    );
    let wanted = input.permissions.as_deref().map(set_of);
    let editor = actor.person.id;
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_policy(transaction).await?;
                // Nobody adds a permission they do not hold; taking one away needs nothing. Both
                // the role and what the person holds are read now, after the lock.
                let Some(current) = S::role_in(transaction, id).await? else {
                    return Err(S::aborted());
                };
                let held = held_in::<S>(transaction, editor, Place::Organization).await?;
                if let Some(wanted) = &wanted {
                    let added: BTreeSet<Permission> =
                        wanted.difference(&current.permissions).copied().collect();
                    ensure_no_escalation(&held, &added).map_err(S::refused)?;
                }
                let state_before = S::organization_state_in(transaction).await?;
                if !S::update_role(transaction, id, &new_name, two_step).await? {
                    return Err(S::aborted());
                }
                S::replace_role_permissions(transaction, id, &new_permissions).await?;
                let state_after = S::organization_state_in(transaction).await?;
                ensure_a_manager_remains(&state_before, &state_after).map_err(S::refused)
            })
        })
        .await
        .map_err(|error| stopped::<S>(&error, ErrorCode::NotFound, ErrorCode::NameTaken))?;
    ctx.note(
        SecurityAction::RoleChanged,
        Some(actor.person.id),
        Some(entity("role", id)?),
    )
    .await;
    let mut role = before.role;
    role.name = name;
    role.two_step = two_step;
    Ok(access.authorize(view(&RoleRecord { role, permissions })))
}

/// Deletes a role, with the levels it set. A role that somebody has is deleted only with a
/// `replacement`: every row that gave it gives the replacement instead (nobody gives a role they
/// do not hold themselves).
///
/// # Errors
/// `access.forbidden` without *Manage roles*, `resource.not_found` for an unknown role,
/// `request.invalid` when the replacement is the role itself, `role.in_use` for a role that
/// somebody has, deleted without a replacement, `access.too_much` when the replacement holds more
/// than the person who asks holds where it would be given, `access.would_lock_out` when it would
/// leave nobody who can manage roles.
pub async fn delete_role<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: DeleteRole,
) -> Result<Done, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    ctx.require(&actor, Permission::ManageRoles, Place::Organization)
        .await?;
    let id = ctx.role_or_missing(input.role).await?.role.id;
    let replacement = match input.replacement {
        Some(other) if other == id => return Err(ErrorCode::InvalidRequest.into()),
        Some(other) => Some(ctx.role_or_missing(other).await?.role.id),
        None => None,
    };
    let event = ctx.event(
        by(actor.person.id),
        entity("role", id)?,
        vec![
            changed("role"),
            set_to(
                "replacement",
                json!(replacement.map(|replacement| replacement.to_string())),
            ),
        ],
    )?;
    let (giver, now, mut ids) = (actor.person.id, ctx.now(), ctx.owned_ids());
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_policy(transaction).await?;
                let before = S::organization_state_in(transaction).await?;
                let rows = S::assignments_of_role_in(transaction, id).await?;
                if !rows.is_empty() {
                    // Judged on the rows that are stored now, after the lock.
                    let Some(replacement) = replacement else {
                        return Err(S::refused(Refusal::InUse));
                    };
                    replace::give_to_replacement::<S>(
                        transaction,
                        giver,
                        &rows,
                        replacement,
                        now,
                        &mut ids,
                    )
                    .await?;
                }
                if !S::delete_role(transaction, id).await? {
                    return Err(S::aborted());
                }
                let after = S::organization_state_in(transaction).await?;
                ensure_a_manager_remains(&before, &after).map_err(S::refused)
            })
        })
        .await
        .map_err(|error| stopped::<S>(&error, ErrorCode::NotFound, ErrorCode::NotFound))?;
    ctx.note(
        SecurityAction::RoleDeleted,
        Some(actor.person.id),
        Some(entity("role", id)?),
    )
    .await;
    Ok(Done)
}

/// Gives a starting role back the permissions and levels it had at the start (*Reset to the
/// starting permissions*). The name stays; so do the levels set for single projects.
///
/// # Errors
/// `access.forbidden` without *Manage roles*, `resource.not_found` for an unknown role,
/// `request.invalid` for a role that is not a copy of a starting role, `access.too_much` when
/// the reset adds a permission or a level the person who asks does not hold themselves,
/// `access.would_lock_out` when it would leave nobody who can manage roles.
pub async fn reset_role<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ResetRole,
) -> Result<Authorized<RoleView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(&actor, Permission::ManageRoles, Place::Organization)
        .await?;
    let before = ctx.role_or_missing(input.role).await?;
    let starting = before
        .role
        .starting
        .as_ref()
        .map(TechnicalName::as_str)
        .and_then(StartingRole::from_key)
        .ok_or(ErrorCode::InvalidRequest)?;
    let permissions: BTreeSet<Permission> = starting.permissions().into_iter().collect();
    let fields: Vec<_> = bb_core::fields::Field::ALL
        .iter()
        .map(|field| (*field, starting.field_level(*field)))
        .collect();
    let people: Vec<_> = bb_core::people_kinds::PeopleKind::ALL
        .iter()
        .map(|kind| (*kind, starting.people_level(*kind)))
        .collect();
    let id = before.role.id;
    let event = ctx.event(
        by(actor.person.id),
        entity("role", id)?,
        vec![FieldChange {
            field: "permissions".to_owned(),
            before: Some(json!(keys(&before.permissions))),
            after: Some(json!(keys(&permissions))),
        }],
    )?;
    let (name, new_permissions) = (
        before.role.name.clone(),
        permissions.iter().copied().collect::<Vec<_>>(),
    );
    let editor = actor.person.id;
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_policy(transaction).await?;
                // The reset adds what the role lacks now and sets levels: nobody gives what they
                // do not hold, judged on the role and on the person as they are stored now.
                let Some(current) = S::role_in(transaction, id).await? else {
                    return Err(S::aborted());
                };
                let held = held_in::<S>(transaction, editor, Place::Organization).await?;
                let added: BTreeSet<Permission> = new_permissions
                    .iter()
                    .copied()
                    .filter(|permission| !current.permissions.contains(permission))
                    .collect();
                ensure_no_escalation(&held, &added).map_err(S::refused)?;
                if fields
                    .iter()
                    .any(|(field, level)| held.field_level(*field) < *level)
                    || people
                        .iter()
                        .any(|(kind, level)| held.kind_level(*kind) < *level)
                {
                    return Err(S::refused(Refusal::TooMuch));
                }
                let state_before = S::organization_state_in(transaction).await?;
                if !S::update_role(transaction, id, &name, starting.two_step()).await? {
                    return Err(S::aborted());
                }
                S::replace_role_permissions(transaction, id, &new_permissions).await?;
                S::replace_role_levels(transaction, id, &fields, &people).await?;
                let state_after = S::organization_state_in(transaction).await?;
                ensure_a_manager_remains(&state_before, &state_after).map_err(S::refused)
            })
        })
        .await
        .map_err(|error| stopped::<S>(&error, ErrorCode::NotFound, ErrorCode::NotFound))?;
    ctx.note(
        SecurityAction::RoleReset,
        Some(actor.person.id),
        Some(entity("role", id)?),
    )
    .await;
    let mut role = before.role;
    role.two_step = starting.two_step();
    Ok(access.authorize(view(&RoleRecord { role, permissions })))
}

/// The roles, in the order of roles, a page at a time. In *Organization settings* (`place` is
/// `None`) it takes *Manage roles*; on a project it takes *Decide who has which role* there,
/// because that is where a person chooses a role to give.
///
/// # Errors
/// `access.forbidden` without the permission, `resource.not_found` for a project the person may
/// not know about, `request.invalid` for a cursor that is not one.
pub async fn roles<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: RolesQuery,
) -> Result<Authorized<RoleList>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = match input.place {
        None => {
            ctx.require(&actor, Permission::ManageRoles, Place::Organization)
                .await?
        }
        Some(project) => {
            ctx.require(&actor, Permission::DecideRoles, Place::Project(project))
                .await?
        }
    };
    let records = ctx
        .store()
        .roles()
        .await
        .map_err(|error| internal::<S>(&error))?;
    let page = bb_core::paging::window(records.len(), input.cursor.as_deref(), input.limit)
        .ok_or(ErrorCode::InvalidRequest)?;
    Ok(access.authorize(RoleList {
        roles: records[page.start..page.end].iter().map(view).collect(),
        next_cursor: page.next,
    }))
}
