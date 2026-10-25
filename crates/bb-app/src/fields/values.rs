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

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use crate::event_members::own_change;
use bb_core::ErrorCode;
use bb_core::access::FieldLevel;
use bb_core::event::{EntityKind, FieldChange, RecordKind};
use bb_core::fields::FieldKey;
use bb_core::id::{ChoiceId, EntityId, FieldId, GroupId, IssueId, IssueTypeId, ProjectId, UserId};
use bb_core::own_fields::{FieldFor, FieldType};
use bb_core::own_fields_value::{Cell, FieldValue, OwnValues};
use bb_core::people_kinds::PeopleKind;
use bb_policy::{Evaluation, PeopleContext, PersonRef};
use serde_json::Value;

use super::manage::{current_default, must_be_asked_for};
use super::pattern::Linear;
use super::types::Definition;
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, entity, internal};
use crate::issues::{IssueView, OwnHtml};

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

    /// Whether every own field the item has a value in is offered in a project whose projects from
    /// the top down are `chain`, for an issue of this type: whether the item can live on there
    /// without losing a value.
    pub(crate) fn all_reach(
        &self,
        item: EntityId,
        chain: &[ProjectId],
        issue_type: IssueTypeId,
    ) -> bool {
        self.values.get(&item).is_none_or(|values| {
            values.keys().all(|field| {
                self.definitions
                    .iter()
                    .find(|definition| definition.field.id == *field)
                    .is_none_or(|definition| definition.scope.reaches(chain, Some(issue_type)))
            })
        })
    }

    /// The own fields the item has a value in.
    pub(crate) fn filled_fields(&self, item: EntityId) -> BTreeSet<FieldId> {
        self.values
            .get(&item)
            .map(|values| values.keys().copied().collect())
            .unwrap_or_default()
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
            .map(|set| own_change(set.field, set.kind, set.old.as_ref(), set.new.as_ref()))
            .collect()
    }

    /// The own fields that have a value once this is written, given the ones that have one now.
    pub(crate) fn filled_after(&self, mut filled: BTreeSet<FieldId>) -> BTreeSet<FieldId> {
        for set in &self.sets {
            if set.new.is_some() {
                filled.insert(set.field);
            } else {
                filled.remove(&set.field);
            }
        }
        filled
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

/// Whether a value stays because the field holds it already: a value that is kept is not judged
/// again, so a retired choice, a person who lost access or an issue that moved stays until it is
/// changed. The one rule for the fields of the catalogue (*Assigned to*) and for own fields.
pub(crate) fn is_kept<T: PartialEq>(held: &[T], value: &T) -> bool {
    held.contains(value)
}

/// The definition of an own field when the viewer may change the field on an item of this issue
/// type: `None` for a field that is not there and for one the level does not let them change,
/// which look alike (SEC-04, SEC-06).
fn may_change<'a>(
    definitions: &'a [Definition],
    viewer: &Evaluation,
    id: FieldId,
    issue_type: Option<IssueTypeId>,
) -> Option<&'a Definition> {
    definitions
        .iter()
        .find(|definition| definition.field.id == id)
        .filter(|_| viewer.field_level_for(issue_type, FieldKey::Own(id)) >= FieldLevel::CanChange)
}

/// A refusal about one own field, so the client shows it at that field.
fn refused_at(code: ErrorCode, field: FieldId) -> AppError {
    AppError::RefusedAtField {
        code,
        field: FieldKey::Own(field).code().into_owned(),
    }
}

/// `field.not-here` unless the field is used for this kind of item, is not retired, and reaches an
/// item in a project of this chain and of this issue type.
fn ensure_here(
    definition: &Definition,
    used_for: FieldFor,
    chain: &[ProjectId],
    issue_type: Option<IssueTypeId>,
) -> Result<(), AppError> {
    if definition.field.applies_to != used_for
        || definition.field.retired_at.is_some()
        || !definition.scope.reaches(chain, issue_type)
    {
        return Err(refused_at(ErrorCode::FieldNotHere, definition.field.id));
    }
    Ok(())
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
    /// Who reported it, for an issue: a person named in a field has to be able to see the issue.
    pub(crate) reported_by: Option<UserId>,
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
        self.own_fields_at(item, |field| mask.can_change(field))
            .await
    }

    /// The own fields of an item that the mask lets the person see: used for the item and not
    /// retired. The layouts of the issue page ask it, next to `changeable_own_fields`.
    pub(crate) async fn visible_own_fields(
        &self,
        mask: &bb_policy::FieldMask,
        item: &Item,
    ) -> Result<Vec<FieldKey>, AppError> {
        self.own_fields_at(item, |field| mask.can_see(field)).await
    }

    /// The own fields that reach an item and that `level_allows`: the one filter of the two reads.
    pub(crate) async fn own_fields_at(
        &self,
        item: &Item,
        level_allows: impl Fn(FieldKey) -> bool,
    ) -> Result<Vec<FieldKey>, AppError> {
        self.own_fields_reaching(
            item,
            |scope| scope.reaches(&item.chain, item.issue_type),
            level_allows,
        )
        .await
    }

    /// The own fields that are used for an item and not retired, whose scope `reached` and that
    /// `level_allows`. The layouts of the whole organization, which has no project, say `reached`
    /// by the issue type alone (`Scope::reaches_type`).
    pub(crate) async fn own_fields_reaching(
        &self,
        item: &Item,
        reached: impl Fn(&bb_core::own_fields::Scope) -> bool,
        level_allows: impl Fn(FieldKey) -> bool,
    ) -> Result<Vec<FieldKey>, AppError> {
        Ok(self
            .field_catalogue()
            .await?
            .iter()
            .filter(|definition| {
                let field = &definition.field;
                field.applies_to == item.field_for()
                    && field.retired_at.is_none()
                    && reached(&definition.scope)
                    && level_allows(FieldKey::Own(field.id))
            })
            .map(|definition| FieldKey::Own(definition.field.id))
            .collect())
    }

    /// What an own field of people is used for, when the viewer may choose its value in a project
    /// for an issue of this type (the list to choose from): `access.forbidden` for a field that
    /// is not there or that they may not change, `field.not-here` for one that is not of people,
    /// is retired, or does not reach the project and the type. The same judgement as for setting
    /// a value (`plan_values`).
    pub(crate) async fn people_field_to_choose(
        &self,
        viewer: &Evaluation,
        field: FieldId,
        project: ProjectId,
        issue_type: Option<IssueTypeId>,
    ) -> Result<FieldFor, AppError> {
        let definitions = self.field_catalogue().await?;
        // A project has no type of issue, so a field of projects is judged without one, as
        // `plan_values` does for a change of a project.
        let issue_type = definitions
            .iter()
            .find(|definition| definition.field.id == field)
            .filter(|definition| definition.field.applies_to == FieldFor::Issues)
            .and(issue_type);
        let definition = may_change(&definitions, viewer, field, issue_type)
            .ok_or(AppError::Refused(ErrorCode::Forbidden))?;
        let used_for = definition.field.applies_to;
        ensure_here(
            definition,
            used_for,
            &self.chain_of(project).await?,
            issue_type,
        )?;
        if !matches!(definition.field.kind, FieldType::Person | FieldType::People) {
            return Err(ErrorCode::FieldNotHere.into());
        }
        Ok(used_for)
    }

    /// Whether an issue can live on in a project whose projects from the top down are `chain`
    /// without losing the value of an own field: every field it has a value in is used there.
    pub(crate) async fn fields_follow(
        &self,
        issue: &bb_core::issue::Issue,
        chain: &[ProjectId],
    ) -> Result<bool, AppError> {
        let item: EntityId = issue.id.into();
        let values = self.load_values(RecordKind::Issue, &[item]).await?;
        Ok(values.all_reach(item, chain, issue.issue_type))
    }

    /// The projects from the top down to this one: where the scope of a field is judged.
    pub(crate) async fn chain_of(&self, project: ProjectId) -> Result<Vec<ProjectId>, AppError> {
        self.store()
            .project_chain(project)
            .await
            .map_err(|error| internal::<S>(&error))?
            .ok_or(AppError::Refused(ErrorCode::NotFound))
    }

    /// A person as the viewer is shown them in an own field; `people` caches them for a page.
    async fn field_person(
        &self,
        viewer: &Evaluation,
        people: &mut HashMap<(PeopleKind, UserId), PersonRef>,
        kind: PeopleKind,
        person: UserId,
    ) -> Result<Value, AppError> {
        let key = (kind, person);
        let to_json = |shown: &PersonRef| {
            serde_json::to_value(shown).map_err(|error| AppError::Internal(error.to_string()))
        };
        if let Some(shown) = people.get(&key) {
            return to_json(shown);
        }
        let shown = self
            .person_ref(viewer, kind, PeopleContext::Ordinary, person)
            .await?;
        let json = to_json(&shown);
        people.insert(key, shown);
        json
    }

    /// A value as a viewer is shown it: a person or a group as the kind of people information of
    /// the type says (`FieldType::people_kind`), an issue by its ID when the viewer sees it.
    pub(crate) async fn display_value(
        &self,
        actor: &Actor,
        viewer: &Evaluation,
        people: &mut HashMap<(PeopleKind, UserId), PersonRef>,
        kind: FieldType,
        json: Value,
    ) -> Result<Value, AppError> {
        let id_of = |value: &Value| value.as_str().map(str::to_owned);
        match (kind, &json) {
            (FieldType::Person, Value::String(id)) => {
                match (id.parse::<UserId>(), kind.people_kind()) {
                    (Ok(person), Some(each)) => {
                        self.field_person(viewer, people, each, person).await
                    }
                    _ => Ok(Value::Null),
                }
            }
            (FieldType::People, Value::Array(ids)) => {
                let mut shown = Vec::with_capacity(ids.len());
                for id in ids.iter().filter_map(id_of) {
                    if let (Ok(person), Some(each)) = (id.parse::<UserId>(), kind.people_kind()) {
                        shown.push(self.field_person(viewer, people, each, person).await?);
                    }
                }
                Ok(Value::Array(shown))
            }
            (FieldType::Group, Value::String(id)) => {
                let name = match id.parse::<GroupId>() {
                    Ok(group) if viewer.sees_names_in(kind) => self
                        .store()
                        .group(group)
                        .await
                        .map_err(|error| internal::<S>(&error))?
                        .map(|found| found.name),
                    _ => None,
                };
                Ok(serde_json::json!({ "id": id, "name": name }))
            }
            (FieldType::Issue, Value::String(id)) => {
                let key = match id.parse::<IssueId>() {
                    Ok(issue) => self.readable_id_for(actor, issue).await?,
                    Err(_) => None,
                };
                Ok(key.map_or(
                    Value::Null,
                    |key| serde_json::json!({ "id": id, "readable_id": key }),
                ))
            }
            _ => Ok(json),
        }
    }

    /// The long-text own fields of an issue as safe HTML, one for each the viewer may see on an
    /// issue of this type and that has a text.
    pub(crate) async fn long_texts_html(
        &self,
        actor: &Actor,
        viewer: &Evaluation,
        issue: &bb_core::issue::Issue,
        view: &IssueView,
    ) -> Result<Vec<OwnHtml>, AppError> {
        let mut shown = Vec::new();
        for definition in self.field_catalogue().await?.iter() {
            let field = &definition.field;
            let key = FieldKey::Own(field.id);
            if !field.kind.is_long_text()
                || viewer.field_level_for(Some(issue.issue_type), key) < FieldLevel::CanSee
            {
                continue;
            }
            let Some(Value::String(text)) = view.own_fields.0.get(&key.code().into_owned()) else {
                continue;
            };
            let html = self.html_of(actor, viewer, issue, text).await?;
            shown.push(OwnHtml { field: key, html });
        }
        Ok(shown)
    }

    /// The readable ID of an issue, when the actor sees the issue.
    async fn readable_id_for(
        &self,
        actor: &Actor,
        issue: IssueId,
    ) -> Result<Option<String>, AppError> {
        let Some((found, _)) = self.seen_issue_by_id(actor, issue).await? else {
            return Ok(None);
        };
        let Some(project) = self
            .store()
            .project(found.project)
            .await
            .map_err(|error| internal::<S>(&error))?
        else {
            return Ok(None);
        };
        Ok(Some(
            bb_core::readable_id::ReadableId::new(project.short_name, found.code).to_string(),
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

    /// Checks the values a request asks for on an item and plans their writing.
    ///
    /// `asked` maps the key of an own field to the JSON of its value (`null` takes it away). When
    /// the item is being made (`current` is empty and `creating` is true) the defaults are set
    /// and the required fields the person can fill in have to be there.
    ///
    /// # Errors
    /// `access.forbidden` for a field the person may not change (or that is not there),
    /// `field.not-here` for a field that is retired or not used for this item, `request.invalid`
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
            let allowed = FieldKey::from_code(key)
                .and_then(FieldKey::own)
                .and_then(|id| may_change(&definitions, viewer, id, item.issue_type));
            let Some(definition) = allowed else {
                return Err(ErrorCode::Forbidden.into());
            };
            targets.push((definition, value));
        }
        let mut sets: Vec<Set> = Vec::new();
        for (definition, json) in targets {
            let field = &definition.field;
            ensure_here(definition, item.field_for(), &item.chain, item.issue_type)?;
            if field.kind.is_long_text() && !creating {
                // A long text is saved under an edit lock, on its own route.
                return Err(ErrorCode::InvalidRequest.into());
            }
            let old = item.id.and_then(|id| current.value(id, field.id)).cloned();
            let new = FieldValue::from_json(field.kind, json)
                .map_err(|violation| refused_at(violation.error(), field.id))?;
            self.check_value(actor, viewer, item, definition, new.as_ref(), old.as_ref())
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
        viewer: &Evaluation,
        item: &Item,
        definition: &Definition,
        new: Option<&FieldValue>,
        old: Option<&FieldValue>,
    ) -> Result<(), AppError> {
        let field = &definition.field;
        field
            .rules
            .check(field.kind, new, &Linear)
            .map_err(|violation| refused_at(violation.error(), field.id))?;
        let Some(value) = new else {
            return Ok(());
        };
        let held = old.map_or(&[][..], FieldValue::ids);
        // Whether this is *Undo*: asked only once, and only when a value is not fine as it is.
        let mut undo: Option<bool> = None;
        for bytes in value.ids() {
            let (fine, refusal) = if field.kind.has_choices() {
                // A retired choice may stay where it already is, but is not set anew.
                let fine = ChoiceId::from_bytes(*bytes).is_ok_and(|choice| {
                    definition.choices.iter().any(|offered| {
                        offered.id == choice
                            && (offered.retired_at.is_none() || is_kept(held, bytes))
                    })
                });
                (fine, ErrorCode::FieldUnknownChoice)
            } else {
                // What the field already holds is not asked again, so a person who went away or
                // an issue that moved stays where it is until it is changed.
                let fine = is_kept(held, bytes)
                    || self
                        .reference_may_be_named(actor, viewer, item, field.kind, *bytes, true)
                        .await?;
                (fine, ErrorCode::FieldUnknownReference)
            };
            if fine {
                continue;
            }
            // Putting back exactly what the field held before the last change (*Undo*) skips only
            // what may have changed since: a choice may be retired, a person may have lost
            // access. What is put back still has to be there, and to be nameable by the viewer.
            let undoing = match undo {
                Some(known) => known,
                None => *undo.insert(self.puts_back(item, definition, value, old).await?),
            };
            if !(undoing
                && self
                    .still_there(actor, viewer, item, definition, *bytes)
                    .await?)
            {
                return Err(refused_at(refusal, field.id));
            }
        }
        Ok(())
    }

    /// Whether a value of *Undo* can be put back: a choice of the field (retired or not), or a
    /// person, group or issue that still exists and that the viewer may name, judged without
    /// the access of the person named (`reference_may_be_named`).
    async fn still_there(
        &self,
        actor: &Actor,
        viewer: &Evaluation,
        item: &Item,
        definition: &Definition,
        bytes: [u8; 16],
    ) -> Result<bool, AppError> {
        let field = &definition.field;
        if field.kind.has_choices() {
            return Ok(ChoiceId::from_bytes(bytes).is_ok_and(|choice| {
                definition
                    .choices
                    .iter()
                    .any(|offered| offered.id == choice)
            }));
        }
        self.reference_may_be_named(actor, viewer, item, field.kind, bytes, false)
            .await
    }

    /// Whether `value` is what the field held before the last change of the item, and that
    /// change is the one that set the value it holds now: *Undo* of that change.
    async fn puts_back(
        &self,
        item: &Item,
        definition: &Definition,
        value: &FieldValue,
        now: Option<&FieldValue>,
    ) -> Result<bool, AppError> {
        let Some(id) = item.id else {
            return Ok(false);
        };
        let field = &definition.field;
        let last = self
            .store()
            .latest_version_event_about(&entity(item.kind, id))
            .await
            .map_err(|error| internal::<S>(&error))?;
        let key = FieldKey::Own(field.id).code().into_owned();
        Ok(last.is_some_and(|event| {
            event.changes.iter().any(|change| {
                change.field == key
                    && change.before == value.to_json(field.kind)
                    && change.after == now.and_then(|now| now.to_json(field.kind))
            })
        }))
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
            if must_be_asked_for(definition) {
                let may_fill = viewer.field_level_for(item.issue_type, FieldKey::Own(field.id))
                    >= FieldLevel::CanChange;
                if may_fill {
                    return Err(refused_at(ErrorCode::FieldRequired, field.id));
                }
                continue;
            }
            let Some(default) = current_default(definition) else {
                continue;
            };
            sets.push(Set {
                field: field.id,
                kind: field.kind,
                old: None,
                new: Some(default),
            });
        }
        Ok(())
    }
}
