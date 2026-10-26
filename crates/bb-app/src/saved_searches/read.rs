//! Seeing saved searches (QRY-06): the person's own, the ones shared with them, one of them, and
//! the places they may share with.

use bb_core::ErrorCode;
use bb_core::id::SavedSearchId;
use bb_core::paging::{Cut, LOOKED_AT_MAX, PAGE_SIZE_MAX, page_len, read_len, window};
use bb_core::saved_search::{SearchSet, ShareKind};
use bb_policy::{Authorized, Permission, Place};

use super::judge::Groups;
use super::reach::{Reaching, Stored, may_share_with_group};
use super::types::{
    SavedSearchPage, SavedSearchView, SearchQuery, SearchesQuery, ShareChoice, ShareChoicePage,
    ShareChoicesQuery,
};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::input::cursor_or_invalid;

/// How many searches the store is asked for at a time while a page of *Shared with me* is filled
/// (the most a page of any list holds).
const READ_AT_A_TIME: u64 = PAGE_SIZE_MAX as u64;

/// A page of searches with their columns, and the cursor of the next page.
struct Found {
    items: Vec<Stored>,
    next: Option<String>,
}

/// Where a page of *Shared with me* starts: after a search the person was shown (or the start) and
/// past a number of searches, always a multiple of [`LOOKED_AT_MAX`].
///
/// A cursor is readable, so it holds nothing a person may not know about: the ID is the last search
/// of the page before, which the person has seen, and the number only counts whole stretches that
/// were looked at without filling a page. It never holds the ID of a search that does not check
/// for the person (the same rule as the cursor of the projects to choose from for an access key).
struct Position {
    after: Option<SavedSearchId>,
    skip: u64,
}

impl Position {
    /// What the cursor text says: `ID`, `ID+500`, `+500`, or nothing for the start.
    fn read(cursor: Option<&str>) -> Result<Self, AppError> {
        let invalid = || AppError::Refused(ErrorCode::InvalidRequest);
        let Some(text) = cursor else {
            return Ok(Self {
                after: None,
                skip: 0,
            });
        };
        let (id, skip) = match text.split_once('+') {
            Some((id, skip)) => (id, skip.parse::<u64>().map_err(|_| invalid())?),
            None => (text, 0),
        };
        if skip % (LOOKED_AT_MAX as u64) != 0 {
            return Err(invalid());
        }
        let after = if id.is_empty() {
            None
        } else {
            Some(id.parse::<SavedSearchId>().map_err(|_| invalid())?)
        };
        Ok(Self { after, skip })
    }

    fn write(&self) -> String {
        let id = self.after.map(|id| id.to_string()).unwrap_or_default();
        if self.skip == 0 {
            id
        } else {
            format!("{id}+{}", self.skip)
        }
    }
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
    let len = page_len(Some(input.limit));
    let reaching = ctx.reaching(&actor).await?;
    let found = match input.scope {
        SearchSet::Mine => {
            let after = cursor_or_invalid(input.cursor.as_deref(), |text| {
                text.parse::<SavedSearchId>().ok()
            })?;
            ctx.made_by_the_person(&actor, after, len).await?
        }
        SearchSet::Shared => {
            let start = Position::read(input.cursor.as_deref())?;
            ctx.shared_that_read_cleanly(&actor, &reaching, &start, len)
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
        next_cursor: found.next,
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
        Ok(Found { items, next })
    }

    /// The searches shared with the person that check cleanly for them, from `start`, at most
    /// `len`. A search that does not is skipped without a trace: searches are looked at until the
    /// page is full, they are used up or [`LOOKED_AT_MAX`] are looked at, so the work is bounded
    /// and a short page tells nothing (ADR-0034, decision 4).
    async fn shared_that_read_cleanly(
        &self,
        actor: &Actor,
        reaching: &Reaching,
        start: &Position,
        len: usize,
    ) -> Result<Found, AppError> {
        let mut judge = self.judge(actor, reaching, Groups::WhenNamed).await?;
        let store = self.store();
        let (mut kept, mut looked_at, mut more): (Vec<Stored>, u64, bool) = (Vec::new(), 0, false);
        'scan: while looked_at < LOOKED_AT_MAX as u64 {
            let found = store
                .saved_searches_shared_with(
                    reaching.reach(),
                    start.after,
                    start.skip + looked_at,
                    READ_AT_A_TIME,
                )
                .await
                .map_err(|error| internal::<S>(&error))?;
            let ids: Vec<SavedSearchId> = found.iter().map(|search| search.id).collect();
            let mut columns = store
                .columns_of_searches(&ids)
                .await
                .map_err(|error| internal::<S>(&error))?;
            let full = u64::try_from(found.len()).is_ok_and(|count| count == READ_AT_A_TIME);
            for search in found {
                let id = search.id;
                let stored = Stored {
                    search,
                    columns: take_where(&mut columns, |column| column.search == id),
                    shares: Vec::new(),
                };
                if self.reads_cleanly(&mut judge, &stored).await? {
                    if kept.len() == len {
                        more = true;
                        break 'scan;
                    }
                    kept.push(stored);
                }
                looked_at += 1;
            }
            if !full {
                break;
            }
        }
        let next = if more {
            // A full page continues after its last search, which the person has just been shown.
            kept.last().map(|last| Position {
                after: Some(last.search.id),
                skip: 0,
            })
        } else if looked_at >= LOOKED_AT_MAX as u64 {
            // A page that looked at enough searches continues past them.
            Some(Position {
                after: start.after,
                skip: start.skip + LOOKED_AT_MAX as u64,
            })
        } else {
            None
        };
        Ok(Found {
            items: kept,
            next: next.as_ref().map(Position::write),
        })
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
