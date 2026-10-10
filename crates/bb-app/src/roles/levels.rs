//! Field levels and *Who sees people* (SEC-04, PRIV-12, ADR-0025 decisions 10 and 11): what each
//! role sees, for the whole organization (the role's own level) or for one project and its
//! sub-projects.
//!
//! The two kinds of level work the same way, so every step is written once over
//! [`LevelSubject`]; `set_field_level` and `set_people_level` only say which kind.
//!
//! *Decide what roles see* is needed where the level is set. Nobody sets a level above their own
//! at that place (the same rule as for permissions: nobody gives what they do not have).

use bb_core::ErrorCode;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::id::{IssueTypeId, ProjectId, RoleId};
use bb_core::sign_in::SecurityAction;
use bb_core::words::Word as _;
use bb_policy::guards::Refusal;
use bb_policy::{Authorized, LevelKind, LevelSubject, Permission, Place};
use serde_json::json;

use super::types::{FieldLevelView, LevelRequest, Levels, LevelsQuery, PeopleLevelView};
use crate::access::held_in;
use crate::auth::{AppError, AuthStore, Ctx, Done, Stops, by, change_failed, entity, internal};
use crate::projects::sub_projects_in;

/// What the security log says about a changed level of this kind.
const fn logged_as(kind: LevelKind) -> SecurityAction {
    match kind {
        LevelKind::Field => SecurityAction::FieldLevelChanged,
        LevelKind::People => SecurityAction::PeopleLevelChanged,
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The level a subject has for the role in the project once the project's own setting is
    /// taken away: the nearest parent's, else the role's own, else the closed one. Taking an
    /// override away must not raise a level above what the person who does it holds.
    async fn level_without_override<K: LevelSubject>(
        &self,
        role: RoleId,
        project: ProjectId,
        issue_type: Option<IssueTypeId>,
        subject: K,
    ) -> Result<K::Level, AppError> {
        let found = self
            .store()
            .level_rules(vec![role], Some(project))
            .await
            .map_err(|error| internal::<S>(&error))?
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        // The chain ends with the project itself, whose own setting is the one taken away.
        let parents = found
            .chain
            .split_last()
            .map_or(&[][..], |(_, parents)| parents);
        Ok(
            K::level_for(&found.rules, parents, role, subject, issue_type)
                .map_or(K::CLOSED, |(_, _, level)| level),
        )
    }

    /// The level a role has set for a subject in one place, if it has set one there.
    async fn level_set<K: LevelSubject>(
        &self,
        role: RoleId,
        project: Option<ProjectId>,
        issue_type: Option<IssueTypeId>,
        subject: K,
    ) -> Result<Option<K::Level>, AppError> {
        Ok(self
            .store()
            .levels::<K>(role, project, issue_type)
            .await
            .map_err(|error| internal::<S>(&error))?
            .into_iter()
            .find(|(known, _)| *known == subject)
            .map(|(_, level)| level))
    }
}

/// The name of the member of the event of a changed level: `field.title`, `people.followers`, and
/// for a field on one issue type `field.<key>.<issue type>`.
fn level_member<K: LevelSubject>(subject: K, issue_type: Option<IssueTypeId>) -> String {
    match issue_type {
        Some(issue_type) => format!("{}.{}.{issue_type}", K::KIND, subject.subject_code()),
        None => format!("{}.{}", K::KIND, subject.subject_code()),
    }
}

/// Sets, or takes away, the level of a field or of a kind of people information for a role
/// ([`SetFieldLevel`](super::SetFieldLevel) or [`SetPeopleLevel`](super::SetPeopleLevel)). A
/// role's own level cannot be taken away, only changed. Taking a project's setting away is the
/// same as setting the level that then applies there (the nearest parent's, else the role's own),
/// so that level is held to the same rule.
///
/// # Errors
/// `access.forbidden` without *Decide what roles see* there, `resource.not_found` for an unknown
/// role or a project the person may not know about, `request.invalid` for taking away a role's
/// own level, `access.too_much` for a level above the person's own.
pub async fn set_level<S: AuthStore, R: LevelRequest>(
    ctx: &Ctx<'_, S>,
    request: R,
) -> Result<Done, AppError> {
    let input = request.into_change();
    let actor = ctx.actor(&input.session_token).await?;
    ctx.require(
        &actor,
        Permission::DecideWhatRolesSee,
        Place::from(input.project),
    )
    .await?;
    let role = ctx.role_or_missing(input.role).await?.role.id;
    // A role's own level for all issue types always exists; one for an issue type is optional.
    if input.project.is_none() && input.issue_type.is_none() && input.level.is_none() {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let level_then = match (input.level, input.project) {
        (Some(level), _) => Some(level),
        (None, Some(project)) => Some(
            ctx.level_without_override(role, project, input.issue_type, input.subject)
                .await?,
        ),
        (None, None) => None,
    };
    let (project, issue_type, subject, level) =
        (input.project, input.issue_type, input.subject, input.level);
    let editor = actor.person.id;
    // What was there before, so the history can tell what a change did.
    let before = ctx.level_set(role, project, issue_type, subject).await?;
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Role, role),
        vec![FieldChange {
            field: level_member::<R::Subject>(subject, issue_type),
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
                // Nobody gives more than they hold, judged now, after the lock: in the place itself
                // and, for a level set on a project, in what is inside it.
                if let Some(wanted) = level_then {
                    let mut places = vec![Place::from(project)];
                    if let Some(project) = project {
                        places.extend(
                            sub_projects_in::<S>(transaction, project)
                                .await?
                                .into_iter()
                                .map(Place::Project),
                        );
                    }
                    for place in places {
                        let held = held_in::<S>(transaction, editor, place).await?;
                        if R::Subject::held_level(&held, subject, issue_type) < wanted {
                            return Err(S::refused(Refusal::TooMuch));
                        }
                    }
                }
                S::put_level(transaction, role, project, issue_type, subject, level).await
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::both(ErrorCode::NotFound)))?;
    ctx.note(
        logged_as(<R::Subject as LevelSubject>::KIND),
        Some(actor.person.id),
        Some(entity(RecordKind::Role, role)),
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
            Place::from(input.project),
        )
        .await?;
    let role = ctx.role_or_missing(input.role).await?.role.id;
    let fields = ctx
        .store()
        .levels(role, input.project, None)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let people = ctx
        .store()
        .levels(role, input.project, None)
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
