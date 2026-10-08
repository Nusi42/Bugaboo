//! Roles with their permissions, and what changes them.

use std::collections::{BTreeMap, BTreeSet};

use bb_app::access::RoleRecord;
use bb_core::access::{FieldLevel, PeopleLevel, Role};
use bb_core::fields::Field;
use bb_core::id::RoleId;
use bb_core::people_kinds::PeopleKind;
use bb_core::technical::TechnicalName;
use bb_policy::Permission;
use sea_query::{Expr, ExprTrait as _, Order, Query, SelectStatement};

use super::{delete_by, in_list, levels};
use crate::db::{Reader, Row as DbRow, StoredId as _, Transaction};
use crate::error::StoreError;
use crate::schema::{AssignmentRow, RolePermissionRow, RoleRow};

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
        id: row.id::<RoleId>(RoleRow::Id)?,
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
    reader: &mut impl Reader,
    only: Option<&[RoleId]>,
) -> Result<Vec<RoleRecord>, StoreError> {
    let mut select = role_select();
    if let Some(only) = only {
        select.and_where(Expr::col(RoleRow::Id).is_in(in_list(only)));
    }
    let roles = reader
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
            .and_where(Expr::col(RolePermissionRow::Role).is_in(in_list(&ids)));
        for row in reader.fetch_all(&select).await? {
            let role = row.id::<RoleId>(RolePermissionRow::Role)?;
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

/// One role with its permissions.
pub(crate) async fn role_of(
    reader: &mut impl Reader,
    id: RoleId,
) -> Result<Option<RoleRecord>, StoreError> {
    Ok(roles_in(reader, Some(&[id])).await?.pop())
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
            .values([Expr::val(role.stored()), Expr::val(permission.code())])
            .map_err(|_| StoreError::Corrupt("a permission"))?;
        transaction.execute(&insert).await?;
    }
    Ok(())
}

/// Adds a role with its permissions and the levels it has of its own.
pub(crate) async fn insert_role(
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
            Expr::val(role.id.stored()),
            Expr::val(role.name.as_str()),
            Expr::val(role.starting.as_ref().map(TechnicalName::as_str)),
            Expr::val(role.position),
            Expr::val(role.two_step),
        ])
        .map_err(|_| StoreError::Corrupt("a role"))?;
    transaction.execute(&insert).await?;
    write_permissions(transaction, role.id, permissions).await?;
    levels::insert_own(transaction, role.id, fields).await?;
    levels::insert_own(transaction, role.id, people).await
}

/// Changes the name and the two-step switch of a role; `false` if there is no such role.
pub(crate) async fn update_role(
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
        .and_where(Expr::col(RoleRow::Id).eq(id.stored()));
    Ok(transaction.execute(&update).await? > 0)
}

/// Replaces the permissions of a role.
pub(crate) async fn replace_role_permissions(
    transaction: &mut Transaction,
    id: RoleId,
    permissions: &[Permission],
) -> Result<(), StoreError> {
    delete_by(
        transaction,
        RolePermissionRow::Table,
        RolePermissionRow::Role,
        &id.stored(),
    )
    .await?;
    write_permissions(transaction, id, permissions).await
}

/// Replaces the levels a role has of its own (not those set for projects).
pub(crate) async fn replace_role_levels(
    transaction: &mut Transaction,
    id: RoleId,
    fields: &[(Field, FieldLevel)],
    people: &[(PeopleKind, PeopleLevel)],
) -> Result<(), StoreError> {
    levels::replace_own(transaction, id, fields).await?;
    levels::replace_own(transaction, id, people).await
}

/// Removes a role, with the rows that give it and the levels it set.
pub(crate) async fn delete_role(
    transaction: &mut Transaction,
    id: RoleId,
) -> Result<bool, StoreError> {
    let role = id.stored();
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
    levels::delete_all_of(transaction, id).await?;
    Ok(delete_by(transaction, RoleRow::Table, RoleRow::Id, &role).await? > 0)
}
