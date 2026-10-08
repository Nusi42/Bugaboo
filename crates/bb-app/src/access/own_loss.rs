//! What a change would take away from the person who makes it (SEC-08, ADR-0025 decision 6).
//!
//! Nobody is left without a way to manage roles (that has its own guard), but a person can still
//! lose their *own* way: by taking away the row that gives it to them or to a group they are in,
//! by putting *No access* on themselves, by leaving the group that has it, or by changing or
//! deleting the role that gives it. The page asks first, and it asks only when this is so, so the
//! engine answers the question here, from the same rows and roles it decides with: the actor's
//! inputs with the change applied, evaluated again. Nothing is changed or written.

use std::collections::BTreeSet;

use bb_auth::looks_like_access_key;
use bb_core::ErrorCode;
use bb_core::access::AppliesTo;
use bb_core::id::{AssignmentId, GroupId, ProjectId, RoleId, UserId};
use bb_policy::{Authorized, Inputs, Masked, Permission, Place, RoleData, Row, Who, evaluate};
use serde::Serialize;

use super::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::roles::Subject;

/// The permissions that manage who may do what: losing one of them is asked about.
const MANAGING: [Permission; 2] = [Permission::ManageRoles, Permission::DecideRoles];

/// A change that is about to be made.
pub enum OwnChange {
    /// Take a row of *Who has which role* away.
    RemoveAssignment(AssignmentId),
    /// Put *No access* on a person or a group in a project.
    NoAccess {
        /// Whom.
        subject: Subject,
        /// The project.
        project: ProjectId,
    },
    /// Take a person out of a group.
    LeaveGroup {
        /// The group.
        group: GroupId,
        /// The person.
        person: UserId,
    },
    /// Delete a group (and with it the rows it has).
    DeleteGroup(GroupId),
    /// Give a role exactly these permissions.
    ChangeRole {
        /// The role.
        role: RoleId,
        /// What it allows afterwards.
        permissions: BTreeSet<Permission>,
    },
    /// Delete a role; everybody who has it gets `replacement`.
    DeleteRole {
        /// The role.
        role: RoleId,
        /// The role that takes its place.
        replacement: Option<RoleId>,
    },
}

/// Ask what a change would take away from the person who makes it.
pub struct OwnLossQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The change.
    pub change: OwnChange,
}

/// What the person would lose of the permissions that manage who may do what.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OwnLoss {
    /// The permissions the person holds now and would not hold after the change.
    pub loses: Vec<Permission>,
}

impl Masked for OwnLoss {}

/// How the change looks in the rows and roles the engine decides with.
enum Edit {
    /// Nothing the person holds changes.
    Nothing,
    /// A row is gone.
    DropRow(AssignmentId),
    /// The rows of a group are gone for the person.
    DropGroup(GroupId),
    /// A row of *No access* is there.
    AddRow(Row),
    /// A role allows these permissions.
    Permissions(RoleId, BTreeSet<Permission>),
    /// The rows that give a role give another one.
    Replace(RoleId, RoleData),
}

impl Edit {
    fn apply(&self, inputs: &mut Inputs) {
        match self {
            Self::Nothing => {}
            Self::DropRow(id) => inputs.rows.retain(|row| row.id != *id),
            Self::DropGroup(group) => inputs.rows.retain(|row| row.who != Who::Group(*group)),
            Self::AddRow(row) => inputs.rows.push(*row),
            Self::Permissions(role, permissions) => {
                for data in inputs.roles.iter_mut().filter(|data| data.id == *role) {
                    data.permissions.clone_from(permissions);
                }
            }
            Self::Replace(role, replacement) => {
                for row in inputs.rows.iter_mut().filter(|row| row.role == Some(*role)) {
                    row.role = Some(replacement.id);
                }
                if inputs.roles.iter().all(|data| data.id != replacement.id) {
                    inputs.roles.push(replacement.clone());
                }
            }
        }
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The role as the engine needs it, or `resource.not_found`.
    async fn role_data(&self, role: RoleId) -> Result<RoleData, AppError> {
        let record = self
            .store()
            .role(role)
            .await
            .map_err(|error| internal::<S>(&error))?
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        Ok(record.into())
    }

    /// The places to look at and the edit, for a change; the person must be allowed to make it.
    async fn edit_of(
        &self,
        actor: &Actor,
        change: &OwnChange,
    ) -> Result<(Option<ProjectId>, Edit, bb_policy::Evaluation), AppError> {
        let person = actor.person.id;
        match change {
            OwnChange::RemoveAssignment(id) => {
                let row = self
                    .store()
                    .assignment(*id)
                    .await
                    .map_err(|error| internal::<S>(&error))?
                    .ok_or(AppError::Refused(ErrorCode::NotFound))?;
                let place = Place::from(row.project);
                let access = self.require(actor, Permission::DecideRoles, place).await?;
                Ok((row.project, Edit::DropRow(*id), (*access).clone()))
            }
            OwnChange::NoAccess { subject, project } => {
                let access = self
                    .require(actor, Permission::DecideRoles, Place::Project(*project))
                    .await?;
                let who = match subject {
                    Subject::Person(who) if *who == person => Some(Who::Person),
                    Subject::Group(group) => self
                        .store()
                        .groups_of(person)
                        .await
                        .map_err(|error| internal::<S>(&error))?
                        .iter()
                        .any(|found| found.id == *group)
                        .then_some(Who::Group(*group)),
                    Subject::Person(_) => None,
                };
                let edit = match who {
                    Some(who) => Edit::AddRow(Row {
                        id: AssignmentId::generate(&mut self.ids())
                            .map_err(|_| AppError::Internal("no ID".into()))?,
                        who,
                        role: None,
                        project: Some(*project),
                        applies_to: AppliesTo::ProjectAndSubProjects,
                    }),
                    None => Edit::Nothing,
                };
                Ok((Some(*project), edit, (*access).clone()))
            }
            OwnChange::LeaveGroup {
                group,
                person: leaving,
            } => {
                let access = self
                    .require(actor, Permission::ManageGroups, Place::Organization)
                    .await?;
                let edit = if *leaving == person {
                    Edit::DropGroup(*group)
                } else {
                    Edit::Nothing
                };
                Ok((None, edit, (*access).clone()))
            }
            OwnChange::DeleteGroup(group) => {
                let access = self
                    .require(actor, Permission::ManageGroups, Place::Organization)
                    .await?;
                Ok((None, Edit::DropGroup(*group), (*access).clone()))
            }
            OwnChange::ChangeRole { role, permissions } => {
                let access = self
                    .require(actor, Permission::ManageRoles, Place::Organization)
                    .await?;
                Ok((
                    None,
                    Edit::Permissions(*role, permissions.clone()),
                    (*access).clone(),
                ))
            }
            OwnChange::DeleteRole { role, replacement } => {
                let access = self
                    .require(actor, Permission::ManageRoles, Place::Organization)
                    .await?;
                // A role that nobody has is deleted without a replacement: nothing is lost.
                let edit = match replacement {
                    Some(replacement) => Edit::Replace(*role, self.role_data(*replacement).await?),
                    None => Edit::Nothing,
                };
                Ok((None, edit, (*access).clone()))
            }
        }
    }

    /// The managing permissions the person holds at `place` now and would not hold after `edit`.
    async fn losses_at(
        &self,
        person: UserId,
        place: Place,
        edit: &Edit,
    ) -> Result<Vec<Permission>, AppError> {
        let Some(inputs) = self
            .store()
            .policy_inputs(person, place)
            .await
            .map_err(|error| internal::<S>(&error))?
        else {
            return Ok(Vec::new());
        };
        let before = evaluate(&inputs);
        let mut changed = inputs;
        edit.apply(&mut changed);
        let after = evaluate(&changed);
        Ok(MANAGING
            .into_iter()
            .filter(|permission| before.allows(*permission) && !after.allows(*permission))
            .collect())
    }
}

/// What a change would take away from the person who is about to make it: the permissions that
/// manage roles and who has which role, for the whole organization and for the project the change
/// is about. The page asks first when the list is not empty. Nothing is changed.
///
/// A person who may not make the change (or for whom the row, group or role is not there) is told
/// that it takes nothing: they could not make it, and nothing is said about what exists.
///
/// # Errors
/// The errors of the store, and `request.invalid` for a change that is not understood.
pub async fn own_loss<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: OwnLossQuery,
) -> Result<Authorized<OwnLoss>, AppError> {
    // An access key never manages anything (ADR-0025, decision 8), so it never asks.
    if looks_like_access_key(&input.session_token) {
        return Err(AppError::Refused(ErrorCode::AccessKeyNotAllowed));
    }
    let actor = ctx.actor(&input.session_token).await?;
    let person = actor.person.id;
    let nothing = |access: &bb_policy::Evaluation| access.authorize(OwnLoss { loses: Vec::new() });
    let (project, edit, access) = match ctx.edit_of(&actor, &input.change).await {
        Ok(found) => found,
        Err(AppError::Refused(ErrorCode::Forbidden | ErrorCode::NotFound)) => {
            return Ok(nothing(
                &*ctx.evaluation_of(&actor, Place::Organization).await?,
            ));
        }
        Err(error) => return Err(error),
    };
    let mut loses = ctx.losses_at(person, Place::Organization, &edit).await?;
    if let Some(project) = project {
        for permission in ctx
            .losses_at(person, Place::Project(project), &edit)
            .await?
        {
            if !loses.contains(&permission) {
                loses.push(permission);
            }
        }
    }
    Ok(access.authorize(OwnLoss { loses }))
}
