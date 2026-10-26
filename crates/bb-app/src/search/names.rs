//! What the names in a search stand for (ADR-0005 amendment M5, decision 9): a status, an issue
//! type, a choice of an own field, a person, a group. Each name is turned into the things of that
//! name that this person may know, by the one rule of the same name (`names::same_name`), before the
//! statement is made, so the statement compares IDs and keys and never text. A name the person
//! cannot know is not in the lists, and is then as unknown as a name that does not exist.

use bb_core::fields::{ASSIGNED_TO_PEOPLE, REPORTED_BY_PEOPLE};
use bb_core::id::{GroupId, UserId};
use bb_core::issue::StatusGroup;
use bb_core::names::{name_key, same_name};
use bb_core::own_fields::FieldType;
use bb_core::people_kinds::PeopleKind;
use bb_policy::Place as Where;
use bb_searchlang::{CheckedSearch, Expr as Node, Operand, Test};

use super::readable::Looked;
use super::request::Names;
use super::target::{FieldTarget, target_of};
use crate::access::{Actor, names_other_people};
use crate::auth::{AppError, AuthStore, Ctx, SearchIn, internal};

/// The most people read from the store for one name in a search.
const PEOPLE_PER_NAME: u64 = 100;

/// Which people or groups a search names, and as what kind of people information.
struct Asked {
    people: Vec<(PeopleKind, String)>,
    groups: Vec<String>,
}

fn people_kind_of(target: FieldTarget) -> Option<PeopleKind> {
    match target {
        FieldTarget::AssignedTo => Some(ASSIGNED_TO_PEOPLE),
        FieldTarget::ReportedBy => Some(REPORTED_BY_PEOPLE),
        FieldTarget::Own { kind, .. } => kind.people_kind().filter(|_| kind != FieldType::Group),
        _ => None,
    }
}

fn texts(values: &[&Operand]) -> Vec<String> {
    values
        .iter()
        .filter_map(|value| match value {
            Operand::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn ask(node: &Node, looked: &Looked, asked: &mut Asked) {
    match node {
        Node::Or { items, .. } | Node::And { items, .. } => {
            for item in items {
                ask(item, looked, asked);
            }
        }
        Node::Not { item, .. } => ask(item, looked, asked),
        Node::Field { field, test, .. } => {
            let Some(target) = target_of(&looked.fields, &field.word) else {
                return;
            };
            let values: Vec<&Operand> = match test {
                Test::Is { value, .. } | Test::IsNot { value, .. } => vec![value],
                Test::In { values, .. } => values.iter().collect(),
                Test::MemberOf { group, .. } => {
                    asked.groups.push(group.text.clone());
                    return;
                }
                _ => return,
            };
            if matches!(
                target,
                FieldTarget::Own {
                    kind: FieldType::Group,
                    ..
                }
            ) {
                asked.groups.extend(texts(&values));
            } else if let Some(kind) = people_kind_of(target) {
                asked
                    .people
                    .extend(texts(&values).into_iter().map(|text| (kind, text)));
            }
        }
        Node::Under { .. }
        | Node::Blocks { .. }
        | Node::BlockedBy { .. }
        | Node::ConnectedTo { .. } => {}
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The things the names of a checked search stand for, for this person.
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(crate) async fn search_names(
        &self,
        actor: &Actor,
        looked: &Looked,
        checked: &CheckedSearch,
    ) -> Result<Names, AppError> {
        let mut names = Names::default();
        for status in &looked.statuses {
            let (key, stored) = (name_key(&status.name), status.key.to_string());
            match names.statuses.iter_mut().find(|(each, _)| *each == key) {
                Some((_, keys)) if !keys.contains(&stored) => keys.push(stored),
                Some(_) => {}
                None => names.statuses.push((key, vec![stored])),
            }
        }
        for found in &looked.types {
            let key = name_key(&found.name);
            match names.issue_types.iter_mut().find(|(each, _)| *each == key) {
                Some((_, ids)) => ids.push(found.id),
                None => names.issue_types.push((key, vec![found.id])),
            }
        }
        for definition in &looked.own {
            if !definition.choices.is_empty() {
                names.choices.push((
                    definition.field.id,
                    definition
                        .choices
                        .iter()
                        .map(|choice| (name_key(&choice.name), choice.id))
                        .collect(),
                ));
            }
        }
        let mut asked = Asked {
            people: Vec::new(),
            groups: Vec::new(),
        };
        if let Some(filter) = &checked.search().filter {
            ask(filter, looked, &mut asked);
        }
        let viewer = self.evaluation_of(actor, Where::Organization).await?;
        for (kind, text) in &asked.people {
            if names.people.iter().any(|(each, _)| *each == name_key(text)) {
                continue;
            }
            let found = self.people_named(actor, &viewer, *kind, text).await?;
            names.people.push((name_key(text), found));
        }
        if viewer.sees_names_in(FieldType::Group) {
            let groups = self
                .store()
                .groups()
                .await
                .map_err(|error| internal::<S>(&error))?;
            for text in &asked.groups {
                if names.groups.iter().any(|(each, _)| *each == name_key(text)) {
                    continue;
                }
                let ids: Vec<GroupId> = groups
                    .iter()
                    .filter(|group| same_name(&group.name, text))
                    .map(|group| group.id)
                    .collect();
                names.groups.push((name_key(text), ids));
            }
        }
        Ok(names)
    }

    /// The people of this name whom the viewer may name: themselves, or anybody when they are
    /// shown people by name for the kind of the field (`names_other_people`). The store gives the
    /// candidates whose names hold the words; the one rule of the same name decides.
    async fn people_named(
        &self,
        actor: &Actor,
        viewer: &bb_policy::Evaluation,
        kind: PeopleKind,
        text: &str,
    ) -> Result<Vec<UserId>, AppError> {
        if !names_other_people(viewer, kind) {
            let own = self
                .store()
                .person(actor.person.id)
                .await
                .map_err(|error| internal::<S>(&error))?;
            return Ok(own
                .filter(|person| is_named(person, text))
                .map(|person| person.id)
                .into_iter()
                .collect());
        }
        let mut found = Vec::new();
        let mut after = None;
        loop {
            let page = self
                .store()
                .people_page(Some(text), SearchIn::Names, after.as_ref(), PEOPLE_PER_NAME)
                .await
                .map_err(|error| internal::<S>(&error))?;
            found.extend(
                page.iter()
                    .filter(|person| is_named(person, text))
                    .map(|person| person.id),
            );
            match page.last() {
                Some(last) if page.len() as u64 == PEOPLE_PER_NAME => {
                    after = Some(crate::access::position_of(last));
                }
                _ => break,
            }
        }
        Ok(found)
    }
}

/// Whether the person has this name by the one rule of the same name: the only place a search reads
/// the name of a person, to compare it with the name typed in the search (no name is shown).
fn is_named(person: &bb_core::person::Person, text: &str) -> bool {
    same_name(&person.display_name, text)
}

/// The groups of status keys by status group, in the order of the groups.
pub(crate) fn status_groups(looked: &Looked) -> Vec<(StatusGroup, Vec<String>)> {
    StatusGroup::ALL
        .iter()
        .map(|group| {
            let mut keys: Vec<String> = Vec::new();
            for status in looked
                .statuses
                .iter()
                .filter(|status| status.group == *group)
            {
                let stored = status.key.to_string();
                if !keys.contains(&stored) {
                    keys.push(stored);
                }
            }
            (*group, keys)
        })
        .collect()
}
