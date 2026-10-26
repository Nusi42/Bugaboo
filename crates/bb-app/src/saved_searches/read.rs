//! Seeing saved searches (QRY-06): the person's own, the ones shared with them, one of them, and
//! the places they may share with.

use bb_core::ErrorCode;
use bb_core::id::SavedSearchId;
use bb_core::paging::{Cut, page_len, read_len, window};
use bb_core::saved_search::{SearchSet, ShareKind};
use bb_policy::{Authorized, Permission, Place};

use super::reach::{Reaching, Stored, may_share_with_group};
use super::types::{
    SavedSearchPage, SavedSearchView, SearchQuery, SearchesQuery, ShareChoice, ShareChoicePage,
    ShareChoicesQuery,
};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::input::cursor_or_invalid;

/// The most searches that one request for *Shared with me* looks at, kept or skipped: the work of
/// a request does not depend on how many searches that do not check cleanly reach the person
/// (PERF-05). A page that ends here is short and has a cursor that goes on after the last search
/// looked at, which tells nothing about why it is short.
const LOOKED_AT_MAX: u64 = 500;

/// How many searches one read of *Shared with me* fetches at least.
const LOOK_BATCH: u64 = 100;

/// A page of searches with their columns, and where the next page starts.
struct Found {
    items: Vec<Stored>,
    next: Option<SavedSearchId>,
}

/// The saved searches of one scope, a page at a time: the ones the person made, or the ones other
/// people shared with a group of theirs or a project they see. Oldest first.
///
/// # Errors
/// `request.invalid` for a cursor that is not one.
pub async fn saved_searches<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SearchesQuery,
) -> Result<Authorized<SavedSearchPage>, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let after = cursor_or_invalid(input.cursor.as_deref(), |text| {
        text.parse::<SavedSearchId>().ok()
    })?;
    let len = page_len(Some(input.limit));
    let reaching = ctx.reaching(&actor).await?;
    let found = match input.scope {
        SearchSet::Mine => ctx.made_by_the_person(&actor, after, len).await?,
        SearchSet::Shared => {
            ctx.shared_that_read_cleanly(&actor, &reaching, after, len)
                .await?
        }
    };
    let ids: Vec<SavedSearchId> = found.items.iter().map(|each| each.search.id).collect();
    let mut shares = ctx
        .store()
        .shares_of_searches(&ids)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let stored = found
        .items
        .into_iter()
        .map(|mut each| {
            let id = each.search.id;
            each.shares = take_where(&mut shares, |share| share.search == id);
            each
        })
        .collect();
    let items = ctx.search_views(&actor, &reaching, stored).await?;
    let organization = ctx.evaluation_of(&actor, Place::Organization).await?;
    Ok(organization.authorize(SavedSearchPage {
        items,
        next_cursor: found.next.map(|id| id.to_string()),
    }))
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The searches the person made, a page of them with their columns.
    async fn made_by_the_person(
        &self,
        actor: &Actor,
        after: Option<SavedSearchId>,
        len: usize,
    ) -> Result<Found, AppError> {
        let store = self.store();
        let found = store
            .saved_searches_made_by(actor.person.id, after, read_len(len))
            .await
            .map_err(|error| internal::<S>(&error))?;
        let cut = Cut::of(found, len);
        let next = cut.next_cursor(|search| search.id.to_string());
        let ids: Vec<SavedSearchId> = cut.items.iter().map(|search| search.id).collect();
        let mut columns = store
            .columns_of_searches(&ids)
            .await
            .map_err(|error| internal::<S>(&error))?;
        let items = cut
            .items
            .into_iter()
            .map(|search| {
                let id = search.id;
                Stored {
                    search,
                    columns: take_where(&mut columns, |column| column.search == id),
                    shares: Vec::new(),
                }
            })
            .collect();
        Ok(Found {
            items,
            next: next.and_then(|text| text.parse().ok()),
        })
    }

    /// The searches shared with the person that check cleanly for them, after `after`, at most
    /// `len`. A search that does not is skipped without a trace: the batches go on until the page
    /// is full, the searches are used up or [`LOOKED_AT_MAX`] are looked at, so a short page never
    /// tells that something was left out (ADR-0034, decision 4), and the work is bounded.
    async fn shared_that_read_cleanly(
        &self,
        actor: &Actor,
        reaching: &Reaching,
        after: Option<SavedSearchId>,
        len: usize,
    ) -> Result<Found, AppError> {
        let mut judge = self.judge(actor, reaching).await?;
        let store = self.store();
        let batch = read_len(len).max(LOOK_BATCH);
        let (mut kept, mut from, mut looked_at): (Vec<Stored>, _, u64) = (Vec::new(), after, 0);
        loop {
            let found = store
                .saved_searches_shared_with(reaching.reach(), from, batch)
                .await
                .map_err(|error| internal::<S>(&error))?;
            let ids: Vec<SavedSearchId> = found.iter().map(|search| search.id).collect();
            let mut columns = store
                .columns_of_searches(&ids)
                .await
                .map_err(|error| internal::<S>(&error))?;
            let exhausted = u64::try_from(found.len()).is_ok_and(|count| count < batch);
            for search in found {
                looked_at += 1;
                from = Some(search.id);
                let id = search.id;
                let stored = Stored {
                    search,
                    columns: take_where(&mut columns, |column| column.search == id),
                    shares: Vec::new(),
                };
                if self.reads_cleanly(&mut judge, &stored).await? {
                    kept.push(stored);
                    if kept.len() > len {
                        // One more than a page: the page is full and there is a next one.
                        kept.truncate(len);
                        let last = kept.last().map(|each| each.search.id);
                        return Ok(Found {
                            items: kept,
                            next: last,
                        });
                    }
                }
            }
            if exhausted {
                return Ok(Found {
                    items: kept,
                    next: None,
                });
            }
            if looked_at >= LOOKED_AT_MAX {
                return Ok(Found {
                    items: kept,
                    next: from,
                });
            }
        }
    }
}

/// The items that `is` is true for, taken out of `from` in their order.
fn take_where<T>(from: &mut Vec<T>, is: impl Fn(&T) -> bool) -> Vec<T> {
    let (taken, kept) = std::mem::take(from).into_iter().partition(is);
    *from = kept;
    taken
}

/// One saved search, opened: the person's own, or one shared with them.
///
/// # Errors
/// `resource.not-found` for a search that does not exist, is in the trash, or is neither theirs
/// nor shared with them: one answer for all three (ADR-0034, decision 4).
pub async fn saved_search<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SearchQuery,
) -> Result<Authorized<SavedSearchView>, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let opened = ctx.open_search(&actor, input.search).await?;
    let view = ctx
        .one_view(&actor, &opened.reaching, opened.stored)
        .await?;
    let organization = ctx.evaluation_of(&actor, Place::Organization).await?;
    Ok(organization.authorize(view))
}

/// The groups or the projects the person may share a saved search with, by name, a page at a
/// time (ADR-0034, decision 2): *Share searches* in the project; for a group, being in it with
/// *Share searches* for the whole organization, or *Manage groups*.
///
/// # Errors
/// `request.invalid` for a cursor that is not one.
pub async fn share_choices<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ShareChoicesQuery,
) -> Result<Authorized<ShareChoicePage>, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let organization = ctx.evaluation_of(&actor, Place::Organization).await?;
    let mut choices: Vec<ShareChoice> = match input.kind {
        ShareKind::Group => {
            let mine: Vec<_> = ctx
                .store()
                .groups_of(actor.person.id)
                .await
                .map_err(|error| internal::<S>(&error))?
                .into_iter()
                .map(|group| group.id)
                .collect();
            ctx.store()
                .groups()
                .await
                .map_err(|error| internal::<S>(&error))?
                .into_iter()
                .filter(|group| may_share_with_group(&organization, mine.contains(&group.id)))
                .map(|group| ShareChoice {
                    id: group.id.to_string(),
                    name: group.name,
                    short_name: None,
                })
                .collect()
        }
        ShareKind::Project => {
            let shown = ctx.shown_to(&actor).await?;
            shown
                .holding(Permission::ShareSearches)
                .into_iter()
                .filter_map(|project| shown.get(project))
                .map(|project| ShareChoice {
                    id: project.id.to_string(),
                    name: project.name.clone(),
                    short_name: Some(project.short_name.to_string()),
                })
                .collect()
        }
    };
    choices.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.id.cmp(&b.id))
    });
    let page = window(choices.len(), input.cursor.as_deref(), input.limit)
        .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
    let items = choices[page.start..page.end].to_vec();
    Ok(organization.authorize(ShareChoicePage {
        items,
        next_cursor: page.next,
    }))
}
