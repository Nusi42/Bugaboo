//! The adapter of the policy port (`bb_app::access::PolicyStore`): projects, groups, roles, who has
//! which role, the levels of fields and of *Who sees people*, and the one call that loads what the
//! engine needs for a person at a place (SEC-03).
//!
//! The work is in the modules by job: `roles`, `groups`, `assignments`, `levels` and `loading`.
//! Every read is written once, on a transaction (`*_in`, a snapshot or an open change); the port
//! makes the plain read from it. Writes are associated functions that run inside the transaction
//! of a change ([`crate::Database::record_change`]), so a change and its event are committed
//! together.

mod assignments;
mod groups;
mod levels;
mod loading;
mod roles;

use bb_app::access::{PlaceLevels, PolicyStore, RoleRecord};
use bb_core::access::{Assignment, FieldLevel, Group, GroupMember, PeopleLevel, Role};
use bb_core::fields::FieldKey;
use bb_core::id::{AssignmentId, GroupId, IssueTypeId, ProjectId, RoleId, UserId};
use bb_core::people_kinds::PeopleKind;
use bb_core::project::{Project, ShortName};
use bb_policy::guards::OrganizationState;
use bb_policy::{Inputs, LevelRules, LevelSubject, Permission, Place};
use jiff::Timestamp;
use sea_query::{Expr, ExprTrait as _, Order, Query};

use self::levels::Which;
pub(crate) use self::loading::{Wanted, load_policy};
use crate::auth::AuthStorage;
use crate::db::{Reader, StoredId, Transaction, to_micros};
use crate::error::StoreError;
use crate::locks::LockClass;
use crate::projects::{chain_of, insert_project, project_by_short_name, project_in};
use crate::schema::{LegacyAdministratorRow, StartingRolesMadeRow};

/// The IDs as a list of values to bind to a query.
pub(crate) fn in_list<T: StoredId>(ids: &[T]) -> Vec<Vec<u8>> {
    ids.iter().map(|id| id.stored()).collect()
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

/// Whether the starting roles were made.
async fn starting_roles_made(reader: &mut impl Reader) -> Result<bool, StoreError> {
    let mut select = Query::select();
    select
        .column(StartingRolesMadeRow::MadeAt)
        .from(StartingRolesMadeRow::Table)
        .limit(1);
    Ok(reader.fetch_optional(&select).await?.is_some())
}

/// Every level these roles have set, read on `reader`; none for no roles, without a statement.
pub(crate) async fn level_rules_of_roles_in(
    reader: &mut impl Reader,
    roles: Vec<RoleId>,
) -> Result<LevelRules, StoreError> {
    if roles.is_empty() {
        return Ok(LevelRules::default());
    }
    Ok(loading::rules_of(reader, Which::All)
        .await?
        .of_roles(&roles.into_iter().collect()))
}

impl PolicyStore for AuthStorage {
    async fn policy_inputs_in(
        transaction: &mut Transaction,
        person: UserId,
        place: Place,
    ) -> Result<Option<Inputs>, StoreError> {
        loading::inputs_in(transaction, person, place).await
    }

    async fn organization_state_in(
        transaction: &mut Transaction,
    ) -> Result<OrganizationState, StoreError> {
        loading::organization_state_in(transaction).await
    }

    async fn starting_roles_made_in(transaction: &mut Transaction) -> Result<bool, StoreError> {
        starting_roles_made(transaction).await
    }

    async fn role_in(
        transaction: &mut Transaction,
        id: RoleId,
    ) -> Result<Option<RoleRecord>, StoreError> {
        roles::role_of(transaction, id).await
    }

    async fn assignments_on_in(
        transaction: &mut Transaction,
        project: Option<ProjectId>,
    ) -> Result<Vec<Assignment>, StoreError> {
        assignments::assignments_on(transaction, project).await
    }

    async fn assignments_of_group_in(
        transaction: &mut Transaction,
        group: GroupId,
    ) -> Result<Vec<Assignment>, StoreError> {
        assignments::assignments_of_group(transaction, group).await
    }

    async fn assignments_of_role_in(
        transaction: &mut Transaction,
        role: RoleId,
    ) -> Result<Vec<Assignment>, StoreError> {
        assignments::assignments_of_role(transaction, role).await
    }

    async fn project_in(
        transaction: &mut Transaction,
        id: ProjectId,
    ) -> Result<Option<Project>, StoreError> {
        project_in(transaction, id).await
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
            .map(|row| row.id::<UserId>(LegacyAdministratorRow::Person))
            .collect()
    }

    async fn roles(&self) -> Result<Vec<RoleRecord>, StoreError> {
        let mut transaction = self.database().begin_read().await?;
        roles::roles_in(&mut transaction, None).await
    }

    async fn level_rules(
        &self,
        roles: Vec<RoleId>,
        project: Option<ProjectId>,
    ) -> Result<Option<PlaceLevels>, StoreError> {
        let mut transaction = self.database().begin_read().await?;
        let chain = match project {
            None => Vec::new(),
            Some(project) => match chain_of(&mut transaction, project).await? {
                Some(chain) => chain,
                None => return Ok(None),
            },
        };
        let levels = if roles.is_empty() {
            LevelRules::default()
        } else {
            // The query narrows to the roles and the chain; what applies at the place is decided
            // once, by `LevelRules::at`, here as in the test double.
            loading::rules_of(
                &mut transaction,
                Which::At {
                    roles: &roles,
                    chain: &chain,
                },
            )
            .await?
            .at(&roles.iter().copied().collect(), &chain)
        };
        Ok(Some(PlaceLevels {
            chain,
            rules: levels,
        }))
    }

    async fn level_rules_of_roles(&self, roles: Vec<RoleId>) -> Result<LevelRules, StoreError> {
        let mut transaction = self.database().begin_read().await?;
        level_rules_of_roles_in(&mut transaction, roles).await
    }

    async fn groups(&self) -> Result<Vec<Group>, StoreError> {
        groups::groups(&mut self.database()).await
    }

    async fn group(&self, id: GroupId) -> Result<Option<Group>, StoreError> {
        groups::group(&mut self.database(), id).await
    }

    async fn group_members(&self, group: GroupId) -> Result<Vec<GroupMember>, StoreError> {
        groups::members(&mut self.database(), group).await
    }

    async fn assignments_of_person(&self, person: UserId) -> Result<Vec<Assignment>, StoreError> {
        assignments::assignments_of_person(&mut self.database(), person).await
    }

    async fn groups_of(&self, person: UserId) -> Result<Vec<Group>, StoreError> {
        groups::groups_of(&mut self.database(), person).await
    }

    async fn assignment(&self, id: AssignmentId) -> Result<Option<Assignment>, StoreError> {
        assignments::assignment_of(&mut self.database(), id).await
    }

    async fn project_chain(&self, id: ProjectId) -> Result<Option<Vec<ProjectId>>, StoreError> {
        let mut transaction = self.database().begin_read().await?;
        chain_of(&mut transaction, id).await
    }

    async fn project_by_short_name(&self, name: &ShortName) -> Result<Option<Project>, StoreError> {
        project_by_short_name(&mut self.database(), name).await
    }

    async fn levels<K: LevelSubject>(
        &self,
        role: RoleId,
        project: Option<ProjectId>,
        issue_type: Option<IssueTypeId>,
    ) -> Result<Vec<(K, K::Level)>, StoreError> {
        levels::read_levels(&mut self.database(), role, project, issue_type).await
    }

    async fn lock_policy(transaction: &mut Transaction) -> Result<(), StoreError> {
        transaction.advisory_lock(LockClass::Policy, 0).await
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
        delete_by(
            transaction,
            LegacyAdministratorRow::Table,
            LegacyAdministratorRow::Person,
            &person.stored(),
        )
        .await
        .map(|_| ())
    }

    async fn insert_role(
        transaction: &mut Transaction,
        role: &Role,
        permissions: &[Permission],
        fields: &[(FieldKey, FieldLevel)],
        people: &[(PeopleKind, PeopleLevel)],
    ) -> Result<(), StoreError> {
        roles::insert_role(transaction, role, permissions, fields, people).await
    }

    async fn update_role(
        transaction: &mut Transaction,
        id: RoleId,
        name: &str,
        two_step: bool,
        version: i64,
    ) -> Result<bool, StoreError> {
        roles::update_role(transaction, id, name, two_step, version).await
    }

    async fn replace_role_permissions(
        transaction: &mut Transaction,
        id: RoleId,
        permissions: &[Permission],
    ) -> Result<(), StoreError> {
        roles::replace_role_permissions(transaction, id, permissions).await
    }

    async fn replace_role_levels(
        transaction: &mut Transaction,
        id: RoleId,
        fields: &[(FieldKey, FieldLevel)],
        people: &[(PeopleKind, PeopleLevel)],
    ) -> Result<(), StoreError> {
        roles::replace_role_levels(transaction, id, fields, people).await
    }

    async fn delete_role(transaction: &mut Transaction, id: RoleId) -> Result<bool, StoreError> {
        roles::delete_role(transaction, id).await
    }

    async fn insert_group(transaction: &mut Transaction, group: &Group) -> Result<(), StoreError> {
        groups::insert_group(transaction, group).await
    }

    async fn rename_group(
        transaction: &mut Transaction,
        id: GroupId,
        name: &str,
        version: i64,
    ) -> Result<bool, StoreError> {
        groups::rename_group(transaction, id, name, version).await
    }

    async fn delete_group(transaction: &mut Transaction, id: GroupId) -> Result<bool, StoreError> {
        groups::delete_group(transaction, id).await
    }

    async fn insert_group_member(
        transaction: &mut Transaction,
        member: &GroupMember,
    ) -> Result<(), StoreError> {
        groups::insert_group_member(transaction, member).await
    }

    async fn delete_group_member(
        transaction: &mut Transaction,
        group: GroupId,
        person: UserId,
    ) -> Result<bool, StoreError> {
        groups::delete_group_member(transaction, group, person).await
    }

    async fn insert_assignment(
        transaction: &mut Transaction,
        assignment: &Assignment,
    ) -> Result<(), StoreError> {
        assignments::insert_assignment(transaction, assignment).await
    }

    async fn delete_assignment(
        transaction: &mut Transaction,
        id: AssignmentId,
    ) -> Result<bool, StoreError> {
        assignments::delete_assignment(transaction, id).await
    }

    async fn put_level<K: LevelSubject>(
        transaction: &mut Transaction,
        role: RoleId,
        project: Option<ProjectId>,
        issue_type: Option<IssueTypeId>,
        subject: K,
        level: Option<K::Level>,
    ) -> Result<(), StoreError> {
        levels::put_level(transaction, role, project, issue_type, subject, level).await
    }

    async fn insert_project(
        transaction: &mut Transaction,
        project: &Project,
    ) -> Result<(), StoreError> {
        insert_project(transaction, project).await
    }
}
