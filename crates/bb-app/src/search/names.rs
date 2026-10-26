//! What the names in a search stand for (ADR-0005 amendment M5, decision 9): a status, an issue
//! type, a choice of an own field, a person, a group, a tag, a label, a project, an issue. **One
//! function resolves every name of a search** ([`Ctx::search_names`]), for running a search and for
//! checking a saved one: each name is turned into the things of that name that this person may know,
//! by the one rule of the same name (`names::same_name`), before the statement is made, so the
//! statement compares IDs and keys and never text. A name the person cannot know is as unknown as a
//! name that does not exist: it is not in the lists and it is told in [`Names::unknown`] with where
//! it stands, so a caller refuses the text ([`Names::names_unknown`]).
//!
//! The rules: a person or a group by *Who sees people* for the people kind of the field;
//! `member of "…"` also needs being in the group or *Manage groups*, as its results show who is in
//! it; a tag or a label only when the person may know it; a project only by its current short name
//! and only when the person sees it; an issue only by its Issue ID, in a project they see, and only
//! when they read the issue.

use bb_core::id::{GroupId, UserId};
use bb_core::issue::StatusGroup;
use bb_core::names::{name_key, same_name};
use bb_core::own_fields::FieldType;
use bb_core::people_kinds::PeopleKind;
use bb_core::readable_id::ReadableId;
use bb_policy::{Permission, Place as Where};
use bb_searchlang::{CheckedSearch, Expr as Node, Operand, Span, Test};

use super::readable::Looked;
use super::request::{Names, ProjectLink, UnknownName};
use super::target::{FieldTarget, target_of};
use crate::access::{Actor, names_other_people};
use crate::auth::{AppError, AuthStore, Ctx, SearchIn, internal};
use crate::issues::Where as IssueWhere;

/// The most people read from the store in one page for a name in a search.
const PEOPLE_PER_NAME: u64 = 100;

/// The most pages read for one name, so a name that fits very many people costs a bounded number of
/// reads (the people past them are not found by that name).
const PEOPLE_PAGES_PER_NAME: usize = 5;

/// The most names that a request resolves with reads (a person, a group, a tag, a label, an
/// issue): the texts of a list of saved searches are written by other people, so what they name
/// has a bound (`LOOKED_AT_MAX` searches of up to 50 names each is 25,000 names otherwise). A name
/// past it is as unknown as one that does not exist, for the rest of the request; a search the
/// person runs holds far fewer.
pub(crate) const NAME_RESOLUTIONS_MAX: usize = 200;

/// What a name in a search is the name of.
#[derive(Clone, Copy, PartialEq, Eq)]
enum What {
    /// A person, in a field whose people are shown as this kind.
    Person(PeopleKind),
    /// A group of people, in a field that holds one.
    Group,
    /// The group after `member of`.
    MemberOf,
    Tag,
    Label,
    Project,
    Issue,
}

/// A name in the text of the search, with where it stands.
struct Ask {
    what: What,
    text: String,
    span: Span,
    field: String,
}

/// What the values of a field name, if they are names this module resolves.
fn what_of(target: FieldTarget) -> Option<What> {
    match target {
        FieldTarget::Tags => Some(What::Tag),
        FieldTarget::Labels => Some(What::Label),
        FieldTarget::Project => Some(What::Project),
        FieldTarget::Parent
        | FieldTarget::Own {
            kind: FieldType::Issue,
            ..
        } => Some(What::Issue),
        FieldTarget::Own {
            kind: FieldType::Group,
            ..
        } => Some(What::Group),
        other => other.people_kind().map(What::Person),
    }
}

fn ask(node: &Node, looked: &Looked, asked: &mut Vec<Ask>) {
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
            let mut put = |what: What, text: &str, span: Span| {
                asked.push(Ask {
                    what,
                    text: text.to_owned(),
                    span,
                    field: field.word.clone(),
                });
            };
            match test {
                Test::MemberOf { group, .. } => put(What::MemberOf, &group.text, group.span),
                Test::Within { project, .. } => put(What::Project, &project.text, project.span),
                Test::Is { value, .. } | Test::IsNot { value, .. } => {
                    put_values(target, std::slice::from_ref(value), &mut put);
                }
                Test::In { values, .. } => put_values(target, values, &mut put),
                _ => {}
            }
        }
        Node::Contains { .. } => {}
        Node::Under { id, .. }
        | Node::Blocks { id, .. }
        | Node::BlockedBy { id, .. }
        | Node::ConnectedTo { id, .. } => asked.push(Ask {
            what: What::Issue,
            text: id.text.clone(),
            span: id.span,
            field: String::new(),
        }),
    }
}

/// The values of a test that are names.
fn put_values(target: FieldTarget, values: &[Operand], put: &mut impl FnMut(What, &str, Span)) {
    let Some(what) = what_of(target) else {
        return;
    };
    for value in values {
        match value {
            Operand::Text { text, span } => put(what, text, *span),
            Operand::Issue { id, span } => put(What::Issue, &id.text, *span),
            _ => {}
        }
    }
}

/// What resolving the names of the texts of one request has read and found, so that each text of a
/// list does not read it again: what every name stood for (also the names that stood for nothing),
/// and the groups, the tags and the labels the person may know, read once.
#[derive(Default)]
pub(crate) struct NameCache {
    /// What the names stood for, with the unknown names of the last text.
    pub(crate) names: Names,
    seeded: bool,
    viewer: Option<std::sync::Arc<bb_policy::Evaluation>>,
    groups: Option<Vec<bb_core::access::Group>>,
    mine: Option<Vec<GroupId>>,
    tags_known: Option<Vec<(String, bb_core::id::TagId)>>,
    labels_known: Option<Vec<(String, bb_core::id::LabelId)>>,
    missing_issues: Vec<String>,
    /// How many names were resolved with reads for the request, against [`NAME_RESOLUTIONS_MAX`].
    resolutions: usize,
}

impl NameCache {
    /// Whether a name that has to be read may be: counts it, and says no past the bound.
    fn may_resolve(&mut self) -> bool {
        if self.resolutions >= NAME_RESOLUTIONS_MAX {
            return false;
        }
        self.resolutions += 1;
        true
    }
}

/// What a text can name that needs no read: the statuses, the issue types and the choices of the
/// own fields, which `looked` holds already.
fn seed(names: &mut Names, looked: &Looked) {
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
}

impl Names {
    /// The groups after `member of`, or the groups of a field that holds one.
    fn group_list(&mut self, member_of: bool) -> &mut Vec<(String, Vec<GroupId>)> {
        if member_of {
            &mut self.member_groups
        } else {
            &mut self.groups
        }
    }

    /// Whether the search names something the person cannot know: a text that is run or saved with
    /// such a name is refused, as it would find nothing or something else (decision 9).
    #[must_use]
    pub fn names_unknown(&self) -> bool {
        !self.unknown.is_empty()
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The things the names of a checked search stand for, for this person, and the names that stand
    /// for nothing they may know. The one function that resolves the names of a search.
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(crate) async fn search_names(
        &self,
        actor: &Actor,
        looked: &Looked,
        checked: &CheckedSearch,
    ) -> Result<Names, AppError> {
        let mut cache = NameCache::default();
        self.search_names_in(actor, looked, checked, &mut cache)
            .await?;
        Ok(cache.names)
    }

    /// The same, for the many texts of one request (the list of the saved searches shared with a
    /// person): what a name stood for, or that it stood for nothing, and what was read to find out
    /// (the groups, the tags and labels, the viewer), stays in `cache`, so each is read once for
    /// the request. `cache.names.unknown` holds the names of the last text only.
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(crate) async fn search_names_in(
        &self,
        actor: &Actor,
        looked: &Looked,
        checked: &CheckedSearch,
        cache: &mut NameCache,
    ) -> Result<(), AppError> {
        if !cache.seeded {
            seed(&mut cache.names, looked);
            cache.seeded = true;
        }
        cache.names.unknown.clear();
        let mut asked: Vec<Ask> = Vec::new();
        if let Some(filter) = &checked.search().filter {
            ask(filter, looked, &mut asked);
        }
        let viewer = if let Some(viewer) = &cache.viewer {
            viewer.clone()
        } else {
            let viewer = self.evaluation_of(actor, Where::Organization).await?;
            cache.viewer = Some(viewer.clone());
            viewer
        };
        for each in &asked {
            let found = self
                .resolve_name(actor, looked, &viewer, each, cache)
                .await?;
            if !found {
                cache.names.unknown.push(UnknownName {
                    span: each.span,
                    field: each.field.clone(),
                    text: each.text.clone(),
                });
            }
        }
        Ok(())
    }

    /// Resolves one name into `cache.names`; whether it stands for anything the person may know.
    async fn resolve_name(
        &self,
        actor: &Actor,
        looked: &Looked,
        viewer: &bb_policy::Evaluation,
        each: &Ask,
        cache: &mut NameCache,
    ) -> Result<bool, AppError> {
        let key = name_key(&each.text);
        match each.what {
            What::Person(kind) => {
                if let Some((_, _, found)) = cache
                    .names
                    .people
                    .iter()
                    .find(|(of, name, _)| *of == kind && *name == key)
                {
                    return Ok(!found.is_empty());
                }
                if !cache.may_resolve() {
                    return Ok(false);
                }
                let found = self.people_named(actor, viewer, kind, &each.text).await?;
                let known = !found.is_empty();
                cache.names.people.push((kind, key, found));
                Ok(known)
            }
            What::Group | What::MemberOf => {
                let member_of = each.what == What::MemberOf;
                // The groups after `member of` and the groups of a group field are told apart.
                if let Some((_, found)) = cache
                    .names
                    .group_list(member_of)
                    .iter()
                    .find(|(name, _)| *name == key)
                {
                    return Ok(!found.is_empty());
                }
                if !cache.may_resolve() {
                    return Ok(false);
                }
                let found = self
                    .groups_named(actor, viewer, &each.text, member_of, cache)
                    .await?;
                let known = !found.is_empty();
                cache.names.group_list(member_of).push((key, found));
                Ok(known)
            }
            What::Tag | What::Label => self.tag_or_label_named(looked, each, cache).await,
            What::Project => Ok(ProjectLink::named_in(
                &looked.seen.links(&looked.projects),
                &each.text,
            )
            .is_some()),
            What::Issue => {
                let Ok(id) = each.text.parse::<ReadableId>() else {
                    return Ok(false);
                };
                let key = id.to_string();
                if cache.names.issues.iter().any(|(name, _)| *name == key) {
                    return Ok(true);
                }
                if cache.missing_issues.contains(&key) {
                    return Ok(false);
                }
                if !cache.may_resolve() {
                    return Ok(false);
                }
                let found = self.issue_named(actor, looked, &id).await?;
                match found {
                    Some(issue) => cache.names.issues.push((key, issue)),
                    None => cache.missing_issues.push(key),
                }
                Ok(found.is_some())
            }
        }
    }

    /// The issue an Issue ID names, when it is in a project the person sees by its current short
    /// name and they read the issue.
    async fn issue_named(
        &self,
        actor: &Actor,
        looked: &Looked,
        id: &ReadableId,
    ) -> Result<Option<bb_core::id::IssueId>, AppError> {
        let links = looked.seen.links(&looked.projects);
        let Some(project) = ProjectLink::named(&links, &id.short_name) else {
            return Ok(None);
        };
        let Some(found) = self
            .store()
            .issue_at(project.id, id.code.clone())
            .await
            .map_err(|error| internal::<S>(&error))?
        else {
            return Ok(None);
        };
        Ok(self
            .seen_in(actor, &found.issue, IssueWhere::Live)
            .await?
            .map(|_| found.issue.id))
    }

    /// The groups of this name the person may name: all of them where they are shown groups by
    /// name; for `member of`, only the groups they are in, unless they manage groups. The groups
    /// and the groups of the person are read once for the cache.
    async fn groups_named(
        &self,
        actor: &Actor,
        viewer: &bb_policy::Evaluation,
        text: &str,
        member_of: bool,
        cache: &mut NameCache,
    ) -> Result<Vec<GroupId>, AppError> {
        if !viewer.sees_names_in(FieldType::Group) {
            return Ok(Vec::new());
        }
        if cache.groups.is_none() {
            cache.groups = Some(
                self.store()
                    .groups()
                    .await
                    .map_err(|error| internal::<S>(&error))?,
            );
        }
        let everyone = !member_of || viewer.allows(Permission::ManageGroups);
        if !everyone && cache.mine.is_none() {
            cache.mine = Some(
                self.store()
                    .groups_of(actor.person.id)
                    .await
                    .map_err(|error| internal::<S>(&error))?
                    .iter()
                    .map(|group| group.id)
                    .collect(),
            );
        }
        let mine = cache.mine.as_deref().unwrap_or_default();
        Ok(cache
            .groups
            .iter()
            .flatten()
            .filter(|group| same_name(&group.name, text) && (everyone || mine.contains(&group.id)))
            .map(|group| group.id)
            .collect())
    }

    /// A tag or a label the search names, among those the person may know (the same rule as the
    /// lists of tags and labels). The catalogues are read once for the cache.
    async fn tag_or_label_named(
        &self,
        looked: &Looked,
        each: &Ask,
        cache: &mut NameCache,
    ) -> Result<bool, AppError> {
        let key = name_key(&each.text);
        if each.what == What::Tag {
            if let Some((_, found)) = cache.names.tags.iter().find(|(name, _)| *name == key) {
                return Ok(!found.is_empty());
            }
            if !cache.may_resolve() {
                return Ok(false);
            }
            if cache.tags_known.is_none() {
                let catalogue = self.tag_catalogue().await?;
                cache.tags_known = Some(
                    self.tags_known(&catalogue, &looked.seen)
                        .await?
                        .into_iter()
                        .map(|tag| (tag.name.clone(), tag.id))
                        .collect(),
                );
            }
            let ids: Vec<_> = cache
                .tags_known
                .iter()
                .flatten()
                .filter(|(name, _)| same_name(name, &each.text))
                .map(|(_, id)| *id)
                .collect();
            let found = !ids.is_empty();
            cache.names.tags.push((key, ids));
            return Ok(found);
        }
        if let Some((_, found)) = cache.names.labels.iter().find(|(name, _)| *name == key) {
            return Ok(!found.is_empty());
        }
        if !cache.may_resolve() {
            return Ok(false);
        }
        if cache.labels_known.is_none() {
            let mut catalogue = self.label_catalogue().await?;
            cache.labels_known = Some(
                self.labels_known(&looked.seen, &mut catalogue, None)
                    .await?
                    .into_iter()
                    .map(|label| (label.name.clone(), label.id))
                    .collect(),
            );
        }
        let ids: Vec<_> = cache
            .labels_known
            .iter()
            .flatten()
            .filter(|(name, _)| same_name(name, &each.text))
            .map(|(_, id)| *id)
            .collect();
        let found = !ids.is_empty();
        cache.names.labels.push((key, ids));
        Ok(found)
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
        // A short name may fit very many people; the pages read for one name are bounded, as
        // the length of a search is.
        for _ in 0..PEOPLE_PAGES_PER_NAME {
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

#[cfg(test)]
mod tests {
    use super::{NAME_RESOLUTIONS_MAX, NameCache};

    #[test]
    fn perf_05_a_request_resolves_no_more_than_a_bounded_number_of_names_with_reads() {
        let mut cache = NameCache::default();
        for _ in 0..NAME_RESOLUTIONS_MAX {
            assert!(cache.may_resolve());
        }
        // Past the bound a name is as unknown as one that does not exist, however many come.
        assert!(!cache.may_resolve());
        assert!(!cache.may_resolve());
    }
}
