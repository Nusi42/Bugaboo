//! Running a search (QRY-04, QRY-05, SEC-17): a text becomes the issues a person may read, a page at
//! a time, the count of all of them, or the count in each group.
//!
//! The steps are always the same and in this order: the fields the person may read where the search
//! looks; the text read and checked against them (a mistake is answered before anything else is
//! done); the names in the search turned into what they stand for for this person; the sight of the
//! person limited to the same places the fields were judged for; and the statement of the store,
//! which compiles the checked tree. The fields, the sight and the names are all decided here, by the
//! rules of the person, and handed to the store as data.

use std::collections::HashMap;

use bb_core::ErrorCode;
use bb_core::id::{IssueId, ProjectId, UserId};
use bb_core::paging::{Cut, page_len, read_len};
use bb_core::tags::IssueSight;
use bb_policy::{Authorized, Masked, PeopleContext, PersonRef, Place as Where};
use bb_searchlang::{Mistake, check, parse_with};
use jiff::tz::TimeZone;
use serde::Serialize;

use super::names::status_groups;
use super::readable::{Looked, SearchScope, readable_of};
use super::request::{GroupKey, SearchFailure, SearchRequest, SearchStore};
use super::target::{FieldTarget, target_of};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::input::cursor_or_invalid;
use crate::issues::{IssueView, PagePlace};

/// Ask for a page of the issues a search finds.
pub struct SearchQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The search, as the canonical text or any text the language reads.
    pub text: String,
    /// Where the search looks.
    pub scope: SearchScope,
    /// Where the page after the last one starts (`next_cursor` of the page before), if any.
    pub cursor: Option<String>,
    /// How many issues a page holds at most.
    pub limit: u32,
}

/// Ask who is searching, before a search is run, so the limit of searches is the person's whether
/// they come with a session or with an access key.
pub struct SearcherQuery {
    /// The secret of the session or the access key of the request.
    pub session_token: String,
}

/// Ask how many issues a search finds.
pub struct SearchCountQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The search.
    pub text: String,
    /// Where the search looks.
    pub scope: SearchScope,
}

/// Ask how many issues a search finds in each group of a field.
pub struct SearchGroupsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The search.
    pub text: String,
    /// Where the search looks.
    pub scope: SearchScope,
    /// The word of the field to group by.
    pub by: String,
}

/// A page of issues. Each issue is shown with the rights of the person in its own project, so the
/// page holds issues that are each authorized by themselves.
#[derive(Debug, Serialize)]
pub struct SearchPage {
    /// The issues, in the order of the search.
    pub items: Vec<Authorized<IssueView>>,
    /// The cursor of the next page, if there is one.
    pub next_cursor: Option<String>,
}

impl Masked for SearchPage {}

/// How many issues a search finds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SearchCount {
    /// The number of issues the person may read that the search finds.
    pub issues: u64,
}

impl Masked for SearchCount {}

/// How many issues a search finds in each group of a field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SearchGroups {
    /// The groups, with the issues without a value last.
    pub groups: Vec<GroupShown>,
    /// Whether more groups than `MAX_GROUPS` were found and the rest left out.
    pub truncated: bool,
}

/// The most groups one answer holds; a field with more values is searched by narrowing the search.
const MAX_GROUPS: usize = 200;

/// One group of a search, the thing it stands for shown as the person may see it (ADR-0022): a
/// person as a `PersonRef`, everything else by its identity or word.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GroupShown {
    /// What the group is.
    pub key: ShownKey,
    /// How many issues the search finds in it.
    pub issues: u64,
}

/// The key of a group, as it is shown.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ShownKey {
    /// A group of anything else than a person: the identity or the word.
    Other(GroupKey),
    /// A person, as the viewer sees them.
    Person(PersonRef),
    /// A tag, by its name if the person may know it.
    Tag {
        /// The tag.
        id: bb_core::id::TagId,
        /// Its name, when the person may know it.
        name: Option<String>,
    },
    /// A label, by its name if the person may know it.
    Label {
        /// The label.
        id: bb_core::id::LabelId,
        /// Its name, when the person may know it.
        name: Option<String>,
    },
}

impl Masked for SearchGroups {}

/// A search that is checked and ready for the store.
struct Prepared {
    request: SearchRequest,
    looked: Looked,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Reads and checks the text, and gathers what the store needs to compile it.
    async fn prepare_search(
        &self,
        actor: &Actor,
        text: &str,
        scope: SearchScope,
    ) -> Result<Prepared, AppError> {
        let looked = self.looked_at(actor, scope).await?;
        let readable = readable_of(&looked.fields);
        let parsed =
            parse_with(text, &readable).map_err(|mistake| AppError::Mistakes(vec![mistake]))?;
        let checked = check(parsed, &readable).map_err(AppError::Mistakes)?;
        let names = self.search_names(actor, &looked, &checked).await?;
        // A name the person cannot know is refused, as unknown as one that does not exist.
        if names.names_unknown() {
            return Err(AppError::Mistakes(
                names
                    .unknown
                    .iter()
                    .map(|each| Mistake::unknown_value(each.span, &each.field, &each.text))
                    .collect(),
            ));
        }
        let mut sight = looked.seen.issue_sight();
        sight
            .all
            .retain(|project| looked.projects.contains(project));
        sight
            .own
            .retain(|project| looked.projects.contains(project));
        let layers = self.setting_layers_of(Some(actor.person.id)).await?;
        let blocking = self.blocking_kinds().await?;
        let request = SearchRequest {
            search: checked,
            fields: looked.fields.clone(),
            sight: IssueSight {
                with_trashed: false,
                ..sight
            },
            me: actor.person.id,
            now: self.now(),
            zone: TimeZone::clone(bb_core::wording::Region::of(&layers).zone()),
            projects: looked.seen.links(&looked.projects),
            names,
            status_groups: status_groups(&looked),
            blocking_kinds: blocking,
            own_reach: looked.own_reach.clone(),
            after: None,
            limit: 0,
        };
        Ok(Prepared { request, looked })
    }
}

fn stopped(error: SearchFailure) -> AppError {
    match error {
        SearchFailure::TooSlow => AppError::Refused(ErrorCode::SearchTooSlow),
        SearchFailure::BadCursor => AppError::Refused(ErrorCode::InvalidRequest),
        SearchFailure::Failed(reason) => AppError::Internal(reason),
    }
}

/// A page of the issues the search finds, in its order.
///
/// # Errors
/// The mistakes of the text; `resource.not-found` for a list about a project the person does not
/// see; `request.invalid` for a cursor that is not one; `search.too-slow` for a search that takes
/// longer than the limit.
pub async fn search<S: AuthStore + SearchStore>(
    ctx: &Ctx<'_, S>,
    query: SearchQuery,
) -> Result<Authorized<SearchPage>, AppError> {
    let actor = ctx.actor(&query.session_token).await?;
    search_page(ctx, &actor, &query, true).await
}

/// The page of a search for a person who is known already; the people each issue is assigned to
/// are read only when `with_people`. The list of a project is this too, with `within`.
///
/// # Errors
/// As [`search`].
pub(crate) async fn search_page<S: AuthStore + SearchStore>(
    ctx: &Ctx<'_, S>,
    actor: &Actor,
    query: &SearchQuery,
    with_people: bool,
) -> Result<Authorized<SearchPage>, AppError> {
    let mut prepared = ctx.prepare_search(actor, &query.text, query.scope).await?;
    let len = page_len(Some(query.limit));
    prepared.request.limit = read_len(len);
    // A cursor is the ID of the last issue of the page before: the store reads its place in the
    // order itself, so no value of any issue is ever written into a cursor.
    prepared.request.after =
        cursor_or_invalid(query.cursor.as_deref(), |text| text.parse::<IssueId>().ok())?;
    let found = ctx
        .store()
        .search_page(&prepared.request)
        .await
        .map_err(stopped)?;
    let page = Cut::of(found, len);
    let next_cursor = page.next_cursor(|hit| hit.issue.id.to_string());
    // The rights and the chain of each project come from the tree the person's view already holds;
    // everything else of the page is read once by the one way a page is made (PERF-N1).
    let seen = &prepared.looked.seen;
    let places: HashMap<ProjectId, PagePlace> = seen
        .seen_places(None)
        .into_iter()
        .filter_map(|(chain, held)| {
            let project = *chain.last()?;
            Some((
                project,
                PagePlace {
                    chain,
                    held,
                    project: seen.stored(project)?.clone(),
                },
            ))
        })
        .collect();
    let issues: Vec<_> = page.items.iter().map(|hit| hit.issue.clone()).collect();
    let items = ctx
        .issue_items(actor, &issues, &places, with_people)
        .await?
        .into_iter()
        .map(|(held, view)| held.authorize(view))
        .collect();
    let organization = ctx.evaluation_of(actor, Where::Organization).await?;
    Ok(organization.authorize(SearchPage { items, next_cursor }))
}

/// The person who searches, by a session or an access key: the one the search is run for.
///
/// # Errors
/// `auth.not-signed-in` without a signed-in person.
pub async fn searcher<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    query: SearcherQuery,
) -> Result<UserId, AppError> {
    Ok(ctx.actor(&query.session_token).await?.person.id)
}

/// How many issues the search finds, the same search without the paging.
///
/// # Errors
/// As [`search`], without the cursor.
pub async fn search_count<S: AuthStore + SearchStore>(
    ctx: &Ctx<'_, S>,
    query: SearchCountQuery,
) -> Result<Authorized<SearchCount>, AppError> {
    let actor = ctx.actor(&query.session_token).await?;
    let prepared = ctx.prepare_search(&actor, &query.text, query.scope).await?;
    let issues = ctx
        .store()
        .search_count(&prepared.request)
        .await
        .map_err(stopped)?;
    let organization = ctx.evaluation_of(&actor, Where::Organization).await?;
    Ok(organization.authorize(SearchCount { issues }))
}

/// How many issues the search finds in each group of a field. A field the person may not read is
/// refused like a field that does not exist.
///
/// # Errors
/// As [`search_count`]; `request.invalid` for a field that is not among the readable ones.
pub async fn search_groups<S: AuthStore + SearchStore>(
    ctx: &Ctx<'_, S>,
    query: SearchGroupsQuery,
) -> Result<Authorized<SearchGroups>, AppError> {
    let actor = ctx.actor(&query.session_token).await?;
    let prepared = ctx.prepare_search(&actor, &query.text, query.scope).await?;
    if !prepared
        .looked
        .fields
        .iter()
        .any(|each| each.field.word == query.by)
    {
        return Err(AppError::Refused(ErrorCode::InvalidRequest));
    }
    let mut found = ctx
        .store()
        .search_groups(&prepared.request, &query.by)
        .await
        .map_err(stopped)?;
    // Over the cap the groups are cut, but the one for the issues without a value stays: it is how
    // many issues the field does not reach, and the answer says it cut the rest (`truncated`).
    let truncated = found.len() > MAX_GROUPS;
    if truncated {
        let none = found
            .iter()
            .position(|group| group.key == GroupKey::Empty)
            .map(|at| found.remove(at));
        found.truncate(MAX_GROUPS - usize::from(none.is_some()));
        found.extend(none);
    }
    let organization = ctx.evaluation_of(&actor, Where::Organization).await?;
    // The store names a group by identity; a person is shown as the viewer may see them.
    let target = target_of(&prepared.looked.fields, &query.by);
    let kind = target.and_then(FieldTarget::people_kind);
    // A tag or a label is shown by its name, among those the person may know.
    let tags = if found
        .iter()
        .any(|group| matches!(group.key, GroupKey::Tag(_)))
    {
        let catalogue = ctx.tag_catalogue().await?;
        ctx.tags_known(&catalogue, &prepared.looked.seen)
            .await?
            .into_iter()
            .cloned()
            .collect()
    } else {
        Vec::new()
    };
    let labels = if found
        .iter()
        .any(|group| matches!(group.key, GroupKey::Label(_)))
    {
        let mut catalogue = ctx.label_catalogue().await?;
        ctx.labels_known(&prepared.looked.seen, &mut catalogue, None)
            .await?
            .into_iter()
            .cloned()
            .collect()
    } else {
        Vec::new()
    };
    // The people of the groups are read in one go, as the people of a page are.
    let people: Vec<_> = found
        .iter()
        .filter_map(|group| match group.key {
            GroupKey::Person(person) => Some(person),
            _ => None,
        })
        .collect();
    ctx.preload_people(&people).await?;
    let mut groups = Vec::with_capacity(found.len());
    for group in found {
        let key = match (group.key, kind) {
            (GroupKey::Person(person), Some(kind)) => ShownKey::Person(
                ctx.person_ref(&organization, kind, PeopleContext::Ordinary, person)
                    .await?,
            ),
            // A person is never given out but through `PersonRef`.
            (GroupKey::Person(_), None) => {
                return Err(AppError::Internal(
                    "a group of people without a kind".into(),
                ));
            }
            (GroupKey::Tag(id), _) => ShownKey::Tag {
                id,
                name: tags
                    .iter()
                    .find(|tag| tag.id == id)
                    .map(|tag| tag.name.clone()),
            },
            (GroupKey::Label(id), _) => ShownKey::Label {
                id,
                name: labels
                    .iter()
                    .find(|label| label.id == id)
                    .map(|label| label.name.clone()),
            },
            (other, _) => ShownKey::Other(other),
        };
        groups.push(GroupShown {
            key,
            issues: group.issues,
        });
    }
    Ok(organization.authorize(SearchGroups { groups, truncated }))
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The kinds of relation that mean *blocks*: those whose forward name is the starting one.
    async fn blocking_kinds(&self) -> Result<Vec<bb_core::id::RelationKindId>, AppError> {
        let blocks =
            bb_core::message::Message::from(bb_core::message::RELATION_KIND_BLOCKS).render();
        Ok(self
            .store()
            .relation_kinds()
            .await
            .map_err(|error| internal::<S>(&error))?
            .into_iter()
            .filter(|kind| bb_core::names::same_name(&kind.forward, &blocks))
            .map(|kind| kind.id)
            .collect())
    }
}
