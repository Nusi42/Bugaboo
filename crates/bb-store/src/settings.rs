//! Setting values per level (ADR-0018). The registry in `bb-core` says which settings exist, on
//! which levels they can be set and what type they have; this file stores the values and is the
//! only place that reads or writes the tables of setting values (ARCH-SETTINGS).
//!
//! A value is stored for the organization or for one project (table `setting`), or for one person
//! (table `person_setting`, so the person is a column of its own and goes with the person's
//! data). Which value counts is decided by `bb_core::settings::Layers`: the organization's, the
//! nearest project up the tree that sets it, the person's own, unless the organization locked the
//! setting. A key that is not in the registry is refused, so no value can exist that the registry
//! does not know about. A change is written through [`Database::record_change`], so it comes with
//! its event.

use std::collections::BTreeMap;

use bb_app::settings::SettingsStore;
use bb_core::event::DomainEvent;
use bb_core::id::{EntityId, ProjectId, UserId};
use bb_core::settings::{
    Integer, Layers, Level, OrganizationValue, Scope, Setting, SettingKind, SettingValue, Text,
};
use bb_core::technical::TechnicalName;
use bb_core::version::next;
use jiff::Timestamp;
use sea_query::{Expr, ExprTrait as _, OnConflict, Order, Query};

use crate::auth::AuthStorage;
use crate::db::{Database, Row, Transaction, to_micros};
use crate::error::{SettingProblem, StoreError};
use crate::projects::chain_of;
use crate::schema::{PersonSettingRow, PolicyVersionRow, SettingRow as Columns};

/// A value to store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingChange {
    /// The key, as declared in the registry.
    pub key: String,
    /// Where the value is set: the organization, a project or a person.
    pub scope: Scope,
    /// The value.
    pub value: SettingValue,
    /// Whether lower levels may not override it (only organization values of lockable settings).
    pub locked: bool,
}

/// A stored value as it was read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredValue {
    /// The value.
    pub value: SettingValue,
    /// Whether it is locked (only organization values are).
    pub locked: bool,
    /// When it was last changed.
    pub updated_at: Timestamp,
}

/// Whose settings are being read: lower levels are only looked at when given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Scopes {
    /// The project, if the value is read in the context of one; the projects above it count too.
    pub project: Option<ProjectId>,
    /// The person, if the value is read for one.
    pub person: Option<UserId>,
}

/// Checks a change against the registry.
fn validate(change: &SettingChange) -> Result<TechnicalName, SettingProblem> {
    let definition = bb_core::settings::find(&change.key).ok_or(SettingProblem::UnknownKey)?;
    if !definition.levels().contains(&change.scope.level()) {
        return Err(SettingProblem::LevelNotAllowed);
    }
    if !definition.has_type_of(&change.value) {
        return Err(SettingProblem::WrongType);
    }
    if !definition.accepts(&change.value) {
        return Err(SettingProblem::OutOfBounds);
    }
    if change.locked && !(definition.lockable() && change.scope == Scope::Organization) {
        return Err(SettingProblem::NotLockable);
    }
    TechnicalName::new(definition.key()).map_err(|_| SettingProblem::UnknownKey)
}

/// The key of a setting looked up for reading: it must be in the registry.
fn known(key: &str) -> Result<TechnicalName, SettingProblem> {
    let definition = bb_core::settings::find(key).ok_or(SettingProblem::UnknownKey)?;
    TechnicalName::new(definition.key()).map_err(|_| SettingProblem::UnknownKey)
}

fn id_bytes(id: impl Into<EntityId>) -> Vec<u8> {
    id.into().to_bytes().to_vec()
}

/// What the `scope` column holds for an organization or project value.
fn scope_bytes(scope: Scope) -> Vec<u8> {
    match scope {
        Scope::Organization | Scope::Person(_) => Vec::new(),
        Scope::Project(project) => id_bytes(project),
    }
}

/// Writes the value of a change inside the transaction of a change.
async fn upsert(
    transaction: &mut Transaction,
    key: &TechnicalName,
    change: &SettingChange,
    at: Timestamp,
) -> Result<(), StoreError> {
    let value = serde_json::to_string(&change.value)?;
    if let Scope::Person(person) = change.scope {
        let mut insert = Query::insert();
        insert
            .into_table(PersonSettingRow::Table)
            .columns([
                PersonSettingRow::Person,
                PersonSettingRow::Key,
                PersonSettingRow::Value,
                PersonSettingRow::UpdatedAt,
            ])
            .values([
                Expr::val(id_bytes(person)),
                Expr::val(key.as_str()),
                Expr::val(value),
                Expr::val(to_micros(at)),
            ])
            .map_err(|_| StoreError::Corrupt("a setting"))?
            .on_conflict(
                OnConflict::columns([PersonSettingRow::Person, PersonSettingRow::Key])
                    .update_columns([PersonSettingRow::Value, PersonSettingRow::UpdatedAt])
                    .to_owned(),
            );
        transaction.execute(&insert).await?;
        return Ok(());
    }
    let mut insert = Query::insert();
    insert
        .into_table(Columns::Table)
        .columns([
            Columns::Key,
            Columns::Level,
            Columns::Scope,
            Columns::Value,
            Columns::Locked,
            Columns::UpdatedAt,
        ])
        .values([
            Expr::val(key.as_str()),
            Expr::val(change.scope.level().code()),
            Expr::val(scope_bytes(change.scope)),
            Expr::val(value),
            Expr::val(change.locked),
            Expr::val(to_micros(at)),
        ])
        .map_err(|_| StoreError::Corrupt("a setting"))?
        .on_conflict(
            OnConflict::columns([Columns::Key, Columns::Level, Columns::Scope])
                .update_columns([Columns::Value, Columns::Locked, Columns::UpdatedAt])
                .to_owned(),
        );
    transaction.execute(&insert).await?;
    Ok(())
}

/// Removes the value of one setting on one scope.
async fn delete(
    transaction: &mut Transaction,
    key: &TechnicalName,
    scope: Scope,
) -> Result<bool, StoreError> {
    if let Scope::Person(person) = scope {
        let mut delete = Query::delete();
        delete
            .from_table(PersonSettingRow::Table)
            .and_where(Expr::col(PersonSettingRow::Person).eq(id_bytes(person)))
            .and_where(Expr::col(PersonSettingRow::Key).eq(key.as_str()));
        return Ok(transaction.execute(&delete).await? > 0);
    }
    let mut delete = Query::delete();
    delete
        .from_table(Columns::Table)
        .and_where(Expr::col(Columns::Key).eq(key.as_str()))
        .and_where(Expr::col(Columns::Level).eq(scope.level().code()))
        .and_where(Expr::col(Columns::Scope).eq(scope_bytes(scope)));
    Ok(transaction.execute(&delete).await? > 0)
}

/// The organization's row of one setting, if there is one.
async fn organization_row(
    transaction: &mut Transaction,
    key: &TechnicalName,
) -> Result<Option<StoredValue>, StoreError> {
    let mut select = Query::select();
    select
        .columns([Columns::Value, Columns::Locked, Columns::UpdatedAt])
        .from(Columns::Table)
        .and_where(Expr::col(Columns::Key).eq(key.as_str()))
        .and_where(Expr::col(Columns::Level).eq(Level::Organization.code()))
        .and_where(Expr::col(Columns::Scope).eq(Vec::<u8>::new()));
    transaction
        .fetch_optional(&select)
        .await?
        .as_ref()
        .map(read_value)
        .transpose()
}

/// The version of a policy that is made of several settings, `0` while it has no row.
pub(crate) async fn policy_version(database: &Database, policy: &str) -> Result<i64, StoreError> {
    let mut select = Query::select();
    select
        .column(PolicyVersionRow::Version)
        .from(PolicyVersionRow::Table)
        .and_where(Expr::col(PolicyVersionRow::Policy).eq(policy));
    match database.fetch_optional(&select).await? {
        Some(row) => row.get(PolicyVersionRow::Version),
        None => Ok(0),
    }
}

/// Counts the version of a policy up, inside a change; `false` when the policy is not at
/// `version`. The first change makes the row.
pub(crate) async fn count_policy_version_up(
    transaction: &mut Transaction,
    policy: &str,
    version: i64,
) -> Result<bool, StoreError> {
    if version == 0 {
        let mut insert = Query::insert();
        insert
            .into_table(PolicyVersionRow::Table)
            .columns([PolicyVersionRow::Policy, PolicyVersionRow::Version])
            .values([Expr::val(policy), Expr::val(next(version))])
            .map_err(|_| StoreError::Corrupt("a policy version"))?
            .on_conflict(
                OnConflict::column(PolicyVersionRow::Policy)
                    .do_nothing()
                    .to_owned(),
            );
        return Ok(transaction.execute(&insert).await? > 0);
    }
    let mut update = Query::update();
    update
        .table(PolicyVersionRow::Table)
        .value(PolicyVersionRow::Version, next(version))
        .and_where(Expr::col(PolicyVersionRow::Policy).eq(policy))
        .and_where(Expr::col(PolicyVersionRow::Version).eq(version));
    Ok(transaction.execute(&update).await? > 0)
}

fn read_value(row: &Row) -> Result<StoredValue, StoreError> {
    Ok(StoredValue {
        value: serde_json::from_str(&row.get::<String>(Columns::Value)?)?,
        locked: row.get(Columns::Locked)?,
        updated_at: row.time(Columns::UpdatedAt)?,
    })
}

/// A key and a value read from a row, if the key is one the registry has and the value has the
/// type it declares. A row of a setting that no longer exists, or whose value is of another type
/// (a damaged or hand-edited database), counts for nothing: the layers below apply.
fn usable(key: &str, value: &str) -> Result<Option<(String, SettingValue)>, StoreError> {
    let Some(definition) = bb_core::settings::find(key) else {
        return Ok(None);
    };
    let value: SettingValue = serde_json::from_str(value)?;
    if definition.has_type_of(&value) {
        Ok(Some((key.to_owned(), value)))
    } else {
        tracing::warn!(
            setting = key,
            "a stored setting has the wrong type; it counts for nothing"
        );
        Ok(None)
    }
}

/// Everything stored that can matter for `scopes`, read in one snapshot.
async fn layers_in(transaction: &mut Transaction, scopes: &Scopes) -> Result<Layers, StoreError> {
    let mut layers = Layers::default();

    let mut organization = Query::select();
    organization
        .columns([Columns::Key, Columns::Value, Columns::Locked])
        .from(Columns::Table)
        .and_where(Expr::col(Columns::Level).eq(Level::Organization.code()));
    for row in transaction.fetch_all(&organization).await? {
        let key: String = row.get(Columns::Key)?;
        if let Some((key, value)) = usable(&key, &row.get::<String>(Columns::Value)?)? {
            layers.organization.insert(
                key,
                OrganizationValue {
                    value,
                    locked: row.get(Columns::Locked)?,
                },
            );
        }
    }

    if let Some(project) = scopes.project {
        // The nearest project first. A project that does not exist is still looked at itself.
        let mut chain = chain_of(transaction, project)
            .await?
            .unwrap_or_else(|| vec![project]);
        chain.reverse();
        let mut select = Query::select();
        select
            .columns([Columns::Scope, Columns::Key, Columns::Value])
            .from(Columns::Table)
            .and_where(Expr::col(Columns::Level).eq(Level::Project.code()))
            .and_where(Expr::col(Columns::Scope).is_in(chain.iter().map(|id| id_bytes(*id))))
            .order_by(Columns::Key, Order::Asc);
        let rows = transaction.fetch_all(&select).await?;
        for id in chain {
            let mut values = BTreeMap::new();
            for row in &rows {
                if row.get::<Vec<u8>>(Columns::Scope)? == id_bytes(id)
                    && let Some((key, value)) = usable(
                        &row.get::<String>(Columns::Key)?,
                        &row.get::<String>(Columns::Value)?,
                    )?
                {
                    values.insert(key, value);
                }
            }
            layers.projects.push((id, values));
        }
    }

    if let Some(person) = scopes.person {
        let mut select = Query::select();
        select
            .columns([PersonSettingRow::Key, PersonSettingRow::Value])
            .from(PersonSettingRow::Table)
            .and_where(Expr::col(PersonSettingRow::Person).eq(id_bytes(person)));
        for row in transaction.fetch_all(&select).await? {
            if let Some((key, value)) = usable(
                &row.get::<String>(PersonSettingRow::Key)?,
                &row.get::<String>(PersonSettingRow::Value)?,
            )? {
                layers.person.insert(key, value);
            }
        }
    }
    Ok(layers)
}

/// Stores the value of a setting of the whole organization inside the transaction of a change
/// that has its own event (the sign-in use cases change who may sign up this way). A lock the
/// setting has is kept.
pub(crate) async fn put_organization_setting(
    transaction: &mut Transaction,
    key: &str,
    value: &SettingValue,
    at: Timestamp,
) -> Result<(), StoreError> {
    let mut change = SettingChange {
        key: key.to_owned(),
        scope: Scope::Organization,
        value: value.clone(),
        locked: false,
    };
    let key = validate(&change).map_err(StoreError::SettingRejected)?;
    change.locked = organization_row(transaction, &key)
        .await?
        .is_some_and(|row| row.locked);
    upsert(transaction, &key, &change, at).await
}

impl Database {
    /// Stores a value, together with its event (`event` is appended in the same transaction).
    ///
    /// # Errors
    /// Fails with [`StoreError::SettingRejected`] when the key is not in the registry, the level,
    /// the type or the value does not fit, or the setting cannot be locked; and when the database
    /// fails.
    pub async fn set_setting(
        &self,
        event: &DomainEvent,
        change: &SettingChange,
    ) -> Result<(), StoreError> {
        let key = validate(change).map_err(StoreError::SettingRejected)?;
        self.record_change(event, async |transaction| {
            upsert(transaction, &key, change, event.time).await
        })
        .await
    }

    /// Removes a stored value, so the level below the removed one (or the default) applies
    /// again. The event is appended in the same transaction.
    ///
    /// # Errors
    /// Fails with [`StoreError::SettingRejected`] for a key that is not in the registry, and when
    /// the database fails.
    pub async fn clear_setting(
        &self,
        event: &DomainEvent,
        key: &str,
        scope: Scope,
    ) -> Result<bool, StoreError> {
        let key = known(key).map_err(StoreError::SettingRejected)?;
        self.record_change(event, async |transaction| {
            delete(transaction, &key, scope).await
        })
        .await
    }

    /// The value stored for one setting on one scope, if there is one.
    ///
    /// # Errors
    /// Fails with [`StoreError::SettingRejected`] for a key that is not in the registry, and when
    /// the database fails or the stored value is unreadable.
    pub async fn stored_setting(
        &self,
        key: &str,
        scope: Scope,
    ) -> Result<Option<StoredValue>, StoreError> {
        let key = known(key).map_err(StoreError::SettingRejected)?;
        let mut select = Query::select();
        if let Scope::Person(person) = scope {
            select
                .columns([PersonSettingRow::Value, PersonSettingRow::UpdatedAt])
                .from(PersonSettingRow::Table)
                .and_where(Expr::col(PersonSettingRow::Person).eq(id_bytes(person)))
                .and_where(Expr::col(PersonSettingRow::Key).eq(key.as_str()));
            return self
                .fetch_optional(&select)
                .await?
                .map(|row| {
                    Ok(StoredValue {
                        value: serde_json::from_str(&row.get::<String>(PersonSettingRow::Value)?)?,
                        locked: false,
                        updated_at: row.time(PersonSettingRow::UpdatedAt)?,
                    })
                })
                .transpose();
        }
        select
            .columns([Columns::Value, Columns::Locked, Columns::UpdatedAt])
            .from(Columns::Table)
            .and_where(Expr::col(Columns::Key).eq(key.as_str()))
            .and_where(Expr::col(Columns::Level).eq(scope.level().code()))
            .and_where(Expr::col(Columns::Scope).eq(scope_bytes(scope)));
        self.fetch_optional(&select)
            .await?
            .as_ref()
            .map(read_value)
            .transpose()
    }

    /// Everything stored that can matter for a person in a place, read in one snapshot.
    ///
    /// # Errors
    /// Fails when the database fails or a stored value is unreadable.
    pub async fn setting_layers(&self, scopes: &Scopes) -> Result<Layers, StoreError> {
        let mut transaction = self.begin_read().await?;
        layers_in(&mut transaction, scopes).await
    }

    /// The value to use for a setting: the organization's if it is locked, else the person's,
    /// else the nearest project's, else the organization's, else the default. A stored value of
    /// the wrong type is logged (by the key of the setting, never the value) and counts for
    /// nothing.
    ///
    /// # Errors
    /// Fails when the database fails or a stored value is unreadable.
    pub async fn resolve_setting<T: SettingKind>(
        &self,
        setting: &Setting<T>,
        scopes: &Scopes,
    ) -> Result<T::Value, StoreError> {
        let layers = self.setting_layers(scopes).await?;
        resolved(setting, &layers)
    }
}

/// A row of the table of settings as migration 3 left it, written around the registry.
#[cfg(feature = "testing")]
pub struct M3Setting<'a> {
    /// The key, which can be one the registry has not got any more.
    pub key: &'a str,
    /// `organization` or `person`.
    pub level: &'a str,
    /// The person (16 bytes), or nothing for the organization.
    pub scope: Vec<u8>,
    /// The value as stored: JSON text.
    pub value: &'a str,
    /// Whether it is locked.
    pub locked: bool,
}

#[cfg(feature = "testing")]
impl Database {
    /// Stores rows as migration 3 knew the table: the database must be at migration 3 exactly
    /// (`migrate_up_to`).
    ///
    /// # Errors
    /// Fails when the database fails.
    pub async fn seed_old_setting_rows(&self, rows: &[M3Setting<'_>]) -> Result<(), StoreError> {
        for row in rows {
            let mut insert = Query::insert();
            insert
                .into_table(Columns::Table)
                .columns([
                    Columns::Key,
                    Columns::Level,
                    Columns::Scope,
                    Columns::Value,
                    Columns::Locked,
                    Columns::UpdatedAt,
                ])
                .values([
                    Expr::val(row.key),
                    Expr::val(row.level),
                    Expr::val(row.scope.clone()),
                    Expr::val(row.value),
                    Expr::val(row.locked),
                    Expr::val(7_i64),
                ])
                .map_err(|_| StoreError::Corrupt("a setting"))?;
            self.execute(&insert).await?;
        }
        Ok(())
    }

    /// How many rows the table of the organization's and the projects' settings holds, whatever
    /// they are.
    ///
    /// # Errors
    /// Fails when the database fails.
    pub async fn setting_row_count(&self) -> Result<usize, StoreError> {
        let mut select = Query::select();
        select.column(Columns::Key).from(Columns::Table);
        Ok(self.fetch_all(&select).await?.len())
    }
}

/// The value of `setting` in `layers`, as its type.
pub(crate) fn resolved<T: SettingKind>(
    setting: &Setting<T>,
    layers: &Layers,
) -> Result<T::Value, StoreError> {
    setting
        .resolve(Some(&layers.resolve(setting.definition()).value))
        .map_err(|_| StoreError::Corrupt("a setting default"))
}

/// A whole-number setting in `layers`, cut to its bounds: a value that was stored before the
/// bounds, or by hand, is cut to them.
pub(crate) fn resolved_in_bounds(
    setting: &Setting<Integer>,
    layers: &Layers,
) -> Result<i64, StoreError> {
    Ok(setting.definition().clamp(resolved(setting, layers)?))
}

/// A list setting in `layers`, cut to the limits of the setting.
pub(crate) fn resolved_list(
    setting: &Setting<Text>,
    layers: &Layers,
) -> Result<String, StoreError> {
    Ok(setting.definition().cap_list(&resolved(setting, layers)?))
}

impl SettingsStore for AuthStorage {
    async fn setting_layers_in(
        transaction: &mut Transaction,
        project: Option<ProjectId>,
        person: Option<UserId>,
    ) -> Result<Layers, StoreError> {
        layers_in(transaction, &Scopes { project, person }).await
    }

    async fn put_setting_value(
        transaction: &mut Transaction,
        scope: Scope,
        key: &str,
        value: &SettingValue,
        at: Timestamp,
    ) -> Result<(), StoreError> {
        let mut change = SettingChange {
            key: key.to_owned(),
            scope,
            value: value.clone(),
            locked: false,
        };
        let name = validate(&change).map_err(StoreError::SettingRejected)?;
        let organization = organization_row(transaction, &name).await?;
        match scope {
            Scope::Organization => {
                // The value of the organization changes; its lock stays.
                change.locked = organization.is_some_and(|row| row.locked);
            }
            Scope::Project(_) | Scope::Person(_) => {
                // The lock is read inside the change: a lock made at the same moment is seen.
                if organization.is_some_and(|row| row.locked) {
                    return Err(StoreError::Aborted);
                }
            }
        }
        upsert(transaction, &name, &change, at).await
    }

    async fn clear_setting_value(
        transaction: &mut Transaction,
        scope: Scope,
        key: &str,
    ) -> Result<bool, StoreError> {
        let name = known(key).map_err(StoreError::SettingRejected)?;
        delete(transaction, &name, scope).await
    }

    async fn put_setting_lock(
        transaction: &mut Transaction,
        key: &str,
        locked: bool,
        at: Timestamp,
    ) -> Result<(), StoreError> {
        let definition = bb_core::settings::find(key)
            .ok_or(StoreError::SettingRejected(SettingProblem::UnknownKey))?;
        let name = known(key).map_err(StoreError::SettingRejected)?;
        let row = organization_row(transaction, &name).await?;
        let standard = definition.default().to_value();
        if !locked {
            // Unlocking leaves a value of the organization alone, but a row that only held the
            // lock at the standard value has nothing left to hold: it goes away, and a setting
            // that has no row stays as it is.
            let Some(row) = row else {
                return Ok(());
            };
            if row.value == standard {
                delete(transaction, &name, Scope::Organization).await?;
                return Ok(());
            }
            let change = SettingChange {
                key: key.to_owned(),
                scope: Scope::Organization,
                value: row.value,
                locked: false,
            };
            validate(&change).map_err(StoreError::SettingRejected)?;
            return upsert(transaction, &name, &change, at).await;
        }
        let change = SettingChange {
            key: key.to_owned(),
            scope: Scope::Organization,
            value: row.map_or(standard, |row| row.value),
            locked,
        };
        validate(&change).map_err(StoreError::SettingRejected)?;
        upsert(transaction, &name, &change, at).await
    }
}

#[cfg(test)]
#[path = "tests/settings.rs"]
mod tests;
