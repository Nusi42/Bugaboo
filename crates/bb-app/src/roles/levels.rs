//! Field levels and *Who sees people* (SEC-04, PRIV-12, ADR-0025 decisions 10 and 11): what each
//! role sees, for the whole organization (the role's own level) or for one project and its
//! sub-projects.
//!
//! *Decide what roles see* is needed where the level is set. Nobody sets a level above their own
//! at that place (the same rule as for permissions: nobody gives what they do not have).

use bb_core::ErrorCode;
use bb_core::event::FieldChange;
use bb_core::id::ProjectId;
use bb_core::sign_in::SecurityAction;
use bb_policy::{Authorized, Permission, Place};
use serde_json::json;

use super::types::{
    FieldLevelView, Levels, LevelsQuery, PeopleLevelView, SetFieldLevel, SetPeopleLevel,
};
use crate::auth::{AppError, AuthStore, Ctx, Done, by, change_failed, entity, internal};

/// The place a level is set at.
fn place_of(project: Option<ProjectId>) -> Place {
    project.map_or(Place::Organization, Place::Project)
}

/// Sets, or takes away, the level of a field for a role. A role's own level cannot be taken
/// away, only changed.
///
/// # Errors
/// `access.forbidden` without *Decide what roles see* there, `resource.not_found` for an unknown
/// role or a project the person may not know about, `request.invalid` for taking away a role's
/// own level, `access.too_much` for a level above the person's own.
pub async fn set_field_level<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SetFieldLevel,
) -> Result<Done, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(
            &actor,
            Permission::DecideWhatRolesSee,
            place_of(input.project),
        )
        .await?;
    let role = ctx.role_or_missing(input.role).await?.role.id;
    if input.project.is_none() && input.level.is_none() {
        return Err(ErrorCode::InvalidRequest.into());
    }
    if input
        .level
        .is_some_and(|level| level > access.field_level(input.field))
    {
        return Err(ErrorCode::TooMuch.into());
    }
    let (project, field, level) = (input.project, input.field, input.level);
    let event = ctx.event(
        by(actor.person.id),
        entity("role", role)?,
        vec![FieldChange {
            field: format!("field.{}", field.code()),
            before: None,
            after: Some(json!(level.map(bb_core::access::FieldLevel::code))),
        }],
    )?;
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(
                async move { S::put_field_level(transaction, role, project, field, level).await },
            )
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Some(ErrorCode::NotFound)))?;
    ctx.note(
        SecurityAction::FieldLevelChanged,
        Some(actor.person.id),
        Some(entity("role", role)?),
    )
    .await;
    Ok(Done)
}

/// Sets, or takes away, how a role sees one kind of people information.
///
/// # Errors
/// The same as [`set_field_level`].
pub async fn set_people_level<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SetPeopleLevel,
) -> Result<Done, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(
            &actor,
            Permission::DecideWhatRolesSee,
            place_of(input.project),
        )
        .await?;
    let role = ctx.role_or_missing(input.role).await?.role.id;
    if input.project.is_none() && input.level.is_none() {
        return Err(ErrorCode::InvalidRequest.into());
    }
    if input
        .level
        .is_some_and(|level| level > access.kind_level(input.kind))
    {
        return Err(ErrorCode::TooMuch.into());
    }
    let (project, kind, level) = (input.project, input.kind, input.level);
    let event = ctx.event(
        by(actor.person.id),
        entity("role", role)?,
        vec![FieldChange {
            field: format!("people.{}", kind.code()),
            before: None,
            after: Some(json!(level.map(bb_core::access::PeopleLevel::code))),
        }],
    )?;
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(
                async move { S::put_people_level(transaction, role, project, kind, level).await },
            )
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Some(ErrorCode::NotFound)))?;
    ctx.note(
        SecurityAction::PeopleLevelChanged,
        Some(actor.person.id),
        Some(entity("role", role)?),
    )
    .await;
    Ok(Done)
}

/// The levels a role has set in one place: for a project, or its own.
///
/// # Errors
/// `access.forbidden` without *Decide what roles see* there, `resource.not_found` for an unknown
/// role or a project the person may not know about.
pub async fn role_levels<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: LevelsQuery,
) -> Result<Authorized<Levels>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(
            &actor,
            Permission::DecideWhatRolesSee,
            place_of(input.project),
        )
        .await?;
    let role = ctx.role_or_missing(input.role).await?.role.id;
    let fields = ctx
        .store()
        .field_levels(role, input.project)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let people = ctx
        .store()
        .people_levels(role, input.project)
        .await
        .map_err(|error| internal::<S>(&error))?;
    Ok(access.authorize(Levels {
        fields: fields
            .into_iter()
            .map(|(field, level)| FieldLevelView { field, level })
            .collect(),
        people: people
            .into_iter()
            .map(|(kind, level)| PeopleLevelView { kind, level })
            .collect(),
    }))
}
