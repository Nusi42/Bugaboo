//! How the totals are shown: the keys of the groups as the viewer may see them, and each sum in the
//! unit of its field.

use bb_core::id::UserId;
use bb_core::own_fields::FieldType;
use bb_core::own_fields_value::FieldValue;
use bb_policy::{Evaluation, PeopleContext};
use serde::Serialize;

use super::super::readable::Looked;
use super::super::request::{GroupKey, Total};
use super::super::run::{GroupShown, ShownKey};
use super::super::target::FieldTarget;
use crate::auth::{AppError, AuthStore, Ctx};

/// One group of a total, shown as the reader may see it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GroupTotalShown {
    /// The group, with its count, its filter and its name.
    pub group: GroupShown,
    /// The sums, in the order asked for.
    pub sums: Vec<SumShown>,
}

/// The count and the sums of some issues, as the answer gives them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TotalShown {
    /// How many issues.
    pub issues: u64,
    /// The sums, in the order asked for.
    pub sums: Vec<SumShown>,
}

/// One sum, as the answer gives it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SumShown {
    /// The sum: whole minutes for a length and the count for `count`, a value in the form of the
    /// field for an own field (a decimal as text); none when no issue gave a value.
    pub value: Option<serde_json::Value>,
    /// How many issues gave a value the reader may read.
    pub valued: u64,
}

/// What a word of the sums is in, with the place of its sum among those of the store.
#[derive(Clone, Copy)]
pub(super) enum Unit {
    Count,
    Minutes(usize),
    Own(usize, FieldType),
}

impl TotalShown {
    pub(super) fn of(total: &Total, units: &[Unit]) -> Self {
        let sum = |at: usize| total.sums.get(at).copied().unwrap_or_default();
        Self {
            issues: total.issues,
            sums: units
                .iter()
                .map(|unit| match *unit {
                    Unit::Count => SumShown {
                        value: Some(serde_json::Value::from(total.issues)),
                        valued: total.issues,
                    },
                    Unit::Minutes(at) => SumShown {
                        value: sum(at).sum.map(serde_json::Value::from),
                        valued: sum(at).valued,
                    },
                    Unit::Own(at, kind) => SumShown {
                        value: sum(at)
                            .sum
                            .and_then(|value| FieldValue::Number(value).to_json(kind)),
                        valued: sum(at).valued,
                    },
                })
                .collect(),
        }
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The keys of groups as the viewer may see them: a person as a `PersonRef` of the kind of the
    /// field, a tag and a label by the name the person may know, everything else by its identity
    /// or word. The people, their roles, the tags and the labels are each read once.
    pub(crate) async fn shown_keys(
        &self,
        viewer: &Evaluation,
        looked: &Looked,
        target: FieldTarget,
        keys: &[GroupKey],
    ) -> Result<Vec<ShownKey>, AppError> {
        let kind = target.people_kind();
        let tags = if keys.iter().any(|key| matches!(key, GroupKey::Tag(_))) {
            let catalogue = self.tag_catalogue().await?;
            self.tags_known(&catalogue, &looked.seen)
                .await?
                .into_iter()
                .cloned()
                .collect()
        } else {
            Vec::new()
        };
        let labels = if keys.iter().any(|key| matches!(key, GroupKey::Label(_))) {
            let mut catalogue = self.label_catalogue().await?;
            self.labels_known(&looked.seen, &mut catalogue, None)
                .await?
                .into_iter()
                .cloned()
                .collect()
        } else {
            Vec::new()
        };
        let people: Vec<UserId> = keys
            .iter()
            .filter_map(|key| match key {
                GroupKey::Person(person) => Some(*person),
                _ => None,
            })
            .collect();
        self.preload_people(&people).await?;
        // People shown by their role have their roles read at once, not one by one (PERF-05).
        if let Some(kind) = kind {
            let by_role: Vec<_> = people
                .iter()
                .filter(|person| viewer.shows_role_label(kind, PeopleContext::Ordinary, **person))
                .map(|person| (*person, viewer.place()))
                .collect();
            self.preload_role_labels(&looked.seen, &by_role).await?;
        }
        let mut shown = Vec::with_capacity(keys.len());
        for key in keys {
            shown.push(match (key, kind) {
                (GroupKey::Person(person), Some(kind)) => ShownKey::Person(
                    self.person_ref(viewer, kind, PeopleContext::Ordinary, *person)
                        .await?,
                ),
                // A person is never given out but through `PersonRef`.
                (GroupKey::Person(_), None) => {
                    return Err(AppError::Internal(
                        "a group of people without a kind".into(),
                    ));
                }
                (GroupKey::Tag(id), _) => ShownKey::Tag {
                    id: *id,
                    name: tags
                        .iter()
                        .find(|tag: &&bb_core::tags::Tag| tag.id == *id)
                        .map(|tag| tag.name.clone()),
                },
                (GroupKey::Label(id), _) => ShownKey::Label {
                    id: *id,
                    name: labels
                        .iter()
                        .find(|label: &&bb_core::tags::Label| label.id == *id)
                        .map(|label| label.name.clone()),
                },
                (other, _) => ShownKey::Other(other.clone()),
            });
        }
        Ok(shown)
    }
}
