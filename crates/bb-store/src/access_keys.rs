//! Access keys and the questions about a second step that two-step sign-in by role asks (AUTH-07,
//! AUTH-03, ADR-0025 decisions 7 and 8): the reads and writes behind the access-key part of
//! `AuthStore` (`auth.rs` delegates here).
//!
//! A key is found by the hash of its secret (one unique column), so an unknown key and a wrong
//! secret are the same lookup that finds nothing. Its single permissions and its projects are
//! rows of their own, loaded together with the keys of one page.

use std::collections::{BTreeMap, BTreeSet};

use bb_app::auth::AccessKeyRecord;
use bb_core::access_key::{AccessKey, KeyChoice};
use bb_core::id::{AccessKeyId, IdError, ProjectId, RoleId, UserId};
use bb_core::security_log::Digest;
use bb_core::settings::SECURITY_TWO_STEP_REQUIRED;
use bb_policy::Permission;
use jiff::Timestamp;
use sea_query::{Cond, Expr, ExprTrait as _, Order, Query};

use crate::db::{Database, Row, Transaction, to_micros};
use crate::error::StoreError;
use crate::schema::{
    AccessKeyPermissionRow, AccessKeyProjectRow, AccessKeyRow, AssignmentRow, GroupMemberRow,
    PasskeyRow, RoleRow, TwoStep,
};
use crate::settings::Scopes;

/// How many people are asked about at once, so the list of IDs stays below what the databases
/// take in one statement.
const CHUNK: usize = 500;

fn corrupt(what: &'static str) -> impl Fn(IdError) -> StoreError {
    move |_| StoreError::Corrupt(what)
}

fn bytes(id: [u8; 16]) -> Vec<u8> {
    id.to_vec()
}

fn key_columns() -> [AccessKeyRow; 9] {
    [
        AccessKeyRow::Id,
        AccessKeyRow::Owner,
        AccessKeyRow::Name,
        AccessKeyRow::Prefix,
        AccessKeyRow::SecretHash,
        AccessKeyRow::Choice,
        AccessKeyRow::CreatedAt,
        AccessKeyRow::ExpiresAt,
        AccessKeyRow::LastUsedAt,
    ]
}

fn read_key(row: &Row) -> Result<AccessKey, StoreError> {
    Ok(AccessKey {
        id: AccessKeyId::from_bytes(row.id_bytes(AccessKeyRow::Id)?)
            .map_err(corrupt("an access key ID"))?,
        owner: UserId::from_bytes(row.id_bytes(AccessKeyRow::Owner)?)
            .map_err(corrupt("a person ID"))?,
        name: row.get(AccessKeyRow::Name)?,
        prefix: row.get(AccessKeyRow::Prefix)?,
        secret_hash: Digest::from_slice(&row.get::<Vec<u8>>(AccessKeyRow::SecretHash)?)
            .ok_or(StoreError::Corrupt("a hash"))?,
        choice: KeyChoice::from_code(&row.get::<String>(AccessKeyRow::Choice)?)
            .ok_or(StoreError::Corrupt("an access key choice"))?,
        created_at: row.time(AccessKeyRow::CreatedAt)?,
        expires_at: row.time(AccessKeyRow::ExpiresAt)?,
        last_used_at: row.optional_time(AccessKeyRow::LastUsedAt)?,
    })
}

/// The keys with their single permissions and their projects.
async fn with_scope(
    database: &Database,
    keys: Vec<AccessKey>,
) -> Result<Vec<AccessKeyRecord>, StoreError> {
    if keys.is_empty() {
        return Ok(Vec::new());
    }
    let ids: Vec<Vec<u8>> = keys.iter().map(|key| bytes(key.id.to_bytes())).collect();
    let mut permissions: BTreeMap<AccessKeyId, Vec<Permission>> = BTreeMap::new();
    let mut select = Query::select();
    select
        .columns([
            AccessKeyPermissionRow::Key,
            AccessKeyPermissionRow::Permission,
        ])
        .from(AccessKeyPermissionRow::Table)
        .and_where(Expr::col(AccessKeyPermissionRow::Key).is_in(ids.clone()))
        .order_by(AccessKeyPermissionRow::Permission, Order::Asc);
    for row in database.fetch_all(&select).await? {
        let key = AccessKeyId::from_bytes(row.id_bytes(AccessKeyPermissionRow::Key)?)
            .map_err(corrupt("an access key ID"))?;
        // A permission that this program does not know any more is left out: the key can only
        // do less because of it, never more.
        if let Some(permission) =
            Permission::from_code(&row.get::<String>(AccessKeyPermissionRow::Permission)?)
        {
            permissions.entry(key).or_default().push(permission);
        }
    }
    let mut projects: BTreeMap<AccessKeyId, Vec<ProjectId>> = BTreeMap::new();
    let mut select = Query::select();
    select
        .columns([AccessKeyProjectRow::Key, AccessKeyProjectRow::Project])
        .from(AccessKeyProjectRow::Table)
        .and_where(Expr::col(AccessKeyProjectRow::Key).is_in(ids))
        .order_by(AccessKeyProjectRow::Project, Order::Asc);
    for row in database.fetch_all(&select).await? {
        let key = AccessKeyId::from_bytes(row.id_bytes(AccessKeyProjectRow::Key)?)
            .map_err(corrupt("an access key ID"))?;
        let project = ProjectId::from_bytes(row.id_bytes(AccessKeyProjectRow::Project)?)
            .map_err(corrupt("a project ID"))?;
        projects.entry(key).or_default().push(project);
    }
    Ok(keys
        .into_iter()
        .map(|key| AccessKeyRecord {
            permissions: permissions.remove(&key.id).unwrap_or_default(),
            projects: projects.remove(&key.id).unwrap_or_default(),
            key,
        })
        .collect())
}

impl Database {
    /// The access key with this secret hash.
    pub(crate) async fn access_key_by_hash(
        &self,
        hash: &Digest,
    ) -> Result<Option<AccessKeyRecord>, StoreError> {
        let mut select = Query::select();
        select
            .columns(key_columns())
            .from(AccessKeyRow::Table)
            .and_where(Expr::col(AccessKeyRow::SecretHash).eq(hash.as_bytes().to_vec()));
        let Some(row) = self.fetch_optional(&select).await? else {
            return Ok(None);
        };
        Ok(with_scope(self, vec![read_key(&row)?]).await?.pop())
    }

    /// The access keys of a person, newest first (then by ID), after `after`, at most `limit`.
    pub(crate) async fn access_keys_of(
        &self,
        owner: UserId,
        after: Option<(Timestamp, AccessKeyId)>,
        limit: u64,
    ) -> Result<Vec<AccessKeyRecord>, StoreError> {
        let mut select = Query::select();
        select
            .columns(key_columns())
            .from(AccessKeyRow::Table)
            .and_where(Expr::col(AccessKeyRow::Owner).eq(bytes(owner.to_bytes())))
            .order_by(AccessKeyRow::CreatedAt, Order::Desc)
            .order_by(AccessKeyRow::Id, Order::Asc)
            .limit(limit);
        if let Some((created, id)) = after {
            // Keyset paging: what comes after the last one in this order.
            let micros = to_micros(created);
            select.cond_where(
                Cond::any()
                    .add(Expr::col(AccessKeyRow::CreatedAt).lt(micros))
                    .add(
                        Cond::all()
                            .add(Expr::col(AccessKeyRow::CreatedAt).eq(micros))
                            .add(Expr::col(AccessKeyRow::Id).gt(bytes(id.to_bytes()))),
                    ),
            );
        }
        let keys = self
            .fetch_all(&select)
            .await?
            .iter()
            .map(read_key)
            .collect::<Result<Vec<_>, _>>()?;
        with_scope(self, keys).await
    }

    /// How many access keys a person has.
    pub(crate) async fn access_key_count(&self, owner: UserId) -> Result<i64, StoreError> {
        let mut select = Query::select();
        select
            .expr_as(
                Expr::col(AccessKeyRow::Id).count(),
                crate::schema::Computed::Total,
            )
            .from(AccessKeyRow::Table)
            .and_where(Expr::col(AccessKeyRow::Owner).eq(bytes(owner.to_bytes())));
        match self.fetch_optional(&select).await? {
            Some(row) => row.get(crate::schema::Computed::Total),
            None => Ok(0),
        }
    }

    /// Records the last use of a key, to the minute `at`: only when that is later than what is
    /// kept, so a busy key writes once a minute.
    pub(crate) async fn touch_access_key(
        &self,
        id: AccessKeyId,
        at: Timestamp,
    ) -> Result<(), StoreError> {
        let micros = to_micros(at);
        let mut update = Query::update();
        update
            .table(AccessKeyRow::Table)
            .value(AccessKeyRow::LastUsedAt, micros)
            .and_where(Expr::col(AccessKeyRow::Id).eq(bytes(id.to_bytes())))
            .cond_where(
                Cond::any()
                    .add(Expr::col(AccessKeyRow::LastUsedAt).is_null())
                    .add(Expr::col(AccessKeyRow::LastUsedAt).lt(micros)),
            );
        self.execute(&update).await.map(|_| ())
    }

    /// Whether the person has a confirmed code app or a passkey.
    pub(crate) async fn has_second_step(&self, person: UserId) -> Result<bool, StoreError> {
        let mut code_app = Query::select();
        code_app
            .column(TwoStep::Person)
            .from(TwoStep::Table)
            .and_where(Expr::col(TwoStep::Person).eq(bytes(person.to_bytes())))
            .and_where(Expr::col(TwoStep::Confirmed).eq(true))
            .limit(1);
        if self.fetch_optional(&code_app).await?.is_some() {
            return Ok(true);
        }
        let mut passkey = Query::select();
        passkey
            .column(PasskeyRow::Id)
            .from(PasskeyRow::Table)
            .and_where(Expr::col(PasskeyRow::Person).eq(bytes(person.to_bytes())))
            .limit(1);
        Ok(self.fetch_optional(&passkey).await?.is_some())
    }

    /// Whether the organization asks a second step of everybody, or any role the person holds
    /// anywhere (their own rows and their groups', at any place) has the switch.
    pub(crate) async fn second_step_required(&self, person: UserId) -> Result<bool, StoreError> {
        let everybody = self
            .resolve_setting(&SECURITY_TWO_STEP_REQUIRED, &Scopes::default())
            .await?;
        if everybody {
            return Ok(true);
        }
        let mut groups = Query::select();
        groups
            .column(GroupMemberRow::Group)
            .from(GroupMemberRow::Table)
            .and_where(Expr::col(GroupMemberRow::Person).eq(bytes(person.to_bytes())));
        let mut select = Query::select();
        select
            .column((AssignmentRow::Table, AssignmentRow::Id))
            .from(AssignmentRow::Table)
            .inner_join(
                RoleRow::Table,
                Expr::col((AssignmentRow::Table, AssignmentRow::Role))
                    .equals((RoleRow::Table, RoleRow::Id)),
            )
            .and_where(Expr::col((RoleRow::Table, RoleRow::TwoStep)).eq(true))
            .cond_where(
                Cond::any()
                    .add(
                        Expr::col((AssignmentRow::Table, AssignmentRow::Person))
                            .eq(bytes(person.to_bytes())),
                    )
                    .add(
                        Expr::col((AssignmentRow::Table, AssignmentRow::Group)).in_subquery(groups),
                    ),
            )
            .limit(1);
        Ok(self.fetch_optional(&select).await?.is_some())
    }

    /// The people who hold the role, through their own rows or their groups' at any place, and
    /// have no confirmed code app and no passkey.
    pub(crate) async fn people_without_second_step(
        &self,
        role: RoleId,
    ) -> Result<Vec<UserId>, StoreError> {
        let mut direct = Query::select();
        direct
            .column(AssignmentRow::Person)
            .from(AssignmentRow::Table)
            .and_where(Expr::col(AssignmentRow::Role).eq(bytes(role.to_bytes())))
            .and_where(Expr::col(AssignmentRow::Person).is_not_null());
        let mut through_groups = Query::select();
        through_groups
            .column((GroupMemberRow::Table, GroupMemberRow::Person))
            .from(GroupMemberRow::Table)
            .inner_join(
                AssignmentRow::Table,
                Expr::col((AssignmentRow::Table, AssignmentRow::Group))
                    .equals((GroupMemberRow::Table, GroupMemberRow::Group)),
            )
            .and_where(
                Expr::col((AssignmentRow::Table, AssignmentRow::Role)).eq(bytes(role.to_bytes())),
            );
        let mut holders: BTreeSet<UserId> = BTreeSet::new();
        for row in self.fetch_all(&direct).await? {
            holders.insert(
                UserId::from_bytes(row.id_bytes(AssignmentRow::Person)?)
                    .map_err(corrupt("a person ID"))?,
            );
        }
        for row in self.fetch_all(&through_groups).await? {
            holders.insert(
                UserId::from_bytes(row.id_bytes(GroupMemberRow::Person)?)
                    .map_err(corrupt("a person ID"))?,
            );
        }
        let holders: Vec<UserId> = holders.into_iter().collect();
        let mut without = Vec::new();
        for chunk in holders.chunks(CHUNK) {
            let ids: Vec<Vec<u8>> = chunk.iter().map(|id| bytes(id.to_bytes())).collect();
            let mut have: BTreeSet<UserId> = BTreeSet::new();
            let mut code_apps = Query::select();
            code_apps
                .column(TwoStep::Person)
                .from(TwoStep::Table)
                .and_where(Expr::col(TwoStep::Person).is_in(ids.clone()))
                .and_where(Expr::col(TwoStep::Confirmed).eq(true));
            for row in self.fetch_all(&code_apps).await? {
                have.insert(
                    UserId::from_bytes(row.id_bytes(TwoStep::Person)?)
                        .map_err(corrupt("a person ID"))?,
                );
            }
            let mut passkeys = Query::select();
            passkeys
                .column(PasskeyRow::Person)
                .from(PasskeyRow::Table)
                .and_where(Expr::col(PasskeyRow::Person).is_in(ids));
            for row in self.fetch_all(&passkeys).await? {
                have.insert(
                    UserId::from_bytes(row.id_bytes(PasskeyRow::Person)?)
                        .map_err(corrupt("a person ID"))?,
                );
            }
            without.extend(
                chunk
                    .iter()
                    .copied()
                    .filter(|person| !have.contains(person)),
            );
        }
        Ok(without)
    }
}

/// Adds a key with its single permissions and its projects, inside the change that makes it.
pub(crate) async fn insert_access_key(
    transaction: &mut Transaction,
    record: &AccessKeyRecord,
) -> Result<(), StoreError> {
    let key = &record.key;
    let mut insert = Query::insert();
    insert
        .into_table(AccessKeyRow::Table)
        .columns(key_columns())
        .values([
            Expr::val(bytes(key.id.to_bytes())),
            Expr::val(bytes(key.owner.to_bytes())),
            Expr::val(key.name.as_str()),
            Expr::val(key.prefix.as_str()),
            Expr::val(key.secret_hash.as_bytes().to_vec()),
            Expr::val(key.choice.code()),
            Expr::val(to_micros(key.created_at)),
            Expr::val(to_micros(key.expires_at)),
            Expr::val(key.last_used_at.map(to_micros)),
        ])
        .map_err(|_| StoreError::Corrupt("an access key"))?;
    transaction.execute(&insert).await?;
    let unique: BTreeSet<Permission> = record.permissions.iter().copied().collect();
    for permission in unique {
        let mut insert = Query::insert();
        insert
            .into_table(AccessKeyPermissionRow::Table)
            .columns([
                AccessKeyPermissionRow::Key,
                AccessKeyPermissionRow::Permission,
            ])
            .values([
                Expr::val(bytes(key.id.to_bytes())),
                Expr::val(permission.code()),
            ])
            .map_err(|_| StoreError::Corrupt("a permission"))?;
        transaction.execute(&insert).await?;
    }
    let unique: BTreeSet<ProjectId> = record.projects.iter().copied().collect();
    for project in unique {
        let mut insert = Query::insert();
        insert
            .into_table(AccessKeyProjectRow::Table)
            .columns([AccessKeyProjectRow::Key, AccessKeyProjectRow::Project])
            .values([
                Expr::val(bytes(key.id.to_bytes())),
                Expr::val(bytes(project.to_bytes())),
            ])
            .map_err(|_| StoreError::Corrupt("a project"))?;
        transaction.execute(&insert).await?;
    }
    Ok(())
}

/// Removes a key of its owner with its permissions and projects; `false` when the owner has no
/// such key.
pub(crate) async fn delete_access_key(
    transaction: &mut Transaction,
    owner: UserId,
    id: AccessKeyId,
) -> Result<bool, StoreError> {
    let mut held = Query::select();
    held.column(AccessKeyRow::Id)
        .from(AccessKeyRow::Table)
        .and_where(Expr::col(AccessKeyRow::Id).eq(bytes(id.to_bytes())))
        .and_where(Expr::col(AccessKeyRow::Owner).eq(bytes(owner.to_bytes())));
    if transaction.fetch_optional(&held).await?.is_none() {
        return Ok(false);
    }
    let mut permissions = Query::delete();
    permissions
        .from_table(AccessKeyPermissionRow::Table)
        .and_where(Expr::col(AccessKeyPermissionRow::Key).eq(bytes(id.to_bytes())));
    transaction.execute(&permissions).await?;
    let mut projects = Query::delete();
    projects
        .from_table(AccessKeyProjectRow::Table)
        .and_where(Expr::col(AccessKeyProjectRow::Key).eq(bytes(id.to_bytes())));
    transaction.execute(&projects).await?;
    let mut delete = Query::delete();
    delete
        .from_table(AccessKeyRow::Table)
        .and_where(Expr::col(AccessKeyRow::Id).eq(bytes(id.to_bytes())));
    Ok(transaction.execute(&delete).await? > 0)
}

#[cfg(test)]
#[path = "tests/access_keys.rs"]
mod tests;
