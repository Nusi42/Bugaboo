//! The own-field part of the in-memory store: definitions with their scope and choices, and the
//! values items hold.

use bb_core::event::EntityKind;
use bb_core::id::{ChoiceId, EntityId, FieldId, ProjectId};
use bb_core::own_fields::{FieldChoice, OwnField, Scope};
use bb_core::own_fields_value::Cell;

use super::fake::{FakeError, FakeStore, State, Tx};
use crate::fields::{Definition, FieldStore, StoredValues};

/// One value as the fake keeps it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FakeValue {
    pub(crate) kind: EntityKind,
    pub(crate) entity: EntityId,
    pub(crate) field: FieldId,
    pub(crate) cells: Vec<Cell>,
}

fn definitions(state: &State) -> Vec<Definition> {
    let mut found = state.field_definitions.clone();
    found.sort_by_key(|definition| (definition.field.position, definition.field.id));
    for definition in &mut found {
        definition
            .choices
            .sort_by_key(|choice| (choice.position, choice.id));
    }
    found
}

fn values(state: &State, kind: &EntityKind, entities: &[EntityId]) -> Vec<StoredValues> {
    let mut found: Vec<StoredValues> = state
        .field_values
        .iter()
        .filter(|value| &value.kind == kind && entities.contains(&value.entity))
        .map(|value| StoredValues {
            entity: value.entity,
            field: value.field,
            cells: value.cells.clone(),
        })
        .collect();
    found.sort_by_key(|value| (value.entity, value.field));
    found
}

fn holding(state: &State, field: FieldId, choice: ChoiceId) -> Vec<(EntityKind, EntityId)> {
    let mut found: Vec<(EntityKind, EntityId)> = state
        .field_values
        .iter()
        .filter(|value| {
            value.field == field
                && value
                    .cells
                    .iter()
                    .any(|cell| cell.reference == Some(choice.to_bytes()))
        })
        .map(|value| (value.kind.clone(), value.entity))
        .collect();
    found.sort_by_key(|(_, entity)| *entity);
    found.dedup();
    found
}

impl FieldStore for FakeStore {
    async fn field_definitions_in(tx: &mut Tx) -> Result<Vec<Definition>, FakeError> {
        Ok(definitions(&tx.state))
    }

    async fn field_values_in(
        tx: &mut Tx,
        kind: EntityKind,
        entities: Vec<EntityId>,
    ) -> Result<Vec<StoredValues>, FakeError> {
        Ok(values(&tx.state, &kind, &entities))
    }

    async fn items_with_choice_in(
        tx: &mut Tx,
        field: FieldId,
        choice: ChoiceId,
    ) -> Result<Vec<(EntityKind, EntityId)>, FakeError> {
        Ok(holding(&tx.state, field, choice))
    }

    async fn projects_with_values(&self, field: FieldId) -> Result<Vec<ProjectId>, FakeError> {
        Ok(self.read(|state| {
            let mut projects: Vec<ProjectId> = Vec::new();
            for value in state
                .field_values
                .iter()
                .filter(|value| value.field == field)
            {
                if let Some(issue) = state
                    .issues
                    .iter()
                    .find(|issue| EntityId::from(issue.id) == value.entity)
                {
                    projects.push(issue.project);
                } else if let Ok(project) = ProjectId::from_bytes(value.entity.to_bytes()) {
                    projects.push(project);
                }
            }
            projects.sort();
            projects.dedup();
            projects
        }))
    }

    async fn insert_field(tx: &mut Tx, definition: &Definition) -> Result<(), FakeError> {
        if tx
            .state
            .field_definitions
            .iter()
            .any(|known| known.field.id == definition.field.id)
        {
            return Err(FakeError::Duplicate);
        }
        tx.state.rules.own_fields.push(definition.field.id);
        tx.state.field_definitions.push(definition.clone());
        Ok(())
    }

    async fn update_field(
        tx: &mut Tx,
        field: &OwnField,
        scope: &Scope,
        version: i64,
    ) -> Result<bool, FakeError> {
        let Some(found) = tx
            .state
            .field_definitions
            .iter_mut()
            .find(|found| found.field.id == field.id && found.field.version == version)
        else {
            return Ok(false);
        };
        found.field = OwnField {
            version: version + 1,
            ..field.clone()
        };
        found.scope = scope.clone();
        Ok(true)
    }

    async fn bump_field(tx: &mut Tx, field: FieldId, version: i64) -> Result<bool, FakeError> {
        let Some(found) = tx
            .state
            .field_definitions
            .iter_mut()
            .find(|found| found.field.id == field && found.field.version == version)
        else {
            return Ok(false);
        };
        found.field.version += 1;
        Ok(true)
    }

    async fn insert_choice(tx: &mut Tx, choice: &FieldChoice) -> Result<(), FakeError> {
        let Some(found) = tx
            .state
            .field_definitions
            .iter_mut()
            .find(|found| found.field.id == choice.field)
        else {
            return Err(FakeError::Aborted);
        };
        found.choices.push(choice.clone());
        Ok(())
    }

    async fn update_choice(tx: &mut Tx, choice: &FieldChoice) -> Result<(), FakeError> {
        for definition in &mut tx.state.field_definitions {
            if let Some(found) = definition
                .choices
                .iter_mut()
                .find(|found| found.id == choice.id)
            {
                *found = choice.clone();
            }
        }
        Ok(())
    }

    async fn order_choices(
        tx: &mut Tx,
        field: FieldId,
        order: &[ChoiceId],
    ) -> Result<(), FakeError> {
        if let Some(definition) = tx
            .state
            .field_definitions
            .iter_mut()
            .find(|found| found.field.id == field)
        {
            for choice in &mut definition.choices {
                if let Some(at) = order.iter().position(|id| *id == choice.id) {
                    choice.position = i64::try_from(at).unwrap_or(i64::MAX);
                }
            }
        }
        Ok(())
    }

    async fn put_field_value(
        tx: &mut Tx,
        kind: EntityKind,
        entity: EntityId,
        field: FieldId,
        cells: &[Cell],
    ) -> Result<(), FakeError> {
        tx.state.field_values.retain(|value| {
            !(value.kind == kind && value.entity == entity && value.field == field)
        });
        if !cells.is_empty() {
            tx.state.field_values.push(FakeValue {
                kind,
                entity,
                field,
                cells: cells.to_vec(),
            });
        }
        Ok(())
    }

    async fn combine_choice_values(
        tx: &mut Tx,
        field: FieldId,
        from: ChoiceId,
        into: ChoiceId,
    ) -> Result<Vec<(EntityKind, EntityId)>, FakeError> {
        let holders = holding(&tx.state, field, from);
        for value in &mut tx.state.field_values {
            if value.field != field {
                continue;
            }
            let has_into = value
                .cells
                .iter()
                .any(|cell| cell.reference == Some(into.to_bytes()));
            for cell in &mut value.cells {
                if cell.reference == Some(from.to_bytes()) {
                    cell.reference = Some(into.to_bytes());
                }
            }
            if has_into {
                // The item held both: one row is enough.
                let mut seen = false;
                value.cells.retain(|cell| {
                    if cell.reference == Some(into.to_bytes()) {
                        let keep = !seen;
                        seen = true;
                        keep
                    } else {
                        true
                    }
                });
            }
        }
        Ok(holders)
    }
}
