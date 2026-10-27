//! The filter that selects the issues of a group (QRY-04, M5 WP-08): each group of a search says, as
//! search text, which issues it holds, and by which name it is shown. The text is made here, with
//! the printer of the language, from the name the person may know (ADR-0005 decision 9), so the
//! browser adds it to the search as it is and never builds a filter from an ID.
//!
//! A filter is **derived and then checked**: the text is read and checked by the language, and its
//! name is resolved by the one function that resolves the names of every search
//! ([`Ctx::search_names_in`]). It is given only when the name stands for exactly the thing of the
//! group; a name that stands for several things (two people of one name, two types of one name,
//! a status renamed to the name of another) or for nothing this person may use gives no filter.

use std::collections::HashMap;

use bb_core::id::UserId;
use bb_core::id::{ChoiceId, FieldId, GroupId, IssueId, IssueTypeId, LabelId, ProjectId, TagId};
use bb_core::issue::Issue;
use bb_core::names::name_key;
use bb_core::own_fields::{DECIMAL_PARTS, FieldType};
use bb_core::own_fields_value::{Cell, FieldValue, date_of};
use bb_core::people_kinds::PeopleKind;
use bb_core::readable_id::ReadableId;
use bb_core::time::SECONDS_PER_MINUTE;
use bb_policy::{Evaluation, PersonRef};
use bb_searchlang::{
    Date, Decimal, IssueRef, NO, Operand, Span, YES, check, empty_filter_text, is_filter_text,
    parse_with,
};

use super::names::NameCache;
use super::readable::{Looked, readable_of};
use super::request::{GroupKey, Names, ProjectLink};
use super::run::{GroupShown, ShownKey};
use super::target::FieldTarget;
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};

/// The thing a group stands for, as far as its name has to stand for exactly it.
enum Thing {
    /// A word of the language or a plain value: nothing to resolve.
    Plain,
    Status(String),
    Type(IssueTypeId),
    Person(PeopleKind, UserId),
    Group(GroupId),
    Tag(TagId),
    Label(LabelId),
    Choice(FieldId, ChoiceId),
    Issue(IssueId),
    Project(ProjectId),
}

/// What a group is asked for: the name it is shown by, the value of its filter (none for the issues
/// without a value) and what the name has to stand for.
struct Wanted {
    name: Option<String>,
    value: Option<Operand>,
    thing: Thing,
}

impl Wanted {
    fn plain(value: Operand) -> Self {
        Self {
            name: None,
            value: Some(value),
            thing: Thing::Plain,
        }
    }

    fn named(name: &str, thing: Thing) -> Self {
        Self {
            name: Some(name.to_owned()),
            value: Some(text(name)),
            thing,
        }
    }
}

type IssuesRead = HashMap<IssueId, Issue>;

fn text(value: impl Into<String>) -> Operand {
    Operand::Text {
        text: value.into(),
        span: Span::default(),
    }
}

/// A number as the language writes it: no zero ends the fraction.
fn decimal(mut value: i64, parts: i64) -> Decimal {
    let mut scale = 0_u8;
    let mut left = parts;
    while left > 1 {
        left /= 10;
        scale += 1;
    }
    while scale > 0 && value % 10 == 0 {
        value /= 10;
        scale -= 1;
    }
    Decimal { value, scale }
}

/// The value of a group of a field of an own kind, as the store counts it, read as the value the
/// field holds (the one decoding of values, `FieldValue::from_cells`).
fn value_of(kind: FieldType, whole: i64) -> Option<FieldValue> {
    let mut cell = Cell {
        text: None,
        number: None,
        date: None,
        reference: None,
    };
    if matches!(kind, FieldType::Date | FieldType::DateTime) {
        cell.date = Some(whole);
    } else {
        cell.number = Some(whole);
    }
    FieldValue::from_cells(kind, &[cell])
}

/// The value as the language writes it, for the types that the language asks for by a value. A
/// moment has no value to ask for: its groups are not groups of a day.
fn operand_of(kind: FieldType, value: &FieldValue) -> Option<Operand> {
    let per_minute = i64::from(SECONDS_PER_MINUTE);
    match (kind, value) {
        (FieldType::WholeNumber, FieldValue::Number(whole)) => Some(Operand::Number {
            number: decimal(*whole, 1),
            span: Span::default(),
        }),
        (FieldType::Decimal, FieldValue::Number(millionths)) => Some(Operand::Number {
            number: decimal(*millionths, DECIMAL_PARTS),
            span: Span::default(),
        }),
        (FieldType::Duration, FieldValue::Number(seconds)) if seconds % per_minute == 0 => {
            Some(Operand::Duration {
                minutes: u32::try_from(seconds / per_minute).ok()?,
                span: Span::default(),
            })
        }
        (FieldType::YesNo, FieldValue::Flag(flag)) => Some(text(if *flag { YES } else { NO })),
        (FieldType::Date, FieldValue::Day(days)) => {
            let day = date_of(*days)?;
            Some(Operand::Date {
                date: Date {
                    year: u16::try_from(day.year()).ok()?,
                    month: u8::try_from(day.month()).ok()?,
                    day: u8::try_from(day.day()).ok()?,
                },
                span: Span::default(),
            })
        }
        _ => None,
    }
}

/// The entry of a list of names for a name key.
fn entry<'a, T>(list: &'a [(String, T)], key: &str) -> Option<&'a T> {
    list.iter()
        .find(|(name, _)| name == key)
        .map(|(_, found)| found)
}

/// Whether the things a name stands for are exactly this one.
fn alone<T: PartialEq>(found: &[T], one: &T) -> bool {
    found.len() == 1 && found[0] == *one
}

/// Whether the name, as the search resolved it, stands for exactly the thing of the group.
fn stands_for_only(names: &Names, links: &[ProjectLink], wanted: &Wanted) -> bool {
    let Some(shown) = &wanted.name else {
        return true;
    };
    let key = name_key(shown);
    match &wanted.thing {
        Thing::Plain => true,
        Thing::Status(stored) => {
            entry(&names.statuses, &key).is_some_and(|keys| alone(keys, stored))
        }
        Thing::Type(id) => entry(&names.issue_types, &key).is_some_and(|ids| alone(ids, id)),
        Thing::Person(kind, id) => names
            .people
            .iter()
            .find(|(of, name, _)| of == kind && *name == key)
            .is_some_and(|(_, _, ids)| alone(ids, id)),
        Thing::Group(id) => entry(&names.groups, &key).is_some_and(|ids| alone(ids, id)),
        Thing::Tag(id) => entry(&names.tags, &key).is_some_and(|ids| alone(ids, id)),
        Thing::Label(id) => entry(&names.labels, &key).is_some_and(|ids| alone(ids, id)),
        Thing::Choice(field, id) => names
            .choices
            .iter()
            .find(|(of, _)| of == field)
            .is_some_and(|(_, list)| {
                let same: Vec<ChoiceId> = list
                    .iter()
                    .filter(|(name, _)| *name == key)
                    .map(|(_, choice)| *choice)
                    .collect();
                alone(&same, id)
            }),
        Thing::Issue(id) => shown.parse::<ReadableId>().is_ok_and(|readable| {
            let wanted_key = readable.to_string();
            names
                .issues
                .iter()
                .any(|(name, found)| *name == wanted_key && found == id)
        }),
        Thing::Project(id) => {
            ProjectLink::named_in(links, shown).is_some_and(|link| link.id == *id)
        }
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The groups of a search, each with the name it is shown by and the filter that selects its
    /// issues. The issues the groups stand for are read in one go.
    pub(crate) async fn groups_with_filters(
        &self,
        actor: &Actor,
        viewer: &Evaluation,
        looked: &Looked,
        by: (&str, Option<FieldTarget>),
        keyed: Vec<(ShownKey, u64)>,
    ) -> Result<Vec<GroupShown>, AppError> {
        let (word, target) = by;
        let readable = readable_of(&looked.fields);
        let links = looked.seen.links(&looked.projects);
        let issue_ids: Vec<IssueId> = keyed
            .iter()
            .filter_map(|(key, _)| match key {
                ShownKey::Other(GroupKey::Issue(id)) => Some(*id),
                _ => None,
            })
            .collect();
        let issues: IssuesRead = if issue_ids.is_empty() {
            HashMap::new()
        } else {
            self.store()
                .issues_by_ids(&issue_ids)
                .await
                .map_err(|error| internal::<S>(&error))?
                .into_iter()
                .map(|issue| (issue.id, issue))
                .collect()
        };
        // The names of all the groups are resolved with one cache, as the texts of a list of saved
        // searches are, and never out of budget: a group is one name.
        let mut cache = NameCache::default();
        cache.lift();
        let mut groups = Vec::with_capacity(keyed.len());
        for (key, count) in keyed {
            let wanted = self
                .wanted_of(viewer, looked, &links, (target, &issues), &key, &mut cache)
                .await?;
            let mut filter = None;
            let mut name = None;
            if let Some(wanted) = wanted {
                let written = match wanted.value.clone() {
                    Some(value) => is_filter_text(word, value),
                    None => empty_filter_text(word),
                };
                if let Some(written) = written
                    && let Ok(parsed) = parse_with(&written, &readable)
                    && let Ok(checked) = check(parsed, &readable)
                {
                    self.search_names_in(actor, looked, &checked, &mut cache)
                        .await?;
                    if !cache.names.names_unknown()
                        && stands_for_only(&cache.names, &links, &wanted)
                    {
                        filter = Some(written);
                    }
                }
                name = wanted.name;
            }
            groups.push(GroupShown {
                key,
                issues: count,
                filter,
                name,
            });
        }
        Ok(groups)
    }

    /// What the group asks for, or nothing when it has no name this person may use.
    async fn wanted_of(
        &self,
        viewer: &Evaluation,
        looked: &Looked,
        links: &[ProjectLink],
        (target, issues): (Option<FieldTarget>, &IssuesRead),
        key: &ShownKey,
        cache: &mut NameCache,
    ) -> Result<Option<Wanted>, AppError> {
        let kind = target.and_then(FieldTarget::people_kind);
        Ok(match key {
            ShownKey::Person(PersonRef::Name { id, name }) => {
                kind.map(|kind| Wanted::named(name, Thing::Person(kind, *id)))
            }
            ShownKey::Person(_) => None,
            ShownKey::Tag { id, name } => name
                .as_deref()
                .map(|name| Wanted::named(name, Thing::Tag(*id))),
            ShownKey::Label { id, name } => name
                .as_deref()
                .map(|name| Wanted::named(name, Thing::Label(*id))),
            ShownKey::Other(GroupKey::Empty) => Some(Wanted {
                name: None,
                value: None,
                thing: Thing::Plain,
            }),
            ShownKey::Other(other) => {
                self.other_wanted(viewer, looked, links, (target, issues), other, cache)
                    .await?
            }
        })
    }

    async fn other_wanted(
        &self,
        viewer: &Evaluation,
        looked: &Looked,
        links: &[ProjectLink],
        (target, issues): (Option<FieldTarget>, &IssuesRead),
        key: &GroupKey,
        cache: &mut NameCache,
    ) -> Result<Option<Wanted>, AppError> {
        Ok(match key {
            GroupKey::Priority(priority) => Some(Wanted::plain(text(priority.search_word()))),
            GroupKey::StatusGroup(group) => Some(Wanted::plain(text(group.search_word()))),
            GroupKey::Text(stored) if target == Some(FieldTarget::Status) => looked
                .statuses
                .iter()
                .find(|status| status.key.to_string() == *stored)
                .map(|status| Wanted::named(&status.name, Thing::Status(stored.clone()))),
            GroupKey::Text(stored) => Some(Wanted::plain(text(stored))),
            GroupKey::Whole(whole) => match target {
                Some(FieldTarget::Own { kind, .. }) => value_of(kind, *whole)
                    .and_then(|value| operand_of(kind, &value))
                    .map(Wanted::plain),
                _ => None,
            },
            GroupKey::Project(id) => links
                .iter()
                .find(|link| link.id == *id)
                .map(|link| Wanted::named(&link.short_name.to_string(), Thing::Project(*id))),
            GroupKey::IssueType(id) => looked
                .types
                .iter()
                .find(|each| each.id == *id)
                .map(|each| Wanted::named(&each.name, Thing::Type(*id))),
            GroupKey::Choice(id) => looked
                .own
                .iter()
                .find_map(|definition| {
                    definition
                        .choices
                        .iter()
                        .find(|choice| choice.id == *id)
                        .map(|choice| (definition.field.id, choice))
                })
                .map(|(field, choice)| Wanted::named(&choice.name, Thing::Choice(field, *id))),
            GroupKey::Group(id) => self
                .group_name_of(viewer, *id, cache)
                .await?
                .map(|name| Wanted::named(&name, Thing::Group(*id))),
            GroupKey::Issue(id) => issues.get(id).and_then(|issue| {
                let link = links.iter().find(|link| link.id == issue.project)?;
                let readable = format!("{}-{}", link.short_name, issue.code);
                Some(Wanted {
                    value: Some(Operand::Issue {
                        id: IssueRef {
                            text: readable.clone(),
                            span: Span::default(),
                        },
                        span: Span::default(),
                    }),
                    name: Some(readable),
                    thing: Thing::Issue(*id),
                })
            }),
            GroupKey::Empty | GroupKey::Person(_) | GroupKey::Tag(_) | GroupKey::Label(_) => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use bb_core::id::{ChoiceId, FieldId, IssueTypeId, LabelId, SequentialIds};

    use super::*;

    fn names_of(wanted: &Wanted) -> String {
        wanted.name.clone().unwrap_or_default()
    }

    #[test]
    fn qry_04_a_name_of_two_statuses_of_two_workflows_is_no_filter_but_of_one_is() {
        let wanted = Wanted::named("Review", Thing::Status("review".into()));
        let key = name_key(&names_of(&wanted));
        let mut names = Names::default();
        names.statuses.push((key.clone(), vec!["review".into()]));
        assert!(stands_for_only(&names, &[], &wanted));
        // A status of another workflow was renamed to the same name: the name stands for both.
        names.statuses[0].1.push("in-review".into());
        assert!(!stands_for_only(&names, &[], &wanted));
        // A name that stands for another status only is not this group's.
        names.statuses[0].1 = vec!["in-review".into()];
        assert!(!stands_for_only(&names, &[], &wanted));
    }

    #[test]
    fn qry_04_two_types_two_labels_and_two_choices_of_one_name_are_no_filter() {
        let mut ids = SequentialIds::default();
        let (one, two) = (
            IssueTypeId::generate(&mut ids).unwrap(),
            IssueTypeId::generate(&mut ids).unwrap(),
        );
        let wanted = Wanted::named("Question", Thing::Type(one));
        let key = name_key("Question");
        let mut names = Names::default();
        names.issue_types.push((key.clone(), vec![one]));
        assert!(stands_for_only(&names, &[], &wanted));
        names.issue_types[0].1.push(two);
        assert!(!stands_for_only(&names, &[], &wanted));

        let (first, second) = (
            LabelId::generate(&mut ids).unwrap(),
            LabelId::generate(&mut ids).unwrap(),
        );
        let wanted = Wanted::named("Urgent", Thing::Label(first));
        let key = name_key("Urgent");
        names.labels.push((key.clone(), vec![first]));
        assert!(stands_for_only(&names, &[], &wanted));
        names.labels[0].1.push(second);
        assert!(!stands_for_only(&names, &[], &wanted));

        let field = FieldId::generate(&mut ids).unwrap();
        let (major, other) = (
            ChoiceId::generate(&mut ids).unwrap(),
            ChoiceId::generate(&mut ids).unwrap(),
        );
        let wanted = Wanted::named("Major", Thing::Choice(field, major));
        let key = name_key("Major");
        names.choices.push((field, vec![(key.clone(), major)]));
        assert!(stands_for_only(&names, &[], &wanted));
        names.choices[0].1.push((key, other));
        assert!(!stands_for_only(&names, &[], &wanted));
    }

    #[test]
    fn qry_04_a_value_is_decoded_by_the_one_decoding_of_values() {
        // Ninety minutes, a half and a day as the store counts them.
        let value = |kind, whole| value_of(kind, whole).and_then(|value| operand_of(kind, &value));
        assert!(matches!(
            value(FieldType::Duration, 5_400),
            Some(Operand::Duration { minutes: 90, .. })
        ));
        assert!(value(FieldType::Duration, 59).is_none());
        assert!(matches!(
            value(FieldType::Decimal, 1_500_000),
            Some(Operand::Number {
                number: Decimal {
                    value: 15,
                    scale: 1
                },
                ..
            })
        ));
        assert!(matches!(
            value(FieldType::Date, 0),
            Some(Operand::Date {
                date: Date {
                    year: 1970,
                    month: 1,
                    day: 1
                },
                ..
            })
        ));
        assert!(value(FieldType::DateTime, 0).is_none());
        assert!(matches!(
            value(FieldType::YesNo, 0),
            Some(Operand::Text { .. })
        ));
    }
}
