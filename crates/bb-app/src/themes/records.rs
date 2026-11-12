//! An own theme is a record the edit locks and drafts work on (ADR-0019, ADR-0026 update M8): where
//! it sits, its version, and what a person needs to change it. Its draft is the editor's unsaved
//! values, from which the style sheet of the preview is made.

use bb_core::event::EntityRef;
use bb_core::id::ThemeId;
use bb_core::theme::ThemeKind;
use bb_policy::{Permission, Place};

use super::rules::known_to;
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::editing::RecordFacts;

impl<S: AuthStore> Ctx<'_, S> {
    /// The facts of a theme for the editing use cases; `None` when there is no such theme or the
    /// person may not use it: they do not know it. A theme for everyone is changed with *Change
    /// organization settings*; a personal theme by its maker, who is the only one it is known to.
    pub(crate) async fn theme_facts(
        &self,
        actor: &Actor,
        record: &EntityRef,
    ) -> Result<Option<RecordFacts>, AppError> {
        let Ok(id) = ThemeId::from_bytes(record.id.to_bytes()) else {
            return Ok(None);
        };
        let held = self.evaluation_of(actor, Place::Organization).await?;
        let Some(theme) = self
            .store()
            .own_theme(id)
            .await
            .map_err(|error| internal::<S>(&error))?
            .filter(|theme| known_to(theme, actor.person.id, &held))
        else {
            return Ok(None);
        };
        Ok(Some(RecordFacts {
            place: Place::Organization,
            version: theme.version,
            edit: match theme.kind {
                ThemeKind::ForEveryone => Some(Permission::ChangeOrganizationSettings),
                ThemeKind::Personal => None,
            },
        }))
    }
}
