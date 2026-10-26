//! Making and changing an own field (CF-01, CF-03, CF-06): the definition, its rules, its look and
//! where it is used. The choices of a field are in `choices.rs`.
//!
//! Changing fields needs *Manage fields* for the whole organization (M3-Q6). A field is never
//! deleted: it is retired, so no new values are set while the old ones stay. A name is used once,
//! without regard to capitals; that is judged inside the change, after the policy lock, so two
//! people who make the same name at the same moment cannot both succeed.
//!
//! A new field starts with a level for every role (*Can see* for each role that sees projects,
//! *Can change* for those that change issues), so whoever makes it can change that at once
//! (architecture §7, *The field catalogue*).

use bb_core::ErrorCode;
use bb_core::access::FieldLevel;
use bb_core::event::{FieldChange, RecordKind};
use bb_core::fields::FieldKey;
use bb_core::id::{FieldId, RoleId};
use bb_core::names::{is_new_or_changed, name_among};
use bb_core::own_fields::{
    CHOICES_MAX, ChoiceColor, Edit, FIELDS_MAX, FieldChoice, FieldFor, FieldType, HELP_MAX, Look,
    OwnField, Rules, RulesRefused, Scope,
};
use bb_core::own_fields_value::FieldValue;
use bb_core::version::{FIRST_VERSION, counted_up, next};
use bb_policy::{Authorized, Permission, Place, starting_own_field_level};
use serde_json::{Value, json};

use super::member;
use super::pattern::{Linear, is_usable};
use super::types::{ChangeField, CreateField, Definition, FieldView, NewChoice};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, by, entity, internal, set_to};
use crate::input::{clean_name, clean_text};
use crate::projects::Why;

/// The help text as it is kept: trimmed, at most [`HELP_MAX`] characters, no control characters;
/// it may be empty.
fn clean_help(help: Option<&str>) -> Result<String, AppError> {
    match help.map(str::trim) {
        None | Some("") => Ok(String::new()),
        Some(text) => clean_text(text, HELP_MAX),
    }
}

/// Whether another field than `except` already has the name (`bb_core::names`), or the name would
/// make a word in the search language that a built-in field or another field already has.
fn name_taken(definitions: &[Definition], name: &str, except: Option<FieldId>) -> bool {
    let others = || {
        definitions
            .iter()
            .filter(|found| Some(found.field.id) != except)
            .map(|found| found.field.name.as_str())
    };
    name_among(others(), name) || crate::search::word_is_taken(name, others())
}

/// Checks that the rules make sense for the type and that the pattern can be checked.
fn check_rules(kind: FieldType, rules: &Rules) -> Result<(), AppError> {
    rules
        .check_definition(kind)
        .map_err(|refused| match refused {
            RulesRefused::NotForThisType
            | RulesRefused::Backwards
            | RulesRefused::PatternTooLong => AppError::Refused(ErrorCode::InvalidRequest),
        })?;
    if rules
        .pattern
        .as_deref()
        .is_some_and(|pattern| !is_usable(pattern))
    {
        return Err(ErrorCode::InvalidRequest.into());
    }
    Ok(())
}

/// Whether a value can still be the default of a field: it keeps the rules (but "required": a
/// default is what fills a required field) and, for choices, only offers choices that are not
/// retired.
pub(super) fn default_fits(
    kind: FieldType,
    rules: &Rules,
    choices: &[FieldChoice],
    value: &FieldValue,
) -> bool {
    let without_required = Rules {
        required: false,
        ..rules.clone()
    };
    without_required.check(kind, Some(value), &Linear).is_ok()
        && (!kind.has_choices()
            || value.ids().iter().all(|bytes| {
                choices
                    .iter()
                    .any(|choice| choice.retired_at.is_none() && choice.id.to_bytes() == *bytes)
            }))
}

/// The default a field starts new items with, if it still fits: a default that the rules or the
/// choices have left behind (a retired choice, a tighter range) is not used.
pub(super) fn current_default(definition: &Definition) -> Option<FieldValue> {
    let field = &definition.field;
    if field.default_value.is_empty() || !field.kind.takes_default() {
        return None;
    }
    let json = serde_json::from_str::<Value>(&field.default_value).ok()?;
    let value = FieldValue::from_json(field.kind, &json).ok().flatten()?;
    default_fits(field.kind, &field.rules, &definition.choices, &value).then_some(value)
}

/// Whether a new item must be asked for this field: it is required and has no default to start
/// with. The one judgement of the server (a new item without a value is refused), of the form a
/// person gets (the field stays on it) and of the "required" mark in the layout editor.
pub(crate) fn must_be_asked_for(definition: &Definition) -> bool {
    definition.field.rules.required && current_default(definition).is_none()
}

/// The default value as it is kept (the JSON of its type), checked against the rules and, for a
/// *Pick one* or *Pick several* field, against its choices. Empty text for no default.
fn default_text(
    kind: FieldType,
    rules: &Rules,
    choices: &[FieldChoice],
    default: Option<&Value>,
) -> Result<String, AppError> {
    let Some(json) = default.filter(|json| !json.is_null()) else {
        return Ok(String::new());
    };
    if !kind.takes_default() {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let value = FieldValue::from_json(kind, json)
        .map_err(|_| AppError::Refused(ErrorCode::InvalidRequest))?
        .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
    if !default_fits(kind, rules, choices, &value) {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let json = value
        .to_json(kind)
        .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
    Ok(json.to_string())
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Checks where a field is used: the projects and issue types are there (for the person: a type
    /// they may not know is as good as none, `offers/known.rs`), and a field for projects has no
    /// issue types.
    async fn check_scope(
        &self,
        actor: &Actor,
        applies_to: FieldFor,
        scope: &Scope,
    ) -> Result<Scope, AppError> {
        if applies_to == FieldFor::Projects && !scope.issue_types.is_empty() {
            return Err(ErrorCode::InvalidRequest.into());
        }
        let mut checked = Scope::default();
        for project in &scope.projects {
            if checked.projects.contains(project) {
                continue;
            }
            self.store()
                .project(*project)
                .await
                .map_err(|error| internal::<S>(&error))?
                .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
            checked.projects.push(*project);
        }
        let shown = self.shown_to(actor).await?;
        let types = self
            .store()
            .issue_types()
            .await
            .map_err(|error| internal::<S>(&error))?;
        let types = self.known_types(&shown, types).await?;
        for issue_type in &scope.issue_types {
            if checked.issue_types.contains(issue_type) {
                continue;
            }
            if !types.iter().any(|found| found.id == *issue_type) {
                return Err(ErrorCode::InvalidRequest.into());
            }
            checked.issue_types.push(*issue_type);
        }
        Ok(checked)
    }

    /// The choices a new field starts with: named, unique, colored in turn.
    pub(super) fn new_choices(
        &self,
        field: FieldId,
        choices: &[NewChoice],
    ) -> Result<Vec<FieldChoice>, AppError> {
        if choices.len() > CHOICES_MAX {
            return Err(ErrorCode::InvalidRequest.into());
        }
        let mut made: Vec<FieldChoice> = Vec::with_capacity(choices.len());
        for (position, choice) in (0_i64..).zip(choices) {
            let name = clean_name(&choice.name)?;
            if name_among(made.iter().map(|known| known.name.as_str()), &name) {
                return Err(ErrorCode::NameTaken.into());
            }
            made.push(FieldChoice {
                id: bb_core::id::ChoiceId::generate(&mut self.ids())
                    .map_err(|error| AppError::Internal(error.to_string()))?,
                field,
                name,
                color: choice
                    .color
                    .unwrap_or_else(|| ChoiceColor::nth(usize::try_from(position).unwrap_or(0))),
                icon: choice.icon,
                position,
                retired_at: None,
            });
        }
        Ok(made)
    }

    /// The level each role starts with for a new field: the one the starting roles have, found
    /// from what the role allows (architecture §7).
    async fn starting_levels(
        &self,
        applies_to: FieldFor,
    ) -> Result<Vec<(RoleId, FieldLevel)>, AppError> {
        let changes_with = match applies_to {
            FieldFor::Issues => Permission::ChangeIssues,
            FieldFor::Projects => Permission::ChangeProjectSettings,
        };
        Ok(self
            .store()
            .roles()
            .await
            .map_err(|error| internal::<S>(&error))?
            .iter()
            .map(|record| {
                (
                    record.role.id,
                    starting_own_field_level(&record.permissions, changes_with),
                )
            })
            .collect())
    }
}

/// Creates an own field, at the end of the order, with its choices and the level every role starts
/// with.
///
/// # Errors
/// `access.forbidden` without *Manage fields*, `request.invalid` for a name, help text, rule,
/// default, look, scope or choice that cannot be, `name.taken` for a name another field (or two
/// choices) has.
pub async fn create_field<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CreateField,
) -> Result<Authorized<FieldView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx
        .require(&actor, Permission::ManageFields, Place::Organization)
        .await?;
    let name = clean_name(&input.name)?;
    let help_text = clean_help(input.help.as_deref())?;
    let kind = input.kind;
    // A long text is saved under an edit lock of an issue; projects have no such route yet (M3-F49).
    if kind.is_long_text() && input.applies_to != FieldFor::Issues {
        return Err(ErrorCode::InvalidRequest.into());
    }
    check_rules(kind, &input.rules)?;
    let look = input.look.unwrap_or_else(|| Look::starting(kind));
    if !look.fits(kind) || (!kind.has_choices() && !input.choices.is_empty()) {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let scope = ctx
        .check_scope(&actor, input.applies_to, &input.scope)
        .await?;
    let id =
        FieldId::generate(&mut ctx.ids()).map_err(|error| AppError::Internal(error.to_string()))?;
    let choices = ctx.new_choices(id, &input.choices)?;
    let default_value = default_text(kind, &input.rules, &choices, input.default_value.as_ref())?;
    let definitions = ctx.field_catalogue().await?;
    if name_taken(&definitions, &name, None) {
        return Err(ErrorCode::NameTaken.into());
    }
    if definitions.len() >= FIELDS_MAX {
        return Err(ErrorCode::LimitReached.into());
    }
    let definition = Definition {
        field: OwnField {
            id,
            name,
            help: help_text,
            kind,
            applies_to: input.applies_to,
            rules: input.rules,
            default_value,
            look,
            never_to_assistant: input.never_to_assistant,
            position: definitions
                .iter()
                .map(|found| found.field.position)
                .max()
                .map_or(0, |last| last + 1),
            retired_at: None,
            version: FIRST_VERSION,
        },
        scope,
        choices,
    };
    let changes = made_changes(&definition);
    let event = ctx.event(by(actor.person.id), entity(RecordKind::Field, id), changes)?;
    let levels = ctx.starting_levels(input.applies_to).await?;
    let why = Why::default();
    let (stopped, stored) = (why.clone(), definition.clone());
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_policy(transaction).await?;
                let known = S::field_definitions_in(transaction).await?;
                if name_taken(&known, &stored.field.name, None) {
                    return Err(stopped.stop::<S>(ErrorCode::NameTaken));
                }
                S::insert_field(transaction, &stored).await?;
                let key = FieldKey::Own(stored.field.id);
                for (role, level) in &levels {
                    S::put_level(transaction, *role, None, None, key, Some(*level)).await?;
                }
                Ok(())
            })
        })
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::NameTaken))?;
    let view = ctx.field_shown(&actor, &held, &definition).await?;
    Ok(held.authorize(view))
}

/// What a new field says in its event.
fn made_changes(definition: &Definition) -> Vec<FieldChange> {
    let field = &definition.field;
    let mut changes = vec![
        set_to(member::NAME, json!(field.name)),
        set_to(member::KIND, json!(field.kind.code())),
        set_to(member::APPLIES_TO, json!(field.applies_to.code())),
        set_to(member::RULES, json!(field.rules)),
        set_to(member::LOOK, json!(field.look.code())),
        set_to(member::NEVER_TO_ASSISTANT, json!(field.never_to_assistant)),
    ];
    if !field.help.is_empty() {
        changes.push(set_to(member::HELP, json!(field.help)));
    }
    if !field.default_value.is_empty() {
        changes.push(set_to(
            member::DEFAULT,
            serde_json::from_str(&field.default_value).unwrap_or(Value::Null),
        ));
    }
    if !definition.scope.projects.is_empty() {
        changes.push(set_to(member::PROJECTS, json!(definition.scope.projects)));
    }
    if !definition.scope.issue_types.is_empty() {
        changes.push(set_to(
            member::ISSUE_TYPES,
            json!(definition.scope.issue_types),
        ));
    }
    if !definition.choices.is_empty() {
        changes.push(set_to(
            member::CHOICES,
            json!(
                definition
                    .choices
                    .iter()
                    .map(|choice| choice.name.as_str())
                    .collect::<Vec<_>>()
            ),
        ));
    }
    changes
}

/// What a change of a field changes, before and after, for the event.
fn differences(before: &Definition, after: &Definition) -> Vec<FieldChange> {
    let mut changes = Vec::new();
    let mut add = |name: &str, was: Value, now: Value| {
        if was != now {
            changes.push(FieldChange {
                field: name.to_owned(),
                before: Some(was),
                after: Some(now),
            });
        }
    };
    let (old, new) = (&before.field, &after.field);
    add(member::NAME, json!(old.name), json!(new.name));
    add(member::HELP, json!(old.help), json!(new.help));
    add(member::RULES, json!(old.rules), json!(new.rules));
    add(
        member::DEFAULT,
        json!(old.default_value),
        json!(new.default_value),
    );
    add(member::LOOK, json!(old.look.code()), json!(new.look.code()));
    add(
        member::NEVER_TO_ASSISTANT,
        json!(old.never_to_assistant),
        json!(new.never_to_assistant),
    );
    add(
        member::RETIRED,
        json!(old.retired_at.is_some()),
        json!(new.retired_at.is_some()),
    );
    add(
        member::PROJECTS,
        json!(before.scope.projects),
        json!(after.scope.projects),
    );
    add(
        member::ISSUE_TYPES,
        json!(before.scope.issue_types),
        json!(after.scope.issue_types),
    );
    changes
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The field with what the request changes applied, after every part has been checked.
    async fn changed_definition(
        &self,
        actor: &Actor,
        input: &ChangeField,
        current: &Definition,
        definitions: &[Definition],
    ) -> Result<Definition, AppError> {
        let kind = current.field.kind;
        let mut after = current.clone();
        if let Some(name) = &input.name {
            after.field.name = clean_name(name)?;
            if is_new_or_changed(Some(&current.field.name), &after.field.name)
                && name_taken(definitions, &after.field.name, Some(after.field.id))
            {
                return Err(ErrorCode::NameTaken.into());
            }
        }
        if input.help.is_some() {
            after.field.help = clean_help(input.help.as_deref())?;
        }
        if let Some(rules) = &input.rules {
            check_rules(kind, rules)?;
            after.field.rules = rules.clone();
            // A default that the new rules no longer allow is taken away.
            if input.default_value.is_keep() && current_default(&after).is_none() {
                after.field.default_value = String::new();
            }
        }
        if !input.default_value.is_keep() {
            let default = match &input.default_value {
                Edit::Set(value) => Some(value),
                Edit::Keep | Edit::Clear => None,
            };
            after.field.default_value =
                default_text(kind, &after.field.rules, &after.choices, default)?;
        }
        if let Some(look) = input.look {
            if !look.fits(kind) {
                return Err(ErrorCode::InvalidRequest.into());
            }
            after.field.look = look;
        }
        if let Some(never) = input.never_to_assistant {
            after.field.never_to_assistant = never;
        }
        if let Some(scope) = &input.scope {
            after.scope = self
                .check_scope(actor, current.field.applies_to, scope)
                .await?;
        }
        if let Some(retired) = input.retired {
            after.field.retired_at = if retired {
                current.field.retired_at.or(Some(self.now()))
            } else {
                None
            };
        }
        Ok(after)
    }
}

/// Changes an own field: what is `Some` is changed. The type and what the field is for never
/// change.
///
/// # Errors
/// `access.forbidden` without *Manage fields*, `resource.not-found` for a field that is not
/// there, `request.invalid` for nothing asked or something that cannot be, `name.taken` for a name
/// another field has, `edit.version-changed` when somebody changed the field since it was looked
/// at.
pub async fn change_field<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangeField,
) -> Result<Authorized<FieldView>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let held = ctx
        .require(&actor, Permission::ManageFields, Place::Organization)
        .await?;
    if input.name.is_none()
        && input.help.is_none()
        && input.rules.is_none()
        && input.default_value.is_keep()
        && input.look.is_none()
        && input.never_to_assistant.is_none()
        && input.scope.is_none()
        && input.retired.is_none()
    {
        return Err(ErrorCode::InvalidRequest.into());
    }
    let definitions = ctx.field_catalogue().await?;
    let current = definitions
        .iter()
        .find(|found| found.field.id == input.field)
        .cloned()
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    let record = entity(RecordKind::Field, current.field.id);
    if current.field.version != input.version {
        return Err(AppError::stale(
            record,
            Place::Organization,
            actor.person.id,
        ));
    }
    let mut after = ctx
        .changed_definition(&actor, &input, &current, &definitions)
        .await?;
    let mut changes = differences(&current, &after);
    if changes.is_empty() {
        let view = ctx.field_shown(&actor, &held, &current).await?;
        return Ok(held.authorize(view));
    }
    changes.push(counted_up(current.field.version));
    let event = ctx.event(by(actor.person.id), record.clone(), changes)?;
    let why = Why::default();
    let (stopped, written, version) = (why.clone(), after.clone(), current.field.version);
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                S::lock_policy(transaction).await?;
                let known = S::field_definitions_in(transaction).await?;
                let renamed = is_new_or_changed(
                    known
                        .iter()
                        .find(|stored| stored.field.id == written.field.id)
                        .map(|stored| stored.field.name.as_str()),
                    &written.field.name,
                );
                if renamed && name_taken(&known, &written.field.name, Some(written.field.id)) {
                    return Err(stopped.stop::<S>(ErrorCode::NameTaken));
                }
                if S::update_field(transaction, &written.field, &written.scope, version).await? {
                    Ok(())
                } else {
                    Err(stopped.stop::<S>(ErrorCode::VersionChanged))
                }
            })
        })
        .await
        .map_err(|error| {
            why.answer::<S>(&error, ErrorCode::VersionChanged).about(
                &record,
                Place::Organization,
                actor.person.id,
            )
        })?;
    after.field.version = next(version);
    let view = ctx.field_shown(&actor, &held, &after).await?;
    Ok(held.authorize(view))
}
