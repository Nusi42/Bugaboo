//! What each role sees, as the tables the screens show (SEC-04, PRIV-12, ADR-0025 decisions 10
//! and 11): the field permissions (fields by roles) and *Who sees people* (kinds of people
//! information by roles), for the whole organization or for one project.
//!
//! A cell is the level a role has in the place: the one set there, else the nearest parent's,
//! else the role's own, else the closed one (the same walk the engine makes, step 7 and 8 of
//! the evaluation), together with where it was set, so a screen can say *from ACME* or *set
//! here*. A parent the person may not know about is *a project you can't see* and never named.
//! Setting a cell is [`crate::roles::set_field_level`] and [`crate::roles::set_people_level`].

use bb_core::access::{FieldLevel, PeopleLevel};
use bb_core::fields::FieldKey;
use bb_core::id::{IssueTypeId, ProjectId, RoleId};
use bb_core::people_kinds::PeopleKind;
use bb_policy::{Authorized, Evaluation, LevelSource, LevelSubject, Masked, Permission, Place};
use serde::Serialize;

use super::{Actor, PlaceLevels, ProjectLabel, RoleRecord};
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::roles::{LevelPlace, RoleRef};

/// The most roles a table holds, the columns of the matrix. More roles than this are cut off and
/// the answer says so (`truncated`).
pub const ROLES_MAX: usize = 100;

/// List the field permissions of the whole organization or of a project.
pub struct FieldPermissionsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project, or `None` for the levels the roles have of their own.
    pub project: Option<ProjectId>,
    /// The issue type: the levels that count for issues of this type (ADR-0027), or `None` for
    /// the levels for all issue types.
    pub issue_type: Option<IssueTypeId>,
}

/// List how roles see people, for the whole organization or for a project.
pub struct PeopleVisibilityQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project, or `None` for the levels the roles have of their own.
    pub project: Option<ProjectId>,
}

/// Where the level of a cell was set.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LevelOrigin {
    /// For this project itself.
    SetHere,
    /// For a project above this one, which this project and its sub-projects inherit.
    Inherited {
        /// The parent, as the person who asks may name it.
        from: ProjectLabel,
    },
    /// The role's own level, for the whole organization.
    RoleOwn,
    /// Nothing is set: closed (default deny).
    Unset,
}

/// The level of one role.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LevelCell<L> {
    /// The role.
    pub role: RoleId,
    /// Its level in the place.
    pub level: L,
    /// The level was set for the issue type the table is about (not for all issue types).
    pub for_type: bool,
    /// Where it was set.
    pub origin: LevelOrigin,
}

/// One field or kind of people information, with the level of every role.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LevelRow<K, L> {
    /// The field or the kind.
    pub subject: K,
    /// The name of an own field; none for a field of the catalogue or a kind, whose names are
    /// messages.
    pub name: Option<String>,
    /// The highest level the person who asks may set here: their own.
    pub yours: L,
    /// One cell for each role, in the order of `roles`.
    pub cells: Vec<LevelCell<L>>,
}

/// The table of a place.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LevelTable<K, L> {
    /// The project, or `None` for the whole organization.
    pub place: Option<ProjectLabel>,
    /// The issue type the levels are for, or `None` for all issue types.
    pub issue_type: Option<IssueTypeId>,
    /// The roles, in the order of roles: the columns.
    pub roles: Vec<RoleRef>,
    /// More roles than [`ROLES_MAX`] exist: the first ones are shown.
    pub truncated: bool,
    /// The fields or kinds, in the order of their catalogue: the rows.
    pub rows: Vec<LevelRow<K, L>>,
}

impl<K, L> Masked for LevelTable<K, L> {}

/// The field permissions: fields by roles.
pub type FieldPermissions = LevelTable<FieldKey, FieldLevel>;
/// *Who sees people*: kinds of people information by roles.
pub type PeopleVisibility = LevelTable<PeopleKind, PeopleLevel>;

impl<S: AuthStore> Ctx<'_, S> {
    /// The roles for the columns, in the order of roles, and whether some were cut off.
    async fn columns(&self) -> Result<(Vec<RoleRecord>, bool), AppError> {
        let mut roles = self
            .store()
            .roles()
            .await
            .map_err(|error| internal::<S>(&error))?;
        let truncated = roles.len() > ROLES_MAX;
        roles.truncate(ROLES_MAX);
        Ok((roles, truncated))
    }

    /// Every level the shown roles have set, for the organization and the projects down to
    /// `project`, in one read.
    async fn rules_of(
        &self,
        roles: &[RoleRecord],
        project: Option<ProjectId>,
    ) -> Result<PlaceLevels, AppError> {
        self.store()
            .level_rules(roles.iter().map(|record| record.role.id).collect(), project)
            .await
            .map_err(|error| internal::<S>(&error))?
            .ok_or(AppError::Refused(bb_core::ErrorCode::NotFound))
    }

    /// Where a level was set, for a person who may or may not know the project it was set on.
    async fn origin_of(
        &self,
        actor: &Actor,
        place: Option<ProjectId>,
        source: LevelSource,
        named: &mut Vec<(ProjectId, ProjectLabel)>,
    ) -> Result<LevelOrigin, AppError> {
        Ok(match source {
            LevelSource::Project(above) if Some(above) == place => LevelOrigin::SetHere,
            other => match self.level_place(actor, other, named).await? {
                LevelPlace::Project { project } => LevelOrigin::Inherited { from: project },
                LevelPlace::RoleOwn => LevelOrigin::RoleOwn,
                LevelPlace::Unset => LevelOrigin::Unset,
            },
        })
    }

    /// The table: for each subject and role the nearest level of the walk the engine makes (steps
    /// 7 and 8 of the evaluation), and where it was set.
    async fn level_table<K: LevelSubject>(
        &self,
        actor: &Actor,
        wanted: Wanted<'_>,
    ) -> Result<LevelTable<K, K::Level>, AppError> {
        let Wanted {
            project,
            issue_type,
            roles,
            truncated,
            levels,
            access,
        } = wanted;
        let mut named: Vec<(ProjectId, ProjectLabel)> = Vec::new();
        let subjects = K::universe(&levels.rules);
        let mut rows = Vec::with_capacity(subjects.len());
        for subject in subjects {
            let mut cells = Vec::with_capacity(roles.len());
            for record in roles {
                let role = record.role.id;
                let (source, for_type, level) =
                    K::level_for(&levels.rules, &levels.chain, role, subject, issue_type)
                        .unwrap_or((LevelSource::Unset, false, K::CLOSED));
                cells.push(LevelCell {
                    role,
                    level,
                    for_type,
                    origin: self.origin_of(actor, project, source, &mut named).await?,
                });
            }
            rows.push(LevelRow {
                subject,
                name: None,
                yours: K::held_level(access, subject, issue_type),
                cells,
            });
        }
        Ok(LevelTable {
            place: self.place_label(actor, project).await?,
            issue_type,
            roles: roles
                .iter()
                .map(|record| RoleRef {
                    id: record.role.id,
                    name: record.role.name.clone(),
                })
                .collect(),
            truncated,
            rows,
        })
    }

    /// The table of one kind of level at a place, for the roles in the columns: what the person
    /// who asks sees of *Decide what roles see* there.
    async fn permissions_table<K: LevelSubject>(
        &self,
        actor: &Actor,
        project: Option<ProjectId>,
        issue_type: Option<IssueTypeId>,
    ) -> Result<Authorized<LevelTable<K, K::Level>>, AppError> {
        let access = self
            .require(actor, Permission::DecideWhatRolesSee, Place::from(project))
            .await?;
        let (roles, truncated) = self.columns().await?;
        let levels = self.rules_of(&roles, project).await?;
        let table = self
            .level_table(
                actor,
                Wanted {
                    project,
                    issue_type,
                    roles: &roles,
                    truncated,
                    levels: &levels,
                    access: &access,
                },
            )
            .await?;
        Ok(access.authorize(table))
    }
}

/// What [`Ctx::level_table`] makes a table from.
struct Wanted<'a> {
    project: Option<ProjectId>,
    issue_type: Option<IssueTypeId>,
    roles: &'a [RoleRecord],
    truncated: bool,
    /// The levels the roles have set, and the projects from the top down to the place.
    levels: &'a PlaceLevels,
    /// The levels of the person who asks, which they may set at most.
    access: &'a Evaluation,
}

/// The field permissions of the whole organization (the level each role has of its own) or of a
/// project (what applies there and where it was set), for every role.
///
/// # Errors
/// `access.forbidden` without *Decide what roles see* there, `resource.not_found` for a project
/// the person may not know about.
pub async fn field_permissions<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: FieldPermissionsQuery,
) -> Result<Authorized<FieldPermissions>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let table = ctx
        .permissions_table::<FieldKey>(&actor, input.project, input.issue_type)
        .await?;
    // Only the own fields the person may know at this place are listed, with their names.
    let place = input.project.map_or(Place::Organization, Place::Project);
    let held = ctx.evaluation_of(&actor, place).await?;
    let names = ctx.known_field_names(&actor, &held, input.project).await?;
    Ok(table.map(|mut table| {
        table
            .rows
            .retain(|row| row.subject.own().is_none() || names.contains_key(&row.subject));
        for row in &mut table.rows {
            row.name = names.get(&row.subject).cloned();
        }
        table
    }))
}

/// *Who sees people* of the whole organization (the level each role has of its own) or of a
/// project (what applies there and where it was set), for every role.
///
/// # Errors
/// The same as [`field_permissions`].
pub async fn people_visibility<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: PeopleVisibilityQuery,
) -> Result<Authorized<PeopleVisibility>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    ctx.permissions_table(&actor, input.project, None).await
}
