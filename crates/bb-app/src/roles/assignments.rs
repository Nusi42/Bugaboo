//! *Who has which role*: giving a role, or *No access*, to a person or a group, taking it away,
//! and listing the rows of a place (SEC-02, ADR-0025 decisions 2 and 6).

use bb_core::ErrorCode;
use bb_core::access::{AppliesTo, Assignment};
use bb_core::event::{FieldChange, RecordKind};
use bb_core::id::{AssignmentId, ProjectId};
use bb_core::people_kinds::PeopleKind;
use bb_core::sign_in::SecurityAction;
use bb_policy::guards::{Holder, ensure_a_manager_remains};
use bb_policy::{Authorized, PeopleContext, Permission, Place, row_reaches};
use serde_json::json;

use super::types::{
    AssignRole, AssignmentList, AssignmentView, AssignmentsQuery, Given, RemoveAssignment, Subject,
    WhoView,
};
use crate::access::{Actor, ensure_may_give_in};
use crate::auth::{AppError, AuthStore, Ctx, Done, Stops, by, change_failed, entity, internal};

/// Whether `new` and `old` are for the same person or group.
pub(super) fn same_subject(new: &Assignment, old: &Assignment) -> bool {
    (new.person.is_some() && new.person == old.person)
        || (new.group.is_some() && new.group == old.group)
}

impl<S: AuthStore> Ctx<'_, S> {
    /// A row as the viewer may see it: people as `PersonRef`, projects only when the viewer may
    /// know them, and whether it comes from above `place`.
    async fn assignment_view(
        &self,
        actor: &Actor,
        viewer: &bb_policy::Evaluation,
        row: &Assignment,
        place: Option<ProjectId>,
    ) -> Result<AssignmentView, AppError> {
        let holder =
            Holder::try_from(row).map_err(|error| AppError::Internal(error.to_string()))?;
        let who = match holder {
            Holder::Person(person) => WhoView::Person {
                person: self
                    .person_ref(
                        viewer,
                        PeopleKind::ProjectMembers,
                        PeopleContext::Management,
                        person,
                    )
                    .await?,
            },
            Holder::Group(group) => WhoView::Group {
                group: self.group_view(group).await?,
            },
        };
        let role = match row.role {
            Some(id) => Some(self.role_ref(id).await?),
            None => None,
        };
        let project = self.place_label(actor, row.project).await?;
        Ok(AssignmentView {
            id: row.id,
            who,
            role,
            project,
            applies_to: row.applies_to,
            inherited: row.project != place,
            given_at: row.given_at,
        })
    }
}

/// Gives a role, or *No access*, to a person or a group. *No access* is set only on a project and
/// always covers the project and its sub-projects.
///
/// # Errors
/// `access.forbidden` without *Decide who has which role* there (`resource.not-found` for a
/// project the person may not know about), `request.invalid` for a row that cannot exist,
/// `resource.not-found` for an unknown person, group or role, `access.too-much` for a role with
/// more than the person who asks holds there, `assignment.conflict` for a role and *No access*
/// on one project for the same person or group, or the same row twice.
pub async fn assign_role<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AssignRole,
) -> Result<Authorized<AssignmentView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let place = Place::from(input.project);
    // Rows for the whole organization give a role and count everywhere; *No access* is set on a
    // project and always covers it and its sub-projects.
    let valid = matches!(
        (input.project, input.given, input.applies_to),
        (None, Given::Role(_), AppliesTo::Organization)
            | (Some(_), Given::NoAccess, AppliesTo::ProjectAndSubProjects)
            | (
                Some(_),
                Given::Role(_),
                AppliesTo::Project | AppliesTo::ProjectAndSubProjects
            )
    );
    // Who asks is judged before what is asked, so a person who may not decide learns nothing.
    let access = ctx.require(&actor, Permission::DecideRoles, place).await?;
    if !valid {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let not_found = || AppError::Refused(ErrorCode::NotFound);
    let (person, group) = match input.subject {
        Subject::Person(person) => {
            ctx.store()
                .person(person)
                .await
                .map_err(|error| internal::<S>(&error))?
                .ok_or_else(not_found)?;
            (Some(person), None)
        }
        Subject::Group(group) => {
            ctx.store()
                .group(group)
                .await
                .map_err(|error| internal::<S>(&error))?
                .ok_or_else(not_found)?;
            (None, Some(group))
        }
    };
    let role = match input.given {
        Given::Role(role) => Some(ctx.role_or_missing(role).await?.role.id),
        Given::NoAccess => None,
    };
    let row = Assignment {
        id: AssignmentId::generate(&mut ctx.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?,
        person,
        group,
        role,
        project: input.project,
        applies_to: input.applies_to,
        given_at: ctx.now(),
    };
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Assignment, row.id),
        crate::event_members::assignment_given(&row),
    )?;
    let stored = row.clone();
    let giver = actor.person.id;
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_policy(transaction).await?;
                // Both checks look at what is stored now, after the lock: two people who each
                // give a row at the same moment cannot both pass on what was there before.
                ensure_may_give_in::<S>(transaction, giver, &stored).await?;
                for old in S::assignments_on_in(transaction, stored.project).await? {
                    // A role and *No access* on one project for the same person or group, or the
                    // same row twice: the row in the way is the one to take away first.
                    if same_subject(&stored, &old)
                        && (old.role.is_none() != stored.role.is_none()
                            || (old.role == stored.role && old.applies_to == stored.applies_to))
                    {
                        return Err(S::aborted());
                    }
                }
                S::insert_assignment(transaction, &stored).await
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::both(ErrorCode::AssignmentConflict)))?;
    ctx.note(
        SecurityAction::AssignmentCreated,
        Some(actor.person.id),
        Some(entity(RecordKind::Assignment, row.id)),
    )
    .await;
    let view = ctx
        .assignment_view(&actor, &access, &row, input.project)
        .await?;
    Ok(access.authorize(view))
}

/// Takes a row of *Who has which role* away.
///
/// # Errors
/// `resource.not-found` for an unknown row, `access.forbidden` without *Decide who has which
/// role* where the row is, `access.would-lock-out` when it would leave nobody who can manage
/// roles.
pub async fn remove_assignment<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: RemoveAssignment,
) -> Result<Done, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let row = ctx
        .store()
        .assignment(input.assignment)
        .await
        .map_err(|error| internal::<S>(&error))?
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    // A person who decides who has which role in a project can take away a *No access* there even
    // when it hides the project from them (ADR-0025, decision 6).
    ctx.require(&actor, Permission::DecideRoles, Place::from(row.project))
        .await?;
    let id = row.id;
    // The row that is taken away, so the history can say whom it was for and what it gave.
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::Assignment, id),
        vec![FieldChange {
            field: "assignment".to_owned(),
            before: Some(json!({
                "person": row.person.map(|person| person.to_string()),
                "group": row.group.map(|group| group.to_string()),
                "role": row.role.map(|role| role.to_string()),
                "project": row.project.map(|project| project.to_string()),
                "applies_to": row.applies_to.code(),
            })),
            after: None,
        }],
    )?;
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_policy(transaction).await?;
                let before = S::organization_state_in(transaction).await?;
                if !S::delete_assignment(transaction, id).await? {
                    return Err(S::aborted());
                }
                let after = S::organization_state_in(transaction).await?;
                ensure_a_manager_remains(&before, &after).map_err(S::refused)
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::both(ErrorCode::NotFound)))?;
    ctx.note(
        SecurityAction::AssignmentRemoved,
        Some(actor.person.id),
        Some(entity(RecordKind::Assignment, id)),
    )
    .await;
    Ok(Done)
}

/// *Who has which role* for a project (or the whole organization): the rows of the place first,
/// then the rows from above that reach it, a page at a time. People are shown by name (M2-Q3).
///
/// # Errors
/// `access.forbidden` without *Decide who has which role* there, `resource.not-found` for a
/// project the person may not know about, `request.invalid` for a cursor that is not one.
pub async fn assignments<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: AssignmentsQuery,
) -> Result<Authorized<AssignmentList>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let access = ctx
        .require(&actor, Permission::DecideRoles, Place::from(input.project))
        .await?;
    let mut rows = ctx
        .store()
        .assignments_on(input.project)
        .await
        .map_err(|error| internal::<S>(&error))?;
    if let Some(project) = input.project {
        // The parents, nearest first; and the whole organization last.
        let chain = ctx
            .store()
            .project_chain(project)
            .await
            .map_err(|error| internal::<S>(&error))?
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        for parent in chain.iter().rev().skip(1) {
            let above = ctx
                .store()
                .assignments_on(Some(*parent))
                .await
                .map_err(|error| internal::<S>(&error))?;
            rows.extend(
                above
                    .into_iter()
                    .filter(|row| row_reaches(row.project, row.applies_to, &chain)),
            );
        }
        rows.extend(
            ctx.store()
                .assignments_on(None)
                .await
                .map_err(|error| internal::<S>(&error))?,
        );
    }
    let page = bb_core::paging::window(rows.len(), input.cursor.as_deref(), input.limit)
        .ok_or(ErrorCode::InvalidRequest)?;
    let mut views = Vec::with_capacity(page.end - page.start);
    for row in &rows[page.start..page.end] {
        views.push(
            ctx.assignment_view(&actor, &access, row, input.project)
                .await?,
        );
    }
    Ok(access.authorize(AssignmentList {
        rows: views,
        next_cursor: page.next,
    }))
}
