//! The page of the history of an issue (ISS-06, SEC-06): the rows in the order they were written, or,
//! for a reader who may not see the moment of every time entry, in the order of the moments they are
//! shown at (`dated.rs`).

use bb_core::ErrorCode;
use bb_core::history::{HistoryKind, HistoryTab};
use bb_core::paging::{SkipCursor, page_len};
use bb_policy::{Authorized, Permission, Place};

use super::dated::{PageQuery, dated_page};
use super::read::Reading;
use super::types::{HistoryPage, HistoryQuery};
use super::walk::IssueHistory;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::walk::fill_past_hidden;

/// The history of an issue, newest first, a page at a time, in three tabs: *Everything*,
/// *Comments*, *Changes*. The history is caught up with the events first, so it is never behind
/// what the reader just did. Rows the reader may not see are skipped by [`fill_past_hidden`], so
/// a cursor never names one and says no more than "there is more" (M3-F68).
///
/// # Errors
/// `resource.not-found` for an issue that does not exist, is in the trash or that the person may
/// not see, `request.invalid` for a cursor that is not one.
pub async fn history<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: HistoryQuery,
) -> Result<Authorized<HistoryPage>, AppError> {
    let actor = ctx.actor(&input.session_token).await?;
    let (issue, held) = ctx.visible_issue(&actor, &input.issue).await?;
    ctx.catch_up_history().await?;
    let len = page_len(Some(input.limit));
    let kind = match input.tab {
        HistoryTab::Everything => None,
        HistoryTab::Comments => Some(HistoryKind::Comment),
        HistoryTab::Changes => Some(HistoryKind::Change),
    };
    // A reader who may not see the moment of the time entries of others gets those rows placed by
    // their dates: the order the rows were written in would tell it. Looked up once.
    let sees_moments = ctx
        .evaluation_of(&actor, Place::Organization)
        .await?
        .allows(Permission::SeeWorkingHoursOfOthers);
    // The people of the values of the page are resolved once for every row of it.
    let mut reading = Reading::new(&actor, &held, ctx.line_maker().await?, sees_moments);
    let kinds = ctx
        .store()
        .relation_kinds()
        .await
        .map_err(|error| internal::<S>(&error))?;
    if !sees_moments {
        let asked = PageQuery {
            cursor: input.cursor.as_deref(),
            len,
        };
        return dated_page(ctx, &issue, kind, &kinds, &mut reading, &held, asked).await;
    }
    let start = SkipCursor::read(input.cursor.as_deref())
        .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
    let mut walk = IssueHistory {
        ctx,
        issue: &issue,
        kind,
        kinds: &kinds,
        reading: &mut reading,
    };
    let filled = fill_past_hidden(&mut walk, &start, len).await?;
    Ok(held.authorize(HistoryPage {
        items: filled.rows.into_iter().filter_map(|row| row.item).collect(),
        next_cursor: filled.next.as_ref().map(SkipCursor::write),
    }))
}
