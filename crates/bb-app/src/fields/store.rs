//! The port of the own-field use cases: what they need from storage beyond the issues port.
//!
//! Like the other ports, reads take `&self` and **writes** are associated functions that run
//! inside `record_change`, so a change and its domain event are committed together
//! (ARCH-EVENTS). A read that is also needed inside a change is written once with `queries!`.

use std::future::Future;

use bb_core::event::{EntityKind, RecordKind};
use bb_core::id::{ChoiceId, EntityId, FieldId, IssueId, ProjectId};
use bb_core::own_fields::{FieldChoice, OwnField, Scope};
use bb_core::own_fields_value::Cell;
use jiff::Timestamp;

use super::types::Definition;
use crate::issues::IssueStore;
use crate::store_macros::{queries, reads, writes};

/// The values an item holds for one field, as stored: one cell for each value, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredValues {
    /// The item.
    pub entity: EntityId,
    /// The field.
    pub field: FieldId,
    /// The rows of the value.
    pub cells: Vec<Cell>,
}

/// What the own-field use cases need from storage.
pub trait FieldStore: IssueStore + 'static {
    queries! {
        /// Every own field with where it is used and its choices, in the order they are offered
        /// in (then by ID).
        fn field_definitions() -> Vec<Definition> = field_definitions_in;
        /// The values the items of this kind hold for any field, for the items named.
        fn field_values(kind: EntityKind, entities: Vec<EntityId>) -> Vec<StoredValues> = field_values_in;
        /// The items (their kind and ID) that hold a choice as a value of the field.
        fn items_with_choice(field: FieldId, choice: ChoiceId) -> Vec<(EntityKind, EntityId)> = items_with_choice_in;
    }

    reads! {
        /// The projects in which a field has values: the project of each issue that holds one,
        /// and each project that holds one (distinct, in no order).
        fn projects_with_values(field: FieldId) -> Vec<ProjectId>;
    }

    writes! {
        /// Adds a field with its scope and its choices.
        fn insert_field(definition: &Definition) -> ();
        /// Writes everything of a field but its choices (name, help, rules, default, look,
        /// assistant mark, order, retired date) and its scope, counts the version up, and says
        /// whether the field was still at `version`.
        fn update_field(field: &OwnField, scope: &Scope, version: i64) -> bool;
        /// Counts the version of a field up (a choice changed), and says whether it was still at
        /// `version`.
        fn bump_field(field: FieldId, version: i64) -> bool;
        /// Adds a choice.
        fn insert_choice(choice: &FieldChoice) -> ();
        /// Writes the name, color, icon and retired date of a choice.
        fn update_choice(choice: &FieldChoice) -> ();
        /// Puts the choices of a field in the order given.
        fn order_choices(field: FieldId, order: &[ChoiceId]) -> ();
        /// Sets the value of one field of one item to these rows; no rows takes the value away.
        fn put_field_value(
            kind: EntityKind,
            entity: EntityId,
            field: FieldId,
            cells: &[Cell]
        ) -> ();
        /// Moves every value of a choice to another choice of the same field. An item that holds
        /// both keeps one row. Says which items it changed.
        fn combine_choice_values(
            field: FieldId,
            from: ChoiceId,
            into: ChoiceId
        ) -> Vec<(EntityKind, EntityId)>;
        /// Retires a field by date (or offers it again with `None`).
        fn set_field_retired(field: FieldId, at: Option<Timestamp>) -> ();
    }

    /// Counts the versions of many items up at once, as [`Self::touch_item`] does for one, and
    /// says the version each had, in the order of `items`. A store that can do it in fewer
    /// statements says so by writing its own; this one asks for each item in turn.
    fn touch_items(
        transaction: &mut Self::Transaction,
        items: &[(EntityKind, EntityId)],
        at: Timestamp,
    ) -> impl Future<Output = Result<Vec<Option<i64>>, Self::Error>> + Send {
        async move {
            let mut versions = Vec::with_capacity(items.len());
            for (kind, id) in items {
                versions.push(Self::touch_item(transaction, kind, *id, at).await?);
            }
            Ok(versions)
        }
    }

    /// Counts the version of an item up without changing anything else of it (its own fields
    /// changed): an issue or a project. Says the version it had, `None` for a kind of item that
    /// has none or an item that is not there.
    fn touch_item(
        transaction: &mut Self::Transaction,
        kind: &EntityKind,
        id: EntityId,
        at: Timestamp,
    ) -> impl Future<Output = Result<Option<i64>, Self::Error>> + Send {
        async move {
            if *kind == EntityKind::from(RecordKind::Issue) {
                let Ok(issue) = IssueId::from_bytes(id.to_bytes()) else {
                    return Ok(None);
                };
                let Some(mut found) = Self::issue_in(transaction, issue).await? else {
                    return Ok(None);
                };
                let version = found.version;
                found.updated_at = at;
                Self::update_issue(transaction, &found, version)
                    .await
                    .map(|done| done.then_some(version))
            } else if *kind == EntityKind::from(RecordKind::Project) {
                let Ok(project) = ProjectId::from_bytes(id.to_bytes()) else {
                    return Ok(None);
                };
                let Some(found) = Self::project_in(transaction, project).await? else {
                    return Ok(None);
                };
                Self::update_project(
                    transaction,
                    project,
                    &found.name,
                    &found.short_name,
                    found.version,
                )
                .await
                .map(|done| done.then_some(found.version))
            } else {
                Ok(None)
            }
        }
    }
}
