//! Settings in layers (CFG-06, CFG-04, ADR-0018): reading and changing the value of any setting of
//! the registry on the organization, on a project or for a person, with *Reset to default* and
//! the lock an administrator puts on a setting.
//!
//! One set of use cases serves every setting, so a new setting needs no code here. Which value
//! counts is decided by `bb_core::settings::Layers`; who may change what by the registry's
//! `changed_by` (ADR-0025, decision 14):
//!
//! | Where | Who may change it |
//! |-------|-------------------|
//! | The organization | *Change organization settings* for the whole organization |
//! | A project | *Change project settings* in that project (for a setting only administrators change, also *Change organization settings*) |
//! | A person | the person themselves, for a setting each person changes |
//!
//! Every change writes its domain event, and one of the settings that touch security (`security.*`,
//! `auth.*`) also a security-log entry. A value the organization locked wins over every lower
//! layer, and lower layers are refused while it is locked (`setting.locked`). Answers leave only as
//! `Authorized<T>`.

mod store;

use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::event::{EntityRef, FieldChange, RecordKind};
use bb_core::id::ProjectId;
use bb_core::settings::{
    ChangedBy, Layers, Level, OrganizationValue, Scope, SettingDefinition, SettingValue, Source,
    find, list_items, registry,
};
use bb_core::sign_in::SecurityAction;
use bb_policy::{Authorized, Evaluation, Permission, Place};
use serde_json::{Value, json};

pub use store::SettingsStore;

use crate::access::{Actor, ProjectLabel};
use crate::auth::{AppError, AuthStore, Ctx, Stops, by, change_failed, fixed_entity, internal};

/// Where the settings of a request are: the organization, a project, or the person who asks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingsScope {
    /// The whole organization (*Organization settings*).
    Organization,
    /// One project and its sub-projects (*Project settings*), by its ID.
    Project(ProjectId),
    /// One project and its sub-projects, by the address in the browser: its ID or its short name.
    ProjectAt(String),
    /// The person who asks (*My settings*).
    Person,
}

/// List the settings of one scope, with the value that counts and where it comes from.
pub struct SettingsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// Where the settings are.
    pub scope: SettingsScope,
    /// Only the settings whose key starts with one of these texts, separated by commas
    /// (`security.,auth.access-key-`): the settings of one page.
    pub topic: Option<String>,
    /// For a person's own settings: the project they work in (its ID or its short name), so the
    /// values of that project (and of the projects above it) count for them. The person must be
    /// able to see the project.
    pub project: Option<String>,
}

/// Change the value of a setting on one scope.
pub struct ChangeSetting {
    /// The secret of the session of the request.
    pub session_token: String,
    /// Where the value is set.
    pub scope: SettingsScope,
    /// The key of the setting.
    pub key: String,
    /// The new value.
    pub value: SettingValue,
}

/// *Reset to default*: take the value of a setting on one scope away, so the layer above counts
/// again. On the organization's scope a lock stays: unlocking is a separate action.
pub struct ResetSetting {
    /// The secret of the session of the request.
    pub session_token: String,
    /// Where the value is set.
    pub scope: SettingsScope,
    /// The key of the setting.
    pub key: String,
}

/// Lock the organization's value of a setting, so nobody lower down can choose another; or
/// unlock it.
pub struct LockSetting {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The key of the setting.
    pub key: String,
    /// `true` to lock, `false` to unlock.
    pub locked: bool,
}

/// Where the value that counts comes from, as the person who asks may know it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingSource {
    /// Nobody set it: the standard value.
    Default,
    /// The organization set it.
    Organization,
    /// A project set it; the label says which, or that the person cannot see it.
    Project(ProjectLabel),
    /// The person set it themselves.
    Person,
}

/// One setting with its value, as it is shown on a settings page.
#[derive(Clone, Debug)]
pub struct SettingView {
    /// What the registry knows about the setting: name, explanation, type, limits, levels.
    pub definition: &'static SettingDefinition,
    /// The value that counts on this scope.
    pub value: SettingValue,
    /// Where it comes from.
    pub source: SettingSource,
    /// Whether this scope has a value of its own, which *Reset to default* takes away. A lock
    /// that holds the standard value is no value of its own.
    pub own: bool,
    /// On the organization's scope: whether the value is locked, which administrators can change.
    /// On a lower scope: whether the organization's lock is in effect, so the value is read only.
    pub locked: bool,
}

/// The settings of one scope, in the order of the registry.
#[derive(Clone, Debug)]
pub struct SettingList {
    /// The settings.
    pub settings: Vec<SettingView>,
}

/// The ID that names the record an event is about: the same setting on the same scope is always
/// the same record.
fn record_of(scope: Scope, key: &str) -> String {
    match scope {
        Scope::Organization => key.to_owned(),
        Scope::Project(project) => format!("{project}/{key}"),
        Scope::Person(person) => format!("{person}/{key}"),
    }
}

/// The name of a field in an event: the key with underscores, like the other events.
fn field_of(key: &str) -> String {
    key.replace(['.', '-'], "_")
}

fn json_of(value: &SettingValue) -> Value {
    match value {
        SettingValue::Bool(value) => json!(value),
        SettingValue::Integer(value) => json!(value),
        SettingValue::Text(value) => json!(value),
    }
}

/// The setting with this key, or what the person is told about a key that does not exist.
fn definition_of(key: &str) -> Result<&'static SettingDefinition, AppError> {
    find(key).ok_or_else(|| ErrorCode::NotFound.into())
}

/// The value the organization holds as its own for a setting: one that is not just the standard
/// value held by a lock.
fn organization_value<'a>(
    layers: &'a Layers,
    definition: &SettingDefinition,
) -> Option<&'a OrganizationValue> {
    layers
        .organization
        .get(definition.key())
        .filter(|set| set.value != definition.default().to_value())
}

/// The value a scope holds as its own, as it was before a change.
fn own_value(layers: &Layers, scope: Scope, definition: &SettingDefinition) -> Option<Value> {
    let key = definition.key();
    match scope {
        Scope::Organization => {
            organization_value(layers, definition).map(|set| json_of(&set.value))
        }
        Scope::Project(project) => layers
            .projects
            .iter()
            .find(|(id, _)| *id == project)
            .and_then(|(_, values)| values.get(key))
            .map(json_of),
        Scope::Person(_) => layers.person.get(key).map(json_of),
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The project an address names (an ID, or a short name, also an old one). A project that is
    /// not there answers like one the person may not know about.
    async fn project_named(&self, address: &str) -> Result<ProjectId, AppError> {
        self.find_project(address)
            .await?
            .map(|found| found.project.id)
            .ok_or_else(|| ErrorCode::NotFound.into())
    }

    /// The scope a request names, as a stored scope: a project by its address is found, and the
    /// person's own scope is the actor's.
    async fn scope_of(&self, actor: &Actor, scope: &SettingsScope) -> Result<Scope, AppError> {
        Ok(match scope {
            SettingsScope::Organization => Scope::Organization,
            SettingsScope::Project(project) => Scope::Project(*project),
            SettingsScope::ProjectAt(address) => Scope::Project(self.project_named(address).await?),
            SettingsScope::Person => Scope::Person(actor.person.id),
        })
    }

    /// The evaluation that lets the actor change `definition` on `scope`: *Change organization
    /// settings* for the organization, *Change project settings* in the project (for a setting
    /// only administrators change, also the organization's), and nothing but being the person for
    /// a setting each person changes.
    async fn may_change(
        &self,
        actor: &Actor,
        scope: Scope,
        definition: &SettingDefinition,
    ) -> Result<Arc<Evaluation>, AppError> {
        // A project the person may not know about answers like one that does not exist, before
        // anything is said about the setting.
        let evaluation = match scope {
            Scope::Organization => {
                self.require(
                    actor,
                    Permission::ChangeOrganizationSettings,
                    Place::Organization,
                )
                .await?
            }
            Scope::Project(project) => {
                let held = self
                    .require(
                        actor,
                        Permission::ChangeProjectSettings,
                        Place::Project(project),
                    )
                    .await?;
                if definition.changed_by() == ChangedBy::Administrators {
                    self.require(
                        actor,
                        Permission::ChangeOrganizationSettings,
                        Place::Organization,
                    )
                    .await?;
                }
                held
            }
            Scope::Person(_) => {
                if !definition.levels().contains(&Level::Person) {
                    return Err(ErrorCode::InvalidRequest.into());
                }
                if definition.changed_by() != ChangedBy::EachPerson {
                    return Err(ErrorCode::Forbidden.into());
                }
                // Changing one's own settings needs no permission, only an actor that may change
                // things at all: a key that may only look reads them and writes nothing.
                if actor
                    .key
                    .as_ref()
                    .is_some_and(|key| !key.narrowing.may_change())
                {
                    return Err(ErrorCode::Forbidden.into());
                }
                self.evaluation_of(actor, Place::Organization).await?
            }
        };
        if !definition.levels().contains(&scope.level()) {
            return Err(ErrorCode::InvalidRequest.into());
        }
        Ok(evaluation)
    }

    /// The layers a view of `scope` needs; for a person's own settings, in a project if one is
    /// named.
    async fn layers_of(
        &self,
        scope: Scope,
        working_in: Option<ProjectId>,
    ) -> Result<Layers, AppError> {
        let (project, person) = match scope {
            Scope::Organization => (None, None),
            Scope::Project(project) => (Some(project), None),
            Scope::Person(person) => (working_in, Some(person)),
        };
        self.store()
            .setting_layers(project, person)
            .await
            .map_err(|error| internal::<S>(&error))
    }

    /// How one setting looks on `scope`, given what is stored.
    async fn view_of(
        &self,
        actor: &Actor,
        scope: Scope,
        definition: &'static SettingDefinition,
        layers: &Layers,
    ) -> Result<SettingView, AppError> {
        let key = definition.key();
        // The organization's own scope looks at the organization's value only.
        let only_organization;
        let looked_at = if scope == Scope::Organization {
            only_organization = Layers {
                organization: layers.organization.clone(),
                ..Layers::default()
            };
            &only_organization
        } else {
            layers
        };
        let resolved = looked_at.resolve(definition);
        let own = own_value(layers, scope, definition).is_some();
        let source = match resolved.source {
            Source::Default => SettingSource::Default,
            Source::Organization => SettingSource::Organization,
            Source::Person => SettingSource::Person,
            Source::Project(project) => {
                SettingSource::Project(self.project_label(actor, project).await?)
            }
        };
        let locked = match scope {
            Scope::Organization => layers
                .organization
                .get(key)
                .is_some_and(|set| set.locked && definition.lockable()),
            Scope::Project(_) | Scope::Person(_) => resolved.locked,
        };
        Ok(SettingView {
            definition,
            value: resolved.value,
            source,
            own,
            locked,
        })
    }

    /// A change of a setting that touches security also goes into the security log (SEC-15).
    async fn note_security(
        &self,
        actor: &Actor,
        definition: &SettingDefinition,
        action: SecurityAction,
        subject: EntityRef,
    ) {
        if definition.touches_security() {
            self.note(action, Some(actor.person.id), Some(subject))
                .await;
        }
    }

    /// The settings of `scope` as they are now, after a change.
    async fn view_after(
        &self,
        actor: &Actor,
        scope: Scope,
        definition: &'static SettingDefinition,
    ) -> Result<SettingView, AppError> {
        let layers = self.layers_of(scope, None).await?;
        self.view_of(actor, scope, definition, &layers).await
    }
}

/// The settings of the organization, of a project or of the person who asks.
///
/// # Errors
/// `access.forbidden` without *Change organization settings* (organization) or *Change project
/// settings* (project); `resource.not_found` for a project the person may not know about.
pub async fn settings<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SettingsQuery,
) -> Result<Authorized<SettingList>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let scope = ctx.scope_of(&actor, &input.scope).await?;
    let working_in = match (&input.project, scope) {
        (Some(address), Scope::Person(_)) => Some(ctx.project_named(address).await?),
        _ => None,
    };
    let evaluation = match scope {
        Scope::Organization => {
            ctx.require(
                &actor,
                Permission::ChangeOrganizationSettings,
                Place::Organization,
            )
            .await?
        }
        Scope::Project(project) => {
            ctx.require(
                &actor,
                Permission::ChangeProjectSettings,
                Place::Project(project),
            )
            .await?
        }
        Scope::Person(_) => {
            if let Some(project) = working_in {
                // The project must be one the person may see; any other answers like a missing one.
                ctx.require(&actor, Permission::SeeProjects, Place::Project(project))
                    .await?;
            }
            ctx.evaluation_of(&actor, Place::Organization).await?
        }
    };
    let layers = ctx.layers_of(scope, working_in).await?;
    let mut settings = Vec::new();
    for definition in registry().filter(|definition| {
        definition.levels().contains(&scope.level())
            && input.topic.as_deref().is_none_or(|topic| {
                list_items(topic).any(|start| definition.key().starts_with(start))
            })
    }) {
        settings.push(ctx.view_of(&actor, scope, definition, &layers).await?);
    }
    Ok(evaluation.authorize(SettingList { settings }))
}

/// Changes the value of a setting on one scope.
///
/// # Errors
/// `resource.not_found` for a key that does not exist (or a project the person may not know
/// about), `access.forbidden` without the right to change it there, `request.invalid` for a
/// value the setting does not accept or a scope it cannot be set on, `setting.locked` while the
/// organization's lock holds for a project or a person.
pub async fn change_setting<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangeSetting,
) -> Result<Authorized<SettingView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let definition = definition_of(&input.key)?;
    let scope = ctx.scope_of(&actor, &input.scope).await?;
    let evaluation = ctx.may_change(&actor, scope, definition).await?;
    if !definition.has_type_of(&input.value) || !definition.accepts(&input.value) {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let layers = ctx.layers_of(scope, None).await?;
    if scope != Scope::Organization && layers.resolve(definition).locked {
        return Err(ErrorCode::SettingLocked.into());
    }
    let entity = fixed_entity(RecordKind::Setting, &record_of(scope, definition.key()))?;
    let event = ctx.event(
        by(actor.person.id),
        entity.clone(),
        vec![FieldChange {
            field: field_of(definition.key()),
            before: own_value(&layers, scope, definition),
            after: Some(json_of(&input.value)),
        }],
    )?;
    let (store_key, value, now) = (definition.key(), input.value, ctx.now());
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::put_setting_value(transaction, scope, store_key, &value, now).await
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::both(ErrorCode::SettingLocked)))?;
    ctx.note_security(&actor, definition, SecurityAction::SettingChanged, entity)
        .await;
    let view = ctx.view_after(&actor, scope, definition).await?;
    Ok(evaluation.authorize(view))
}

/// *Reset to default*: takes the value of a setting on one scope away. A scope that has no value
/// of its own is left alone, and no event is written. On the organization's scope a lock stays:
/// the standard value is stored, still locked, and unlocking is [`lock_setting`].
///
/// # Errors
/// The same as [`change_setting`], except that a locked setting can be reset: the value that is
/// taken away counted for nothing below the organization.
pub async fn reset_setting<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ResetSetting,
) -> Result<Authorized<SettingView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let definition = definition_of(&input.key)?;
    let scope = ctx.scope_of(&actor, &input.scope).await?;
    let evaluation = ctx.may_change(&actor, scope, definition).await?;
    let layers = ctx.layers_of(scope, None).await?;
    let view = ctx.view_of(&actor, scope, definition, &layers).await?;
    if !view.own {
        return Ok(evaluation.authorize(view));
    }
    let keeps_the_lock = scope == Scope::Organization && view.locked;
    let standard = definition.default().to_value();
    let entity = fixed_entity(RecordKind::Setting, &record_of(scope, definition.key()))?;
    let event = ctx.event(
        by(actor.person.id),
        entity.clone(),
        vec![FieldChange {
            field: field_of(definition.key()),
            before: own_value(&layers, scope, definition),
            after: keeps_the_lock.then(|| json_of(&standard)),
        }],
    )?;
    let (store_key, now) = (definition.key(), ctx.now());
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if keeps_the_lock {
                    S::put_setting_value(transaction, scope, store_key, &standard, now).await
                } else {
                    S::clear_setting_value(transaction, scope, store_key)
                        .await
                        .map(|_| ())
                }
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    ctx.note_security(&actor, definition, SecurityAction::SettingReset, entity)
        .await;
    let view = ctx.view_after(&actor, scope, definition).await?;
    Ok(evaluation.authorize(view))
}

/// Locks the organization's value of a setting, or unlocks it. A setting that has no value of the
/// organization is locked at its standard value, which is not a value of its own (*Reset to
/// default* still has nothing to take away); unlocking one that is not locked changes nothing.
///
/// # Errors
/// `access.forbidden` without *Change organization settings*, `request.invalid` for a setting
/// that cannot be locked, `resource.not_found` for a key that does not exist.
pub async fn lock_setting<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: LockSetting,
) -> Result<Authorized<SettingView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let definition = definition_of(&input.key)?;
    let evaluation = ctx
        .may_change(&actor, Scope::Organization, definition)
        .await?;
    if !definition.lockable() {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let layers = ctx.layers_of(Scope::Organization, None).await?;
    let was = layers
        .organization
        .get(definition.key())
        .is_some_and(|set| set.locked);
    if was == input.locked {
        let view = ctx
            .view_of(&actor, Scope::Organization, definition, &layers)
            .await?;
        return Ok(evaluation.authorize(view));
    }
    let entity = fixed_entity(
        RecordKind::Setting,
        &record_of(Scope::Organization, definition.key()),
    )?;
    let event = ctx.event(
        by(actor.person.id),
        entity.clone(),
        vec![FieldChange {
            field: format!("{}_locked", field_of(definition.key())),
            before: Some(json!(was)),
            after: Some(json!(input.locked)),
        }],
    )?;
    let (store_key, locked, now) = (definition.key(), input.locked, ctx.now());
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move { S::put_setting_lock(transaction, store_key, locked, now).await })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    let action = if locked {
        SecurityAction::SettingLocked
    } else {
        SecurityAction::SettingUnlocked
    };
    ctx.note_security(&actor, definition, action, entity).await;
    let view = ctx
        .view_after(&actor, Scope::Organization, definition)
        .await?;
    Ok(evaluation.authorize(view))
}
