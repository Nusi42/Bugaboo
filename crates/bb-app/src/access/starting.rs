//! The starting roles are made once, at the first start (SEC-08, ADR-0025 decisions 4 and 5).
//!
//! A system use case makes editable copies of the six roles from the one definition in
//! `bb-policy`, with an event and a security-log entry (never in a migration); after that the
//! definition is read only by *Reset to the starting permissions*. The same use case gives every
//! person who was an administrator in M1 the Administrator role for the whole organization, one
//! change, one event and one log entry each. It runs at every start and does only what is still
//! to do, so it can be stopped half way and run again.

use bb_core::access::{AppliesTo, Assignment, Role};
use bb_core::event::{Actor, RecordKind};
use bb_core::id::{AssignmentId, RoleId};
use bb_core::message::Message;
use bb_core::sign_in::SecurityAction;
use bb_core::technical::TechnicalName;
use bb_core::version::FIRST_VERSION;
use bb_policy::StartingRole;
use serde_json::json;

use crate::auth::{
    AppError, AuthStore, Ctx, Stops, change_failed, changed, entity, fixed_entity, internal, set_to,
};

/// Makes the starting roles if they were not made yet, and gives the people who were
/// administrators in M1 their role. There is nothing to say: it only does what is left.
pub struct MakeStartingRoles;

/// The role of a starting role, ready to be stored: the role, with the levels it starts with.
pub(crate) struct NewRole {
    pub(crate) role: Role,
    pub(crate) permissions: Vec<bb_policy::Permission>,
    pub(crate) fields: Vec<(bb_core::fields::Field, bb_core::access::FieldLevel)>,
    pub(crate) people: Vec<(
        bb_core::people_kinds::PeopleKind,
        bb_core::access::PeopleLevel,
    )>,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The copy of a starting role as it is made at the start.
    pub(crate) fn new_starting_role(&self, starting: StartingRole) -> Result<NewRole, AppError> {
        let key = TechnicalName::new(starting.code())
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let id = RoleId::generate(&mut self.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?;
        Ok(NewRole {
            role: Role {
                id,
                name: Message::from(starting.name()).render(),
                starting: Some(key),
                position: i64::try_from(starting.index()).unwrap_or(i64::MAX),
                two_step: starting.two_step(),
                version: FIRST_VERSION,
            },
            permissions: starting.permissions(),
            fields: bb_core::fields::Field::ALL
                .iter()
                .map(|field| (*field, starting.field_level(*field)))
                .collect(),
            people: bb_core::people_kinds::PeopleKind::ALL
                .iter()
                .map(|kind| (*kind, starting.people_level(*kind)))
                .collect(),
        })
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The row that gives `person` the Administrator role for the whole organization, after the
    /// starting roles were made (once): what setup stores together with the first account
    /// (ADR-0025, decision 5).
    pub(crate) async fn administrator_assignment(
        &self,
        person: bb_core::id::UserId,
    ) -> Result<Assignment, AppError> {
        make_starting_roles(self, MakeStartingRoles).await?;
        let administrator = role_of(self, StartingRole::Administrator).await?;
        Ok(Assignment {
            id: AssignmentId::generate(&mut self.ids())
                .map_err(|error| AppError::Internal(error.to_string()))?,
            person: Some(person),
            group: None,
            role: Some(administrator),
            project: None,
            applies_to: AppliesTo::Organization,
            given_at: self.now(),
        })
    }
}

/// Makes the starting roles once and gives the M1 administrators the Administrator role.
///
/// # Errors
/// Fails when the store fails.
pub async fn make_starting_roles<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    _input: MakeStartingRoles,
) -> Result<(), AppError> {
    let store = ctx.store();
    let made = store
        .starting_roles_made()
        .await
        .map_err(|error| internal::<S>(&error))?;
    if !made {
        let roles = StartingRole::ALL
            .iter()
            .map(|starting| ctx.new_starting_role(*starting))
            .collect::<Result<Vec<_>, _>>()?;
        let names: Vec<serde_json::Value> = roles.iter().map(|new| json!(new.role.name)).collect();
        let event = ctx.event(
            Actor::System,
            fixed_entity(RecordKind::StartingRoles, "made")?,
            vec![set_to("roles", serde_json::Value::Array(names))],
        )?;
        let now = ctx.now();
        let made_now = store
            .record_change(&event, move |transaction| {
                Box::pin(async move {
                    S::lock_policy(transaction).await?;
                    // Two servers that start together both saw "not made" above; the one that
                    // gets the lock second finds them made and stops, which is not a failure.
                    if S::starting_roles_made_in(transaction).await? {
                        return Err(S::aborted());
                    }
                    for new in &roles {
                        S::insert_role(
                            transaction,
                            &new.role,
                            &new.permissions,
                            &new.fields,
                            &new.people,
                        )
                        .await?;
                    }
                    S::mark_starting_roles_made(transaction, now).await?;
                    Ok(true)
                })
            })
            .await;
        match made_now {
            Ok(_) => {
                ctx.note(
                    SecurityAction::StartingRolesMade,
                    None,
                    Some(fixed_entity(RecordKind::StartingRoles, "made")?),
                )
                .await;
            }
            Err(error) if S::is_aborted(&error) => {}
            Err(error) => return Err(change_failed::<S>(&error, Stops::NONE)),
        }
    }
    give_the_old_administrators_their_role(ctx).await
}

/// The ID of the role that is the copy of the starting role `starting`, made again from the
/// definition when nobody has it any more (only possible where nobody could use the roles yet).
async fn role_of<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    starting: StartingRole,
) -> Result<RoleId, AppError> {
    let roles = ctx
        .store()
        .roles()
        .await
        .map_err(|error| internal::<S>(&error))?;
    if let Some(found) = roles.iter().find(|record| {
        record.role.starting.as_ref().map(TechnicalName::as_str) == Some(starting.code())
    }) {
        return Ok(found.role.id);
    }
    let new = ctx.new_starting_role(starting)?;
    let id = new.role.id;
    let event = ctx.event(
        Actor::System,
        entity(RecordKind::Role, id),
        vec![changed("role")],
    )?;
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_policy(transaction).await?;
                S::insert_role(
                    transaction,
                    &new.role,
                    &new.permissions,
                    &new.fields,
                    &new.people,
                )
                .await
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    ctx.note(
        SecurityAction::RoleMade,
        None,
        Some(entity(RecordKind::Role, id)),
    )
    .await;
    Ok(id)
}

/// Gives each person of the list of M1 administrators the Administrator role for the whole
/// organization, and takes them off the list in the same change.
async fn give_the_old_administrators_their_role<S: AuthStore>(
    ctx: &Ctx<'_, S>,
) -> Result<(), AppError> {
    let waiting = ctx
        .store()
        .legacy_administrators()
        .await
        .map_err(|error| internal::<S>(&error))?;
    if waiting.is_empty() {
        return Ok(());
    }
    let administrator = role_of(ctx, StartingRole::Administrator).await?;
    for person in waiting {
        let assignment = Assignment {
            id: AssignmentId::generate(&mut ctx.ids())
                .map_err(|error| AppError::Internal(error.to_string()))?,
            person: Some(person),
            group: None,
            role: Some(administrator),
            project: None,
            applies_to: AppliesTo::Organization,
            given_at: ctx.now(),
        };
        let id = assignment.id;
        let event = ctx.event(
            Actor::System,
            entity(RecordKind::Assignment, id),
            vec![
                set_to("role", json!(administrator.to_string())),
                set_to("applies_to", json!(AppliesTo::Organization.code())),
            ],
        )?;
        ctx.store()
            .record_change(&event, move |transaction| {
                Box::pin(async move {
                    S::lock_policy(transaction).await?;
                    // Another server that started at the same time may have given it already.
                    let has_it = S::assignments_on_in(transaction, None)
                        .await?
                        .iter()
                        .any(|row| row.person == Some(person) && row.role == Some(administrator));
                    if !has_it {
                        S::insert_assignment(transaction, &assignment).await?;
                    }
                    S::delete_legacy_administrator(transaction, person).await
                })
            })
            .await
            .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
        ctx.note(
            SecurityAction::AssignmentMade,
            None,
            Some(entity(RecordKind::Assignment, id)),
        )
        .await;
    }
    Ok(())
}
