//! The settings part of the in-memory store: the values on the three levels, read as layers and
//! written the way the database adapter does (a value below a lock is refused inside the change).

use std::collections::BTreeMap;

use bb_core::id::{ProjectId, UserId};
use bb_core::settings::{Layers, OrganizationValue, Scope, SettingValue, find};
use bb_policy::Tree;
use jiff::Timestamp;

use super::fake::{FakeError, FakeStore, State, Tx};
use crate::settings::SettingsStore;

/// One stored value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FakeSetting {
    pub(crate) scope: Scope,
    pub(crate) key: String,
    pub(crate) value: SettingValue,
    pub(crate) locked: bool,
}

/// Everything stored that can matter for a person in a place.
pub(crate) fn layers_of(
    state: &State,
    project: Option<ProjectId>,
    person: Option<UserId>,
) -> Layers {
    let known = |row: &&FakeSetting| find(&row.key).is_some();
    let mut layers = Layers::default();
    for row in state.settings.iter().filter(known) {
        if row.scope == Scope::Organization {
            layers.organization.insert(
                row.key.clone(),
                OrganizationValue {
                    value: row.value.clone(),
                    locked: row.locked,
                },
            );
        }
    }
    if let Some(project) = project {
        let mut chain = Tree::of(&state.projects)
            .chain(project)
            .unwrap_or_else(|| vec![project]);
        chain.reverse();
        for id in chain {
            let values: BTreeMap<String, SettingValue> = state
                .settings
                .iter()
                .filter(known)
                .filter(|row| row.scope == Scope::Project(id))
                .map(|row| (row.key.clone(), row.value.clone()))
                .collect();
            layers.projects.push((id, values));
        }
    }
    if let Some(person) = person {
        for row in state.settings.iter().filter(known) {
            if row.scope == Scope::Person(person) {
                layers.person.insert(row.key.clone(), row.value.clone());
            }
        }
    }
    layers
}

/// Stores a value on a scope; the lock of an organization value stays.
pub(crate) fn put(state: &mut State, scope: Scope, key: &str, value: &SettingValue) {
    let locked = scope == Scope::Organization
        && state
            .settings
            .iter()
            .any(|row| row.scope == scope && row.key == key && row.locked);
    state
        .settings
        .retain(|row| !(row.scope == scope && row.key == key));
    state.settings.push(FakeSetting {
        scope,
        key: key.to_owned(),
        value: value.clone(),
        locked,
    });
}

impl SettingsStore for FakeStore {
    async fn person_settings(
        &self,
        people: Vec<UserId>,
    ) -> Result<Vec<(UserId, BTreeMap<String, SettingValue>)>, FakeError> {
        Ok(self.read(|state| {
            people
                .iter()
                .filter_map(|person| {
                    let layers = layers_of(state, None, Some(*person));
                    (!layers.person.is_empty()).then_some((*person, layers.person))
                })
                .collect()
        }))
    }

    async fn setting_layers_in(
        tx: &mut Tx,
        project: Option<ProjectId>,
        person: Option<UserId>,
    ) -> Result<Layers, FakeError> {
        Ok(layers_of(&tx.state, project, person))
    }

    async fn put_setting_value(
        tx: &mut Tx,
        scope: Scope,
        key: &str,
        value: &SettingValue,
        _at: Timestamp,
    ) -> Result<(), FakeError> {
        let locked_above = scope != Scope::Organization
            && tx
                .state
                .settings
                .iter()
                .any(|row| row.scope == Scope::Organization && row.key == key && row.locked);
        if locked_above {
            return Err(FakeError::Aborted);
        }
        put(&mut tx.state, scope, key, value);
        Ok(())
    }

    async fn clear_setting_value(tx: &mut Tx, scope: Scope, key: &str) -> Result<bool, FakeError> {
        let before = tx.state.settings.len();
        tx.state
            .settings
            .retain(|row| !(row.scope == scope && row.key == key));
        Ok(tx.state.settings.len() < before)
    }

    async fn put_setting_lock(
        tx: &mut Tx,
        key: &str,
        locked: bool,
        _at: Timestamp,
    ) -> Result<(), FakeError> {
        let Some(definition) = find(key) else {
            return Err(FakeError::Failed);
        };
        if !locked {
            // A row that only held the lock at the standard value goes away on unlocking.
            let standard = definition.default().to_value();
            tx.state.settings.retain(|row| {
                !(row.scope == Scope::Organization && row.key == key && row.value == standard)
            });
            for row in &mut tx.state.settings {
                if row.scope == Scope::Organization && row.key == key {
                    row.locked = false;
                }
            }
            return Ok(());
        }
        if let Some(row) = tx
            .state
            .settings
            .iter_mut()
            .find(|row| row.scope == Scope::Organization && row.key == key)
        {
            row.locked = true;
            return Ok(());
        }
        tx.state.settings.push(FakeSetting {
            scope: Scope::Organization,
            key: key.to_owned(),
            value: definition.default().to_value(),
            locked,
        });
        Ok(())
    }
}
