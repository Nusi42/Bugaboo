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
use bb_policy::{Authorized, Masked, Permission};
use serde::Serialize;

use super::{Actor, ProjectLabel, RoleRecord};
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

/// Where a role's level for a field or a kind came from, before a parent is named.
#[derive(Clone, Copy)]
enum Found {
    Project(ProjectId),
    RoleOwn,
    Unset,
}

/// The levels one role has set in one place.
struct Set<K, L> {
    role: RoleId,
    place: Option<ProjectId>,
    levels: Vec<(K, L)>,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The places a level is looked up in: the project itself, its parents nearest first, and
    /// last the role's own level (`None`).
    async fn places_from(
        &self,
        project: Option<ProjectId>,
    ) -> Result<Vec<Option<ProjectId>>, AppError> {
        let Some(project) = project else {
            return Ok(vec![None]);
        };
        let chain = self
            .store()
            .project_chain(project)
            .await
            .map_err(|error| internal::<S>(&error))?
            .unwrap_or_default();
        Ok(chain
            .iter()
            .rev()
            .map(|parent| Some(*parent))
            .chain(std::iter::once(None))
            .collect())
    }

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

    /// The table from what the roles have set in each place of the walk.
    async fn level_table<K: Copy + PartialEq, L: Copy>(
        &self,
        actor: &Actor,
        wanted: Wanted<'_, K, L>,
    ) -> Result<LevelTable<K, L>, AppError> {
        let Wanted {
            project,
            roles,
            truncated,
            places,
            sets,
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
                let (level, found) = places
                    .iter()
                    .find_map(|place| {
                        let set = sets
                            .iter()
                            .find(|set| set.role == role && set.place == *place)?;
                        let (_, level) = set.levels.iter().find(|(known, _)| known == subject)?;
                        Some((*level, place.map_or(Found::RoleOwn, Found::Project)))
                    })
                    .unwrap_or((closed, Found::Unset));
                let origin = match found {
                    Found::Project(above) if Some(above) == project => LevelOrigin::SetHere,
                    Found::Project(above) => {
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
                    Found::RoleOwn => LevelOrigin::RoleOwn,
                    Found::Unset => LevelOrigin::Unset,
                };
                cells.push(LevelCell {
                    role,
                    level,
                    origin,
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
    places: &'a [Option<ProjectId>],
    sets: &'a [Set<K, L>],
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
    let places = ctx.places_from(input.project).await?;
    let mut sets = Vec::with_capacity(roles.len() * places.len());
    for record in &roles {
        for place in &places {
            sets.push(Set {
                role: record.role.id,
                place: *place,
                levels: ctx
                    .store()
                    .field_levels(record.role.id, *place)
                    .await
                    .map_err(|error| internal::<S>(&error))?,
            });
        }
    }
    let table = ctx
        .level_table(
            &actor,
            Wanted {
                project: input.project,
                roles: &roles,
                truncated,
                places: &places,
                sets: &sets,
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
    let places = ctx.places_from(input.project).await?;
    let mut sets = Vec::with_capacity(roles.len() * places.len());
    for record in &roles {
        for place in &places {
            sets.push(Set {
                role: record.role.id,
                place: *place,
                levels: ctx
                    .store()
                    .people_levels(record.role.id, *place)
                    .await
                    .map_err(|error| internal::<S>(&error))?,
            });
        }
    }
    let table = ctx
        .level_table(
            &actor,
            Wanted {
                project: input.project,
                roles: &roles,
                truncated,
                places: &places,
                sets: &sets,
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
