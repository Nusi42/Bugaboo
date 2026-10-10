//! The values items hold for own fields (CF-01, CF-03, CF-06, CF-08): reading them for a view,
//! and planning the values a request sets on an issue or a project.
//!
//! Three rules run through it:
//! - **The mask first.** A field the person may not change is refused (`access.forbidden`) alike
//!   for any value and for a field that is not there, before the value is read, so the answer
//!   tells nothing about a hidden field (SEC-04, SEC-06).
//! - **The rules, once.** [`Rules::check`](bb_core::own_fields::Rules) decides every value; the
//!   server passes the linear-time pattern engine.
//! - **A hidden field's value is in no view.** A view holds only the values of fields the viewer
//!   may see on that item's issue type (`ItemValues::shown`), and the mask drops what is left by
//!   mistake (`#[masked(own_fields)]`).

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::access::FieldLevel;
use bb_core::access::PeopleLevel;
use bb_core::event::{EntityKind, FieldChange, RecordKind};
use bb_core::fields::FieldKey;
use bb_core::id::{ChoiceId, EntityId, FieldId, GroupId, IssueId, IssueTypeId, ProjectId, UserId};
use bb_core::own_fields::{FieldFor, FieldType};
use bb_core::own_fields_value::{Cell, FieldValue, OwnValues, Violation};
use bb_core::people_kinds::PeopleKind;
use bb_policy::{Evaluation, PeopleContext, Place};
use serde_json::Value;

use super::manage::current_default;
use super::pattern::Linear;
use super::types::Definition;
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};

/// The values of a set of items, as stored, with the definitions they are read with.
#[derive(Clone, Debug, Default)]
pub(crate) struct ItemValues {
    definitions: Arc<Vec<Definition>>,
    values: HashMap<EntityId, BTreeMap<FieldId, FieldValue>>,
}

impl ItemValues {
    /// Whether the values of this item were read.
    pub(crate) fn has(&self, item: EntityId) -> bool {
        self.values.contains_key(&item)
    }

    /// The value the item holds for a field.
    pub(crate) fn value(&self, item: EntityId, field: FieldId) -> Option<&FieldValue> {
        self.values.get(&item)?.get(&field)
    }

    /// The values of an item that the viewer may see on an item of this issue type (`None` for an
    /// item that has none, a project), each as the JSON of its type.
    pub(crate) fn shown(
        &self,
        viewer: &Evaluation,
        issue_type: Option<IssueTypeId>,
        item: EntityId,
    ) -> OwnValues {
        let mut shown = BTreeMap::new();
        let Some(values) = self.values.get(&item) else {
            return OwnValues::default();
        };
        for definition in self.definitions.iter() {
            let Some(value) = values.get(&definition.field.id) else {
                continue;
            };
            let key = FieldKey::Own(definition.field.id);
            if viewer.field_level_for(issue_type, key) < FieldLevel::CanSee {
                continue;
            }
            if let Some(json) = value.to_json(definition.field.kind) {
                shown.insert(key.code().into_owned(), json);
            }
        }
        OwnValues(shown)
    }

    /// The kind of an own field, by its key; none for a key that names no field.
    pub(crate) fn kind_of(&self, key: &str) -> Option<FieldType> {
        let field = FieldKey::from_code(key)?.own()?;
        self.definitions
            .iter()
            .find(|definition| definition.field.id == field)
            .map(|definition| definition.field.kind)
    }

    /// Adds what was read for more items.
    pub(crate) fn merge(&mut self, more: Self) {
        self.values.extend(more.values);
        if self.definitions.is_empty() {
            self.definitions = more.definitions;
        }
    }
}

/// One value a request sets.
#[derive(Clone, Debug)]
struct Set {
    field: FieldId,
    kind: FieldType,
    old: Option<FieldValue>,
    new: Option<FieldValue>,
}

/// The values a request sets on one item, checked and ready to write.
#[derive(Clone, Debug, Default)]
pub(crate) struct PlannedValues {
    sets: Vec<Set>,
}

impl PlannedValues {
    /// What changed, for the event: one change for each field whose value differs.
    pub(crate) fn changes(&self) -> Vec<FieldChange> {
        self.sets
            .iter()
            .filter(|set| set.old != set.new)
            .map(|set| FieldChange {
                field: FieldKey::Own(set.field).code().into_owned(),
                before: set.old.as_ref().and_then(|value| value.to_json(set.kind)),
                after: set.new.as_ref().and_then(|value| value.to_json(set.kind)),
            })
            .collect()
    }

    /// The rows to write for each field that differs: the field and its new cells.
    pub(crate) fn writes(&self) -> Vec<(FieldId, Vec<Cell>)> {
        self.sets
            .iter()
            .filter(|set| set.old != set.new)
            .map(|set| {
                (
                    set.field,
                    set.new
                        .as_ref()
                        .map(FieldValue::to_cells)
                        .unwrap_or_default(),
                )
            })
            .collect()
    }
}

/// The item a request sets values on.
#[derive(Clone, Debug)]
pub(crate) struct Item {
    /// What it is.
    pub(crate) kind: RecordKind,
    /// Which one; `None` while it is being made.
    pub(crate) id: Option<EntityId>,
    /// The projects from the top down to the project it is in (the project itself, for a
    /// project).
    pub(crate) chain: Vec<ProjectId>,
    /// The issue type, for an issue.
    pub(crate) issue_type: Option<IssueTypeId>,
}

impl Item {
    /// What the fields of this item are for.
    const fn field_for(&self) -> FieldFor {
        match self.kind {
            RecordKind::Project => FieldFor::Projects,
            _ => FieldFor::Issues,
        }
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The own fields of an item that the mask lets the person change: they are used for the item
    /// and not retired. A long text is in the list: it is changed on its own route.
    pub(crate) async fn changeable_own_fields(
        &self,
        mask: &bb_policy::FieldMask,
        item: &Item,
    ) -> Result<Vec<FieldKey>, AppError> {
        Ok(self
            .field_catalogue()
            .await?
            .iter()
            .filter(|definition| {
                let field = &definition.field;
                field.applies_to == item.field_for()
                    && field.retired_at.is_none()
                    && definition.scope.reaches(&item.chain, item.issue_type)
                    && mask.can_change(FieldKey::Own(field.id))
            })
            .map(|definition| FieldKey::Own(definition.field.id))
            .collect())
    }

    /// The projects from the top down to this one: where the scope of a field is judged.
    pub(crate) async fn chain_of(&self, project: ProjectId) -> Result<Vec<ProjectId>, AppError> {
        self.store()
            .project_chain(project)
            .await
            .map_err(|error| internal::<S>(&error))?
            .ok_or(AppError::Refused(ErrorCode::NotFound))
    }

    /// A value as a viewer is shown it. A person is a `PersonRef` as the kind *Assigned to* says,
    /// a group is its name when the viewer sees names in *Mentions*, an issue is its readable ID
    /// when the viewer sees it; the other types are the JSON of the value.
    pub(crate) async fn display_value(
        &self,
        actor: &Actor,
        viewer: &Evaluation,
        kind: FieldType,
        json: Value,
    ) -> Result<Value, AppError> {
        let to_json = |error: serde_json::Error| AppError::Internal(error.to_string());
        let id_of = |value: &Value| value.as_str().map(str::to_owned);
        match (kind, &json) {
            (FieldType::Person, Value::String(id)) => match id.parse::<UserId>() {
                Ok(person) => serde_json::to_value(
                    self.person_ref(
                        viewer,
                        PeopleKind::AssignedTo,
                        PeopleContext::Ordinary,
                        person,
                    )
                    .await?,
                )
                .map_err(to_json),
                Err(_) => Ok(Value::Null),
            },
            (FieldType::People, Value::Array(ids)) => {
                let mut shown = Vec::with_capacity(ids.len());
                for id in ids.iter().filter_map(id_of) {
                    if let Ok(person) = id.parse::<UserId>() {
                        shown.push(
                            serde_json::to_value(
                                self.person_ref(
                                    viewer,
                                    PeopleKind::AssignedTo,
                                    PeopleContext::Ordinary,
                                    person,
                                )
                                .await?,
                            )
                            .map_err(to_json)?,
                        );
                    }
                }
                Ok(Value::Array(shown))
            }
            (FieldType::Group, Value::String(id)) => {
                let name = match id.parse::<GroupId>() {
                    Ok(group) if viewer.kind_level(PeopleKind::Mentions) == PeopleLevel::Names => {
                        self.store()
                            .group(group)
                            .await
                            .map_err(|error| internal::<S>(&error))?
                            .map(|found| found.name)
                    }
                    _ => None,
                };
                Ok(serde_json::json!({ "id": id, "name": name }))
            }
            (FieldType::Issue, Value::String(id)) => {
                let key = match id.parse::<IssueId>() {
                    Ok(issue) => self.issue_key_for(actor, issue).await?,
                    Err(_) => None,
                };
                Ok(serde_json::json!({ "id": id, "key": key }))
            }
            _ => Ok(json),
        }
    }

    /// The readable ID of an issue, when the actor sees the issue.
    async fn issue_key_for(
        &self,
        actor: &Actor,
        issue: IssueId,
    ) -> Result<Option<String>, AppError> {
        let Some(found) = self
            .store()
            .issue(issue)
            .await
            .map_err(|error| internal::<S>(&error))?
            .filter(|found| found.deleted_at.is_none())
        else {
            return Ok(None);
        };
        let held = self
            .evaluation_of(actor, Place::Project(found.project))
            .await?;
        if !held.sees_issue(found.reported_by) {
            return Ok(None);
        }
        let Some(project) = self
            .store()
            .project(found.project)
            .await
            .map_err(|error| internal::<S>(&error))?
        else {
            return Ok(None);
        };
        Ok(Some(
            bb_core::issue_key::IssueKey::new(project.short_name, found.code).to_string(),
        ))
    }

    /// The definitions of the own fields.
    pub(crate) async fn field_catalogue(&self) -> Result<Arc<Vec<Definition>>, AppError> {
        Ok(Arc::new(
            self.store()
                .field_definitions()
                .await
                .map_err(|error| internal::<S>(&error))?,
        ))
    }

    /// Reads the values of these items.
    pub(crate) async fn load_values(
        &self,
        kind: RecordKind,
        items: &[EntityId],
    ) -> Result<ItemValues, AppError> {
        let definitions = self.field_catalogue().await?;
        let mut values: HashMap<EntityId, BTreeMap<FieldId, FieldValue>> =
            items.iter().map(|id| (*id, BTreeMap::new())).collect();
        if definitions.is_empty() {
            return Ok(ItemValues {
                definitions,
                values,
            });
        }
        let stored = self
            .store()
            .field_values(EntityKind::from(kind), items.to_vec())
            .await
            .map_err(|error| internal::<S>(&error))?;
        for found in stored {
            let Some(definition) = definitions
                .iter()
                .find(|definition| definition.field.id == found.field)
            else {
                continue;
            };
            if let Some(value) = FieldValue::from_cells(definition.field.kind, &found.cells) {
                values
                    .entry(found.entity)
                    .or_default()
                    .insert(found.field, value);
            }
        }
        Ok(ItemValues {
            definitions,
            values,
        })
    }

    /// Whether a person, group or issue a value names exists and may be named by this viewer.
    async fn reference_exists(
        &self,
        actor: &Actor,
        kind: FieldType,
        bytes: [u8; 16],
    ) -> Result<bool, AppError> {
        let failed = |error| internal::<S>(&error);
        Ok(match kind {
            FieldType::Person | FieldType::People => match UserId::from_bytes(bytes) {
                Ok(person) => self.store().person(person).await.map_err(failed)?.is_some(),
                Err(_) => false,
            },
            FieldType::Group => match GroupId::from_bytes(bytes) {
                Ok(group) => self.store().group(group).await.map_err(failed)?.is_some(),
                Err(_) => false,
            },
            FieldType::Issue => match IssueId::from_bytes(bytes) {
                Ok(id) => match self.store().issue(id).await.map_err(failed)? {
                    Some(issue) if issue.deleted_at.is_none() => {
                        // The viewer may only name an issue they see.
                        let held = self
                            .evaluation_of(actor, Place::Project(issue.project))
                            .await?;
                        held.sees_issue(issue.reported_by)
                    }
                    _ => false,
                },
                Err(_) => false,
            },
            _ => true,
        })
    }

    /// Checks the values a request asks for on an item and plans their writing.
    ///
    /// `asked` maps the key of an own field to the JSON of its value (`null` takes it away). When
    /// the item is being made (`current` is empty and `creating` is true) the defaults are set
    /// and the required fields the person can fill in have to be there.
    ///
    /// # Errors
    /// `access.forbidden` for a field the person may not change (or that is not there),
    /// `field.not_here` for a field that is retired or not used for this item, `request.invalid`
    /// for a long text outside its lock, and the code of the rule a value breaks.
    pub(crate) async fn plan_values(
        &self,
        actor: &Actor,
        viewer: &Evaluation,
        item: &Item,
        asked: &BTreeMap<String, Value>,
        current: &ItemValues,
        creating: bool,
    ) -> Result<PlannedValues, AppError> {
        let definitions = Arc::clone(&current.definitions);
        // The mask first, for every field asked about.
        let mut targets: Vec<(&Definition, &Value)> = Vec::with_capacity(asked.len());
        for (key, value) in asked {
            let definition = FieldKey::from_code(key)
                .and_then(FieldKey::own)
                .and_then(|id| {
                    definitions
                        .iter()
                        .find(|definition| definition.field.id == id)
                });
            let allowed = definition.filter(|definition| {
                viewer.field_level_for(item.issue_type, FieldKey::Own(definition.field.id))
                    >= FieldLevel::CanChange
            });
            let Some(definition) = allowed else {
                return Err(ErrorCode::Forbidden.into());
            };
            targets.push((definition, value));
        }
        let mut sets: Vec<Set> = Vec::new();
        for (definition, json) in targets {
            let field = &definition.field;
            if field.applies_to != item.field_for()
                || field.retired_at.is_some()
                || !definition.scope.reaches(&item.chain, item.issue_type)
            {
                return Err(ErrorCode::FieldNotHere.into());
            }
            if field.kind.is_long_text() && !creating {
                // A long text is saved under an edit lock, on its own route.
                return Err(ErrorCode::InvalidRequest.into());
            }
            let old = item.id.and_then(|id| current.value(id, field.id)).cloned();
            let new = FieldValue::from_json(field.kind, json).map_err(Violation::error)?;
            self.check_value(actor, definition, new.as_ref(), old.as_ref())
                .await?;
            sets.push(Set {
                field: field.id,
                kind: field.kind,
                old,
                new,
            });
        }
        if creating {
            Self::add_defaults_and_required(viewer, item, &definitions, &mut sets)?;
        }
        Ok(PlannedValues { sets })
    }

    /// Checks one value against the rules and against what the field offers.
    async fn check_value(
        &self,
        actor: &Actor,
        definition: &Definition,
        new: Option<&FieldValue>,
        old: Option<&FieldValue>,
    ) -> Result<(), AppError> {
        let field = &definition.field;
        field
            .rules
            .check(field.kind, new, &Linear)
            .map_err(|violation| AppError::from(violation.error()))?;
        let Some(value) = new else {
            return Ok(());
        };
        for bytes in value.ids() {
            let fine = if field.kind.has_choices() {
                // A retired choice may stay where it already is, but is not set anew.
                ChoiceId::from_bytes(*bytes).is_ok_and(|choice| {
                    definition.choices.iter().any(|offered| {
                        offered.id == choice
                            && (offered.retired_at.is_none()
                                || old.is_some_and(|old| old.ids().contains(bytes)))
                    })
                })
            } else {
                self.reference_exists(actor, field.kind, *bytes).await?
            };
            if !fine {
                return Err(ErrorCode::FieldUnknownChoice.into());
            }
        }
        Ok(())
    }

    /// For a new item: the default of every field that reaches it and was not asked for, and the
    /// check that every required field the person can fill in has a value.
    fn add_defaults_and_required(
        viewer: &Evaluation,
        item: &Item,
        definitions: &[Definition],
        sets: &mut Vec<Set>,
    ) -> Result<(), AppError> {
        for definition in definitions {
            let field = &definition.field;
            let reaches = field.applies_to == item.field_for()
                && field.retired_at.is_none()
                && definition.scope.reaches(&item.chain, item.issue_type);
            if !reaches || sets.iter().any(|set| set.field == field.id) {
                continue;
            }
            let default = current_default(definition);
            if default.is_none() {
                let may_fill = viewer.field_level_for(item.issue_type, FieldKey::Own(field.id))
                    >= FieldLevel::CanChange;
                if field.rules.required && may_fill {
                    return Err(ErrorCode::FieldRequired.into());
                }
                continue;
            }
            sets.push(Set {
                field: field.id,
                kind: field.kind,
                old: None,
                new: default,
            });
        }
        Ok(())
    }
}
