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
use bb_core::fields::Field;
use bb_core::id::{ProjectId, RoleId};
use bb_core::people_kinds::PeopleKind;
use bb_policy::{Authorized, LevelSource, Masked, Permission, nearest_rule};
use serde::Serialize;

use super::{Actor, LevelRules, ProjectLabel, RoleRecord};
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::roles::{RoleRef, place_of};

/// The most roles a table holds, the columns of the matrix. More roles than this are cut off and
/// the answer says so (`truncated`).
pub const ROLES_MAX: usize = 100;

/// List the field permissions of the whole organization or of a project.
pub struct FieldPermissionsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project, or `None` for the levels the roles have of their own.
    pub project: Option<ProjectId>,
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
    /// Where it was set.
    pub origin: LevelOrigin,
}

/// One field or kind of people information, with the level of every role.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LevelRow<K, L> {
    /// The field or the kind.
    pub subject: K,
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
    /// The roles, in the order of roles: the columns.
    pub roles: Vec<RoleRef>,
    /// More roles than [`ROLES_MAX`] exist: the first ones are shown.
    pub truncated: bool,
    /// The fields or kinds, in the order of their catalogue: the rows.
    pub rows: Vec<LevelRow<K, L>>,
}

impl<K, L> Masked for LevelTable<K, L> {}

/// The field permissions: fields by roles.
pub type FieldPermissions = LevelTable<Field, FieldLevel>;
/// *Who sees people*: kinds of people information by roles.
pub type PeopleVisibility = LevelTable<PeopleKind, PeopleLevel>;

/// One level a role has set: the role, the project (`None` for its own) and the subject.
type Rule<K, L> = (RoleId, Option<ProjectId>, K, L);

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
    ) -> Result<LevelRules, AppError> {
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
            LevelSource::Project(above) => {
                let known = named
                    .iter()
                    .find(|(project, _)| *project == above)
                    .map(|(_, label)| label.clone());
                let label = if let Some(label) = known {
                    label
                } else {
                    let label = self.project_label(actor, above).await?;
                    named.push((above, label.clone()));
                    label
                };
                LevelOrigin::Inherited { from: label }
            }
            LevelSource::RoleOwn => LevelOrigin::RoleOwn,
            LevelSource::Unset => LevelOrigin::Unset,
        })
    }

    /// The table: for each subject and role the nearest level of the walk the engine makes (steps
    /// 7 and 8 of the evaluation), and where it was set.
    async fn level_table<K: Copy + PartialEq, L: Copy>(
        &self,
        actor: &Actor,
        wanted: Wanted<'_, K, L>,
    ) -> Result<LevelTable<K, L>, AppError> {
        let Wanted {
            project,
            roles,
            truncated,
            chain,
            rules,
            subjects,
            closed,
            yours,
        } = wanted;
        let mut named: Vec<(ProjectId, ProjectLabel)> = Vec::new();
        let mut rows = Vec::with_capacity(subjects.len());
        for (subject, yours) in subjects.iter().zip(yours) {
            let mut cells = Vec::with_capacity(roles.len());
            for record in roles {
                let role = record.role.id;
                let found = nearest_rule(
                    chain,
                    rules
                        .iter()
                        .filter(|(of, _, known, _)| *of == role && known == subject)
                        .map(|(_, at, _, level)| (*at, *level)),
                );
                let (source, level) = found.unwrap_or((LevelSource::Unset, closed));
                cells.push(LevelCell {
                    role,
                    level,
                    origin: self.origin_of(actor, project, source, &mut named).await?,
                });
            }
            rows.push(LevelRow {
                subject: *subject,
                yours: *yours,
                cells,
            });
        }
        let place = match project {
            Some(project) => Some(self.project_label(actor, project).await?),
            None => None,
        };
        Ok(LevelTable {
            place,
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
}

/// What [`Ctx::level_table`] makes a table from.
struct Wanted<'a, K, L> {
    project: Option<ProjectId>,
    roles: &'a [RoleRecord],
    truncated: bool,
    /// The projects from the top down to the place.
    chain: &'a [ProjectId],
    rules: &'a [Rule<K, L>],
    subjects: &'a [K],
    closed: L,
    /// What the person who asks may set, for each subject in the same order.
    yours: &'a [L],
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
    let access = ctx
        .require(
            &actor,
            Permission::DecideWhatRolesSee,
            place_of(input.project),
        )
        .await?;
    let (roles, truncated) = ctx.columns().await?;
    let read = ctx.rules_of(&roles, input.project).await?;
    let set: Vec<_> = read
        .fields
        .iter()
        .map(|rule| (rule.role, rule.project, rule.field, rule.level))
        .collect();
    let table = ctx
        .level_table(
            &actor,
            Wanted {
                project: input.project,
                roles: &roles,
                truncated,
                chain: &read.chain,
                rules: &set,
                subjects: Field::ALL,
                closed: FieldLevel::Hidden,
                yours: &Field::ALL
                    .iter()
                    .map(|field| access.field_level(*field))
                    .collect::<Vec<_>>(),
            },
        )
        .await?;
    Ok(access.authorize(table))
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
    let access = ctx
        .require(
            &actor,
            Permission::DecideWhatRolesSee,
            place_of(input.project),
        )
        .await?;
    let (roles, truncated) = ctx.columns().await?;
    let read = ctx.rules_of(&roles, input.project).await?;
    let set: Vec<_> = read
        .people
        .iter()
        .map(|rule| (rule.role, rule.project, rule.kind, rule.level))
        .collect();
    let table = ctx
        .level_table(
            &actor,
            Wanted {
                project: input.project,
                roles: &roles,
                truncated,
                chain: &read.chain,
                rules: &set,
                subjects: PeopleKind::ALL,
                closed: PeopleLevel::Hidden,
                yours: &PeopleKind::ALL
                    .iter()
                    .map(|kind| access.kind_level(*kind))
                    .collect::<Vec<_>>(),
            },
        )
        .await?;
    Ok(access.authorize(table))
}
