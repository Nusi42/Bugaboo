//! The filter that selects the issues of a group (QRY-04, M5 WP-08): each group of a search says, as
//! search text, which issues it holds. The text is made here, with the printer of the language,
//! from the name the person may know (ADR-0005 decision 9), so the browser adds it to the search
//! as it is and never builds a filter from an ID. A group whose thing the person may not know
//! (a project they do not see, a person shown only by role) has no filter.

use bb_core::id::GroupId;
use bb_core::own_fields::{DECIMAL_PARTS, FieldType};
use bb_policy::{Evaluation, PersonRef};
use bb_searchlang::{
    Date, Decimal, IssueRef, NO, Operand, Span, YES, empty_filter_text, is_filter_text,
};

use super::readable::Looked;
use super::request::GroupKey;
use super::run::ShownKey;
use super::target::FieldTarget;
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::issues::Where as IssueWhere;

/// Seconds in a minute: the language counts a length of time in whole minutes.
const SECONDS_PER_MINUTE: i64 = 60;

/// The groups of people, read once for a request.
#[derive(Default)]
pub(crate) struct GroupNames(Option<Vec<bb_core::access::Group>>);

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

/// The value of a group of a field of numbers, lengths of time, yes or no and days, as the language
/// writes it. A moment has no value to ask for: its groups are not groups of a day.
fn whole_operand(target: Option<FieldTarget>, whole: i64) -> Option<Operand> {
    let Some(FieldTarget::Own { kind, .. }) = target else {
        return None;
    };
    match kind {
        FieldType::WholeNumber => Some(Operand::Number {
            number: decimal(whole, 1),
            span: Span::default(),
        }),
        FieldType::Decimal => Some(Operand::Number {
            number: decimal(whole, DECIMAL_PARTS),
            span: Span::default(),
        }),
        FieldType::Duration if whole % SECONDS_PER_MINUTE == 0 => Some(Operand::Duration {
            minutes: u32::try_from(whole / SECONDS_PER_MINUTE).ok()?,
            span: Span::default(),
        }),
        FieldType::YesNo => Some(text(if whole == 0 { NO } else { YES })),
        FieldType::Date => {
            let epoch = jiff::civil::date(1970, 1, 1);
            let day = epoch
                .checked_add(jiff::Span::new().try_days(whole).ok()?)
                .ok()?;
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

/// The name of a thing in a list of things the person may know.
fn named<'a, T>(
    things: &'a [T],
    is: impl Fn(&T) -> bool,
    name: impl Fn(&'a T) -> &'a str,
) -> Option<String> {
    things
        .iter()
        .find(|thing| is(thing))
        .map(|thing| name(thing).to_owned())
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The text of the filter of one group of the field `by`, or nothing when the group cannot be
    /// asked for by a name this person may use.
    pub(crate) async fn group_filter(
        &self,
        actor: &Actor,
        viewer: &Evaluation,
        looked: &Looked,
        by: (&str, Option<FieldTarget>),
        key: &ShownKey,
        cache: &mut GroupNames,
    ) -> Result<Option<String>, AppError> {
        let (word, target) = by;
        let value = match key {
            ShownKey::Person(PersonRef::Name { name, .. }) => Some(text(name)),
            ShownKey::Person(_) => None,
            ShownKey::Tag { name, .. } | ShownKey::Label { name, .. } => name.as_deref().map(text),
            ShownKey::Other(GroupKey::Empty) => return Ok(Some(empty_filter_text(word))),
            ShownKey::Other(other) => {
                self.other_group_operand(actor, viewer, looked, (target, other), cache)
                    .await?
            }
        };
        Ok(value.map(|operand| is_filter_text(word, operand)))
    }

    async fn other_group_operand(
        &self,
        actor: &Actor,
        viewer: &Evaluation,
        looked: &Looked,
        (target, key): (Option<FieldTarget>, &GroupKey),
        cache: &mut GroupNames,
    ) -> Result<Option<Operand>, AppError> {
        Ok(match key {
            GroupKey::Priority(priority) => Some(text(priority.search_word())),
            GroupKey::StatusGroup(group) => Some(text(group.search_word())),
            GroupKey::Text(stored) if target == Some(FieldTarget::Status) => looked
                .statuses
                .iter()
                .find(|status| status.key.to_string() == *stored)
                .map(|status| text(&status.name)),
            GroupKey::Text(stored) => Some(text(stored)),
            GroupKey::Whole(whole) => whole_operand(target, *whole),
            GroupKey::Project(id) => looked
                .seen
                .links(&looked.projects)
                .into_iter()
                .find(|link| link.id == *id)
                .map(|link| text(link.short_name.to_string())),
            GroupKey::IssueType(id) => {
                named(&looked.types, |each| each.id == *id, |each| &each.name).map(text)
            }
            GroupKey::Choice(id) => looked
                .own
                .iter()
                .flat_map(|definition| &definition.choices)
                .find(|choice| choice.id == *id)
                .map(|choice| text(&choice.name)),
            GroupKey::Group(id) => self.group_name(viewer, *id, cache).await?.map(text),
            GroupKey::Issue(id) => self.issue_operand(actor, looked, *id).await?,
            GroupKey::Empty | GroupKey::Person(_) | GroupKey::Tag(_) | GroupKey::Label(_) => None,
        })
    }

    /// The name of a group of people, when the person may name groups.
    async fn group_name(
        &self,
        viewer: &Evaluation,
        id: GroupId,
        cache: &mut GroupNames,
    ) -> Result<Option<String>, AppError> {
        if !viewer.sees_names_in(FieldType::Group) {
            return Ok(None);
        }
        if cache.0.is_none() {
            cache.0 = Some(
                self.store()
                    .groups()
                    .await
                    .map_err(|error| internal::<S>(&error))?,
            );
        }
        Ok(cache
            .0
            .iter()
            .flatten()
            .find(|group| group.id == id)
            .map(|group| group.name.clone()))
    }

    /// An issue by its Issue ID, when the person reads it and sees its project.
    async fn issue_operand(
        &self,
        actor: &Actor,
        looked: &Looked,
        id: bb_core::id::IssueId,
    ) -> Result<Option<Operand>, AppError> {
        let found = self
            .store()
            .issues_by_ids(&[id])
            .await
            .map_err(|error| internal::<S>(&error))?;
        let Some(issue) = found.into_iter().next() else {
            return Ok(None);
        };
        let links = looked.seen.links(&looked.projects);
        let Some(link) = links.iter().find(|link| link.id == issue.project) else {
            return Ok(None);
        };
        if self
            .seen_in(actor, &issue, IssueWhere::Live)
            .await?
            .is_none()
        {
            return Ok(None);
        }
        Ok(Some(Operand::Issue {
            id: IssueRef {
                text: format!("{}-{}", link.short_name, issue.code),
                span: Span::default(),
            },
            span: Span::default(),
        }))
    }
}
