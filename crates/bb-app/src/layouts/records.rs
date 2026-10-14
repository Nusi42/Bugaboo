//! A layout is a record the edit locks work on (ADR-0019, ADR-0027 decision 6): where it sits, its
//! version, and what a person needs to change it.

use bb_core::event::EntityRef;
use bb_core::id::LayoutId;
use bb_policy::{Permission, Place};

use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::editing::RecordFacts;

impl<S: AuthStore> Ctx<'_, S> {
    /// The facts of a layout for the editing use cases; `None` when there is no such layout.
    pub(crate) async fn layout_facts(
        &self,
        record: &EntityRef,
    ) -> Result<Option<RecordFacts>, AppError> {
        let Ok(id) = LayoutId::from_bytes(record.id.to_bytes()) else {
            return Ok(None);
        };
        let Some(stored) = self
            .store()
            .stored_layout(id)
            .await
            .map_err(|error| internal::<S>(&error))?
        else {
            return Ok(None);
        };
        Ok(Some(RecordFacts {
            place: stored
                .layout
                .project
                .map_or(Place::Organization, Place::Project),
            version: stored.layout.version,
            edit: Permission::ManageIssueTypes,
        }))
    }
}
