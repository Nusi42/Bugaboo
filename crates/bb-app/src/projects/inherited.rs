//! The defaults of the fields of time entries that a project gets from the projects above it
//! (TIME-13, ADR-0035, decision 1): which value, and which project it comes from, so the settings of
//! a project can say "Set by the project above". The rule is the one of a new entry's defaults,
//! `time::nearest_on_projects`; this only asks it for the project that holds nothing itself.

use std::collections::{BTreeMap, HashMap};

use bb_core::event::RecordKind;
use bb_core::id::{EntityId, ProjectId};
use bb_core::own_fields::FieldFor;
use bb_core::own_fields_value::OwnValues;
use bb_policy::Evaluation;

use super::shown::Shown;
use super::types::ProjectRef;
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx};
use crate::time::nearest_on_projects;

impl<S: AuthStore> Ctx<'_, S> {
    /// The inherited defaults of `project` as `actor` may see them, by the key of the field, and
    /// where each comes from (nothing for a project the person cannot see).
    pub(super) async fn inherited_time_defaults(
        &self,
        actor: &Actor,
        held: &Evaluation,
        shown: &Shown,
        chain: &[ProjectId],
        project: ProjectId,
    ) -> Result<(OwnValues, BTreeMap<String, Option<ProjectRef>>), AppError> {
        let entities: Vec<EntityId> = chain.iter().copied().map(EntityId::from).collect();
        let own: EntityId = project.into();
        let above: Vec<EntityId> = entities
            .iter()
            .copied()
            .filter(|each| *each != own)
            .collect();
        let values = self.load_values(RecordKind::Project, &entities).await?;
        let catalogue = self.field_catalogue().await?;
        let (mut found, mut from) = (OwnValues::default(), BTreeMap::new());
        let mut people = HashMap::new();
        for definition in catalogue.iter().filter(|definition| {
            definition.field.applies_to == FieldFor::TimeEntries
                && definition.field.retired_at.is_none()
        }) {
            let field = definition.field.id;
            if values.value(own, field).is_some() {
                continue;
            }
            let Some((source, _)) = nearest_on_projects(&values, &above, field) else {
                continue;
            };
            // The value as this person may see it, as the project's own values are shown.
            let key = bb_core::fields::FieldKey::Own(field).code().into_owned();
            let Some(mut value) = values.shown(held, None, source).0.remove(&key) else {
                continue;
            };
            value = self
                .display_value(actor, held, &mut people, definition.field.kind, value)
                .await?;
            if value.is_null() {
                continue;
            }
            let label = chain
                .iter()
                .find(|each| EntityId::from(**each) == source)
                .and_then(|id| shown.reference(*id));
            found.0.insert(key.clone(), value);
            from.insert(key, label);
        }
        Ok((found, from))
    }
}
