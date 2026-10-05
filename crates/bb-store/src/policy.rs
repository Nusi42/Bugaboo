//! The adapter of the policy port (`bb_app::access::PolicyStore`): projects, groups, roles, who has
//! which role, the levels of fields and of *Who sees people*, and the one call that loads what the
//! engine needs for a person at a place (SEC-03).
//!
//! Reads run in one snapshot (a read-only transaction), so the rows of one evaluation belong
//! together. Writes are associated functions that run inside the transaction of a change
//! ([`Database::record_change`]), so a change and its event are committed together.

use std::collections::{BTreeMap, BTreeSet};

use bb_app::access::{PolicyStore, RoleRecord};
use bb_core::access::{AppliesTo, Assignment, FieldLevel, Group, GroupMember, PeopleLevel, Role};
use bb_core::fields::Field;
use bb_core::id::{AssignmentId, GroupId, IdError, ProjectId, RoleId, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_core::project::{DEPTH_MAX, Project, ShortName};
use bb_core::technical::TechnicalName;
use bb_policy::guards::{Holder, OrganizationState};
use bb_policy::{FieldRule, Inputs, PeopleRule, Permission, Place, RoleData, Row, Who};
use jiff::Timestamp;
use sea_query::{Expr, ExprTrait as _, Order, Query, SelectStatement};

use crate::auth::AuthStorage;
use crate::db::{Row as DbRow, Transaction, to_micros};
use crate::error::StoreError;
use crate::schema::{
    AssignmentRow, GroupMemberRow, GroupRow, LegacyAdministratorRow, ProjectFieldLevelRow,
    ProjectPeopleLevelRow, ProjectRow, RoleFieldLevelRow, RolePeopleLevelRow, RolePermissionRow,
    RoleRow, StartingRolesMadeRow,
};

/// The lock that changes of roles and rows take on PostgreSQL, so two of them cannot both leave
/// the last administrator in place and together leave none.
const POLICY_LOCK: i64 = 0x4242_504F_4C49_0001;

pub(crate) fn corrupt(what: &'static str) -> impl Fn(IdError) -> StoreError {
    move |_| StoreError::Corrupt(what)
}

pub(crate) fn id_bytes(id: [u8; 16]) -> Vec<u8> {
    id.to_vec()
}

pub(crate) fn read_user(
    row: &DbRow,
    column: impl sea_query::Iden + Copy,
) -> Result<UserId, StoreError> {
    UserId::from_bytes(row.id_bytes(column)?).map_err(corrupt("a person ID"))
}

pub(crate) fn read_role_id(
    row: &DbRow,
    column: impl sea_query::Iden + Copy,
) -> Result<RoleId, StoreError> {
    RoleId::from_bytes(row.id_bytes(column)?).map_err(corrupt("a role ID"))
}

pub(crate) fn read_group_id(
    row: &DbRow,
    column: impl sea_query::Iden + Copy,
) -> Result<GroupId, StoreError> {
    GroupId::from_bytes(row.id_bytes(column)?).map_err(corrupt("a group ID"))
}

pub(crate) fn read_project_id(
    row: &DbRow,
    column: impl sea_query::Iden + Copy,
) -> Result<ProjectId, StoreError> {
    ProjectId::from_bytes(row.id_bytes(column)?).map_err(corrupt("a project ID"))
}

pub(crate) fn optional<T>(
    bytes: Option<[u8; 16]>,
    make: impl FnOnce([u8; 16]) -> Result<T, IdError>,
    what: &'static str,
) -> Result<Option<T>, StoreError> {
    bytes
        .map(|bytes| make(bytes).map_err(corrupt(what)))
        .transpose()
}

pub(crate) fn in_list<T: Copy>(ids: &[T], bytes: impl Fn(T) -> [u8; 16]) -> Vec<Vec<u8>> {
    ids.iter().map(|id| id_bytes(bytes(*id))).collect()
}

// ---- Reads that run in a snapshot ----

/// The projects from the top down to `project`; `None` when it does not exist.
pub(crate) async fn chain_of(
    transaction: &mut Transaction,
    project: ProjectId,
) -> Result<Option<Vec<ProjectId>>, StoreError> {
    let mut chain = vec![project];
    let mut current = project;
    loop {
        let mut select = Query::select();
        select
            .column(ProjectRow::Parent)
            .from(ProjectRow::Table)
            .and_where(Expr::col(ProjectRow::Id).eq(id_bytes(current.to_bytes())));
        let Some(row) = transaction.fetch_optional(&select).await? else {
            return Ok(None);
        };
        let Some(parent) = optional(
            row.optional_id_bytes(ProjectRow::Parent)?,
            ProjectId::from_bytes,
            "a project ID",
        )?
        else {
            chain.reverse();
            return Ok(Some(chain));
        };
        if chain.len() >= DEPTH_MAX || chain.contains(&parent) {
            return Err(StoreError::Corrupt("a tree of projects"));
        }
        chain.push(parent);
        current = parent;
    }
}

fn role_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            RoleRow::Id,
            RoleRow::Name,
            RoleRow::Starting,
            RoleRow::Position,
            RoleRow::TwoStep,
        ])
        .from(RoleRow::Table)
        .order_by(RoleRow::Position, Order::Asc)
        .order_by(RoleRow::Id, Order::Asc);
    select
}

fn read_role(row: &DbRow) -> Result<Role, StoreError> {
    Ok(Role {
        id: read_role_id(row, RoleRow::Id)?,
        name: row.get(RoleRow::Name)?,
        starting: row
            .get::<Option<String>>(RoleRow::Starting)?
            .map(|key| {
                TechnicalName::new(&key).map_err(|_| StoreError::Corrupt("a starting role key"))
            })
            .transpose()?,
        position: row.get(RoleRow::Position)?,
        two_step: row.get(RoleRow::TwoStep)?,
    })
}

/// The roles (all, or those with these IDs) with their permissions. A permission that the
/// catalogue does not know (one a later version dropped) is left out: that is the closed answer.
pub(crate) async fn roles_in(
    transaction: &mut Transaction,
    only: Option<&[RoleId]>,
) -> Result<Vec<RoleRecord>, StoreError> {
    let mut select = role_select();
    if let Some(only) = only {
        select.and_where(Expr::col(RoleRow::Id).is_in(in_list(only, RoleId::to_bytes)));
    }
    let roles = transaction
        .fetch_all(&select)
        .await?
        .iter()
        .map(read_role)
        .collect::<Result<Vec<_>, _>>()?;
    let mut permissions: BTreeMap<RoleId, BTreeSet<Permission>> = BTreeMap::new();
    if !roles.is_empty() {
        let ids: Vec<RoleId> = roles.iter().map(|role| role.id).collect();
        let mut select = Query::select();
        select
            .columns([RolePermissionRow::Role, RolePermissionRow::Permission])
            .from(RolePermissionRow::Table)
            .and_where(Expr::col(RolePermissionRow::Role).is_in(in_list(&ids, RoleId::to_bytes)));
        for row in transaction.fetch_all(&select).await? {
            let role = read_role_id(&row, RolePermissionRow::Role)?;
            if let Some(permission) =
                Permission::from_code(&row.get::<String>(RolePermissionRow::Permission)?)
            {
                permissions.entry(role).or_default().insert(permission);
            }
        }
    }
    Ok(roles
        .into_iter()
        .map(|role| RoleRecord {
            permissions: permissions.remove(&role.id).unwrap_or_default(),
            role,
        })
        .collect())
}

pub(crate) fn read_assignment(row: &DbRow) -> Result<Assignment, StoreError> {
    Ok(Assignment {
        id: AssignmentId::from_bytes(row.id_bytes(AssignmentRow::Id)?)
            .map_err(corrupt("an assignment ID"))?,
        person: optional(
            row.optional_id_bytes(AssignmentRow::Person)?,
            UserId::from_bytes,
            "a person ID",
        )?,
        group: optional(
            row.optional_id_bytes(AssignmentRow::Group)?,
            GroupId::from_bytes,
            "a group ID",
        )?,
        role: optional(
            row.optional_id_bytes(AssignmentRow::Role)?,
            RoleId::from_bytes,
            "a role ID",
        )?,
        project: optional(
            row.optional_id_bytes(AssignmentRow::Project)?,
            ProjectId::from_bytes,
            "a project ID",
        )?,
        applies_to: AppliesTo::from_code(&row.get::<String>(AssignmentRow::AppliesTo)?)
            .ok_or(StoreError::Corrupt("a place of a role"))?,
        given_at: row.time(AssignmentRow::GivenAt)?,
    })
}

pub(crate) fn assignment_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            AssignmentRow::Id,
            AssignmentRow::Person,
            AssignmentRow::Group,
            AssignmentRow::Role,
            AssignmentRow::Project,
            AssignmentRow::AppliesTo,
            AssignmentRow::GivenAt,
        ])
        .from(AssignmentRow::Table)
        .order_by(AssignmentRow::GivenAt, Order::Asc)
        .order_by(AssignmentRow::Id, Order::Asc);
    select
}

/// The rows on a project (`Some`) or for the whole organization (`None`).
fn assignments_on_select(project: Option<ProjectId>) -> SelectStatement {
    let mut select = assignment_select();
    match project {
        Some(project) => {
            select.and_where(Expr::col(AssignmentRow::Project).eq(id_bytes(project.to_bytes())));
        }
        None => {
            select.and_where(Expr::col(AssignmentRow::Project).is_null());
        }
    }
    select
}

/// The rows of a group, wherever they are.
fn assignments_of_group_select(group: GroupId) -> SelectStatement {
    let mut select = assignment_select();
    select.and_where(Expr::col(AssignmentRow::Group).eq(id_bytes(group.to_bytes())));
    select
}

/// Everything the engine needs for `person` at `place`, in one snapshot.
async fn inputs_in(
    transaction: &mut Transaction,
    person: UserId,
    place: Place,
) -> Result<Option<Inputs>, StoreError> {
    let chain = match place {
        Place::Organization => Vec::new(),
        Place::Project(project) => match chain_of(transaction, project).await? {
            Some(chain) => chain,
            None => return Ok(None),
        },
    };
    // The groups of the person, read once with their rows.
    let mut select = Query::select();
    select
        .column(GroupMemberRow::Group)
        .from(GroupMemberRow::Table)
        .and_where(Expr::col(GroupMemberRow::Person).eq(id_bytes(person.to_bytes())));
    let groups = transaction
        .fetch_all(&select)
        .await?
        .iter()
        .map(|row| read_group_id(row, GroupMemberRow::Group))
        .collect::<Result<Vec<_>, _>>()?;
    let mut select = assignment_select();
    let mut whose = Expr::col(AssignmentRow::Person).eq(id_bytes(person.to_bytes()));
    if !groups.is_empty() {
        whose =
            whose.or(Expr::col(AssignmentRow::Group).is_in(in_list(&groups, GroupId::to_bytes)));
    }
    let mut here = Expr::col(AssignmentRow::Project).is_null();
    if !chain.is_empty() {
        here =
            here.or(Expr::col(AssignmentRow::Project).is_in(in_list(&chain, ProjectId::to_bytes)));
    }
    select.and_where(whose).and_where(here);
    let assignments = transaction
        .fetch_all(&select)
        .await?
        .iter()
        .map(read_assignment)
        .collect::<Result<Vec<_>, _>>()?;
    let rows: Vec<Row> = assignments
        .iter()
        .filter_map(|assignment| {
            let who = if assignment.person == Some(person) {
                Who::Person
            } else {
                Who::Group(assignment.group?)
            };
            Some(Row {
                id: assignment.id,
                who,
                role: assignment.role,
                project: assignment.project,
                applies_to: assignment.applies_to,
            })
        })
        .collect();
    let mut role_ids: Vec<RoleId> = rows.iter().filter_map(|row| row.role).collect();
    role_ids.sort();
    role_ids.dedup();
    let roles = if role_ids.is_empty() {
        Vec::new()
    } else {
        roles_in(transaction, Some(&role_ids)).await?
    };
    let (field_rules, people_rules) = if role_ids.is_empty() {
        (Vec::new(), Vec::new())
    } else {
        level_rules(transaction, &role_ids, &chain).await?
    };
    Ok(Some(Inputs {
        person,
        place,
        chain,
        rows,
        roles: roles
            .into_iter()
            .map(|record| RoleData {
                id: record.role.id,
                name: record.role.name,
                position: record.role.position,
                permissions: record.permissions,
                two_step: record.role.two_step,
            })
            .collect(),
        field_rules,
        people_rules,
    }))
}

/// The field levels and the people levels of the roles: their own, and those set for the
/// projects of the chain.
async fn level_rules(
    transaction: &mut Transaction,
    roles: &[RoleId],
    chain: &[ProjectId],
) -> Result<(Vec<FieldRule>, Vec<PeopleRule>), StoreError> {
    let role_list = in_list(roles, RoleId::to_bytes);
    let project_list = in_list(chain, ProjectId::to_bytes);
    let mut fields = Vec::new();
    let mut select = Query::select();
    select
        .columns([
            RoleFieldLevelRow::Role,
            RoleFieldLevelRow::Field,
            RoleFieldLevelRow::Level,
        ])
        .from(RoleFieldLevelRow::Table)
        .and_where(Expr::col(RoleFieldLevelRow::Role).is_in(role_list.clone()));
    for row in transaction.fetch_all(&select).await? {
        fields.push(FieldRule {
            project: None,
            role: read_role_id(&row, RoleFieldLevelRow::Role)?,
            field: read_field(&row, RoleFieldLevelRow::Field)?,
            level: read_field_level(&row, RoleFieldLevelRow::Level)?,
        });
    }
    let mut people = Vec::new();
    let mut select = Query::select();
    select
        .columns([
            RolePeopleLevelRow::Role,
            RolePeopleLevelRow::Kind,
            RolePeopleLevelRow::Level,
        ])
        .from(RolePeopleLevelRow::Table)
        .and_where(Expr::col(RolePeopleLevelRow::Role).is_in(role_list.clone()));
    for row in transaction.fetch_all(&select).await? {
        people.push(PeopleRule {
            project: None,
            role: read_role_id(&row, RolePeopleLevelRow::Role)?,
            kind: read_kind(&row, RolePeopleLevelRow::Kind)?,
            level: read_people_level(&row, RolePeopleLevelRow::Level)?,
        });
    }
    if !chain.is_empty() {
        let mut select = Query::select();
        select
            .columns([
                ProjectFieldLevelRow::Project,
                ProjectFieldLevelRow::Role,
                ProjectFieldLevelRow::Field,
                ProjectFieldLevelRow::Level,
            ])
            .from(ProjectFieldLevelRow::Table)
            .and_where(Expr::col(ProjectFieldLevelRow::Role).is_in(role_list.clone()))
            .and_where(Expr::col(ProjectFieldLevelRow::Project).is_in(project_list.clone()));
        for row in transaction.fetch_all(&select).await? {
            fields.push(FieldRule {
                project: Some(read_project_id(&row, ProjectFieldLevelRow::Project)?),
                role: read_role_id(&row, ProjectFieldLevelRow::Role)?,
                field: read_field(&row, ProjectFieldLevelRow::Field)?,
                level: read_field_level(&row, ProjectFieldLevelRow::Level)?,
            });
        }
        let mut select = Query::select();
        select
            .columns([
                ProjectPeopleLevelRow::Project,
                ProjectPeopleLevelRow::Role,
                ProjectPeopleLevelRow::Kind,
                ProjectPeopleLevelRow::Level,
            ])
            .from(ProjectPeopleLevelRow::Table)
            .and_where(Expr::col(ProjectPeopleLevelRow::Role).is_in(role_list))
            .and_where(Expr::col(ProjectPeopleLevelRow::Project).is_in(project_list));
        for row in transaction.fetch_all(&select).await? {
            people.push(PeopleRule {
                project: Some(read_project_id(&row, ProjectPeopleLevelRow::Project)?),
                role: read_role_id(&row, ProjectPeopleLevelRow::Role)?,
                kind: read_kind(&row, ProjectPeopleLevelRow::Kind)?,
                level: read_people_level(&row, ProjectPeopleLevelRow::Level)?,
            });
        }
    }
    Ok((fields, people))
}

pub(crate) fn read_field(
    row: &DbRow,
    column: impl sea_query::Iden + Copy,
) -> Result<Field, StoreError> {
    Field::from_code(&row.get::<String>(column)?).ok_or(StoreError::Corrupt("a field key"))
}

pub(crate) fn read_kind(
    row: &DbRow,
    column: impl sea_query::Iden + Copy,
) -> Result<PeopleKind, StoreError> {
    PeopleKind::from_code(&row.get::<String>(column)?)
        .ok_or(StoreError::Corrupt("a kind of people information"))
}

pub(crate) fn read_field_level(
    row: &DbRow,
    column: impl sea_query::Iden + Copy,
) -> Result<FieldLevel, StoreError> {
    FieldLevel::from_code(&row.get::<String>(column)?).ok_or(StoreError::Corrupt("a field level"))
}

pub(crate) fn read_people_level(
    row: &DbRow,
    column: impl sea_query::Iden + Copy,
) -> Result<PeopleLevel, StoreError> {
    PeopleLevel::from_code(&row.get::<String>(column)?).ok_or(StoreError::Corrupt("a people level"))
}

/// What the last-administrator rule needs: the rows for the whole organization that give a role,
/// the roles, and the members of the groups that hold one.
async fn organization_state_of(
    transaction: &mut Transaction,
) -> Result<OrganizationState, StoreError> {
    let mut select = assignment_select();
    select
        .and_where(Expr::col(AssignmentRow::Project).is_null())
        .and_where(Expr::col(AssignmentRow::Role).is_not_null());
    let assignments = transaction
        .fetch_all(&select)
        .await?
        .iter()
        .map(read_assignment)
        .collect::<Result<Vec<_>, _>>()?;
    let mut rows = Vec::new();
    let mut groups = Vec::new();
    for assignment in &assignments {
        let Some(role) = assignment.role else {
            continue;
        };
        match (assignment.person, assignment.group) {
            (Some(person), _) => rows.push((Holder::Person(person), role)),
            (None, Some(group)) => {
                rows.push((Holder::Group(group), role));
                groups.push(group);
            }
            (None, None) => return Err(StoreError::Corrupt("a row for nobody")),
        }
    }
    groups.sort();
    groups.dedup();
    let mut members = Vec::new();
    if !groups.is_empty() {
        let mut select = Query::select();
        select
            .columns([GroupMemberRow::Group, GroupMemberRow::Person])
            .from(GroupMemberRow::Table)
            .and_where(Expr::col(GroupMemberRow::Group).is_in(in_list(&groups, GroupId::to_bytes)));
        for row in transaction.fetch_all(&select).await? {
            members.push((
                read_group_id(&row, GroupMemberRow::Group)?,
                read_user(&row, GroupMemberRow::Person)?,
            ));
        }
    }
    let role_ids: Vec<RoleId> = rows.iter().map(|(_, role)| *role).collect();
    let roles = if role_ids.is_empty() {
        Vec::new()
    } else {
        roles_in(transaction, Some(&role_ids)).await?
    };
    Ok(OrganizationState {
        roles: roles
            .into_iter()
            .map(|record| RoleData {
                id: record.role.id,
                name: record.role.name,
                position: record.role.position,
                permissions: record.permissions,
                two_step: record.role.two_step,
            })
            .collect(),
        rows,
        members,
    })
}

pub(crate) fn read_project(row: &DbRow) -> Result<Project, StoreError> {
    Ok(Project {
        id: read_project_id(row, ProjectRow::Id)?,
        parent: optional(
            row.optional_id_bytes(ProjectRow::Parent)?,
            ProjectId::from_bytes,
            "a project ID",
        )?,
        short_name: ShortName::new(&row.get::<String>(ProjectRow::ShortName)?)
            .map_err(|_| StoreError::Corrupt("a short name"))?,
        name: row.get(ProjectRow::Name)?,
        version: row.get(ProjectRow::Version)?,
    })
}

pub(crate) fn project_select() -> SelectStatement {
    let mut select = Query::select();
    select
        .columns([
            ProjectRow::Id,
            ProjectRow::Parent,
            ProjectRow::ShortName,
            ProjectRow::Name,
            ProjectRow::Version,
        ])
        .from(ProjectRow::Table);
    select
}

impl PolicyStore for AuthStorage {
    async fn policy_inputs(
        &self,
        person: UserId,
        place: Place,
    ) -> Result<Option<Inputs>, StoreError> {
        let mut transaction = self.database().begin_read().await?;
        inputs_in(&mut transaction, person, place).await
    }

    async fn organization_state(&self) -> Result<OrganizationState, StoreError> {
        let mut transaction = self.database().begin_read().await?;
        organization_state_of(&mut transaction).await
    }

    async fn starting_roles_made(&self) -> Result<bool, StoreError> {
        let mut select = Query::select();
        select
            .column(StartingRolesMadeRow::MadeAt)
            .from(StartingRolesMadeRow::Table)
            .limit(1);
        Ok(self.database().fetch_optional(&select).await?.is_some())
    }

    async fn legacy_administrators(&self) -> Result<Vec<UserId>, StoreError> {
        let mut select = Query::select();
        select
            .column(LegacyAdministratorRow::Person)
            .from(LegacyAdministratorRow::Table)
            .order_by(LegacyAdministratorRow::Person, Order::Asc);
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(|row| read_user(row, LegacyAdministratorRow::Person))
            .collect()
    }

    async fn roles(&self) -> Result<Vec<RoleRecord>, StoreError> {
        let mut transaction = self.database().begin_read().await?;
        roles_in(&mut transaction, None).await
    }

    async fn role(&self, id: RoleId) -> Result<Option<RoleRecord>, StoreError> {
        let mut transaction = self.database().begin_read().await?;
        Ok(roles_in(&mut transaction, Some(&[id])).await?.pop())
    }

    async fn groups(&self) -> Result<Vec<Group>, StoreError> {
        let mut select = Query::select();
        select
            .columns([GroupRow::Id, GroupRow::Name])
            .from(GroupRow::Table)
            .order_by(GroupRow::Name, Order::Asc)
            .order_by(GroupRow::Id, Order::Asc);
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_group)
            .collect()
    }

    async fn group(&self, id: GroupId) -> Result<Option<Group>, StoreError> {
        let mut select = Query::select();
        select
            .columns([GroupRow::Id, GroupRow::Name])
            .from(GroupRow::Table)
            .and_where(Expr::col(GroupRow::Id).eq(id_bytes(id.to_bytes())));
        self.database()
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_group)
            .transpose()
    }

    async fn group_members(&self, group: GroupId) -> Result<Vec<GroupMember>, StoreError> {
        let mut select = Query::select();
        select
            .columns([
                GroupMemberRow::Group,
                GroupMemberRow::Person,
                GroupMemberRow::AddedAt,
            ])
            .from(GroupMemberRow::Table)
            .and_where(Expr::col(GroupMemberRow::Group).eq(id_bytes(group.to_bytes())))
            .order_by(GroupMemberRow::AddedAt, Order::Asc)
            .order_by(GroupMemberRow::Person, Order::Asc);
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(|row| {
                Ok(GroupMember {
                    group: read_group_id(row, GroupMemberRow::Group)?,
                    person: read_user(row, GroupMemberRow::Person)?,
                    added_at: row.time(GroupMemberRow::AddedAt)?,
                })
            })
            .collect()
    }

    async fn assignments_on(
        &self,
        project: Option<ProjectId>,
    ) -> Result<Vec<Assignment>, StoreError> {
        self.database()
            .fetch_all(&assignments_on_select(project))
            .await?
            .iter()
            .map(read_assignment)
            .collect()
    }

    async fn assignments_of_group(&self, group: GroupId) -> Result<Vec<Assignment>, StoreError> {
        let select = assignments_of_group_select(group);
        self.database()
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_assignment)
            .collect()
    }

    async fn assignment(&self, id: AssignmentId) -> Result<Option<Assignment>, StoreError> {
        let mut select = assignment_select();
        select.and_where(Expr::col(AssignmentRow::Id).eq(id_bytes(id.to_bytes())));
        self.database()
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_assignment)
            .transpose()
    }

    async fn project(&self, id: ProjectId) -> Result<Option<Project>, StoreError> {
        let mut select = project_select();
        select.and_where(Expr::col(ProjectRow::Id).eq(id_bytes(id.to_bytes())));
        self.database()
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_project)
            .transpose()
    }

    async fn project_chain(&self, id: ProjectId) -> Result<Option<Vec<ProjectId>>, StoreError> {
        let mut transaction = self.database().begin_read().await?;
        chain_of(&mut transaction, id).await
    }

    async fn project_by_short_name(&self, name: &ShortName) -> Result<Option<Project>, StoreError> {
        let mut select = project_select();
        select.and_where(Expr::col(ProjectRow::ShortName).eq(name.as_str()));
        self.database()
            .fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_project)
            .transpose()
    }

    async fn field_levels(
        &self,
        role: RoleId,
        project: Option<ProjectId>,
    ) -> Result<Vec<(Field, FieldLevel)>, StoreError> {
        let mut select = Query::select();
        match project {
            None => {
                select
                    .columns([RoleFieldLevelRow::Field, RoleFieldLevelRow::Level])
                    .from(RoleFieldLevelRow::Table)
                    .and_where(Expr::col(RoleFieldLevelRow::Role).eq(id_bytes(role.to_bytes())));
                self.database()
                    .fetch_all(&select)
                    .await?
                    .iter()
                    .map(|row| {
                        Ok((
                            read_field(row, RoleFieldLevelRow::Field)?,
                            read_field_level(row, RoleFieldLevelRow::Level)?,
                        ))
                    })
                    .collect()
            }
            Some(project) => {
                select
                    .columns([ProjectFieldLevelRow::Field, ProjectFieldLevelRow::Level])
                    .from(ProjectFieldLevelRow::Table)
                    .and_where(Expr::col(ProjectFieldLevelRow::Role).eq(id_bytes(role.to_bytes())))
                    .and_where(
                        Expr::col(ProjectFieldLevelRow::Project).eq(id_bytes(project.to_bytes())),
                    );
                self.database()
                    .fetch_all(&select)
                    .await?
                    .iter()
                    .map(|row| {
                        Ok((
                            read_field(row, ProjectFieldLevelRow::Field)?,
                            read_field_level(row, ProjectFieldLevelRow::Level)?,
                        ))
                    })
                    .collect()
            }
        }
    }

    async fn people_levels(
        &self,
        role: RoleId,
        project: Option<ProjectId>,
    ) -> Result<Vec<(PeopleKind, PeopleLevel)>, StoreError> {
        let mut select = Query::select();
        match project {
            None => {
                select
                    .columns([RolePeopleLevelRow::Kind, RolePeopleLevelRow::Level])
                    .from(RolePeopleLevelRow::Table)
                    .and_where(Expr::col(RolePeopleLevelRow::Role).eq(id_bytes(role.to_bytes())));
                self.database()
                    .fetch_all(&select)
                    .await?
                    .iter()
                    .map(|row| {
                        Ok((
                            read_kind(row, RolePeopleLevelRow::Kind)?,
                            read_people_level(row, RolePeopleLevelRow::Level)?,
                        ))
                    })
                    .collect()
            }
            Some(project) => {
                select
                    .columns([ProjectPeopleLevelRow::Kind, ProjectPeopleLevelRow::Level])
                    .from(ProjectPeopleLevelRow::Table)
                    .and_where(Expr::col(ProjectPeopleLevelRow::Role).eq(id_bytes(role.to_bytes())))
                    .and_where(
                        Expr::col(ProjectPeopleLevelRow::Project).eq(id_bytes(project.to_bytes())),
                    );
                self.database()
                    .fetch_all(&select)
                    .await?
                    .iter()
                    .map(|row| {
                        Ok((
                            read_kind(row, ProjectPeopleLevelRow::Kind)?,
                            read_people_level(row, ProjectPeopleLevelRow::Level)?,
                        ))
                    })
                    .collect()
            }
        }
    }

    async fn lock_policy(transaction: &mut Transaction) -> Result<(), StoreError> {
        transaction.advisory_lock(POLICY_LOCK).await
    }

    async fn organization_state_in(
        transaction: &mut Transaction,
    ) -> Result<OrganizationState, StoreError> {
        organization_state_of(transaction).await
    }

    async fn policy_inputs_in(
        transaction: &mut Transaction,
        person: UserId,
        place: Place,
    ) -> Result<Option<Inputs>, StoreError> {
        inputs_in(transaction, person, place).await
    }

    async fn assignments_on_in(
        transaction: &mut Transaction,
        project: Option<ProjectId>,
    ) -> Result<Vec<Assignment>, StoreError> {
        transaction
            .fetch_all(&assignments_on_select(project))
            .await?
            .iter()
            .map(read_assignment)
            .collect()
    }

    async fn assignments_of_group_in(
        transaction: &mut Transaction,
        group: GroupId,
    ) -> Result<Vec<Assignment>, StoreError> {
        transaction
            .fetch_all(&assignments_of_group_select(group))
            .await?
            .iter()
            .map(read_assignment)
            .collect()
    }

    async fn role_in(
        transaction: &mut Transaction,
        id: RoleId,
    ) -> Result<Option<RoleRecord>, StoreError> {
        Ok(roles_in(transaction, Some(&[id])).await?.pop())
    }

    async fn sub_projects_in(
        transaction: &mut Transaction,
        project: ProjectId,
    ) -> Result<Vec<ProjectId>, StoreError> {
        let mut select = Query::select();
        select
            .columns([ProjectRow::Id, ProjectRow::Parent])
            .from(ProjectRow::Table);
        let mut children: BTreeMap<ProjectId, Vec<ProjectId>> = BTreeMap::new();
        for row in transaction.fetch_all(&select).await? {
            let id = read_project_id(&row, ProjectRow::Id)?;
            if let Some(parent) = optional(
                row.optional_id_bytes(ProjectRow::Parent)?,
                ProjectId::from_bytes,
                "a project ID",
            )? {
                children.entry(parent).or_default().push(id);
            }
        }
        // Breadth first; a stored loop is cut short, not followed.
        let mut found: Vec<ProjectId> = Vec::new();
        let mut next = vec![project];
        while let Some(current) = next.pop() {
            for child in children.get(&current).into_iter().flatten() {
                if *child != project && !found.contains(child) {
                    found.push(*child);
                    next.push(*child);
                }
            }
        }
        Ok(found)
    }

    async fn starting_roles_made_in(transaction: &mut Transaction) -> Result<bool, StoreError> {
        let mut select = Query::select();
        select
            .column(StartingRolesMadeRow::MadeAt)
            .from(StartingRolesMadeRow::Table)
            .limit(1);
        Ok(transaction.fetch_optional(&select).await?.is_some())
    }

    async fn mark_starting_roles_made(
        transaction: &mut Transaction,
        at: Timestamp,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(StartingRolesMadeRow::Table)
            .columns([StartingRolesMadeRow::MadeAt])
            .values([Expr::val(to_micros(at))])
            .map_err(|_| StoreError::Corrupt("a mark"))?;
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn delete_legacy_administrator(
        transaction: &mut Transaction,
        person: UserId,
    ) -> Result<(), StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(LegacyAdministratorRow::Table)
            .and_where(Expr::col(LegacyAdministratorRow::Person).eq(id_bytes(person.to_bytes())));
        transaction.execute(&delete).await.map(|_| ())
    }

    async fn insert_role(
        transaction: &mut Transaction,
        role: &Role,
        permissions: &[Permission],
        fields: &[(Field, FieldLevel)],
        people: &[(PeopleKind, PeopleLevel)],
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(RoleRow::Table)
            .columns([
                RoleRow::Id,
                RoleRow::Name,
                RoleRow::Starting,
                RoleRow::Position,
                RoleRow::TwoStep,
            ])
            .values([
                Expr::val(id_bytes(role.id.to_bytes())),
                Expr::val(role.name.as_str()),
                Expr::val(role.starting.as_ref().map(TechnicalName::as_str)),
                Expr::val(role.position),
                Expr::val(role.two_step),
            ])
            .map_err(|_| StoreError::Corrupt("a role"))?;
        transaction.execute(&insert).await?;
        write_permissions(transaction, role.id, permissions).await?;
        write_own_levels(transaction, role.id, fields, people).await
    }

    async fn update_role(
        transaction: &mut Transaction,
        id: RoleId,
        name: &str,
        two_step: bool,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(RoleRow::Table)
            .value(RoleRow::Name, name)
            .value(RoleRow::TwoStep, two_step)
            .and_where(Expr::col(RoleRow::Id).eq(id_bytes(id.to_bytes())));
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn replace_role_permissions(
        transaction: &mut Transaction,
        id: RoleId,
        permissions: &[Permission],
    ) -> Result<(), StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(RolePermissionRow::Table)
            .and_where(Expr::col(RolePermissionRow::Role).eq(id_bytes(id.to_bytes())));
        transaction.execute(&delete).await?;
        write_permissions(transaction, id, permissions).await
    }

    async fn replace_role_levels(
        transaction: &mut Transaction,
        id: RoleId,
        fields: &[(Field, FieldLevel)],
        people: &[(PeopleKind, PeopleLevel)],
    ) -> Result<(), StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(RoleFieldLevelRow::Table)
            .and_where(Expr::col(RoleFieldLevelRow::Role).eq(id_bytes(id.to_bytes())));
        transaction.execute(&delete).await?;
        let mut delete = Query::delete();
        delete
            .from_table(RolePeopleLevelRow::Table)
            .and_where(Expr::col(RolePeopleLevelRow::Role).eq(id_bytes(id.to_bytes())));
        transaction.execute(&delete).await?;
        write_own_levels(transaction, id, fields, people).await
    }

    async fn delete_role(transaction: &mut Transaction, id: RoleId) -> Result<bool, StoreError> {
        let role = id_bytes(id.to_bytes());
        delete_by(
            transaction,
            AssignmentRow::Table,
            AssignmentRow::Role,
            &role,
        )
        .await?;
        delete_by(
            transaction,
            RolePermissionRow::Table,
            RolePermissionRow::Role,
            &role,
        )
        .await?;
        delete_by(
            transaction,
            RoleFieldLevelRow::Table,
            RoleFieldLevelRow::Role,
            &role,
        )
        .await?;
        delete_by(
            transaction,
            ProjectFieldLevelRow::Table,
            ProjectFieldLevelRow::Role,
            &role,
        )
        .await?;
        delete_by(
            transaction,
            RolePeopleLevelRow::Table,
            RolePeopleLevelRow::Role,
            &role,
        )
        .await?;
        delete_by(
            transaction,
            ProjectPeopleLevelRow::Table,
            ProjectPeopleLevelRow::Role,
            &role,
        )
        .await?;
        Ok(delete_by(transaction, RoleRow::Table, RoleRow::Id, &role).await? > 0)
    }

    async fn insert_group(transaction: &mut Transaction, group: &Group) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(GroupRow::Table)
            .columns([GroupRow::Id, GroupRow::Name])
            .values([
                Expr::val(id_bytes(group.id.to_bytes())),
                Expr::val(group.name.as_str()),
            ])
            .map_err(|_| StoreError::Corrupt("a group"))?;
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn rename_group(
        transaction: &mut Transaction,
        id: GroupId,
        name: &str,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(GroupRow::Table)
            .value(GroupRow::Name, name)
            .and_where(Expr::col(GroupRow::Id).eq(id_bytes(id.to_bytes())));
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn delete_group(transaction: &mut Transaction, id: GroupId) -> Result<bool, StoreError> {
        let group = id_bytes(id.to_bytes());
        delete_by(
            transaction,
            AssignmentRow::Table,
            AssignmentRow::Group,
            &group,
        )
        .await?;
        delete_by(
            transaction,
            GroupMemberRow::Table,
            GroupMemberRow::Group,
            &group,
        )
        .await?;
        Ok(delete_by(transaction, GroupRow::Table, GroupRow::Id, &group).await? > 0)
    }

    async fn insert_group_member(
        transaction: &mut Transaction,
        member: &GroupMember,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(GroupMemberRow::Table)
            .columns([
                GroupMemberRow::Group,
                GroupMemberRow::Person,
                GroupMemberRow::AddedAt,
            ])
            .values([
                Expr::val(id_bytes(member.group.to_bytes())),
                Expr::val(id_bytes(member.person.to_bytes())),
                Expr::val(to_micros(member.added_at)),
            ])
            .map_err(|_| StoreError::Corrupt("a group member"))?;
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn delete_group_member(
        transaction: &mut Transaction,
        group: GroupId,
        person: UserId,
    ) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(GroupMemberRow::Table)
            .and_where(Expr::col(GroupMemberRow::Group).eq(id_bytes(group.to_bytes())))
            .and_where(Expr::col(GroupMemberRow::Person).eq(id_bytes(person.to_bytes())));
        Ok(transaction.execute(&delete).await? > 0)
    }

    async fn insert_assignment(
        transaction: &mut Transaction,
        assignment: &Assignment,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(AssignmentRow::Table)
            .columns([
                AssignmentRow::Id,
                AssignmentRow::Person,
                AssignmentRow::Group,
                AssignmentRow::Role,
                AssignmentRow::Project,
                AssignmentRow::AppliesTo,
                AssignmentRow::GivenAt,
            ])
            .values([
                Expr::val(id_bytes(assignment.id.to_bytes())),
                Expr::val(assignment.person.map(|id| id_bytes(id.to_bytes()))),
                Expr::val(assignment.group.map(|id| id_bytes(id.to_bytes()))),
                Expr::val(assignment.role.map(|id| id_bytes(id.to_bytes()))),
                Expr::val(assignment.project.map(|id| id_bytes(id.to_bytes()))),
                Expr::val(assignment.applies_to.code()),
                Expr::val(to_micros(assignment.given_at)),
            ])
            .map_err(|_| StoreError::Corrupt("an assignment"))?;
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn delete_assignment(
        transaction: &mut Transaction,
        id: AssignmentId,
    ) -> Result<bool, StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(AssignmentRow::Table)
            .and_where(Expr::col(AssignmentRow::Id).eq(id_bytes(id.to_bytes())));
        Ok(transaction.execute(&delete).await? > 0)
    }

    async fn put_field_level(
        transaction: &mut Transaction,
        role: RoleId,
        project: Option<ProjectId>,
        field: Field,
        level: Option<FieldLevel>,
    ) -> Result<(), StoreError> {
        let role_bytes = id_bytes(role.to_bytes());
        match project {
            None => {
                let mut delete = Query::delete();
                delete
                    .from_table(RoleFieldLevelRow::Table)
                    .and_where(Expr::col(RoleFieldLevelRow::Role).eq(role_bytes.clone()))
                    .and_where(Expr::col(RoleFieldLevelRow::Field).eq(field.code()));
                transaction.execute(&delete).await?;
                if let Some(level) = level {
                    let mut insert = Query::insert();
                    insert
                        .into_table(RoleFieldLevelRow::Table)
                        .columns([
                            RoleFieldLevelRow::Role,
                            RoleFieldLevelRow::Field,
                            RoleFieldLevelRow::Level,
                        ])
                        .values([
                            Expr::val(role_bytes),
                            Expr::val(field.code()),
                            Expr::val(level.code()),
                        ])
                        .map_err(|_| StoreError::Corrupt("a field level"))?;
                    transaction.execute(&insert).await?;
                }
            }
            Some(project) => {
                let project_bytes = id_bytes(project.to_bytes());
                let mut delete = Query::delete();
                delete
                    .from_table(ProjectFieldLevelRow::Table)
                    .and_where(Expr::col(ProjectFieldLevelRow::Project).eq(project_bytes.clone()))
                    .and_where(Expr::col(ProjectFieldLevelRow::Role).eq(role_bytes.clone()))
                    .and_where(Expr::col(ProjectFieldLevelRow::Field).eq(field.code()));
                transaction.execute(&delete).await?;
                if let Some(level) = level {
                    let mut insert = Query::insert();
                    insert
                        .into_table(ProjectFieldLevelRow::Table)
                        .columns([
                            ProjectFieldLevelRow::Project,
                            ProjectFieldLevelRow::Role,
                            ProjectFieldLevelRow::Field,
                            ProjectFieldLevelRow::Level,
                        ])
                        .values([
                            Expr::val(project_bytes),
                            Expr::val(role_bytes),
                            Expr::val(field.code()),
                            Expr::val(level.code()),
                        ])
                        .map_err(|_| StoreError::Corrupt("a field level"))?;
                    transaction.execute(&insert).await?;
                }
            }
        }
        Ok(())
    }

    async fn put_people_level(
        transaction: &mut Transaction,
        role: RoleId,
        project: Option<ProjectId>,
        kind: PeopleKind,
        level: Option<PeopleLevel>,
    ) -> Result<(), StoreError> {
        let role_bytes = id_bytes(role.to_bytes());
        match project {
            None => {
                let mut delete = Query::delete();
                delete
                    .from_table(RolePeopleLevelRow::Table)
                    .and_where(Expr::col(RolePeopleLevelRow::Role).eq(role_bytes.clone()))
                    .and_where(Expr::col(RolePeopleLevelRow::Kind).eq(kind.code()));
                transaction.execute(&delete).await?;
                if let Some(level) = level {
                    let mut insert = Query::insert();
                    insert
                        .into_table(RolePeopleLevelRow::Table)
                        .columns([
                            RolePeopleLevelRow::Role,
                            RolePeopleLevelRow::Kind,
                            RolePeopleLevelRow::Level,
                        ])
                        .values([
                            Expr::val(role_bytes),
                            Expr::val(kind.code()),
                            Expr::val(level.code()),
                        ])
                        .map_err(|_| StoreError::Corrupt("a people level"))?;
                    transaction.execute(&insert).await?;
                }
            }
            Some(project) => {
                let project_bytes = id_bytes(project.to_bytes());
                let mut delete = Query::delete();
                delete
                    .from_table(ProjectPeopleLevelRow::Table)
                    .and_where(Expr::col(ProjectPeopleLevelRow::Project).eq(project_bytes.clone()))
                    .and_where(Expr::col(ProjectPeopleLevelRow::Role).eq(role_bytes.clone()))
                    .and_where(Expr::col(ProjectPeopleLevelRow::Kind).eq(kind.code()));
                transaction.execute(&delete).await?;
                if let Some(level) = level {
                    let mut insert = Query::insert();
                    insert
                        .into_table(ProjectPeopleLevelRow::Table)
                        .columns([
                            ProjectPeopleLevelRow::Project,
                            ProjectPeopleLevelRow::Role,
                            ProjectPeopleLevelRow::Kind,
                            ProjectPeopleLevelRow::Level,
                        ])
                        .values([
                            Expr::val(project_bytes),
                            Expr::val(role_bytes),
                            Expr::val(kind.code()),
                            Expr::val(level.code()),
                        ])
                        .map_err(|_| StoreError::Corrupt("a people level"))?;
                    transaction.execute(&insert).await?;
                }
            }
        }
        Ok(())
    }

    async fn insert_project(
        transaction: &mut Transaction,
        project: &Project,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(ProjectRow::Table)
            .columns([
                ProjectRow::Id,
                ProjectRow::Parent,
                ProjectRow::ShortName,
                ProjectRow::Name,
                ProjectRow::Version,
            ])
            .values([
                Expr::val(id_bytes(project.id.to_bytes())),
                Expr::val(project.parent.map(|id| id_bytes(id.to_bytes()))),
                Expr::val(project.short_name.as_str()),
                Expr::val(project.name.as_str()),
                Expr::val(project.version),
            ])
            .map_err(|_| StoreError::Corrupt("a project"))?;
        transaction.execute(&insert).await.map(|_| ())
    }
}

/// Deletes the rows of a table whose `column` holds `value`.
async fn delete_by(
    transaction: &mut Transaction,
    table: impl sea_query::Iden + 'static,
    column: impl sea_query::Iden + 'static,
    value: &[u8],
) -> Result<u64, StoreError> {
    let mut delete = Query::delete();
    delete
        .from_table(table)
        .and_where(Expr::col(column).eq(value.to_vec()));
    transaction.execute(&delete).await
}

fn read_group(row: &DbRow) -> Result<Group, StoreError> {
    Ok(Group {
        id: read_group_id(row, GroupRow::Id)?,
        name: row.get(GroupRow::Name)?,
    })
}

async fn write_permissions(
    transaction: &mut Transaction,
    role: RoleId,
    permissions: &[Permission],
) -> Result<(), StoreError> {
    let unique: BTreeSet<Permission> = permissions.iter().copied().collect();
    for permission in unique {
        let mut insert = Query::insert();
        insert
            .into_table(RolePermissionRow::Table)
            .columns([RolePermissionRow::Role, RolePermissionRow::Permission])
            .values([
                Expr::val(id_bytes(role.to_bytes())),
                Expr::val(permission.code()),
            ])
            .map_err(|_| StoreError::Corrupt("a permission"))?;
        transaction.execute(&insert).await?;
    }
    Ok(())
}

async fn write_own_levels(
    transaction: &mut Transaction,
    role: RoleId,
    fields: &[(Field, FieldLevel)],
    people: &[(PeopleKind, PeopleLevel)],
) -> Result<(), StoreError> {
    for (field, level) in fields {
        let mut insert = Query::insert();
        insert
            .into_table(RoleFieldLevelRow::Table)
            .columns([
                RoleFieldLevelRow::Role,
                RoleFieldLevelRow::Field,
                RoleFieldLevelRow::Level,
            ])
            .values([
                Expr::val(id_bytes(role.to_bytes())),
                Expr::val(field.code()),
                Expr::val(level.code()),
            ])
            .map_err(|_| StoreError::Corrupt("a field level"))?;
        transaction.execute(&insert).await?;
    }
    for (kind, level) in people {
        let mut insert = Query::insert();
        insert
            .into_table(RolePeopleLevelRow::Table)
            .columns([
                RolePeopleLevelRow::Role,
                RolePeopleLevelRow::Kind,
                RolePeopleLevelRow::Level,
            ])
            .values([
                Expr::val(id_bytes(role.to_bytes())),
                Expr::val(kind.code()),
                Expr::val(level.code()),
            ])
            .map_err(|_| StoreError::Corrupt("a people level"))?;
        transaction.execute(&insert).await?;
    }
    Ok(())
}
