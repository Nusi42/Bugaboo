//! Setting values on both databases: the layers (CFG-06), the person's own values (CFG-04), the
//! lock, *Reset to default*, and what the registry refuses (ADR-0018).

use bb_app::ports::{BoxFuture, ChangeStore};
use bb_core::event::{Actor, Channel, DomainEvent, EntityKind, EntityRef, FieldChange};
use bb_core::id::{EventId, IdSource, ProjectId, UserId};
use bb_core::project::{Project, ShortName};
use bb_core::settings::{
    APPEARANCE_MODE, SECURITY_SESSION_IDLE_MINUTES, SECURITY_TWO_STEP_REQUIRED, SettingDefinition,
    Source, find,
    for_tests::{TEST_PROJECT_LIMIT, TEST_PROJECT_NOTE, TEST_SWITCH},
};
use bb_testing::{TestResult, each_database};

use super::*;
use crate::test_support::{at, fixture, ids, person};

#[path = "settings/layers.rs"]
mod layers;
#[path = "settings/levels.rs"]
mod levels;
#[path = "settings/refused.rs"]
mod refused;
#[path = "settings/stored.rs"]
mod stored;
#[path = "settings/versions.rs"]
mod versions;

/// The event of a settings change: about the setting `key`, made by the system.
fn event(source: &mut impl IdSource, key: &str) -> DomainEvent {
    let _ = source.now();
    DomainEvent {
        id: EventId::generate(source).unwrap(),
        time: at(5),
        actor: Actor::System,
        channel: Channel::System,
        entity: EntityRef {
            kind: EntityKind::new("setting").unwrap(),
            id: EntityId::from(EventId::generate(source).unwrap()),
        },
        changes: vec![FieldChange {
            field: key.replace(['.', '-'], "_"),
            before: None,
            after: None,
        }],
        reason: None,
    }
}

fn change(key: &str, scope: Scope, value: SettingValue) -> SettingChange {
    SettingChange {
        key: key.to_owned(),
        scope,
        value,
        locked: false,
    }
}

fn text(value: &str) -> SettingValue {
    SettingValue::Text(value.to_owned())
}

/// Runs `write` as a change with its event, the way a use case does.
async fn changed<T: Send>(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    write: impl for<'t> FnOnce(&'t mut Transaction) -> BoxFuture<'t, Result<T, StoreError>> + Send,
) -> Result<T, StoreError> {
    let event = event(source, "x.y");
    ChangeStore::record_change(storage, &event, write).await
}

async fn create_person(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    email: &str,
) -> Result<UserId, StoreError> {
    let person = person(source, email);
    let id = person.id;
    changed(storage, source, move |tx| {
        Box::pin(async move {
            <AuthStorage as bb_app::auth::AccountStore>::insert_person(tx, &person, "hash", at(1))
                .await
        })
    })
    .await?;
    Ok(id)
}

async fn create_project(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    parent: Option<ProjectId>,
    short_name: &str,
) -> Result<ProjectId, StoreError> {
    let project = Project {
        id: ProjectId::generate(source).map_err(|_| StoreError::Corrupt("an ID"))?,
        parent,
        short_name: ShortName::new(short_name).map_err(|_| StoreError::Corrupt("a name"))?,
        name: short_name.to_owned(),
        version: 1,
    };
    let id = project.id;
    changed(storage, source, move |tx| {
        Box::pin(async move {
            <AuthStorage as bb_app::access::PolicyStore>::insert_project(tx, &project).await
        })
    })
    .await?;
    Ok(id)
}

/// The definition of a setting by its key; a key nobody defined is refused as the store did.
fn known(key: &str) -> Result<&'static SettingDefinition, StoreError> {
    find(key).ok_or(StoreError::SettingRejected(SettingProblem::UnknownKey))
}

async fn put(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    scope: Scope,
    key: &'static str,
    value: SettingValue,
) -> Result<(), StoreError> {
    let setting = known(key)?;
    changed(storage, source, move |tx| {
        Box::pin(
            async move { AuthStorage::put_setting_value(tx, scope, setting, &value, at(6)).await },
        )
    })
    .await
}

async fn clear(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    scope: Scope,
    key: &'static str,
) -> Result<bool, StoreError> {
    let setting = known(key)?;
    changed(storage, source, move |tx| {
        Box::pin(async move { AuthStorage::clear_setting_value(tx, scope, setting).await })
    })
    .await
}

async fn lock(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    key: &'static str,
    locked: bool,
) -> Result<(), StoreError> {
    let setting = known(key)?;
    changed(storage, source, move |tx| {
        Box::pin(async move { AuthStorage::put_setting_lock(tx, setting, locked, at(7)).await })
    })
    .await
}

const NOTE: &str = "test.project-note";

fn resolved_note(layers: &Layers) -> (SettingValue, Source, bool) {
    let found = layers.resolve(TEST_PROJECT_NOTE.definition());
    (found.value, found.source, found.locked)
}
