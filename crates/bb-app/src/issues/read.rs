//! Seeing issues (ISS-01): one issue by its readable or internal ID, and the issues of a project a
//! page at a time.
//!
//! An issue the person may not see answers `resource.not-found`, exactly like one that does not
//! exist: the two take the same reads and the same evaluation before either is answered. Without
//! *See issues* a person sees the issues they reported. No answer holds a count (PRIV-13): a page
//! has a cursor, never a total.

use std::collections::HashMap;

use bb_core::ErrorCode;
use bb_core::event::RecordKind;
use bb_core::fields::FieldKey;
use bb_core::id::{IssueId, UserId};
use bb_core::readable_id::ReadableId;
use bb_policy::Authorized;
use jiff::Timestamp;

use super::fields::CHANGEABLE;
use super::store::IssueFound;
use super::types::{IssueDetail, IssueQuery, IssuesQuery};
use crate::access::place_of_address;
use crate::auth::{AppError, AuthStore, Ctx, internal, locked};
use crate::fields::{Held, Item};
use crate::search::{SearchPage, SearchQuery, SearchScope, search_page};

impl<S: AuthStore> Ctx<'_, S> {
    /// The issue an address names: its internal ID, or a readable ID (the code now or an old
    /// one; the short name of the project now or an old one). An address that is neither asks the
    /// store for an issue that is not there, so every address, good or not, costs a question.
    pub(crate) async fn find_issue(&self, address: &str) -> Result<Option<IssueFound>, AppError> {
        if let Some(found) = locked(&self.known.ahead)
            .as_ref()
            .and_then(|ahead| ahead.found.get(address).cloned())
        {
            return Ok(found);
        }
        let store = self.store();
        let nowhere = IssueId::from_parts(Timestamp::UNIX_EPOCH, [0; 10])
            .map_err(|error| AppError::Internal(error.to_string()))?;
        if let Ok(key) = address.parse::<ReadableId>() {
            let Some(project) = self.project_by_address(&key.short_name).await? else {
                return Ok(None);
            };
            return store
                .issue_at(project.project.id, key.code)
                .await
                .map_err(|error| internal::<S>(&error));
        }
        let id = address.parse::<IssueId>().unwrap_or(nowhere);
        Ok(store
            .issue(id)
            .await
            .map_err(|error| internal::<S>(&error))?
            .map(|issue| IssueFound {
                issue,
                old_code: false,
            }))
    }
}

/// A selection of issues read ahead of the work on each: each issue by its address, and the values
/// of their own fields in one read.
pub(crate) struct ReadAhead {
    found: HashMap<String, Option<IssueFound>>,
    pub(crate) values: crate::fields::ItemValues,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Finds the issues of a read-only request by their addresses and reads the values of their own
    /// fields in one go, so the work on each of them reads neither again (PERF-N1).
    ///
    /// # Errors
    /// An internal error when the store fails.
    pub(crate) async fn read_ahead(&self, addresses: &[&str]) -> Result<(), AppError> {
        let mut found = HashMap::with_capacity(addresses.len());
        for address in addresses {
            found.insert((*address).to_owned(), self.find_issue(address).await?);
        }
        let ids: Vec<bb_core::id::EntityId> = found
            .values()
            .flatten()
            .map(|each| each.issue.id.into())
            .collect();
        let values = self.load_values(RecordKind::Issue, &ids).await?;
        *locked(&self.known.ahead) = Some(Box::new(ReadAhead { found, values }));
        Ok(())
    }
}

/// One issue, with what the person may do with it.
///
/// # Errors
/// `resource.not-found` for an issue that does not exist, is in the trash, or that the person may
/// not see: the three cannot be told apart.
pub async fn issue<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: IssueQuery,
) -> Result<Authorized<IssueDetail>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (issue, held) = ctx.visible_issue(&actor, &input.issue).await?;
    let project = ctx
        .store()
        .project(issue.project)
        .await
        .map_err(|error| internal::<S>(&error))?
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    let assigned_to: Vec<UserId> = ctx
        .store()
        .assigned_people(issue.id)
        .await
        .map_err(|error| internal::<S>(&error))?
        .into_iter()
        .map(|assigned_person| assigned_person.person)
        .collect();
    // The labels of the issue are read through their own route, not here.
    let mut views = ctx.issue_views(&actor, held.clone(), project).await?;
    let view = ctx.issue_view(&mut views, &issue, &assigned_to).await?;
    let may_change = held.may_change_issue(issue.reported_by);
    let changeable = ctx.changeable_fields(&held, &issue).await?;
    // A mention or an issue ID is a chip only when the reader may see what it names (SEC-06).
    let description_html = ctx
        .html_of(&actor, &held, &issue, &issue.description)
        .await?;
    let own_fields_html = ctx.long_texts_html(&actor, &held, &issue, &view).await?;
    let description_written_by = ctx
        .written_by(&held, RecordKind::Issue, issue.id, "description")
        .await?;
    Ok(held.authorize(IssueDetail {
        issue: view,
        description_html,
        description_written_by,
        own_fields_html,
        may_change,
        may_delete: held.may_delete_issue(issue.reported_by),
        changeable,
    }))
}

/// The fields every issue has that `held` lets the person change on `issue`; none when they may
/// not change the issue at all. The first part of [`Ctx::changeable_fields`], which a reader of
/// built-in fields only (the timeline of *Start date* and *Due date*) asks alone.
pub(crate) fn changeable_built_in(
    held: &bb_policy::Evaluation,
    issue: &bb_core::issue::Issue,
) -> Vec<FieldKey> {
    if !held.may_change_issue(issue.reported_by) {
        return Vec::new();
    }
    let mask = held.field_mask_for(Some(issue.issue_type));
    CHANGEABLE
        .into_iter()
        .filter(|field| mask.can_change(*field))
        .map(FieldKey::from)
        .collect()
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The fields `held` lets the person change on `issue`, built-in and own, by their keys; none
    /// when they may not change the issue at all. The one rule of the issue page, of the timeline
    /// (whether a bar may be dragged) and of the change itself.
    pub(crate) async fn changeable_fields(
        &self,
        held: &bb_policy::Evaluation,
        issue: &bb_core::issue::Issue,
    ) -> Result<Vec<FieldKey>, AppError> {
        if !held.may_change_issue(issue.reported_by) {
            return Ok(Vec::new());
        }
        let mut keys = changeable_built_in(held, issue);
        let mask = held.field_mask_for(Some(issue.issue_type));
        let item = Item {
            kind: RecordKind::Issue,
            id: Some(issue.id.into()),
            chain: self.chain_of(issue.project).await?,
            issue_type: Some(issue.issue_type),
            reported_by: Some(issue.reported_by),
        };
        keys.extend(
            self.changeable_own_fields(&mask, &item, Held::Every)
                .await?,
        );
        Ok(keys)
    }
}

/// The issues of a project the person sees and of the projects below it that they see, newest
/// first, a page at a time: the search with `within` (ADR-0034 decision 6). Where the person has no
/// *See issues* they see the issues they reported.
///
/// # Errors
/// `resource.not-found` for a project that does not exist or that the person may not see,
/// `request.invalid` for a cursor that is not one.
pub async fn issues<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: IssuesQuery,
) -> Result<Authorized<SearchPage>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let found = ctx.find_project(&input.project).await?;
    let place = place_of_address(found.as_ref().map(|found| found.project.id))?;
    let held = ctx.evaluation_of(&actor, place).await?;
    let Some(project) = found
        .map(|found| found.project)
        .filter(|_| held.sees_project())
    else {
        return Err(AppError::Refused(ErrorCode::NotFound));
    };
    let query = SearchQuery {
        session_token: input.session_token,
        text: String::new(),
        scope: SearchScope::Within(project.id),
        cursor: input.cursor,
        limit: input.limit,
        labels: input.labels,
        agent: input.agent,
        time: input.time,
    };
    // Always read: a status change may name the person assigned. The people are in the answer only
    // when they were asked for.
    search_page(ctx, &actor, &query, input.assigned_to).await
}
