//! Settings in layers (CFG-06): which value counts for a person, and where it comes from.
//!
//! The order is fixed: the default, the organization, the nearest project up the tree that sets
//! the value, the person. A value the organization locked wins over every lower layer. The
//! functions here take the stored values as plain data and do no I/O, so the same rule is used by
//! the store, the use cases and the tests.

use std::collections::BTreeMap;

use super::{Bool, Integer, Level, Setting, SettingDefinition, SettingKind, SettingValue, Text};
use crate::id::{ProjectId, UserId};

/// Where one stored value belongs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// The whole organization.
    Organization,
    /// One project and its sub-projects.
    Project(ProjectId),
    /// One person (*My settings*).
    Person(UserId),
}

impl Scope {
    /// The level a value of this scope is on.
    #[must_use]
    pub const fn level(self) -> Level {
        match self {
            Self::Organization => Level::Organization,
            Self::Project(_) => Level::Project,
            Self::Person(_) => Level::Person,
        }
    }
}

/// Where the value that counts comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// Nobody set it: the default of the registry.
    Default,
    /// The organization set it.
    Organization,
    /// A project set it (the nearest one up the tree).
    Project(ProjectId),
    /// The person set it themselves.
    Person,
}

/// The value that counts, where it comes from, and whether the organization locked it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    /// The value that counts.
    pub value: SettingValue,
    /// Where it comes from.
    pub source: Source,
    /// Whether the organization locked the setting: lower layers cannot change it.
    pub locked: bool,
}

/// The stored value of the organization for one setting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationValue {
    /// The value.
    pub value: SettingValue,
    /// Whether lower layers may not override it.
    pub locked: bool,
}

/// Everything stored that can matter for one person in one place: the organization's values, the
/// values of the projects from the place up to the top (nearest first), and the person's own.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Layers {
    /// The organization's values by key.
    pub organization: BTreeMap<String, OrganizationValue>,
    /// The projects from the nearest one up to the top, each with its values by key.
    pub projects: Vec<(ProjectId, BTreeMap<String, SettingValue>)>,
    /// The person's own values by key.
    pub person: BTreeMap<String, SettingValue>,
    /// The version of each of the person's own values by key, as the stores count it (ADR-0019);
    /// a key that is not here has no value of the person's own, which is version `0`.
    pub person_versions: BTreeMap<String, i64>,
}

impl Layers {
    /// The version of the person's own value of `key`; `0` while there is none.
    #[must_use]
    pub fn person_version(&self, key: &str) -> i64 {
        self.person_versions.get(key).copied().unwrap_or(0)
    }

    /// The value that counts for `definition`, and where it comes from.
    ///
    /// A layer a setting cannot be set on is never looked at, so a value that was stored before
    /// the setting lost a level counts for nothing.
    #[must_use]
    pub fn resolve(&self, definition: &SettingDefinition) -> Resolved {
        let key = definition.key();
        let mut resolved = Resolved {
            value: definition.default().to_value(),
            source: Source::Default,
            locked: false,
        };
        if definition.levels().contains(&Level::Organization)
            && let Some(set) = self.organization.get(key)
        {
            resolved.value = set.value.clone();
            resolved.source = Source::Organization;
            if set.locked && definition.lockable() {
                resolved.locked = true;
                return resolved;
            }
        }
        if definition.levels().contains(&Level::Project)
            && let Some((project, value)) = self
                .projects
                .iter()
                .find_map(|(project, values)| values.get(key).map(|value| (*project, value)))
        {
            resolved.value = value.clone();
            resolved.source = Source::Project(project);
        }
        if definition.levels().contains(&Level::Person)
            && let Some(value) = self.person.get(key)
        {
            resolved.value = value.clone();
            resolved.source = Source::Person;
        }
        resolved
    }

    /// The value of `setting` that counts, as its type. A stored value of another type (written by
    /// hand; the registry accepts nothing else) counts as the default. `None` only when the
    /// default of the registry itself has another type, which would be a defect of the registry.
    #[must_use]
    pub fn get<K: SettingKind>(&self, setting: &Setting<K>) -> Option<K::Value> {
        let resolved = self.resolve(setting.definition()).value;
        setting
            .resolve(Some(&resolved))
            .or_else(|_| setting.default_value())
            .ok()
    }

    /// The yes or no of a setting.
    #[must_use]
    pub fn yes(&self, setting: &Setting<Bool>) -> bool {
        self.get(setting).unwrap_or_default()
    }

    /// The number of a setting, kept inside its bounds. A value stored before the bounds, or by
    /// hand, is cut to them.
    #[must_use]
    pub fn number(&self, setting: &Setting<Integer>) -> i64 {
        setting
            .definition()
            .clamp(self.get(setting).unwrap_or_default())
    }

    /// The text of a setting.
    #[must_use]
    pub fn text(&self, setting: &Setting<Text>) -> String {
        self.get(setting).unwrap_or_default()
    }

    /// The text of a list setting, cut to the limits of the setting.
    #[must_use]
    pub fn list(&self, setting: &Setting<Text>) -> String {
        setting.definition().cap_list(&self.text(setting))
    }

    /// The value that would count if the layer of `scope` had no value of its own: what *Reset to
    /// default* shows. For the organization that is the default.
    #[must_use]
    pub fn without(&self, definition: &SettingDefinition, scope: Scope) -> Resolved {
        let mut lower = self.clone();
        let key = definition.key();
        match scope {
            Scope::Organization => {
                lower.organization.remove(key);
            }
            Scope::Project(project) => {
                for (id, values) in &mut lower.projects {
                    if *id == project {
                        values.remove(key);
                    }
                }
            }
            Scope::Person(_) => {
                lower.person.remove(key);
            }
        }
        lower.resolve(definition)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::SequentialIds;
    use crate::settings::for_tests::{
        TEST_ORGANIZATION_ONLY, TEST_PROJECT_LIMIT, TEST_PROJECT_NOTE,
    };

    fn ids() -> (ProjectId, ProjectId, UserId) {
        let mut source = SequentialIds::default();
        (
            ProjectId::generate(&mut source).unwrap(),
            ProjectId::generate(&mut source).unwrap(),
            UserId::generate(&mut source),
        )
    }

    fn text(value: &str) -> SettingValue {
        SettingValue::Text(value.to_owned())
    }

    fn layers_with(
        key: &str,
        organization: Option<(&str, bool)>,
        projects: &[(ProjectId, &str)],
        person: Option<&str>,
    ) -> Layers {
        let mut layers = Layers::default();
        if let Some((value, locked)) = organization {
            layers.organization.insert(
                key.to_owned(),
                OrganizationValue {
                    value: text(value),
                    locked,
                },
            );
        }
        for (project, value) in projects {
            layers
                .projects
                .push((*project, BTreeMap::from([(key.to_owned(), text(value))])));
        }
        if let Some(value) = person {
            layers.person.insert(key.to_owned(), text(value));
        }
        layers
    }

    #[test]
    fn cfg_06_the_default_counts_when_nobody_set_anything() {
        let resolved = Layers::default().resolve(TEST_PROJECT_NOTE.definition());
        assert_eq!(resolved.value, text("none"));
        assert_eq!(resolved.source, Source::Default);
        assert!(!resolved.locked);
    }

    #[test]
    fn cfg_06_the_nearest_project_wins_over_the_ones_above_and_the_person_over_all() {
        let (near, far, _) = ids();
        let definition = TEST_PROJECT_NOTE.definition();
        let key = definition.key();
        let layers = layers_with(
            key,
            Some(("org", false)),
            &[(near, "near"), (far, "far")],
            None,
        );
        let resolved = layers.resolve(definition);
        assert_eq!(resolved.value, text("near"));
        assert_eq!(resolved.source, Source::Project(near));
        // A project that sets nothing is skipped: the one above counts.
        let mut skipping = layers.clone();
        skipping.projects[0].1.clear();
        assert_eq!(skipping.resolve(definition).source, Source::Project(far));
        // The person's own value wins over every project.
        let mine = layers_with(key, Some(("org", false)), &[(near, "near")], Some("mine"));
        let resolved = mine.resolve(definition);
        assert_eq!(
            (resolved.value, resolved.source),
            (text("mine"), Source::Person)
        );
    }

    #[test]
    fn cfg_06_a_locked_organization_value_wins_over_every_lower_layer() {
        let (near, _, _) = ids();
        let definition = TEST_PROJECT_NOTE.definition();
        let layers = layers_with(
            definition.key(),
            Some(("org", true)),
            &[(near, "near")],
            Some("mine"),
        );
        let resolved = layers.resolve(definition);
        assert_eq!(resolved.value, text("org"));
        assert_eq!(resolved.source, Source::Organization);
        assert!(resolved.locked);
    }

    #[test]
    fn cfg_06_a_lock_on_a_setting_that_cannot_be_locked_counts_for_nothing() {
        let (project, _, _) = ids();
        let definition = TEST_PROJECT_LIMIT.definition();
        assert!(!definition.lockable());
        let mut layers = Layers::default();
        layers.organization.insert(
            definition.key().to_owned(),
            OrganizationValue {
                value: SettingValue::Integer(5),
                locked: true,
            },
        );
        layers.projects.push((
            project,
            BTreeMap::from([(definition.key().to_owned(), SettingValue::Integer(9))]),
        ));
        let resolved = layers.resolve(definition);
        assert_eq!(resolved.value, SettingValue::Integer(9));
        assert!(!resolved.locked);
    }

    #[test]
    fn cfg_06_a_layer_the_setting_cannot_be_set_on_is_not_looked_at() {
        let (near, _, _) = ids();
        let definition = TEST_ORGANIZATION_ONLY.definition();
        let layers = layers_with(
            definition.key(),
            Some(("org", false)),
            &[(near, "x")],
            Some("y"),
        );
        let resolved = layers.resolve(definition);
        assert_eq!(
            (resolved.value, resolved.source),
            (text("org"), Source::Organization)
        );
    }

    #[test]
    fn cfg_06_without_shows_what_reset_to_default_brings_back() {
        let (near, _, person) = ids();
        let definition = TEST_PROJECT_NOTE.definition();
        let layers = layers_with(
            definition.key(),
            Some(("org", false)),
            &[(near, "near")],
            Some("mine"),
        );
        let back = |scope| layers.without(definition, scope);
        assert_eq!(back(Scope::Person(person)).value, text("near"));
        assert_eq!(back(Scope::Project(near)).value, text("mine"));
        assert_eq!(back(Scope::Organization).value, text("mine"));
        let only_org = layers_with(definition.key(), Some(("org", false)), &[], None);
        assert_eq!(
            only_org.without(definition, Scope::Organization).source,
            Source::Default
        );
    }

    #[test]
    fn a_scope_knows_its_level() {
        let (project, _, person) = ids();
        assert_eq!(Scope::Organization.level(), Level::Organization);
        assert_eq!(Scope::Project(project).level(), Level::Project);
        assert_eq!(Scope::Person(person).level(), Level::Person);
    }
}
