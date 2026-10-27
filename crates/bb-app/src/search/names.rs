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

/// The most names that a request resolves with a read for each name (a person, an Issue ID): the
/// texts of a list of saved searches are written by other people, so what they name is bounded
/// (`LOOKED_AT_MAX` searches of up to `MAX_FILTERS` names each would be 25,000 reads otherwise). A
/// name past it is never answered as unknown: the resolver says it is out of budget
/// ([`NameCache::spent`]) and the list ends its page before that search, which the next request
/// reads again with a budget of its own. The groups, the tags and the labels cost one read of the
/// whole list or catalogue and are not counted.
pub(crate) const NAME_RESOLUTIONS_MAX: usize = 200;

// A search that a person runs (at most `MAX_FILTERS` names) is never out of budget.
const _: () = assert!(NAME_RESOLUTIONS_MAX > bb_searchlang::MAX_FILTERS);

/// How the name typed is compared with the name of a person or a group: the whole name (a search
/// that is run or checked), or the start of it (the names a suggestion offers).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Matching {
    Whole,
    Start,
}

impl Matching {
    /// Whether `name` is what `typed` stands for, by the one rule of the same name.
    fn holds(self, name: &str, typed: &str) -> bool {
        match self {
            Self::Whole => same_name(name, typed),
            Self::Start => name_key(name).starts_with(&name_key(typed)),
        }
    }
}

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
    /// How many names were resolved with a read each for the request, against
    /// [`NAME_RESOLUTIONS_MAX`].
    resolutions: usize,
    /// Whether the last text needed a name resolved past the bound: its answer is not an answer.
    spent: bool,
    /// Whether the bound is lifted: a page that has no search yet goes on to the first one it can
    /// show, so a cursor never has to name a search that was left out.
    lifted: bool,
}

impl NameCache {
    /// Whether a name that has to be read may be: counts it, and says no past the bound (and
    /// remembers that the text is out of budget), unless the bound is lifted.
    fn may_resolve(&mut self) -> bool {
        if self.lifted {
            return true;
        }
        if self.resolutions >= NAME_RESOLUTIONS_MAX {
            self.spent = true;
            return false;
        }
        self.resolutions += 1;
        true
    }

    /// Whether the last text needed a name past the bound, so that its answer cannot be used.
    pub(crate) const fn spent(&self) -> bool {
        self.spent
    }

    /// Lifts the bound for the rest of the request.
    pub(crate) const fn lift(&mut self) {
        self.lifted = true;
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
        cache.spent = false;
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
                    return Ok(true);
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
                    return Ok(true);
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
        Ok(self
            .groups_matching(actor, viewer, text, member_of, Matching::Whole, cache)
            .await?
            .into_iter()
            .map(|group| group.id)
            .collect())
    }

    /// The groups whose names match, among those the person may name (see `groups_named`): the one
    /// place that decides it, for a search and for the names a suggestion offers.
    async fn groups_matching(
        &self,
        actor: &Actor,
        viewer: &bb_policy::Evaluation,
        text: &str,
        member_of: bool,
        matching: Matching,
        cache: &mut NameCache,
    ) -> Result<Vec<bb_core::access::Group>, AppError> {
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
            .filter(|group| {
                matching.holds(&group.name, text) && (everyone || mine.contains(&group.id))
            })
            .cloned()
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
        Ok(self
            .people_matching(actor, viewer, kind, text, Matching::Whole)
            .await?
            .into_iter()
            .map(|person| person.id)
            .collect())
    }

    /// The people whose names match, among those the viewer may name for the kind of the field.
    /// The one place that decides which people a search may name, for running and checking it
    /// (`Matching::Whole`) and for the names a suggestion offers (`Matching::Start`).
    async fn people_matching(
        &self,
        actor: &Actor,
        viewer: &bb_policy::Evaluation,
        kind: PeopleKind,
        text: &str,
        matching: Matching,
    ) -> Result<Vec<bb_core::person::Person>, AppError> {
        if !names_other_people(viewer, kind) {
            let own = self
                .store()
                .person(actor.person.id)
                .await
                .map_err(|error| internal::<S>(&error))?;
            return Ok(own
                .filter(|person| matching.holds(&person.display_name, text))
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
                    .filter(|person| matching.holds(&person.display_name, text))
                    .cloned(),
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
        assert!(!cache.spent());
        // Past the bound the text is out of budget, whatever comes.
        assert!(!cache.may_resolve());
        assert!(cache.spent());
        // Lifted, the page that has no search yet goes on.
        cache.lift();
        assert!(cache.may_resolve());
    }
}

/// The kinds of people a name offered for a suggestion has to be nameable for: a suggestion does
/// not know the field it stands in, so it offers only names that every person field takes.
const OFFERED_KINDS: [PeopleKind; 2] = [PeopleKind::AssignedTo, PeopleKind::ReportedBy];

impl<S: AuthStore> Ctx<'_, S> {
    /// The names of people that this person may use in a search and that start with the words
    /// typed, at most `limit`, in the order of the names.
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(crate) async fn people_name_choices(
        &self,
        actor: &Actor,
        starting_with: &str,
        limit: usize,
    ) -> Result<Vec<String>, AppError> {
        let viewer = self.evaluation_of(actor, Where::Organization).await?;
        let mut names: Option<Vec<(UserId, String)>> = None;
        for kind in OFFERED_KINDS {
            let found: Vec<(UserId, String)> = self
                .people_matching(actor, &viewer, kind, starting_with, Matching::Start)
                .await?
                .into_iter()
                .map(|person| (person.id, person.display_name))
                .collect();
            names = Some(match names {
                None => found,
                Some(before) => before
                    .into_iter()
                    .filter(|(id, _)| found.iter().any(|(other, _)| other == id))
                    .collect(),
            });
        }
        Ok(names
            .unwrap_or_default()
            .into_iter()
            .map(|(_, name)| name)
            .take(limit)
            .collect())
    }

    /// The names of groups that this person may name after `member of` and that start with the
    /// words typed, at most `limit`.
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(crate) async fn group_name_choices(
        &self,
        actor: &Actor,
        starting_with: &str,
        limit: usize,
    ) -> Result<Vec<String>, AppError> {
        let viewer = self.evaluation_of(actor, Where::Organization).await?;
        let mut cache = NameCache::default();
        Ok(self
            .groups_matching(
                actor,
                &viewer,
                starting_with,
                true,
                Matching::Start,
                &mut cache,
            )
            .await?
            .into_iter()
            .map(|group| group.name)
            .take(limit)
            .collect())
    }
}
