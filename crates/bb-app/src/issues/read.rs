//! Seeing issues (ISS-01): one issue by its readable or internal ID, and the issues of a project a
//! page at a time.
//!
//! An issue the person may not see answers `resource.not_found`, exactly like one that does not
//! exist: the two take the same reads and the same evaluation before either is answered. Without
//! *See issues* a person sees the issues they reported. No answer holds a count (PRIV-13): a page
//! has a cursor, never a total.

use bb_core::ErrorCode;
use bb_core::event::RecordKind;
use bb_core::fields::FieldKey;
use bb_core::id::{IssueId, UserId};
use bb_core::issue_key::IssueKey;
use bb_core::paging::{Cut, page_len, read_len};
use bb_policy::{Authorized, Permission, Place};
use jiff::Timestamp;

use super::fields::CHANGEABLE;
use super::store::IssueFound;
use super::types::{IssueDetail, IssuePage, IssueQuery, IssuesQuery};
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::fields::Item;
use crate::input::cursor_or_invalid;

impl<S: AuthStore> Ctx<'_, S> {
    /// The issue an address names: its internal ID, or a readable ID (the code now or an old
    /// one; the short name of the project now or an old one). An address that is neither asks the
    /// store for an issue that is not there, so every address, good or not, costs a question.
    pub(crate) async fn find_issue(&self, address: &str) -> Result<Option<IssueFound>, AppError> {
        let store = self.store();
        let nowhere = IssueId::from_parts(Timestamp::UNIX_EPOCH, [0; 10])
            .map_err(|error| AppError::Internal(error.to_string()))?;
        if let Ok(key) = address.parse::<IssueKey>() {
            let Some(project) = store
                .project_by_address(&key.short_name)
                .await
                .map_err(|error| internal::<S>(&error))?
            else {
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

/// One issue, with what the person may do with it.
///
/// # Errors
/// `resource.not_found` for an issue that does not exist, is in the trash, or that the person may
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
        .assignees(issue.id)
        .await
        .map_err(|error| internal::<S>(&error))?
        .into_iter()
        .map(|assignee| assignee.person)
        .collect();
    let mut views = ctx.issue_views(&actor, held.clone(), project).await?;
    let view = ctx.issue_view(&mut views, &issue, &assigned_to).await?;
    let may_change = held.may_change_issue(issue.reported_by);
    let mask = held.field_mask_for(Some(issue.issue_type));
    let changeable = if may_change {
        let mut keys: Vec<FieldKey> = CHANGEABLE
            .into_iter()
            .filter(|field| mask.can_change(*field))
            .map(FieldKey::from)
            .collect();
        let item = Item {
            kind: RecordKind::Issue,
            id: Some(issue.id.into()),
            chain: ctx.chain_of(issue.project).await?,
            issue_type: Some(issue.issue_type),
            reported_by: Some(issue.reported_by),
        };
        keys.extend(ctx.changeable_own_fields(&mask, &item).await?);
        keys
    } else {
        Vec::new()
    };
    // A mention or an issue ID is a chip only when the reader may see what it names (SEC-06).
    let description_html = ctx
        .html_of(&actor, &held, &issue, &issue.description)
        .await?;
    let own_fields_html = ctx.long_texts_html(&actor, &held, &issue, &view).await?;
    Ok(held.authorize(IssueDetail {
        issue: view,
        description_html,
        own_fields_html,
        may_change,
        may_delete: held.may_delete_issue(issue.reported_by),
        changeable,
    }))
}

/// Where a page ends: the ID of the last issue, which the next page starts before (newest first).
fn write_cursor(last: &bb_core::issue::Issue) -> String {
    last.id.to_string()
}

/// The issues of a project the person sees, newest first, a page at a time. Without *See issues*
/// the person sees the issues they reported.
///
/// # Errors
/// `resource.not_found` for a project that does not exist or that the person may not see,
/// `request.invalid` for a cursor that is not one.
pub async fn issues<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: IssuesQuery,
) -> Result<Authorized<IssuePage>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let found = ctx.find_project(&input.project).await?;
    let place = found.as_ref().map_or(Place::Organization, |found| {
        Place::Project(found.project.id)
    });
    let held = ctx.evaluation_of(&actor, place).await?;
    let Some(project) = found
        .map(|found| found.project)
        .filter(|_| held.sees_project())
    else {
        return Err(AppError::Refused(ErrorCode::NotFound));
    };
    let before = cursor_or_invalid(input.cursor.as_deref(), |text| text.parse::<IssueId>().ok())?;
    let limit = page_len(Some(input.limit));
    let reported_by = (!held.allows(Permission::SeeIssues)).then_some(actor.person.id);
    let found = ctx
        .store()
        .issues_page(project.id, reported_by, before, read_len(limit))
        .await
        .map_err(|error| internal::<S>(&error))?;
    let page = Cut::of(found, limit);
    let next_cursor = page.next_cursor(write_cursor);
    let assignees = if input.assigned_to {
        let ids: Vec<IssueId> = page.items.iter().map(|issue| issue.id).collect();
        ctx.store()
            .assignees_of(&ids)
            .await
            .map_err(|error| internal::<S>(&error))?
    } else {
        Vec::new()
    };
    let mut views = ctx.issue_views(&actor, held.clone(), project).await?;
    let shown: Vec<IssueId> = page.items.iter().map(|issue| issue.id).collect();
    ctx.preload_values(&mut views, &shown).await?;
    let mut items = Vec::with_capacity(page.items.len());
    for issue in &page.items {
        let people: Vec<UserId> = assignees
            .iter()
            .filter(|assignee| assignee.issue == issue.id)
            .map(|assignee| assignee.person)
            .collect();
        items.push(ctx.issue_view(&mut views, issue, &people).await?);
    }
    Ok(held.authorize(IssuePage { items, next_cursor }))
}
