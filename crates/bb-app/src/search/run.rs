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
use bb_core::wording::Region;
use bb_policy::{Authorized, Masked, PersonRef, Place as Where};
use bb_searchlang::{Expr, Mistake, check, parse_with};
use serde::Serialize;

use super::names::status_groups;
use super::readable::{Looked, SearchScope, readable_of};
use super::request::{GroupKey, SearchFailure, SearchRequest, SearchStore, WithheldText};
use crate::access::Actor;
use crate::ai::Withheld;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::input::cursor_or_invalid;
use crate::issues::{IssueView, PagePlace, ReadOnRequest, WithAgent};

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
    /// Whether the labels of each issue are wanted: they are read only then, once for the page.
    pub labels: bool,
    /// Whether the releases of each issue are wanted: they are read only then, once for the page.
    pub releases: crate::issues::WithReleases,
    /// Whether the agent that works on each issue is wanted: read only then, once for the page.
    pub agent: WithAgent,
    /// Whether *Time spent* and the worked-out *Time left* of each issue are wanted: the time
    /// spent is read only then, once for the page.
    pub time: bool,
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
const MAX_GROUPS: usize = super::totals::GROUPS_PAGE;

/// One group of a search, the thing it stands for shown as the person may see it (ADR-0022): a
/// person as a `PersonRef`, everything else by its identity or word.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GroupShown {
    /// What the group is.
    pub key: ShownKey,
    /// How many issues the search finds in it.
    pub issues: u64,
    /// The filter that selects exactly its issues, as search text, written by the printer of the
    /// language with the name the person may know and checked by the one resolution of names; none
    /// when the group has no such name or the name stands for more than the group.
    pub filter: Option<String>,
    /// The name the group is shown by (the name of its filter), when it has one the person may use.
    pub name: Option<String>,
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
pub(crate) struct Prepared {
    pub(crate) request: SearchRequest,
    pub(crate) looked: Looked,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Reads and checks the text, and gathers what the store needs to compile it.
    pub(crate) async fn prepare_search(
        &self,
        actor: &Actor,
        text: &str,
        scope: SearchScope,
    ) -> Result<Prepared, AppError> {
        self.prepare_search_as(actor, text, scope, None).await
    }

    /// The same for the AI (AI-03): what is kept from AI is not among the fields, the projects or
    /// the names a search may use.
    pub(crate) async fn prepare_search_within(
        &self,
        actor: &Actor,
        text: &str,
        scope: SearchScope,
        withheld: &Withheld,
    ) -> Result<Prepared, AppError> {
        self.prepare_search_as(actor, text, scope, Some(withheld))
            .await
    }

    /// The search for the person (`withheld` none) or for the AI, which reads it without what is
    /// kept from AI: for a use case that serves both.
    pub(crate) async fn prepare_search_as(
        &self,
        actor: &Actor,
        text: &str,
        scope: SearchScope,
        withheld: Option<&Withheld>,
    ) -> Result<Prepared, AppError> {
        self.prepare(actor, text, scope, withheld, |_| None).await
    }

    /// The same with a filter added to the text: what `narrow` makes from the fields the person may
    /// read (the timeline keeps the issues with a date, or those with none), joined to the filter
    /// of the text with `and`, then checked like the rest.
    pub(crate) async fn prepare_search_narrowed(
        &self,
        actor: &Actor,
        text: &str,
        scope: SearchScope,
        narrow: impl FnOnce(&Looked) -> Option<Expr>,
    ) -> Result<Prepared, AppError> {
        self.prepare(actor, text, scope, None, narrow).await
    }

    /// The one way a search is prepared: for the person, or for the AI with what is kept from AI.
    async fn prepare(
        &self,
        actor: &Actor,
        text: &str,
        scope: SearchScope,
        withheld: Option<&Withheld>,
        narrow: impl FnOnce(&Looked) -> Option<Expr>,
    ) -> Result<Prepared, AppError> {
        let looked = self.looked_at(actor, scope, withheld).await?;
        self.prepare_in(actor, looked, text, withheld, narrow).await
    }

    /// The same, with what the person may search by read already (a view reads it once to write
    /// its project into the text).
    pub(crate) async fn prepare_in(
        &self,
        actor: &Actor,
        looked: Looked,
        text: &str,
        withheld: Option<&Withheld>,
        narrow: impl FnOnce(&Looked) -> Option<Expr>,
    ) -> Result<Prepared, AppError> {
        let readable = readable_of(&looked.fields);
        let mut parsed =
            parse_with(text, &readable).map_err(|mistake| AppError::Mistakes(vec![mistake]))?;
        if let Some(extra) = narrow(&looked) {
            narrow_by(&mut parsed, extra);
        }
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
        let region = Region::of(&self.setting_layers_of(Some(actor.person.id)).await?);
        let blocking = self.blocking_kinds().await?;
        let releases_known = if names_a_release(&checked, &looked) {
            self.releases_known(&looked).await?
        } else {
            Vec::new()
        };
        let request = SearchRequest {
            search: checked,
            fields: looked.fields.clone(),
            sight: IssueSight {
                with_trashed: false,
                ..sight
            },
            me: actor.person.id,
            now: self.now(),
            zone: region.zone().clone(),
            first_day: region.first_day(),
            projects: looked.seen.links(&looked.projects),
            names,
            releases_known,
            status_groups: status_groups(&looked),
            blocking_kinds: blocking,
            own_reach: looked.own_reach.clone(),
            withheld_text: withheld.map_or_else(WithheldText::default, Withheld::text),
            only: self.cut_to_work(actor),
            after: None,
            limit: 0,
        };
        Ok(Prepared { request, looked })
    }
}

/// Joins `extra` to the filter of `search` with `and`: the one way a use case narrows a search (the
/// timeline to its dates, a board to a column, a calendar to its window).
pub(crate) fn narrow_by(search: &mut bb_searchlang::Search, extra: Expr) {
    search.filter = Some(match search.filter.take() {
        Some(filter) => Expr::And {
            items: vec![filter, extra],
            span: search.span,
        },
        None => extra,
    });
}

/// Whether a field of the search is a release field, in its filter or its order: only then does the
/// request read the releases the person may know.
fn names_a_release(checked: &bb_searchlang::CheckedSearch, looked: &Looked) -> bool {
    fn in_filter(node: &Expr, looked: &Looked) -> bool {
        match node {
            Expr::Or { items, .. } | Expr::And { items, .. } => {
                items.iter().any(|item| in_filter(item, looked))
            }
            Expr::Not { item, .. } => in_filter(item, looked),
            Expr::Field { field, .. } => is_release(looked, &field.word),
            _ => false,
        }
    }
    let search = checked.search();
    search
        .filter
        .as_ref()
        .is_some_and(|filter| in_filter(filter, looked))
        || search
            .sort
            .iter()
            .any(|key| is_release(looked, &key.field.word))
}

/// Whether the word is the word of a release field.
fn is_release(looked: &Looked, word: &str) -> bool {
    matches!(
        super::target::target_of(&looked.fields, word),
        Some(super::FieldTarget::FoundInRelease | super::FieldTarget::PlannedForRelease)
    )
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The IDs of the releases the person may know where the search looks: those of the projects
    /// they see (and the AI may see), also the retired ones.
    async fn releases_known(
        &self,
        looked: &Looked,
    ) -> Result<Vec<bb_core::id::ReleaseId>, AppError> {
        Ok(self
            .releases_seen(&looked.seen)
            .await?
            .into_iter()
            .map(|release| release.id)
            .collect())
    }
}

/// What a person is answered when a search did not run.
pub(crate) fn stopped(error: SearchFailure) -> AppError {
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
    let prepared = ctx.prepare_search(actor, &query.text, query.scope).await?;
    page_of(ctx, actor, query, with_people, prepared).await
}

/// The same page for the AI (AI-03): what is kept from AI is not looked at.
///
/// # Errors
/// As [`search`].
pub(crate) async fn search_page_within<S: AuthStore + SearchStore>(
    ctx: &Ctx<'_, S>,
    actor: &Actor,
    query: &SearchQuery,
    with_people: bool,
    withheld: &Withheld,
) -> Result<Authorized<SearchPage>, AppError> {
    let prepared = ctx
        .prepare_search_within(actor, &query.text, query.scope, withheld)
        .await?;
    page_of(ctx, actor, query, with_people, prepared).await
}

/// The page a prepared search finds, for [`search_page`] and [`search_page_within`].
async fn page_of<S: AuthStore + SearchStore>(
    ctx: &Ctx<'_, S>,
    actor: &Actor,
    query: &SearchQuery,
    with_people: bool,
    mut prepared: Prepared,
) -> Result<Authorized<SearchPage>, AppError> {
    let found = ctx
        .issues_page(
            actor,
            &mut prepared,
            query.cursor.as_deref(),
            query.limit,
            ReadOnRequest {
                people: with_people,
                labels: query.labels,
                agent: query.agent,
                time: query.time,
                releases: query.releases,
            },
        )
        .await?;
    let items = found
        .items
        .into_iter()
        .map(|(held, view)| held.authorize(view))
        .collect();
    let organization = ctx.evaluation_of(actor, Where::Organization).await?;
    Ok(organization.authorize(SearchPage {
        items,
        next_cursor: found.next_cursor,
    }))
}

/// A page of issues a prepared search finds: the issues as stored, the view of each with what the
/// person holds in its project, and the cursor of the next page.
pub(crate) struct FoundPage {
    /// The issues, in the order of the search.
    pub(crate) issues: Vec<bb_core::issue::Issue>,
    /// The view of each, with the rights of the person in its project.
    pub(crate) items: Vec<(std::sync::Arc<bb_policy::Evaluation>, IssueView)>,
    /// The cursor of the next page, if there is one.
    pub(crate) next_cursor: Option<String>,
}

impl<S: AuthStore + SearchStore> Ctx<'_, S> {
    /// The one way a page of a search is read: the statement of the store, then everything else of
    /// the page once (PERF-N1). The search and the timeline both read their pages so.
    ///
    /// # Errors
    /// As [`search`].
    pub(crate) async fn issues_page(
        &self,
        actor: &Actor,
        prepared: &mut Prepared,
        cursor: Option<&str>,
        limit: u32,
        extras: ReadOnRequest,
    ) -> Result<FoundPage, AppError> {
        let (issues, next_cursor) = self
            .found_issues(&mut prepared.request, cursor, limit)
            .await?;
        let items = self
            .page_items(actor, &prepared.looked.seen, &issues, extras)
            .await?;
        Ok(FoundPage {
            issues,
            items,
            next_cursor,
        })
    }

    /// The issues of a page of a prepared search as stored, in its order, and the cursor of the
    /// next page: the statement of the store only. A view that reads several pages (the cells of a
    /// board) reads what they show once for all of them ([`Ctx::page_items`]).
    ///
    /// # Errors
    /// As [`search`].
    pub(crate) async fn found_issues(
        &self,
        request: &mut SearchRequest,
        cursor: Option<&str>,
        limit: u32,
    ) -> Result<(Vec<bb_core::issue::Issue>, Option<String>), AppError> {
        let len = page_len(Some(limit));
        request.limit = read_len(len);
        // A cursor is the ID of the last issue of the page before: the store reads its place in the
        // order itself, so no value of any issue is ever written into a cursor.
        request.after = cursor_or_invalid(cursor, |text| text.parse::<IssueId>().ok())?;
        let found = self.store().search_page(request).await.map_err(stopped)?;
        let page = Cut::of(found, len);
        let next_cursor = page.next_cursor(|hit| hit.issue.id.to_string());
        Ok((
            page.items.into_iter().map(|hit| hit.issue).collect(),
            next_cursor,
        ))
    }

    /// The view of each issue of a page, with what the person holds in its project: everything
    /// else of the page read once by the one way a page is made (PERF-N1).
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(crate) async fn page_items(
        &self,
        actor: &Actor,
        seen: &crate::projects::Leveled,
        issues: &[bb_core::issue::Issue],
        extras: ReadOnRequest,
    ) -> Result<Vec<(std::sync::Arc<bb_policy::Evaluation>, IssueView)>, AppError> {
        // The rights and the chain of each project come from the tree the person's view already holds.
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
        self.issue_items(actor, issues, (&**seen, &places), extras)
            .await
    }
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
    let mut prepared = ctx.prepare_search(&actor, &query.text, query.scope).await?;
    // The groups of a list are by a field the person may search by (ADR-0022): a field of people
    // whose names are hidden somewhere is refused here like a field that does not exist; only the
    // totals group people by how they are shown (ADR-0050, decision 7).
    if super::target::target_of(&prepared.looked.fields, &query.by).is_none() {
        return Err(AppError::Refused(ErrorCode::InvalidRequest));
    }
    // The groups by a release are those of the releases the person may know, and of none.
    if is_release(&prepared.looked, &query.by) && prepared.request.releases_known.is_empty() {
        prepared.request.releases_known = ctx.releases_known(&prepared.looked).await?;
    }
    let grouping = ctx
        .grouping_of(&actor, &mut prepared, &query.by, None)
        .await?;
    let mut found: Vec<(ShownKey, u64)> = ctx
        .group_counts(&prepared, &grouping)
        .await?
        .into_iter()
        .map(|(key, total)| (key, total.issues))
        .collect();
    // Over the cap the groups are cut, but the one for the issues without a value stays: it is how
    // many issues the field does not reach, and the answer says it cut the rest (`truncated`).
    let truncated = found.len() > MAX_GROUPS;
    if truncated {
        let none = found
            .iter()
            .position(|(key, _)| *key == ShownKey::Other(GroupKey::Empty))
            .map(|at| found.remove(at));
        found.truncate(MAX_GROUPS - usize::from(none.is_some()));
        found.extend(none);
    }
    let groups = ctx
        .groups_with_filters(
            &actor,
            &grouping.viewer,
            &prepared.looked,
            (&query.by, Some(grouping.target)),
            found,
        )
        .await?;
    let organization = ctx.evaluation_of(&actor, Where::Organization).await?;
    Ok(organization.authorize(SearchGroups { groups, truncated }))
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The kinds of relation that mean *blocks* (see [`blocking_kinds`]).
    async fn blocking_kinds(&self) -> Result<Vec<bb_core::id::RelationKindId>, AppError> {
        Ok(blocking_kinds(
            &self
                .store()
                .relation_kinds()
                .await
                .map_err(|error| internal::<S>(&error))?,
        ))
    }
}

/// The kinds of relation that mean *blocks*: those whose forward name is the starting one. What
/// `blocks` and `blocked by` of the search follow, and the loops the relation map names besides
/// the kinds with a date effect.
pub(crate) fn blocking_kinds(
    kinds: &[bb_core::relation::RelationKind],
) -> Vec<bb_core::id::RelationKindId> {
    let blocks = bb_core::message::Message::from(bb_core::message::RELATION_KIND_BLOCKS).render();
    kinds
        .iter()
        .filter(|kind| bb_core::names::same_name(&kind.forward, &blocks))
        .map(|kind| kind.id)
        .collect()
}
