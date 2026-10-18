//! The names a reader may read in a change of an issue (LM1 of the M4 exit review): the issue type
//! and the labels. An event keeps the names of its time, so a change can name a type or a label
//! that the issue has not got any more and that the reader has never been told about. The one rule
//! is `offers/known.rs`; this file asks it once for a page and shapes the two values.
//!
//! It fails closed. A label is judged by the group the event kept for it; an old event that kept a
//! name only is judged by the labels that have the name now, and a name that no label has any more
//! is shown only to a reader who may know every group.

use std::collections::{BTreeMap, BTreeSet};

use bb_core::fields::Field;
use bb_core::id::{IssueTypeId, LabelGroupId};
use bb_core::message::{self, Message};
use serde_json::Value;

use super::read::{Member, Reading};
use super::types::ChangeValue;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::projects::Shown;
use crate::tags::{EventLabel, event_labels};

/// What the reader may know about labels, from the groups of labels they may know.
struct LabelKnowledge {
    /// The groups of labels the reader may know.
    groups: BTreeSet<LabelGroupId>,
    /// Whether they may know every group there is.
    every_group: bool,
    /// The names that a label the reader may know has.
    known_names: BTreeSet<String>,
    /// The names that only labels of groups the reader may not know have.
    hidden_names: BTreeSet<String>,
}

impl LabelKnowledge {
    /// Whether the reader may read this label of an event.
    fn may_read(&self, label: &EventLabel) -> bool {
        match label {
            EventLabel::Grouped(each) => {
                each.group.is_none_or(|group| self.groups.contains(&group))
            }
            EventLabel::Named(name) => {
                if self.known_names.contains(name) {
                    true
                } else if self.hidden_names.contains(name) {
                    false
                } else {
                    // No label has the name any more: nothing says where it was.
                    self.every_group
                }
            }
        }
    }
}

/// What the reader may know by name, read when a change first needs it.
#[derive(Default)]
pub(crate) struct KnownNames {
    /// What the reader sees of the tree of projects, read once for both lists.
    shown: Option<Shown>,
    types: Option<BTreeMap<IssueTypeId, String>>,
    labels: Option<LabelKnowledge>,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// What the reader sees of the tree of projects, read the first time it is asked for.
    async fn shown_for<'k>(&self, reading: &'k mut Reading<'_>) -> Result<&'k Shown, AppError> {
        if reading.names.shown.is_none() {
            reading.names.shown = Some(self.shown_to(reading.actor).await?);
        }
        reading
            .names
            .shown
            .as_ref()
            .ok_or_else(|| AppError::Internal("the projects the reader sees".into()))
    }

    /// The value of a change to the issue type: the name where the reader may know the type, else
    /// a neutral word.
    pub(super) async fn type_value(
        &self,
        reading: &mut Reading<'_>,
        value: &Value,
    ) -> Result<Option<ChangeValue>, AppError> {
        let Some(id) = value
            .as_str()
            .and_then(|text| text.parse::<IssueTypeId>().ok())
        else {
            return Ok(None);
        };
        if reading.names.types.is_none() {
            let all = self
                .store()
                .issue_types()
                .await
                .map_err(|error| internal::<S>(&error))?;
            let shown = self.shown_for(reading).await?;
            let known = self.known_types(shown, all).await?;
            reading.names.types =
                Some(known.into_iter().map(|each| (each.id, each.name)).collect());
        }
        let text = reading
            .names
            .types
            .as_ref()
            .and_then(|known| known.get(&id))
            .cloned()
            .unwrap_or_else(|| Message::from(message::HISTORY_TYPE_UNKNOWN).render());
        Ok(Some(ChangeValue::Text { text }))
    }

    /// The value of a change to the labels: the labels the reader may read. Nothing left to show is
    /// no value.
    pub(super) async fn labels_value(
        &self,
        reading: &mut Reading<'_>,
        value: &Value,
    ) -> Result<Option<ChangeValue>, AppError> {
        let labels = event_labels(value);
        if reading.names.labels.is_none() {
            let failed = |error| internal::<S>(&error);
            let held = self.store().labels().await.map_err(failed)?;
            let groups = self.store().label_groups().await.map_err(failed)?;
            let every = groups.len();
            let shown = self.shown_for(reading).await?;
            let known: BTreeSet<LabelGroupId> = self
                .known_groups(shown, groups)
                .await?
                .into_iter()
                .map(|group| group.id)
                .collect();
            let (seen, unseen): (Vec<_>, Vec<_>) = held
                .into_iter()
                .partition(|label| label.group.is_none_or(|group| known.contains(&group)));
            let known_names: BTreeSet<String> = seen.into_iter().map(|label| label.name).collect();
            let hidden_names = unseen
                .into_iter()
                .map(|label| label.name)
                .filter(|name| !known_names.contains(name))
                .collect();
            reading.names.labels = Some(LabelKnowledge {
                every_group: known.len() == every,
                groups: known,
                known_names,
                hidden_names,
            });
        }
        let wanted = !labels.is_empty();
        let names: Vec<String> = match &reading.names.labels {
            Some(knowledge) => labels
                .iter()
                .filter(|label| knowledge.may_read(label))
                .map(|label| label.name().to_owned())
                .collect(),
            None => Vec::new(),
        };
        Ok((names.is_empty() != wanted).then_some(ChangeValue::Names { names }))
    }
}

/// Whether a change of the labels shows nothing to the reader: no label on either side, or the same
/// labels on both once the ones they may not read are left out.
pub(super) fn leaves_nothing(
    member: Member,
    before: Option<&ChangeValue>,
    after: Option<&ChangeValue>,
) -> bool {
    if member != Member::Catalogue(Field::Labels) {
        return false;
    }
    let names = |value: Option<&ChangeValue>| match value {
        Some(ChangeValue::Names { names }) => names.clone(),
        _ => Vec::new(),
    };
    names(before) == names(after)
}
