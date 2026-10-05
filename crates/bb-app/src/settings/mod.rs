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

use bb_core::ErrorCode;
use bb_core::event::{EntityRef, FieldChange};
use bb_core::id::ProjectId;
use bb_core::settings::{
    ChangedBy, Layers, Level, Scope, SettingDefinition, SettingValue, Source, find, registry,
};
use bb_core::sign_in::SecurityAction;
use bb_policy::{Authorized, Evaluation, Permission, Place};
use serde_json::{Value, json};

pub use store::SettingsStore;

use crate::access::{Actor, ProjectLabel};
use crate::auth::{AppError, AuthStore, Ctx, by, change_failed, fixed_entity, internal};

/// Where the settings of a request are: the organization, a project, or the person who asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsScope {
    /// The whole organization (*Organization settings*).
    Organization,
    /// One project and its sub-projects (*Project settings*).
    Project(ProjectId),
    /// The person who asks (*My settings*).
    Person,
}

/// List the settings of one scope, with the value that counts and where it comes from.
pub struct SettingsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// Where the settings are.
    pub scope: SettingsScope,
    /// Only the settings whose key starts with this text (`security.`, `auth.access-key-`).
    pub topic: Option<String>,
    /// For a person's own settings: the project they work in, so the values of that project (and
    /// of the projects above it) count for them. The person must be able to see the project.
    pub project: Option<ProjectId>,
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
/// again.
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
    /// Whether this scope has a value of its own, which *Reset to default* takes away.
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

impl SettingsScope {
    /// The scope of a stored value for the person who asks.
    const fn stored(self, actor: &Actor) -> Scope {
        match self {
            Self::Organization => Scope::Organization,
            Self::Project(project) => Scope::Project(project),
            Self::Person => Scope::Person(actor.person.id),
        }
    }
}

/// The setting with this key, or what the person is told about a key that does not exist.
fn definition_of(key: &str) -> Result<&'static SettingDefinition, AppError> {
    find(key).ok_or_else(|| ErrorCode::NotFound.into())
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The evaluation that lets the actor change `definition` on `scope`: *Change organization
    /// settings* for the organization, *Change project settings* in the project (for a setting
    /// only administrators change, also the organization's), and nothing but being the person for
    /// a setting each person changes.
    async fn may_change(
        &self,
        actor: &Actor,
        scope: SettingsScope,
        definition: &SettingDefinition,
    ) -> Result<std::sync::Arc<Evaluation>, AppError> {
        let level = match scope {
            SettingsScope::Organization => Level::Organization,
            SettingsScope::Project(_) => Level::Project,
            SettingsScope::Person => Level::Person,
        };
        // A project the person may not know about answers like one that does not exist, before
        // anything is said about the setting.
        let evaluation = match scope {
            SettingsScope::Organization => {
                self.require(
                    actor,
                    Permission::ChangeOrganizationSettings,
                    Place::Organization,
                )
                .await?
            }
            SettingsScope::Project(project) => {
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
            SettingsScope::Person => {
                if !definition.levels().contains(&Level::Person) {
                    return Err(ErrorCode::InvalidRequest.into());
                }
                if definition.changed_by() != ChangedBy::EachPerson {
                    return Err(ErrorCode::Forbidden.into());
                }
                self.evaluate(actor.person.id, Place::Organization).await?
            }
        };
        if !definition.levels().contains(&level) {
            return Err(ErrorCode::InvalidRequest.into());
        }
        Ok(evaluation)
    }

    /// The layers a view of `scope` needs; for a person's own settings, in a project if one is
    /// named.
    async fn layers_of(
        &self,
        actor: &Actor,
        scope: SettingsScope,
        working_in: Option<ProjectId>,
    ) -> Result<Layers, AppError> {
        let (project, person) = match scope {
            SettingsScope::Organization => (None, None),
            SettingsScope::Project(project) => (Some(project), None),
            SettingsScope::Person => (working_in, Some(actor.person.id)),
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
        scope: SettingsScope,
        definition: &'static SettingDefinition,
        layers: &Layers,
    ) -> Result<SettingView, AppError> {
        let key = definition.key();
        // The organization's own scope looks at the organization's value only.
        let only_organization;
        let looked_at = if scope == SettingsScope::Organization {
            only_organization = Layers {
                organization: layers.organization.clone(),
                ..Layers::default()
            };
            &only_organization
        } else {
            layers
        };
        let resolved = looked_at.resolve(definition);
        let own = match scope {
            SettingsScope::Organization => layers.organization.contains_key(key),
            SettingsScope::Project(project) => layers
                .projects
                .iter()
                .any(|(id, values)| *id == project && values.contains_key(key)),
            SettingsScope::Person => layers.person.contains_key(key),
        };
        let source = match resolved.source {
            Source::Default => SettingSource::Default,
            Source::Organization => SettingSource::Organization,
            Source::Person => SettingSource::Person,
            Source::Project(project) => {
                SettingSource::Project(self.project_label(actor, project).await?)
            }
        };
        let locked = match scope {
            SettingsScope::Organization => layers
                .organization
                .get(key)
                .is_some_and(|set| set.locked && definition.lockable()),
            SettingsScope::Project(_) | SettingsScope::Person => resolved.locked,
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
        scope: SettingsScope,
        definition: &'static SettingDefinition,
    ) -> Result<SettingView, AppError> {
        let layers = self.layers_of(actor, scope, None).await?;
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
    let evaluation = match input.scope {
        SettingsScope::Organization => {
            ctx.require(
                &actor,
                Permission::ChangeOrganizationSettings,
                Place::Organization,
            )
            .await?
        }
        SettingsScope::Project(project) => {
            ctx.require(
                &actor,
                Permission::ChangeProjectSettings,
                Place::Project(project),
            )
            .await?
        }
        SettingsScope::Person => {
            if let Some(project) = input.project {
                // The project must be one the person may see; any other answers like a missing one.
                ctx.require(&actor, Permission::SeeProjects, Place::Project(project))
                    .await?;
            }
            ctx.evaluate(actor.person.id, Place::Organization).await?
        }
    };
    let level = match input.scope {
        SettingsScope::Organization => Level::Organization,
        SettingsScope::Project(_) => Level::Project,
        SettingsScope::Person => Level::Person,
    };
    let layers = ctx.layers_of(&actor, input.scope, input.project).await?;
    let mut settings = Vec::new();
    for definition in registry().filter(|definition| {
        definition.levels().contains(&level)
            && input
                .topic
                .as_deref()
                .is_none_or(|topic| definition.key().starts_with(topic))
    }) {
        settings.push(
            ctx.view_of(&actor, input.scope, definition, &layers)
                .await?,
        );
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
    let evaluation = ctx.may_change(&actor, input.scope, definition).await?;
    if !definition.has_type_of(&input.value) || !definition.accepts(&input.value) {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let scope = input.scope.stored(&actor);
    let layers = ctx.layers_of(&actor, input.scope, None).await?;
    if scope != Scope::Organization && layers.resolve(definition).locked {
        return Err(ErrorCode::SettingLocked.into());
    }
    let before = match scope {
        Scope::Organization => layers
            .organization
            .get(definition.key())
            .map(|set| json_of(&set.value)),
        Scope::Project(project) => layers
            .projects
            .iter()
            .find(|(id, _)| *id == project)
            .and_then(|(_, values)| values.get(definition.key()))
            .map(json_of),
        Scope::Person(_) => layers.person.get(definition.key()).map(json_of),
    };
    let entity = fixed_entity("setting", &record_of(scope, definition.key()))?;
    let event = ctx.event(
        by(actor.person.id),
        entity.clone(),
        vec![FieldChange {
            field: field_of(definition.key()),
            before,
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
        .map_err(|error| change_failed::<S>(&error, Some(ErrorCode::SettingLocked)))?;
    ctx.note_security(&actor, definition, SecurityAction::SettingChanged, entity)
        .await;
    let view = ctx.view_after(&actor, input.scope, definition).await?;
    Ok(evaluation.authorize(view))
}

/// *Reset to default*: takes the value of a setting on one scope away. A scope that has no value
/// of its own is left alone, and no event is written.
///
/// # Errors
/// The same as [`change_setting`], except that a locked setting can be reset: the value that is
/// taken away counted for nothing.
pub async fn reset_setting<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ResetSetting,
) -> Result<Authorized<SettingView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let definition = definition_of(&input.key)?;
    let evaluation = ctx.may_change(&actor, input.scope, definition).await?;
    let scope = input.scope.stored(&actor);
    let layers = ctx.layers_of(&actor, input.scope, None).await?;
    let view = ctx
        .view_of(&actor, input.scope, definition, &layers)
        .await?;
    if !view.own {
        return Ok(evaluation.authorize(view));
    }
    let before = Some(match scope {
        Scope::Organization => layers
            .organization
            .get(definition.key())
            .map_or(Value::Null, |set| json_of(&set.value)),
        Scope::Project(_) | Scope::Person(_) => json_of(&view.value),
    });
    let entity = fixed_entity("setting", &record_of(scope, definition.key()))?;
    let event = ctx.event(
        by(actor.person.id),
        entity.clone(),
        vec![FieldChange {
            field: field_of(definition.key()),
            before,
            after: None,
        }],
    )?;
    let store_key = definition.key();
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::clear_setting_value(transaction, scope, store_key)
                    .await
                    .map(|_| ())
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, None))?;
    ctx.note_security(&actor, definition, SecurityAction::SettingReset, entity)
        .await;
    let view = ctx.view_after(&actor, input.scope, definition).await?;
    Ok(evaluation.authorize(view))
}

/// Locks the organization's value of a setting, or unlocks it. Locking a setting that has no value
/// of the organization yet locks its standard value.
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
        .may_change(&actor, SettingsScope::Organization, definition)
        .await?;
    if !definition.lockable() {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let layers = ctx
        .layers_of(&actor, SettingsScope::Organization, None)
        .await?;
    let was = layers
        .organization
        .get(definition.key())
        .is_some_and(|set| set.locked);
    let entity = fixed_entity("setting", &record_of(Scope::Organization, definition.key()))?;
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
        .map_err(|error| change_failed::<S>(&error, None))?;
    let action = if locked {
        SecurityAction::SettingLocked
    } else {
        SecurityAction::SettingUnlocked
    };
    ctx.note_security(&actor, definition, action, entity).await;
    let view = ctx
        .view_after(&actor, SettingsScope::Organization, definition)
        .await?;
    Ok(evaluation.authorize(view))
}
