//! The adapter of the projects port (`bb_app::projects::ProjectStore`): the tree, what the engine
//! reads for one person or for everybody, and the old short names (PRJ-01, PRJ-03, M2-Q4).
//!
//! Like the policy adapter, reads run in one snapshot, so the rows of one decision belong
//! together, and writes are associated functions that run inside the transaction of a change.

use bb_app::projects::{Found, ProjectStore};
use bb_core::id::{ProjectId, UserId};
use bb_core::project::{DEPTH_MAX, Project, ProjectAlias, ShortName};
use bb_policy::{Policy, Tree};
use sea_query::{Expr, ExprTrait as _, Query, SelectStatement};

use crate::auth::AuthStorage;
use crate::db::{Reader, StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::policy::{Wanted, level_rules_of_roles_in, load_policy};
use crate::schema::{ProjectAliasRow, ProjectRow};

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

pub(crate) fn read_project(row: &crate::db::Row) -> Result<Project, StoreError> {
    Ok(Project {
        id: row.id::<ProjectId>(ProjectRow::Id)?,
        parent: row.optional_id::<ProjectId>(ProjectRow::Parent)?,
        short_name: ShortName::new(&row.get::<String>(ProjectRow::ShortName)?)
            .map_err(|_| StoreError::Corrupt("a short name"))?,
        name: row.get(ProjectRow::Name)?,
        version: row.get(ProjectRow::Version)?,
    })
}

/// The projects from the top down to `project`; `None` when it does not exist. Only the projects
/// on the way up are read; the way up is judged by [`Tree`], like everywhere else.
pub(crate) async fn chain_of(
    reader: &mut impl Reader,
    project: ProjectId,
) -> Result<Option<Vec<ProjectId>>, StoreError> {
    let mut links = Vec::new();
    let mut current = Some(project);
    // One more than the deepest tree, so that a tree that is too deep (or a loop) is seen as such
    // by `Tree::chain` and not followed for ever.
    while let Some(id) = current.filter(|_| links.len() <= DEPTH_MAX) {
        let mut select = Query::select();
        select
            .column(ProjectRow::Parent)
            .from(ProjectRow::Table)
            .and_where(Expr::col(ProjectRow::Id).eq(id.stored()));
        let Some(row) = reader.fetch_optional(&select).await? else {
            return Ok(None);
        };
        let parent = row.optional_id::<ProjectId>(ProjectRow::Parent)?;
        links.push((id, parent));
        current = parent;
    }
    Tree::new(links)
        .chain(project)
        .map(Some)
        .ok_or(StoreError::Corrupt("a tree of projects"))
}

/// The project with this ID.
pub(crate) async fn project_in(
    reader: &mut impl Reader,
    id: ProjectId,
) -> Result<Option<Project>, StoreError> {
    let mut select = project_select();
    select.and_where(Expr::col(ProjectRow::Id).eq(id.stored()));
    reader
        .fetch_optional(&select)
        .await?
        .as_ref()
        .map(read_project)
        .transpose()
}

/// The project with this short name.
pub(crate) async fn project_by_short_name(
    reader: &mut impl Reader,
    name: &ShortName,
) -> Result<Option<Project>, StoreError> {
    let mut select = project_select();
    select.and_where(Expr::col(ProjectRow::ShortName).eq(name.as_str()));
    reader
        .fetch_optional(&select)
        .await?
        .as_ref()
        .map(read_project)
        .transpose()
}

/// Adds a project.
pub(crate) async fn insert_project(
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
            Expr::val(project.id.stored()),
            Expr::val(project.parent.map(ProjectId::stored)),
            Expr::val(project.short_name.as_str()),
            Expr::val(project.name.as_str()),
            Expr::val(project.version),
        ])
        .map_err(|_| StoreError::Corrupt("a project"))?;
    transaction.execute(&insert).await.map(|_| ())
}

/// The projects, in the order of their IDs.
async fn tree_in(reader: &mut impl Reader) -> Result<Vec<Project>, StoreError> {
    let mut projects = reader
        .fetch_all(&project_select())
        .await?
        .iter()
        .map(read_project)
        .collect::<Result<Vec<_>, _>>()?;
    projects.sort_by_key(|project| project.id);
    Ok(projects)
}

/// The project that has this short name now or had it once.
async fn owner_in(
    reader: &mut impl Reader,
    name: &ShortName,
) -> Result<Option<ProjectId>, StoreError> {
    let mut select = Query::select();
    select
        .column(ProjectRow::Id)
        .from(ProjectRow::Table)
        .and_where(Expr::col(ProjectRow::ShortName).eq(name.as_str()));
    if let Some(row) = reader.fetch_optional(&select).await? {
        return row.id::<ProjectId>(ProjectRow::Id).map(Some);
    }
    let mut select = Query::select();
    select
        .column(ProjectAliasRow::Project)
        .from(ProjectAliasRow::Table)
        .and_where(Expr::col(ProjectAliasRow::ShortName).eq(name.as_str()));
    reader
        .fetch_optional(&select)
        .await?
        .map(|row| row.id::<ProjectId>(ProjectAliasRow::Project))
        .transpose()
}

/// The rows of a person and every project, without levels, on one transaction.
async fn policy_and_tree_in(
    transaction: &mut Transaction,
    person: UserId,
) -> Result<(Policy, Vec<Project>), StoreError> {
    let policy = load_policy(transaction, Wanted::Person(person)).await?;
    let projects = tree_in(transaction).await?;
    Ok((policy, projects))
}

impl ProjectStore for AuthStorage {
    async fn project_tree_in(transaction: &mut Transaction) -> Result<Vec<Project>, StoreError> {
        tree_in(transaction).await
    }

    async fn policy_in(transaction: &mut Transaction) -> Result<Policy, StoreError> {
        load_policy(transaction, Wanted::Everybody).await
    }

    async fn policy_and_tree_of(
        &self,
        person: UserId,
    ) -> Result<(Policy, Vec<Project>), StoreError> {
        let mut transaction = self.database().begin_read().await?;
        policy_and_tree_in(&mut transaction, person).await
    }

    async fn leveled_policy_and_tree_of(
        &self,
        person: UserId,
    ) -> Result<(Policy, Vec<Project>), StoreError> {
        let mut transaction = self.database().begin_read().await?;
        let (mut policy, projects) = policy_and_tree_in(&mut transaction, person).await?;
        let roles = policy.role_ids();
        policy.rules = level_rules_of_roles_in(&mut transaction, roles).await?;
        Ok((policy, projects))
    }

    async fn project_by_address(&self, name: &ShortName) -> Result<Option<Found>, StoreError> {
        let mut transaction = self.database().begin_read().await?;
        let Some(owner) = owner_in(&mut transaction, name).await? else {
            return Ok(None);
        };
        Ok(project_in(&mut transaction, owner)
            .await?
            .map(|project| Found {
                old_name: &project.short_name != name,
                project,
            }))
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
            .value(ProjectRow::Parent, parent.map(ProjectId::stored))
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
