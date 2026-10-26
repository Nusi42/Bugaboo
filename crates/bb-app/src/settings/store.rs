//! The port of the settings use cases: what they need from storage.
//!
//! Like [`PolicyStore`], reads take `&self` and **writes** are associated functions that take the
//! open transaction and are called from inside `record_change`, so a change and its domain event
//! are committed together (ARCH-EVENTS). The storage checks every key, level and value against the
//! registry (ADR-0018); the use cases decide who may change what before they ask it to.

use std::collections::BTreeMap;
use std::future::Future;

use bb_core::id::{ProjectId, UserId};
use bb_core::settings::{Layers, Scope, SettingDefinition, SettingValue};
use jiff::Timestamp;

use crate::access::PolicyStore;
use crate::store_macros::{queries, reads, writes};

/// What the settings use cases need from storage.
pub trait SettingsStore: PolicyStore + 'static {
    queries! {
        /// Everything stored that can matter for a person in a place: the organization's values,
        /// the values of the projects from `project` up to the top (nearest first) and the
        /// person's own. A project that does not exist counts as a project without a parent.
        fn setting_layers(project: Option<ProjectId>, person: Option<UserId>) -> Layers = setting_layers_in;
    }

    reads! {
        /// The values of the settings that these people chose themselves, in one read: the person
        /// level of their layers, for each person who has any. What the reader of notifications
        /// needs for all the followers of an event at once.
        fn person_settings(people: Vec<UserId>) -> Vec<(UserId, BTreeMap<String, SettingValue>)>;
    }

    writes! {
        /// Stores the value of a setting on one scope. The change stops itself (the same as
        /// `AuthStore::aborted`) when the organization locked the setting and the scope is a
        /// project or a person: that is decided inside the change, so a lock made at the same
        /// moment is never overtaken.
        fn put_setting_value(
            scope: Scope,
            setting: &'static SettingDefinition,
            value: &SettingValue,
            at: Timestamp
        ) -> ();
        /// Takes the value of a setting on one scope away, so the layer below applies again.
        /// `false` when there was none.
        fn clear_setting_value(scope: Scope, setting: &'static SettingDefinition) -> bool;
        /// Locks or unlocks the organization's value of a setting. Locking a setting that has no
        /// organization value yet gives it the default as its value first; unlocking one that
        /// holds only that default removes it again, and one without a value stays without.
        fn put_setting_lock(setting: &'static SettingDefinition, locked: bool, at: Timestamp) -> ();
    }
}
