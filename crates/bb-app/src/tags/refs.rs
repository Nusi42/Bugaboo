//! A label as the events of an issue keep it (LM1 of the M4 exit review): its name and the group of
//! labels it was in. The group is what a reader of the history is judged by, so a renamed or
//! combined label of a group the reader may not know never shows its old name. Events from before
//! this held names only ([`EventLabel::Named`]); a reader treats them with care.

use bb_core::id::LabelGroupId;
use bb_core::tags::Label;
use serde_json::{Value, json};

/// A tag or a label with the group it is in (a tag has none).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NameRef {
    /// What it is called.
    pub(crate) name: String,
    /// The label group a label is in; none for a tag and for a label in no group.
    pub(crate) group: Option<LabelGroupId>,
}

impl From<&Label> for NameRef {
    /// A label with the group it is in: the one way to write down which label an event is about.
    fn from(label: &Label) -> Self {
        Self {
            name: label.name.clone(),
            group: label.group,
        }
    }
}

impl NameRef {
    /// A name with no group.
    pub(crate) const fn plain(name: String) -> Self {
        Self { name, group: None }
    }
}

/// The names of a list.
pub(crate) fn names_of(refs: &[NameRef]) -> Vec<String> {
    refs.iter().map(|each| each.name.clone()).collect()
}

/// The labels of an event as it keeps them: an object with the name and the group for each.
pub(crate) fn labels_json(refs: &[NameRef]) -> Value {
    Value::Array(
        refs.iter()
            .map(|each| json!({"name": each.name, "group": each.group.map(|group| group.to_string())}))
            .collect(),
    )
}

/// One label in an event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum EventLabel {
    /// A name only: an event from before the group was kept.
    Named(String),
    /// A name and the group the label was in.
    Grouped(NameRef),
}

impl EventLabel {
    /// The name.
    pub(crate) fn name(&self) -> &str {
        match self {
            Self::Named(name) => name,
            Self::Grouped(each) => &each.name,
        }
    }
}

/// The labels a value of the member `labels` of an event holds, old or new.
pub(crate) fn event_labels(value: &Value) -> Vec<EventLabel> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| match item {
                    Value::String(name) => Some(EventLabel::Named(name.clone())),
                    Value::Object(fields) => Some(EventLabel::Grouped(NameRef {
                        name: fields.get("name")?.as_str()?.to_owned(),
                        group: fields
                            .get("group")
                            .and_then(Value::as_str)
                            .and_then(|text| text.parse().ok()),
                    })),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use bb_core::id::SequentialIds;

    use super::*;

    #[test]
    fn lm1_a_label_in_an_event_keeps_its_group_and_an_old_event_holds_a_name() {
        let group = LabelGroupId::generate(&mut SequentialIds::default()).expect("an ID");
        let refs = vec![
            NameRef {
                name: "login".to_owned(),
                group: Some(group),
            },
            NameRef::plain("loose".to_owned()),
        ];
        let read = event_labels(&labels_json(&refs));
        assert_eq!(
            read,
            refs.into_iter()
                .map(EventLabel::Grouped)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            event_labels(&json!(["old"])),
            [EventLabel::Named("old".to_owned())]
        );
    }
}
