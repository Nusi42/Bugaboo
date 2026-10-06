//! Field levels and *Who sees people* (SEC-04, PRIV-12, ADR-0025 decisions 10 and 11): what each
//! role sees, for the whole organization (the role's own level) or for one project and its
//! sub-projects.
//!
//! *Decide what roles see* is needed where the level is set. Nobody sets a level above their own
//! at that place (the same rule as for permissions: nobody gives what they do not have).

use bb_core::ErrorCode;
use bb_core::access::{FieldLevel, PeopleLevel};
use bb_core::event::FieldChange;
use bb_core::fields::Field;
use bb_core::id::{ProjectId, RoleId};
use bb_core::people_kinds::PeopleKind;
use bb_core::sign_in::SecurityAction;
use bb_policy::guards::Refusal;
use bb_policy::{Authorized, Permission, Place};
use serde_json::json;

use super::types::{
    FieldLevelView, Levels, LevelsQuery, PeopleLevelView, SetFieldLevel, SetPeopleLevel,
};
use crate::access::held_in;
use crate::auth::{AppError, AuthStore, Ctx, Done, by, change_failed, entity, internal};

/// The place a level is set at.
pub(crate) fn place_of(project: Option<ProjectId>) -> Place {
    project.map_or(Place::Organization, Place::Project)
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The places a role's level is looked up in when the project's own setting is not there: the
    /// parents, nearest first, and last the role's own level (`None`).
    async fn places_above(&self, project: ProjectId) -> Result<Vec<Option<ProjectId>>, AppError> {
        let chain = self
            .store()
            .project_chain(project)
            .await
            .map_err(|error| internal::<S>(&error))?
            .unwrap_or_default();
        Ok(chain
            .iter()
            .rev()
            .skip(1)
            .map(|parent| Some(*parent))
            .chain(std::iter::once(None))
            .collect())
    }

    /// The level a field has for the role in the project once the project's own setting is taken
    /// away: the nearest parent's, else the role's own, else the closed one. Taking an override away
    /// must not raise a level above what the person who does it holds.
    async fn field_level_without_override(
        &self,
        role: RoleId,
        project: ProjectId,
        field: Field,
    ) -> Result<FieldLevel, AppError> {
        for place in self.places_above(project).await? {
            let levels = self
                .store()
                .field_levels(role, place)
                .await
                .map_err(|error| internal::<S>(&error))?;
            if let Some((_, level)) = levels.into_iter().find(|(known, _)| *known == field) {
                return Ok(level);
            }
        }
        Ok(FieldLevel::Hidden)
    }

    /// The same for a kind of people information.
    async fn people_level_without_override(
        &self,
        role: RoleId,
        project: ProjectId,
        kind: PeopleKind,
    ) -> Result<PeopleLevel, AppError> {
        for place in self.places_above(project).await? {
            let levels = self
                .store()
                .people_levels(role, place)
                .await
                .map_err(|error| internal::<S>(&error))?;
            if let Some((_, level)) = levels.into_iter().find(|(known, _)| *known == kind) {
                return Ok(level);
            }
        }
        Ok(PeopleLevel::Hidden)
    }
}

/// The answer for a change that did not go through: a refusal of a rule (nobody gives more than
/// they hold), a thing that is not there, or a failure.
fn refused_or_failed<S: AuthStore>(error: &S::Error) -> AppError {
    match S::as_refusal(error) {
        Some(refusal) => refusal.into(),
        None => change_failed::<S>(error, Some(ErrorCode::NotFound)),
    }
}

/// Sets, or takes away, the level of a field for a role. A role's own level cannot be taken
/// away, only changed. Taking a project's setting away is the same as setting the level that then
/// applies there (the nearest parent's, else the role's own), so that level is held to the same
/// rule.
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
    let level_then = match (input.level, input.project) {
        (Some(level), _) => Some(level),
        (None, Some(project)) => Some(
            ctx.field_level_without_override(role, project, input.field)
                .await?,
        ),
        (None, None) => None,
    };
    if level_then.is_some_and(|level| level > access.field_level(input.field)) {
        return Err(ErrorCode::TooMuch.into());
    }
    let (project, field, level) = (input.project, input.field, input.level);
    let editor = actor.person.id;
    // What was there before, so the history can tell what a change did.
    let before = ctx
        .store()
        .field_levels(role, project)
        .await
        .map_err(|error| internal::<S>(&error))?
        .into_iter()
        .find(|(known, _)| *known == field)
        .map(|(_, level)| level);
    let event = ctx.event(
        by(actor.person.id),
        entity("role", role)?,
        vec![FieldChange {
            field: format!("field.{}", field.code()),
            before: before.map(|level| json!(level.code())),
            after: level.map(|level| json!(level.code())),
        }],
    )?;
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                // The version of a move preview is made from these levels: a move waits for a
                // change of them, or the change for the move.
                S::lock_policy(transaction).await?;
                // A level set on a project reaches what is inside it: nobody gives there more than
                // they hold there, judged now, after the lock.
                if let (Some(project), Some(wanted)) = (project, level_then) {
                    for below in S::sub_projects_in(transaction, project).await? {
                        let held = held_in::<S>(transaction, editor, Place::Project(below)).await?;
                        if held.field_level(field) < wanted {
                            return Err(S::refused(Refusal::TooMuch));
                        }
                    }
                }
                S::put_field_level(transaction, role, project, field, level).await
            })
        })
        .await
        .map_err(|error| refused_or_failed::<S>(&error))?;
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
    let level_then = match (input.level, input.project) {
        (Some(level), _) => Some(level),
        (None, Some(project)) => Some(
            ctx.people_level_without_override(role, project, input.kind)
                .await?,
        ),
        (None, None) => None,
    };
    if level_then.is_some_and(|level| level > access.kind_level(input.kind)) {
        return Err(ErrorCode::TooMuch.into());
    }
    let (project, kind, level) = (input.project, input.kind, input.level);
    let editor = actor.person.id;
    let before = ctx
        .store()
        .people_levels(role, project)
        .await
        .map_err(|error| internal::<S>(&error))?
        .into_iter()
        .find(|(known, _)| *known == kind)
        .map(|(_, level)| level);
    let event = ctx.event(
        by(actor.person.id),
        entity("role", role)?,
        vec![FieldChange {
            field: format!("people.{}", kind.code()),
            before: before.map(|level| json!(level.code())),
            after: level.map(|level| json!(level.code())),
        }],
    )?;
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                // The version of a move preview is made from these levels: a move waits for a
                // change of them, or the change for the move.
                S::lock_policy(transaction).await?;
                // The same as for fields: what is set on a project reaches what is inside it.
                if let (Some(project), Some(wanted)) = (project, level_then) {
                    for below in S::sub_projects_in(transaction, project).await? {
                        let held = held_in::<S>(transaction, editor, Place::Project(below)).await?;
                        if held.kind_level(kind) < wanted {
                            return Err(S::refused(Refusal::TooMuch));
                        }
                    }
                }
                S::put_people_level(transaction, role, project, kind, level).await
            })
        })
        .await
        .map_err(|error| refused_or_failed::<S>(&error))?;
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
