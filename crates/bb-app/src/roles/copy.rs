//! Making a role from a copy of another (SEC-08, the first step of the M2 exit demo): the copy has
//! the permissions, the two-step switch and the levels of the role it comes from, and is a role of
//! its own that nobody can reset to a starting role.

use bb_core::ErrorCode;
use bb_core::event::RecordKind;
use bb_core::fields::FieldKey;
use bb_core::id::RoleId;
use bb_core::people_kinds::PeopleKind;
use bb_core::sign_in::SecurityAction;
use bb_policy::guards::{Refusal, ensure_no_escalation};
use bb_policy::{Authorized, Permission, Place};
use serde_json::json;

use super::{RoleView, keys, view};
use crate::access::{RoleRecord, held_in};
use crate::auth::{AppError, AuthStore, Ctx, Stops, by, change_failed, entity, internal, set_to};
use crate::input::clean_name;
use crate::member;

/// Make a role that is a copy of another.
pub struct CopyRole {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The role to copy.
    pub role: RoleId,
    /// What the copy is called.
    pub name: String,
}

/// Creates a role from a copy of another: its permissions, its two-step switch and the levels it
/// has set for itself (not those set for single projects). Nobody gives what they do not hold,
/// so the copy holds no permission and no level above the person who makes it.
///
/// # Errors
/// `access.forbidden` without *Manage roles*, `resource.not_found` for an unknown role,
/// `request.invalid` for a name that cannot be one, `name.taken` when another role has the name,
/// `access.too_much` when the role holds a permission or a level the person who copies does not
/// hold.
pub async fn copy_role<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CopyRole,
) -> Result<Authorized<RoleView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(&actor, Permission::ManageRoles, Place::Organization)
        .await?;
    let name = clean_name(&input.name)?;
    let source = ctx.role_or_missing(input.role).await?;
    let fields = ctx
        .store()
        .levels::<FieldKey>(source.role.id, None, None)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let people = ctx
        .store()
        .levels::<PeopleKind>(source.role.id, None, None)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let role = ctx.new_role(name, source.role.two_step).await?;
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Role, role.id),
        vec![
            set_to(member::role::NAME, json!(role.name)),
            set_to(member::role::PERMISSIONS, json!(keys(&source.permissions))),
            set_to(member::role::TWO_STEP, json!(role.two_step)),
            set_to("copy_of", json!(source.role.id.to_string())),
        ],
    )?;
    let (stored, permissions) = (
        role.clone(),
        source.permissions.iter().copied().collect::<Vec<_>>(),
    );
    let (stored_fields, stored_people) = (fields.clone(), people.clone());
    let maker = actor.person.id;
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_policy(transaction).await?;
                // The same rule as for making a role and for resetting one: judged on what the
                // person holds now, after the lock.
                let held = held_in::<S>(transaction, maker, Place::Organization).await?;
                ensure_no_escalation(&held, &permissions.iter().copied().collect())
                    .map_err(S::refused)?;
                if stored_fields
                    .iter()
                    .any(|(field, level)| held.field_level(*field) < *level)
                    || stored_people
                        .iter()
                        .any(|(kind, level)| held.kind_level(*kind) < *level)
                {
                    return Err(S::refused(Refusal::TooMuch));
                }
                S::insert_role(
                    transaction,
                    &stored,
                    &permissions,
                    &stored_fields,
                    &stored_people,
                )
                .await
            })
        })
        .await
        .map_err(|error| {
            change_failed::<S>(
                &error,
                Stops::new(ErrorCode::NotFound, ErrorCode::NameTaken),
            )
        })?;
    ctx.note(
        SecurityAction::RoleCreated,
        Some(actor.person.id),
        Some(entity(RecordKind::Role, role.id)),
    )
    .await;
    Ok(access.authorize(view(&RoleRecord {
        role,
        permissions: source.permissions,
    })))
}
