//! The adapter of the projects port (`bb_app::projects::ProjectStore`): the tree, what the engine
//! reads for one person or for everybody, and the old short names (PRJ-01, PRJ-03, M2-Q4).
//!
//! Like the policy adapter, reads run in one snapshot, so the rows of one decision belong
//! together, and writes are associated functions that run inside the transaction of a change.

use bb_app::projects::{Found, ProjectStore};
use bb_core::access::{Assignment, GroupMember};
use bb_core::id::{GroupId, ProjectId, RoleId, UserId};
use bb_core::project::{Project, ProjectAlias, ShortName};
use bb_policy::{FieldRule, PeopleRule, Policy, RoleData};
use sea_query::{Expr, ExprTrait as _, Query};

use crate::auth::AuthStorage;
use crate::db::{StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::policy::{
    assignment_select, in_list, project_select, read_assignment, read_field, read_field_level,
    read_kind, read_people_level, read_project, roles_in,
};
use crate::schema::{
    GroupMemberRow, ProjectAliasRow, ProjectFieldLevelRow, ProjectPeopleLevelRow, ProjectRow,
    RoleFieldLevelRow, RolePeopleLevelRow,
};

/// The projects, in the order of their IDs.
async fn tree_in(transaction: &mut Transaction) -> Result<Vec<Project>, StoreError> {
    let mut projects = transaction
        .fetch_all(&project_select())
        .await?
        .iter()
        .map(read_project)
        .collect::<Result<Vec<_>, _>>()?;
    projects.sort_by_key(|project| project.id);
    Ok(projects)
}

/// The rows of one person (their own and those of their groups), or of everybody when `person`
/// is `None`, with the roles they name. Only everybody's policy holds the levels of the roles:
/// a decision about permissions does not need them.
async fn policy_in(
    transaction: &mut Transaction,
    person: Option<UserId>,
) -> Result<Policy, StoreError> {
    let mut select = Query::select();
    select
        .columns([
            GroupMemberRow::Group,
            GroupMemberRow::Person,
            GroupMemberRow::AddedAt,
        ])
        .from(GroupMemberRow::Table);
    if let Some(person) = person {
        select.and_where(Expr::col(GroupMemberRow::Person).eq(person.stored()));
    }
    let members = transaction
        .fetch_all(&select)
        .await?
        .iter()
        .map(|row| {
            Ok(GroupMember {
                group: row.id::<GroupId>(GroupMemberRow::Group)?,
                person: row.id::<UserId>(GroupMemberRow::Person)?,
                added_at: row.time(GroupMemberRow::AddedAt)?,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    let mut select = assignment_select();
    if let Some(person) = person {
        let groups: Vec<GroupId> = members.iter().map(|member| member.group).collect();
        let mut whose = Expr::col(crate::schema::AssignmentRow::Person).eq(person.stored());
        if !groups.is_empty() {
            whose =
                whose.or(Expr::col(crate::schema::AssignmentRow::Group).is_in(in_list(&groups)));
        }
        select.and_where(whose);
    }
    let assignments: Vec<Assignment> = transaction
        .fetch_all(&select)
        .await?
        .iter()
        .map(read_assignment)
        .collect::<Result<_, _>>()?;
    let only: Vec<RoleId> = {
        let mut named: Vec<RoleId> = assignments.iter().filter_map(|row| row.role).collect();
        named.sort();
        named.dedup();
        named
    };
    let roles = if person.is_some() && only.is_empty() {
        Vec::new()
    } else {
        roles_in(transaction, person.map(|_| only.as_slice())).await?
    }
    .into_iter()
    .map(|record| RoleData {
        id: record.role.id,
        name: record.role.name,
        position: record.role.position,
        permissions: record.permissions,
        two_step: record.role.two_step,
    })
    .collect();
    let (field_rules, people_rules) = if person.is_none() {
        all_levels(transaction).await?
    } else {
        (Vec::new(), Vec::new())
    };
    Ok(Policy {
        assignments,
        members,
        roles,
        field_rules,
        people_rules,
    })
}

/// Every level that a role sets for a field or a kind of people information: its own, and those
/// set for projects.
async fn all_levels(
    transaction: &mut Transaction,
) -> Result<(Vec<FieldRule>, Vec<PeopleRule>), StoreError> {
    let mut fields = Vec::new();
    let mut select = Query::select();
    select
        .columns([
            RoleFieldLevelRow::Role,
            RoleFieldLevelRow::Field,
            RoleFieldLevelRow::Level,
        ])
        .from(RoleFieldLevelRow::Table);
    for row in transaction.fetch_all(&select).await? {
        fields.push(FieldRule {
            project: None,
            role: row.id::<RoleId>(RoleFieldLevelRow::Role)?,
            field: read_field(&row, RoleFieldLevelRow::Field)?,
            level: read_field_level(&row, RoleFieldLevelRow::Level)?,
        });
    }
    let mut select = Query::select();
    select
        .columns([
            ProjectFieldLevelRow::Project,
            ProjectFieldLevelRow::Role,
            ProjectFieldLevelRow::Field,
            ProjectFieldLevelRow::Level,
        ])
        .from(ProjectFieldLevelRow::Table);
    for row in transaction.fetch_all(&select).await? {
        fields.push(FieldRule {
            project: Some(row.id::<ProjectId>(ProjectFieldLevelRow::Project)?),
            role: row.id::<RoleId>(ProjectFieldLevelRow::Role)?,
            field: read_field(&row, ProjectFieldLevelRow::Field)?,
            level: read_field_level(&row, ProjectFieldLevelRow::Level)?,
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
        .from(RolePeopleLevelRow::Table);
    for row in transaction.fetch_all(&select).await? {
        people.push(PeopleRule {
            project: None,
            role: row.id::<RoleId>(RolePeopleLevelRow::Role)?,
            kind: read_kind(&row, RolePeopleLevelRow::Kind)?,
            level: read_people_level(&row, RolePeopleLevelRow::Level)?,
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
        .from(ProjectPeopleLevelRow::Table);
    for row in transaction.fetch_all(&select).await? {
        people.push(PeopleRule {
            project: Some(row.id::<ProjectId>(ProjectPeopleLevelRow::Project)?),
            role: row.id::<RoleId>(ProjectPeopleLevelRow::Role)?,
            kind: read_kind(&row, ProjectPeopleLevelRow::Kind)?,
            level: read_people_level(&row, ProjectPeopleLevelRow::Level)?,
        });
    }
    Ok((fields, people))
}

/// The project that has this short name now or had it once.
async fn owner_in(
    transaction: &mut Transaction,
    name: &ShortName,
) -> Result<Option<ProjectId>, StoreError> {
    let mut select = Query::select();
    select
        .column(ProjectRow::Id)
        .from(ProjectRow::Table)
        .and_where(Expr::col(ProjectRow::ShortName).eq(name.as_str()));
    if let Some(row) = transaction.fetch_optional(&select).await? {
        return row.id::<ProjectId>(ProjectRow::Id).map(Some);
    }
    let mut select = Query::select();
    select
        .column(ProjectAliasRow::Project)
        .from(ProjectAliasRow::Table)
        .and_where(Expr::col(ProjectAliasRow::ShortName).eq(name.as_str()));
    transaction
        .fetch_optional(&select)
        .await?
        .map(|row| row.id::<ProjectId>(ProjectAliasRow::Project))
        .transpose()
}

async fn project_of(
    transaction: &mut Transaction,
    id: ProjectId,
) -> Result<Option<Project>, StoreError> {
    let mut select = project_select();
    select.and_where(Expr::col(ProjectRow::Id).eq(id.stored()));
    transaction
        .fetch_optional(&select)
        .await?
        .as_ref()
        .map(read_project)
        .transpose()
}

impl ProjectStore for AuthStorage {
    async fn project_tree(&self) -> Result<Vec<Project>, StoreError> {
        let mut transaction = self.database().begin_read().await?;
        tree_in(&mut transaction).await
    }

    async fn policy_of(&self, person: UserId) -> Result<Policy, StoreError> {
        let mut transaction = self.database().begin_read().await?;
        policy_in(&mut transaction, Some(person)).await
    }

    async fn policy_all(&self) -> Result<Policy, StoreError> {
        let mut transaction = self.database().begin_read().await?;
        policy_in(&mut transaction, None).await
    }

    async fn project_by_address(&self, name: &ShortName) -> Result<Option<Found>, StoreError> {
        let mut transaction = self.database().begin_read().await?;
        let Some(owner) = owner_in(&mut transaction, name).await? else {
            return Ok(None);
        };
        Ok(project_of(&mut transaction, owner)
            .await?
            .map(|project| Found {
                old_name: &project.short_name != name,
                project,
            }))
    }

    async fn project_tree_in(transaction: &mut Transaction) -> Result<Vec<Project>, StoreError> {
        tree_in(transaction).await
    }

    async fn policy_in(transaction: &mut Transaction) -> Result<Policy, StoreError> {
        policy_in(transaction, None).await
    }

    async fn project_in(
        transaction: &mut Transaction,
        id: ProjectId,
    ) -> Result<Option<Project>, StoreError> {
        project_of(transaction, id).await
    }

    async fn short_name_owner_in(
        transaction: &mut Transaction,
        name: &ShortName,
    ) -> Result<Option<ProjectId>, StoreError> {
        owner_in(transaction, name).await
    }

    async fn update_project(
        transaction: &mut Transaction,
        id: ProjectId,
        name: &str,
        short_name: &ShortName,
        version: i64,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(ProjectRow::Table)
            .value(ProjectRow::Name, name)
            .value(ProjectRow::ShortName, short_name.as_str())
            .value(ProjectRow::Version, Expr::col(ProjectRow::Version).add(1))
            .and_where(Expr::col(ProjectRow::Id).eq(id.stored()))
            .and_where(Expr::col(ProjectRow::Version).eq(version));
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn move_project(
        transaction: &mut Transaction,
        id: ProjectId,
        parent: Option<ProjectId>,
        version: i64,
    ) -> Result<bool, StoreError> {
        let mut update = Query::update();
        update
            .table(ProjectRow::Table)
            .value(ProjectRow::Parent, parent.map(|parent| parent.stored()))
            .value(ProjectRow::Version, Expr::col(ProjectRow::Version).add(1))
            .and_where(Expr::col(ProjectRow::Id).eq(id.stored()))
            .and_where(Expr::col(ProjectRow::Version).eq(version));
        Ok(transaction.execute(&update).await? > 0)
    }

    async fn insert_project_alias(
        transaction: &mut Transaction,
        alias: &ProjectAlias,
    ) -> Result<(), StoreError> {
        let mut insert = Query::insert();
        insert
            .into_table(ProjectAliasRow::Table)
            .columns([
                ProjectAliasRow::ShortName,
                ProjectAliasRow::Project,
                ProjectAliasRow::RetiredAt,
            ])
            .values([
                Expr::val(alias.short_name.as_str()),
                Expr::val(alias.project.stored()),
                Expr::val(to_micros(alias.retired_at)),
            ])
            .map_err(|_| StoreError::Corrupt("an old short name"))?;
        transaction.execute(&insert).await.map(|_| ())
    }

    async fn delete_project_alias(
        transaction: &mut Transaction,
        name: &ShortName,
    ) -> Result<(), StoreError> {
        let mut delete = Query::delete();
        delete
            .from_table(ProjectAliasRow::Table)
            .and_where(Expr::col(ProjectAliasRow::ShortName).eq(name.as_str()));
        transaction.execute(&delete).await.map(|_| ())
    }
}
