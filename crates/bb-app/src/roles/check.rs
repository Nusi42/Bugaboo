//! *Check access* (SEC-03, ADR-0025 decision 12): what a person can do in a project, which field
//! levels and people levels they have, and **why**, as data the browser turns into sentences.
//!
//! The decision and its reasons come from the engine in one pass, so they cannot disagree. A reason
//! never names a project the person who checks cannot see: it says the row is on *a project you
//! can't see* ([`ProjectLabel::Hidden`]).

use bb_core::ErrorCode;
use bb_core::access::{AppliesTo, FieldLevel, PeopleLevel};
use bb_core::fields::Field;
use bb_core::id::{AssignmentId, ProjectId, RoleId};
use bb_core::people_kinds::PeopleKind;
use bb_policy::{
    AccessReport, Authorized, LevelDecision, LevelSource, Masked, PeopleContext, Permission,
    PersonRef, Place, Reason, ReasonRow, Who,
};
use serde::Serialize;

use super::types::{CheckAccessQuery, RoleRef};
use crate::access::{Actor, ProjectLabel};
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::people::GroupView;

/// Whom a row of a reason was for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReasonWho {
    /// The person who is checked.
    Person,
    /// A group of theirs.
    Group {
        /// The group.
        group: GroupView,
    },
}

/// One row of *Who has which role* that played a part.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ReasonRowView {
    /// The ID of the row.
    pub assignment: AssignmentId,
    /// Whom it is for.
    pub who: ReasonWho,
    /// The project it is on, or `None` for the whole organization.
    pub project: Option<ProjectLabel>,
    /// Where it counts.
    pub applies_to: AppliesTo,
    /// Whether it comes from above the place that was checked.
    pub inherited: bool,
}

/// Why a permission is given or not, with the names filled in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReasonView {
    /// The role of a row gives it.
    GivenByRole {
        /// The row.
        row: ReasonRowView,
        /// The role.
        role: RoleRef,
    },
    /// *Decide who has which role* is kept from a row that *No access* removed.
    KeptFromRemovedRow {
        /// The row.
        row: ReasonRowView,
        /// Its role.
        role: RoleRef,
        /// The *No access* that decided.
        no_access: ReasonRowView,
    },
    /// The role of a row has it, but *No access* removed the role.
    RemovedByNoAccess {
        /// The row.
        row: ReasonRowView,
        /// Its role.
        role: RoleRef,
        /// The *No access* that decided.
        no_access: ReasonRowView,
    },
    /// The role has it, but it counts only for the whole organization.
    CountsOnlyForTheOrganization {
        /// The row.
        row: ReasonRowView,
        /// The role.
        role: RoleRef,
    },
    /// The access key does not keep it.
    NarrowedByKey,
    /// The access key does not count here.
    OutsideKeyProjects,
    /// No role gives it.
    NoRoleGivesIt,
}

/// One permission with *yes* or *no* and the reasons.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PermissionAnswer {
    /// The permission.
    pub permission: Permission,
    /// Whether it counts here.
    pub allowed: bool,
    /// Why.
    pub reasons: Vec<ReasonView>,
}

/// Where the level a role says was set.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LevelPlace {
    /// For a project (the place or the nearest parent that set one).
    Project {
        /// The project, as the viewer may name it.
        project: ProjectLabel,
    },
    /// The role's own level, for the whole organization.
    RoleOwn,
    /// Nothing is set: closed.
    Unset,
}

/// What one role says about a field or a kind.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RoleSays<L> {
    /// The role.
    pub role: RoleRef,
    /// Its level.
    pub level: L,
    /// Where it was set.
    pub set: LevelPlace,
}

/// A field or a kind with its level and what each role says.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LevelAnswer<K, L> {
    /// The field or kind.
    pub subject: K,
    /// The level the person has.
    pub level: L,
    /// What each role that counts says.
    pub roles: Vec<RoleSays<L>>,
    /// An access key lowered the level to *Can see*.
    pub capped_by_key: bool,
}

/// The answer of *Check access*.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AccessAnswer {
    /// The person who was checked, shown as the viewer may see them.
    pub person: PersonRef,
    /// The project that was checked, or `None` for the whole organization.
    pub place: Option<ProjectLabel>,
    /// Every permission.
    pub permissions: Vec<PermissionAnswer>,
    /// Every field.
    pub fields: Vec<LevelAnswer<Field, FieldLevel>>,
    /// Every kind of people information.
    pub kinds: Vec<LevelAnswer<PeopleKind, PeopleLevel>>,
}

impl Masked for AccessAnswer {}

impl<S: AuthStore> Ctx<'_, S> {
    async fn role_ref(&self, id: RoleId) -> Result<RoleRef, AppError> {
        let name = self
            .store()
            .role(id)
            .await
            .map_err(|error| internal::<S>(&error))?
            .map(|record| record.role.name)
            .unwrap_or_default();
        Ok(RoleRef { id, name })
    }

    async fn project_of_reason(
        &self,
        checker: &Actor,
        project: Option<ProjectId>,
    ) -> Result<Option<ProjectLabel>, AppError> {
        match project {
            Some(project) => Ok(Some(self.project_label(checker, project).await?)),
            None => Ok(None),
        }
    }

    async fn reason_row(
        &self,
        checker: &Actor,
        row: &ReasonRow,
    ) -> Result<ReasonRowView, AppError> {
        let who = match row.who {
            Who::Person => ReasonWho::Person,
            Who::Group(id) => ReasonWho::Group {
                group: self
                    .store()
                    .group(id)
                    .await
                    .map_err(|error| internal::<S>(&error))?
                    .as_ref()
                    .map_or_else(
                        || GroupView {
                            id,
                            name: String::new(),
                        },
                        GroupView::from,
                    ),
            },
        };
        Ok(ReasonRowView {
            assignment: row.assignment,
            who,
            project: self.project_of_reason(checker, row.project).await?,
            applies_to: row.applies_to,
            inherited: row.inherited,
        })
    }

    async fn reason_view(&self, checker: &Actor, reason: &Reason) -> Result<ReasonView, AppError> {
        Ok(match reason {
            Reason::GivenByRole { row, role } => ReasonView::GivenByRole {
                row: self.reason_row(checker, row).await?,
                role: self.role_ref(*role).await?,
            },
            Reason::KeptFromRemovedRow {
                row,
                role,
                no_access,
            } => ReasonView::KeptFromRemovedRow {
                row: self.reason_row(checker, row).await?,
                role: self.role_ref(*role).await?,
                no_access: self.reason_row(checker, no_access).await?,
            },
            Reason::RemovedByNoAccess {
                row,
                role,
                no_access,
            } => ReasonView::RemovedByNoAccess {
                row: self.reason_row(checker, row).await?,
                role: self.role_ref(*role).await?,
                no_access: self.reason_row(checker, no_access).await?,
            },
            Reason::CountsOnlyForTheOrganization { row, role } => {
                ReasonView::CountsOnlyForTheOrganization {
                    row: self.reason_row(checker, row).await?,
                    role: self.role_ref(*role).await?,
                }
            }
            Reason::NarrowedByKey => ReasonView::NarrowedByKey,
            Reason::OutsideKeyProjects => ReasonView::OutsideKeyProjects,
            Reason::NoRoleGivesIt => ReasonView::NoRoleGivesIt,
        })
    }

    async fn level_answer<K: Copy, L: Copy>(
        &self,
        checker: &Actor,
        decision: &LevelDecision<K, L>,
    ) -> Result<LevelAnswer<K, L>, AppError> {
        let mut roles = Vec::with_capacity(decision.roles.len());
        for says in &decision.roles {
            let set = match says.source {
                LevelSource::Project(project) => LevelPlace::Project {
                    project: self.project_label(checker, project).await?,
                },
                LevelSource::RoleOwn => LevelPlace::RoleOwn,
                LevelSource::Unset => LevelPlace::Unset,
            };
            roles.push(RoleSays {
                role: self.role_ref(says.role).await?,
                level: says.level,
                set,
            });
        }
        Ok(LevelAnswer {
            subject: decision.subject,
            level: decision.level,
            roles,
            capped_by_key: decision.capped_by_key,
        })
    }

    async fn access_answer(
        &self,
        checker: &Actor,
        report: &AccessReport,
        person: PersonRef,
    ) -> Result<AccessAnswer, AppError> {
        let mut permissions = Vec::with_capacity(report.permissions.len());
        for decision in &report.permissions {
            let mut reasons = Vec::with_capacity(decision.reasons.len());
            for reason in &decision.reasons {
                reasons.push(self.reason_view(checker, reason).await?);
            }
            permissions.push(PermissionAnswer {
                permission: decision.permission,
                allowed: decision.allowed,
                reasons,
            });
        }
        let mut fields = Vec::with_capacity(report.fields.len());
        for decision in &report.fields {
            fields.push(self.level_answer(checker, decision).await?);
        }
        let mut kinds = Vec::with_capacity(report.kinds.len());
        for decision in &report.kinds {
            kinds.push(self.level_answer(checker, decision).await?);
        }
        let place = match report.place {
            Place::Organization => None,
            Place::Project(project) => Some(self.project_label(checker, project).await?),
        };
        Ok(AccessAnswer {
            person,
            place,
            permissions,
            fields,
            kinds,
        })
    }
}

/// What a person can do in a project (or in the whole organization), which field levels and
/// people levels they have, and why.
///
/// # Errors
/// `access.forbidden` without *Check access* there (`resource.not_found` for a project the person
/// may not know about), `resource.not_found` for an unknown person to check.
pub async fn check_access<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CheckAccessQuery,
) -> Result<Authorized<AccessAnswer>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let place = input.place.map_or(Place::Organization, Place::Project);
    let checker = ctx.require(&actor, Permission::CheckAccess, place).await?;
    ctx.store()
        .person(input.person)
        .await
        .map_err(|error| internal::<S>(&error))?
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    let theirs = ctx.evaluate(input.person, place).await?;
    let person = ctx
        .person_ref(
            &checker,
            PeopleKind::ProjectMembers,
            PeopleContext::Management,
            input.person,
        )
        .await?;
    let answer = ctx.access_answer(&actor, &theirs.report(), person).await?;
    Ok(checker.authorize(answer))
}
