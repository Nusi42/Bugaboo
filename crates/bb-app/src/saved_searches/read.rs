//! Seeing saved searches (QRY-06): the person's own, the ones shared with them, one of them, and
//! the places they may share with.

use bb_core::ErrorCode;
use bb_core::id::SavedSearchId;
use bb_core::paging::{Cut, page_len, read_len, window};
use bb_policy::{Authorized, Permission, Place};

use super::reach::Stored;
use super::types::{
    SavedSearchPage, SavedSearchView, SearchQuery, SearchScope, SearchesQuery, ShareChoice,
    ShareChoiceKind, ShareChoicePage, ShareChoicesQuery,
};
use crate::auth::{AppError, AuthStore, Ctx, internal};
use crate::input::cursor_or_invalid;

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
    let store = ctx.store();
    let found = match input.scope {
        SearchScope::Mine => {
            store
                .saved_searches_made_by(actor.person.id, after, read_len(len))
                .await
        }
        SearchScope::Shared => {
            store
                .saved_searches_shared_with(reaching.reach(), after, read_len(len))
                .await
        }
    }
    .map_err(|error| internal::<S>(&error))?;
    let cut = Cut::of(found, len);
    let next_cursor = cut.next_cursor(|search| search.id.to_string());
    let ids: Vec<SavedSearchId> = cut.items.iter().map(|search| search.id).collect();
    let mut columns = store
        .columns_of_searches(&ids)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let mut shares = store
        .shares_of_searches(&ids)
        .await
        .map_err(|error| internal::<S>(&error))?;
    let stored = cut
        .items
        .into_iter()
        .map(|search| {
            let id = search.id;
            Stored {
                search,
                columns: take_where(&mut columns, |column| column.search == id),
                shares: take_where(&mut shares, |share| share.search == id),
            }
        })
        .collect();
    let items = ctx.search_views(&actor, &reaching, stored).await?;
    let organization = ctx.evaluation_of(&actor, Place::Organization).await?;
    Ok(organization.authorize(SavedSearchPage { items, next_cursor }))
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
        ShareChoiceKind::Group => {
            let manages = organization.allows(Permission::ManageGroups);
            let shares = organization.allows(Permission::ShareSearches);
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
                .filter(|group| manages || (shares && mine.contains(&group.id)))
                .map(|group| ShareChoice {
                    id: group.id.to_string(),
                    name: group.name,
                    short_name: None,
                })
                .collect()
        }
        ShareChoiceKind::Project => {
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
