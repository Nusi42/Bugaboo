//! The names a reader may read in a change of an issue (LM1 of the M4 exit review): the issue type
//! and the labels. An event keeps the names of its time, so a change can name a type or a label
//! that the issue has not got any more and that the reader has never been told about. The one rule
//! is `offers/known.rs`; this file asks it for what the page names, once, and shapes the two
//! values.
//!
//! It fails closed. A label is judged by the group the event kept for it; an old event that kept a
//! name only is judged by the labels that have the name now, and a name that no label has any more
//! is shown only to a reader who may know every group.

use std::collections::{BTreeMap, BTreeSet};

use bb_core::fields::Field;
use bb_core::id::{IssueTypeId, LabelGroupId};
use bb_core::message::{self, Message};
use bb_core::tags::LabelGroup;
use serde_json::Value;

use super::read::{Member, Reading};
use super::types::ChangeValue;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::offers::label_is_known;
use crate::projects::Shown;
use crate::tags::{EventLabel, event_labels};

/// What the reader may know about the names of old events that kept a name only, from the labels
/// there are now.
struct LegacyNames {
    /// Whether they may know every label group there is.
    every_group: bool,
    /// The names that a label the reader may know has.
    known: BTreeSet<String>,
    /// The names that only labels of groups the reader may not know have.
    hidden: BTreeSet<String>,
}

/// What the reader may know by name, found when a change first names it.
#[derive(Default)]
pub(crate) struct KnownNames {
    /// What the reader sees of the tree of projects, read once for both lists.
    shown: Option<Shown>,
    /// The issue types a change named: the name the reader may read, or none.
    types: BTreeMap<IssueTypeId, Option<String>>,
    /// Every label group there is, read once.
    groups: Option<Vec<LabelGroup>>,
    /// Whether the reader may know a group a change named.
    known_groups: BTreeMap<LabelGroupId, bool>,
    /// The labels of old events, read when an old event names one.
    legacy: Option<LegacyNames>,
    /// The names of the tags the reader may know, read when a change names a tag.
    tags: Option<BTreeSet<String>>,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The value of a change to the tags: the names of the tags the reader may know (a tag only
    /// projects they do not see know is not told, SEC-06), or none when nothing is left.
    pub(super) async fn tags_value(
        &self,
        reading: &mut Reading<'_>,
        value: &Value,
    ) -> Result<Option<ChangeValue>, AppError> {
        if reading.names.tags.is_none() {
            let shown = self.shown_for(reading, false).await?.clone();
            reading.names.tags = Some(self.known_tag_names(&shown).await?);
        }
        let known = reading.names.tags.clone().unwrap_or_default();
        let named: Vec<&str> = value
            .as_array()
            .map(|items| items.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        let shown_names: Vec<String> = named
            .iter()
            .filter(|each| known.contains(&each.to_lowercase()))
            .map(|each| (*each).to_owned())
            .collect();
        Ok((shown_names.is_empty() == named.is_empty())
            .then_some(ChangeValue::Names { names: shown_names }))
    }

    /// What the reader sees of the tree of projects, read the first time it is asked for; with
    /// the levels only when `levels` is set (the groups of labels in use need them), read again
    /// if the view held so far has none.
    async fn shown_for<'k>(
        &self,
        reading: &'k mut Reading<'_>,
        levels: bool,
    ) -> Result<&'k Shown, AppError> {
        if reading
            .names
            .shown
            .as_ref()
            .is_none_or(|held| levels && !held.has_levels())
        {
            reading.names.shown = Some(if levels {
                self.shown_with_levels_to(reading.actor).await?
            } else {
                self.shown_to(reading.actor).await?
            });
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
        if !reading.names.types.contains_key(&id) {
            // Only the type that is named is judged, from the types the request read once.
            let named = self
                .all_issue_types()
                .await?
                .iter()
                .find(|found| found.id == id)
                .cloned();
            let known = match named {
                Some(named) => {
                    let shown = self.shown_for(reading, false).await?;
                    self.known_types(shown, vec![named]).await?.pop()
                }
                None => None,
            };
            reading
                .names
                .types
                .insert(id, known.map(|found| found.name));
        }
        let text = reading
            .names
            .types
            .get(&id)
            .cloned()
            .flatten()
            .unwrap_or_else(|| Message::from(message::HISTORY_TYPE_UNKNOWN).render());
        Ok(Some(ChangeValue::Text { text }))
    }

    /// Every label group there is, read once for a reader.
    async fn all_groups(&self, reading: &mut Reading<'_>) -> Result<Vec<LabelGroup>, AppError> {
        if reading.names.groups.is_none() {
            reading.names.groups = Some(
                self.store()
                    .label_groups()
                    .await
                    .map_err(|error| internal::<S>(&error))?,
            );
        }
        Ok(reading.names.groups.clone().unwrap_or_default())
    }

    /// Whether the reader may know a label group a change named.
    async fn group_known(
        &self,
        reading: &mut Reading<'_>,
        group: LabelGroupId,
    ) -> Result<bool, AppError> {
        if let Some(known) = reading.names.known_groups.get(&group) {
            return Ok(*known);
        }
        let named = self
            .all_groups(reading)
            .await?
            .into_iter()
            .find(|each| each.id == group);
        let known = match named {
            Some(named) => {
                let shown = self.shown_for(reading, true).await?;
                !self.known_groups(shown, vec![named]).await?.is_empty()
            }
            None => false,
        };
        reading.names.known_groups.insert(group, known);
        Ok(known)
    }

    /// What the reader may know of the names of old events, from the labels there are now.
    async fn legacy_names(&self, reading: &mut Reading<'_>) -> Result<(), AppError> {
        if reading.names.legacy.is_some() {
            return Ok(());
        }
        let held = self
            .store()
            .labels()
            .await
            .map_err(|error| internal::<S>(&error))?;
        let groups = self.all_groups(reading).await?;
        let every = groups.len();
        let shown = self.shown_for(reading, true).await?;
        let known_groups = self.known_groups(shown, groups).await?;
        let (seen, unseen): (Vec<_>, Vec<_>) = held
            .into_iter()
            .partition(|label| label_is_known(label.group, &known_groups));
        let known: BTreeSet<String> = seen.into_iter().map(|label| label.name).collect();
        let hidden = unseen
            .into_iter()
            .map(|label| label.name)
            .filter(|name| !known.contains(name))
            .collect();
        reading.names.legacy = Some(LegacyNames {
            every_group: known_groups.len() == every,
            known,
            hidden,
        });
        Ok(())
    }

    /// Whether the reader may read this label of an event.
    async fn label_readable(
        &self,
        reading: &mut Reading<'_>,
        label: &EventLabel,
    ) -> Result<bool, AppError> {
        match label {
            EventLabel::Grouped(each) => match each.group {
                None => Ok(true),
                Some(group) => self.group_known(reading, group).await,
            },
            EventLabel::Named(name) => {
                self.legacy_names(reading).await?;
                Ok(reading.names.legacy.as_ref().is_some_and(|legacy| {
                    legacy.known.contains(name)
                        || (!legacy.hidden.contains(name) && legacy.every_group)
                }))
            }
        }
    }

    /// The value of a change to the labels: the labels the reader may read. Nothing left to show is
    /// no value.
    pub(super) async fn labels_value(
        &self,
        reading: &mut Reading<'_>,
        value: &Value,
    ) -> Result<Option<ChangeValue>, AppError> {
        let labels = event_labels(value);
        let wanted = !labels.is_empty();
        let mut names = Vec::new();
        for label in &labels {
            if self.label_readable(reading, label).await? {
                names.push(label.name().to_owned());
            }
        }
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
    if !matches!(member, Member::Catalogue(Field::Labels | Field::Tags)) {
        return false;
    }
    let names = |value: Option<&ChangeValue>| match value {
        Some(ChangeValue::Names { names }) => names.clone(),
        _ => Vec::new(),
    };
    names(before) == names(after)
}
