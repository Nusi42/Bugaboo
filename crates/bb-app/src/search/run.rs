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
use bb_core::id::{IssueId, ProjectId};
use bb_core::paging::{Cut, page_len, read_len};
use bb_core::tags::IssueSight;
use bb_policy::{Authorized, Masked, Place as Where};
use bb_searchlang::{Mistake, check, parse_with};
use jiff::tz::TimeZone;
use serde::Serialize;
use serde_json::{Value, json};

use super::names::status_groups;
use super::readable::{Looked, SearchScope, readable_of};
use super::request::{
    GroupCount, SearchFailure, SearchPosition, SearchRequest, SearchStore, SortValue,
};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::issues::IssueView;

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

/// Why a search did not give an answer.
#[derive(Debug)]
pub enum SearchError {
    /// The text is not a search the person may run: every mistake, with where it is.
    Mistakes(Vec<Mistake>),
    /// The request failed for another reason.
    Failed(AppError),
}

impl From<AppError> for SearchError {
    fn from(error: AppError) -> Self {
        Self::Failed(error)
    }
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
    pub groups: Vec<GroupCount>,
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
    ) -> Result<Prepared, SearchError> {
        let looked = self.looked_at(actor, scope).await?;
        let readable = readable_of(&looked.fields);
        let parsed = parse_with(text, &readable).map_err(|mistake| vec![mistake])?;
        let checked = check(parsed, &readable)?;
        let names = self.search_names(actor, &looked, &checked).await?;
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
            after: None,
            limit: 0,
        };
        Ok(Prepared { request, looked })
    }
}

impl From<Vec<Mistake>> for SearchError {
    fn from(mistakes: Vec<Mistake>) -> Self {
        Self::Mistakes(mistakes)
    }
}

fn stopped(error: SearchFailure) -> AppError {
    match error {
        SearchFailure::TooSlow => AppError::Refused(ErrorCode::SearchTooSlow),
        SearchFailure::Failed(reason) => AppError::Internal(reason),
    }
}

/// A position as the text a cursor carries (the API seals it): the values of the keys of the order,
/// then the ID.
fn position_text(position: &SearchPosition) -> String {
    let values: Vec<Value> = position
        .values
        .iter()
        .map(|value| match value {
            SortValue::Empty => json!(null),
            SortValue::Text(text) => json!({ "t": text }),
            SortValue::Whole(number) => json!({ "w": number }),
        })
        .collect();
    json!({ "v": values, "i": position.id.to_string() }).to_string()
}

/// The position [`position_text`] wrote; `None` for text that is not one.
fn position_from(text: &str) -> Option<SearchPosition> {
    let found: Value = serde_json::from_str(text).ok()?;
    let values = found
        .get("v")?
        .as_array()?
        .iter()
        .map(|value| match value {
            Value::Null => Some(SortValue::Empty),
            other => other
                .get("t")
                .and_then(Value::as_str)
                .map(|text| SortValue::Text(text.to_owned()))
                .or_else(|| other.get("w")?.as_i64().map(SortValue::Whole)),
        })
        .collect::<Option<Vec<_>>>()?;
    Some(SearchPosition {
        values,
        id: found.get("i")?.as_str()?.parse::<IssueId>().ok()?,
    })
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
) -> Result<Authorized<SearchPage>, SearchError> {
    let actor = ctx.actor(&query.session_token).await?;
    let mut prepared = ctx.prepare_search(&actor, &query.text, query.scope).await?;
    let len = page_len(Some(query.limit));
    prepared.request.limit = read_len(len);
    prepared.request.after = match query.cursor.as_deref() {
        Some(text) => {
            Some(position_from(text).ok_or(AppError::Refused(ErrorCode::InvalidRequest))?)
        }
        None => None,
    };
    let found = ctx
        .store()
        .search_page(&prepared.request)
        .await
        .map_err(stopped)?;
    let page = Cut::of(found, len);
    let next_cursor = page.next_cursor(|hit| position_text(&hit.position));
    let ids: Vec<IssueId> = page.items.iter().map(|hit| hit.issue.id).collect();
    let assigned = ctx
        .store()
        .assigned_people_of(&ids)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let mut views = HashMap::new();
    let mut items = Vec::with_capacity(page.items.len());
    for hit in &page.items {
        let project = hit.issue.project;
        let (held, shown) = match views.remove(&project) {
            Some(each) => each,
            None => ctx.views_in(&actor, &prepared.looked, project).await?,
        };
        let (held, mut shown) = (held, shown);
        let people: Vec<_> = assigned
            .iter()
            .filter(|each| each.issue == hit.issue.id)
            .map(|each| each.person)
            .collect();
        ctx.preload_values(&mut shown, &[hit.issue.id]).await?;
        let view = ctx.issue_view(&mut shown, &hit.issue, &people).await?;
        items.push(held.authorize(view));
        views.insert(project, (held, shown));
    }
    let organization = ctx.evaluation_of(&actor, Where::Organization).await?;
    Ok(organization.authorize(SearchPage { items, next_cursor }))
}

/// How many issues the search finds, the same search without the paging.
///
/// # Errors
/// As [`search`], without the cursor.
pub async fn search_count<S: AuthStore + SearchStore>(
    ctx: &Ctx<'_, S>,
    query: SearchCountQuery,
) -> Result<Authorized<SearchCount>, SearchError> {
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
) -> Result<Authorized<SearchGroups>, SearchError> {
    let actor = ctx.actor(&query.session_token).await?;
    let prepared = ctx.prepare_search(&actor, &query.text, query.scope).await?;
    if !prepared
        .looked
        .fields
        .iter()
        .any(|each| each.field.word == query.by)
    {
        return Err(AppError::Refused(ErrorCode::InvalidRequest).into());
    }
    let groups = ctx
        .store()
        .search_groups(&prepared.request, &query.by)
        .await
        .map_err(stopped)?;
    let organization = ctx.evaluation_of(&actor, Where::Organization).await?;
    Ok(organization.authorize(SearchGroups { groups }))
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The rights of the person in a project and the views of its issues.
    async fn views_in(
        &self,
        actor: &Actor,
        looked: &Looked,
        project: ProjectId,
    ) -> Result<
        (
            std::sync::Arc<bb_policy::Evaluation>,
            crate::issues::IssueViews,
        ),
        AppError,
    > {
        let held = self.evaluation_of(actor, Where::Project(project)).await?;
        let stored = looked
            .seen
            .stored(project)
            .cloned()
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        let views = self
            .issue_views(actor, std::sync::Arc::clone(&held), stored)
            .await?;
        Ok((held, views))
    }

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
