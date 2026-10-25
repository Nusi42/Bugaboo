//! Making and changing saved searches (QRY-06, ADR-0034, decisions 2, 5 and 7): save, change,
//! delete into the trash and take back (*Undo*), and *Save a copy*. Only the person who made a
//! search changes or deletes it; a person with *Change organization settings* may delete any.

use bb_core::ErrorCode;
use bb_core::event::{DomainEvent, FieldChange, RecordKind};
use bb_core::id::SavedSearchId;
use bb_core::saved_search::{Presentation, SavedSearch, SearchColumn};
use bb_core::text::{NAME_MAX, clean};
use bb_core::version::{FIRST_VERSION, counted_up, next};
use bb_policy::{Authorized, Permission, Place};
use bb_searchlang::MAX_TEXT_BYTES;
use serde_json::json;

use super::events;
use super::reach::{Opened, Reaching, Stored};
use super::types::{
    ChangeSearch, DeleteSearch, RestoreSearch, SaveCopy, SaveSearch, SavedSearchView,
};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, Done, Stops, by, change_failed, entity};
use crate::member::saved_search as member;
use crate::ports::BoxFuture;
use crate::projects::Why;

/// The search text as it is kept: trimmed, in one line, at most [`MAX_TEXT_BYTES`] bytes. An empty
/// text is a search for everything.
fn clean_search_text(text: &str) -> Result<String, AppError> {
    let text = text.trim();
    if text.len() > MAX_TEXT_BYTES || text.chars().any(char::is_control) {
        return Err(ErrorCode::InvalidRequest.into());
    }
    Ok(text.to_owned())
}

/// The name as it is kept.
fn clean_search_name(name: &str) -> Result<String, AppError> {
    clean(name, NAME_MAX)
        .map(str::to_owned)
        .ok_or_else(|| ErrorCode::InvalidRequest.into())
}

/// The presentation, when it can be kept.
fn checked(presentation: Presentation) -> Result<Presentation, AppError> {
    if presentation.is_valid() {
        Ok(presentation)
    } else {
        Err(ErrorCode::InvalidRequest.into())
    }
}

/// A saved search about to be written, with the event of it.
struct NewSearch {
    search: SavedSearch,
    columns: Vec<SearchColumn>,
    event: DomainEvent,
}

/// The change that writes a new saved search with its columns.
fn inserting<S: AuthStore>(
    search: SavedSearch,
    columns: Vec<SearchColumn>,
) -> impl for<'t> FnOnce(&'t mut S::Transaction) -> BoxFuture<'t, Result<(), S::Error>> + Send + 'static
{
    move |transaction: &mut S::Transaction| {
        Box::pin(async move { S::insert_saved_search(transaction, &search, &columns).await })
    }
}

/// The change that puts a saved search into the trash or takes it out. It stops, with the reason
/// in `why`, when the search is gone or was changed meanwhile.
fn trashing<S: AuthStore>(
    search: &SavedSearch,
    version: i64,
    why: Why,
) -> impl for<'t> FnOnce(&'t mut S::Transaction) -> BoxFuture<'t, Result<(), S::Error>> + Send + 'static
{
    let (id, deleted_at) = (search.id, search.deleted_at);
    move |transaction: &mut S::Transaction| {
        Box::pin(async move {
            if S::set_saved_search_deleted(transaction, id, deleted_at, version).await? {
                Ok(())
            } else {
                Err(why.stop::<S>(ErrorCode::NotFound))
            }
        })
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The saved search the actor made and may change. A search that is not theirs to see is not
    /// found; one that they see but did not make is refused.
    pub(crate) async fn own_search(
        &self,
        actor: &Actor,
        id: SavedSearchId,
    ) -> Result<Opened, AppError> {
        let opened = self.open_search(actor, id).await?;
        if opened.stored.search.maker == actor.person.id {
            Ok(opened)
        } else {
            Err(ErrorCode::Forbidden.into())
        }
    }

    /// A new saved search of the actor, private, with the event that says it was made.
    fn new_search(
        &self,
        actor: &Actor,
        name: &str,
        text: String,
        presentation: Presentation,
    ) -> Result<NewSearch, AppError> {
        let name = clean_search_name(name)?;
        let presentation = checked(presentation)?;
        let id = SavedSearchId::generate(&mut self.ids())
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let now = self.now();
        let search = SavedSearch {
            id,
            name,
            text,
            maker: actor.person.id,
            view: presentation.view,
            spacing: presentation.spacing,
            group_by: presentation.group_by.clone(),
            created_at: now,
            updated_at: now,
            deleted_at: None,
            version: FIRST_VERSION,
        };
        let columns = presentation.rows(id);
        let event = self.event(
            by(actor.person.id),
            entity(RecordKind::SavedSearch, id),
            events::made(&search, &columns),
        )?;
        Ok(NewSearch {
            search,
            columns,
            event,
        })
    }

    /// A saved search that was just written, as its maker sees it.
    async fn new_view(
        &self,
        actor: &Actor,
        new: NewSearch,
    ) -> Result<Authorized<SavedSearchView>, AppError> {
        let reaching = self.reaching(actor).await?;
        let stored = Stored {
            search: new.search,
            columns: new.columns,
            shares: Vec::new(),
        };
        self.authorized_view(actor, &reaching, stored).await
    }

    /// A saved search as the actor sees it, wrapped as the organization's answer.
    pub(crate) async fn authorized_view(
        &self,
        actor: &Actor,
        reaching: &Reaching,
        stored: Stored,
    ) -> Result<Authorized<SavedSearchView>, AppError> {
        let view = self.one_view(actor, reaching, stored).await?;
        Ok(self
            .evaluation_of(actor, Place::Organization)
            .await?
            .authorize(view))
    }

    /// The saved search to put into the trash or take out of it: the person's own, or any one for
    /// somebody with *Change organization settings*. A search the person may not delete is refused
    /// when they see it and not found when they do not.
    async fn search_to_trash(
        &self,
        actor: &Actor,
        id: SavedSearchId,
        in_trash: bool,
    ) -> Result<(Stored, Reaching), AppError> {
        let stored = self.stored_search(id).await?;
        let reaching = self.reaching(actor).await?;
        let organization = self.evaluation_of(actor, Place::Organization).await?;
        let found = stored
            .filter(|found| found.search.deleted_at.is_some() == in_trash)
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        let mine = found.search.maker == actor.person.id;
        if mine || organization.allows(Permission::ChangeOrganizationSettings) {
            return Ok((found, reaching));
        }
        // Whoever does not see it is told nothing about it, and nobody is told about the trash.
        if !in_trash && reaching.includes(&found.shares) {
            Err(ErrorCode::Forbidden.into())
        } else {
            Err(ErrorCode::NotFound.into())
        }
    }

    /// The event of putting a saved search into the trash or taking it out (the version counts
    /// up), and the search as it is afterwards.
    fn trash_event(
        &self,
        actor: &Actor,
        found: &SavedSearch,
        deleted: bool,
    ) -> Result<(DomainEvent, SavedSearch), AppError> {
        let now = self.now();
        let deleted_at = deleted.then_some(now);
        let event = self.event(
            by(actor.person.id),
            entity(RecordKind::SavedSearch, found.id),
            vec![
                FieldChange {
                    field: member::DELETED_AT.to_owned(),
                    before: found.deleted_at.map(|at| json!(at.to_string())),
                    after: deleted_at.map(|at| json!(at.to_string())),
                },
                counted_up(found.version),
            ],
        )?;
        let after = SavedSearch {
            deleted_at,
            updated_at: now,
            version: next(found.version),
            ..found.clone()
        };
        Ok((event, after))
    }
}

/// Saves a search under a name, private and the person's own.
///
/// # Errors
/// `request.invalid` for a name that cannot be one, a text that is too long or holds control
/// characters, or a presentation that cannot be kept.
pub async fn save_search<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SaveSearch,
) -> Result<Authorized<SavedSearchView>, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let text = clean_search_text(&input.text)?;
    let new = ctx.new_search(&actor, &input.name, text, input.presentation)?;
    ctx.store()
        .record_change(
            &new.event,
            inserting::<S>(new.search.clone(), new.columns.clone()),
        )
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    ctx.new_view(&actor, new).await
}

/// Changes the name, the text or the way a saved search is shown. Only its maker does.
///
/// # Errors
/// `resource.not-found` for a search the person does not see, `access.forbidden` for one they see
/// and did not make, `request.invalid` as for saving, `edit.version-changed` when it changed since
/// the person looked at it.
pub async fn change_search<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ChangeSearch,
) -> Result<Authorized<SavedSearchView>, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let Opened { stored, reaching } = ctx.own_search(&actor, input.search).await?;
    let before = stored.search.clone();
    if before.version != input.version {
        return Err(ErrorCode::VersionChanged.into());
    }
    let mut search = before.clone();
    if let Some(name) = &input.name {
        search.name = clean_search_name(name)?;
    }
    if let Some(text) = &input.text {
        search.text = clean_search_text(text)?;
    }
    let presentation = match input.presentation {
        Some(presentation) => checked(presentation)?,
        None => Presentation::of(&before, &stored.columns),
    };
    search.view = presentation.view;
    search.spacing = presentation.spacing;
    search.group_by = presentation.group_by.clone();
    let columns = presentation.rows(search.id);
    let changes = events::changed((&before, &stored.columns), (&search, &columns));
    if changes.is_empty() {
        return ctx.authorized_view(&actor, &reaching, stored).await;
    }
    search.updated_at = ctx.now();
    search.version = next(before.version);
    let event = ctx.event(
        by(actor.person.id),
        entity(RecordKind::SavedSearch, search.id),
        changes,
    )?;
    let (written, written_columns, was) = (search.clone(), columns.clone(), before.version);
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                if S::update_saved_search(transaction, &written, &written_columns, was).await? {
                    Ok(())
                } else {
                    Err(S::aborted())
                }
            })
        })
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::both(ErrorCode::VersionChanged)))?;
    let changed = Stored {
        search,
        columns,
        shares: stored.shares,
    };
    ctx.authorized_view(&actor, &reaching, changed).await
}

/// Puts a saved search into the trash. Its maker does, and so does anybody with *Change
/// organization settings*.
///
/// # Errors
/// `resource.not-found` for a search that does not exist, is in the trash already or that the
/// person does not see, `access.forbidden` for one they see and may not delete.
pub async fn delete_search<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: DeleteSearch,
) -> Result<Done, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let (found, _) = ctx.search_to_trash(&actor, input.search, false).await?;
    let (event, after) = ctx.trash_event(&actor, &found.search, true)?;
    let why = Why::default();
    ctx.store()
        .record_change(
            &event,
            trashing::<S>(&after, found.search.version, why.clone()),
        )
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::NotFound))?;
    Ok(Done)
}

/// Takes a saved search out of the trash (*Undo*), for whoever may put it there.
///
/// # Errors
/// `resource.not-found` for a search that is not in the trash or that the person may not take out:
/// whoever may not is not told that it is in the trash (SEC-06).
pub async fn restore_search<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: RestoreSearch,
) -> Result<Authorized<SavedSearchView>, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let (found, reaching) = ctx.search_to_trash(&actor, input.search, true).await?;
    let (event, after) = ctx.trash_event(&actor, &found.search, false)?;
    let why = Why::default();
    ctx.store()
        .record_change(
            &event,
            trashing::<S>(&after, found.search.version, why.clone()),
        )
        .await
        .map_err(|error| why.answer::<S>(&error, ErrorCode::NotFound))?;
    let restored = Stored {
        search: after,
        ..found
    };
    ctx.authorized_view(&actor, &reaching, restored).await
}

/// *Save a copy*: a private saved search of the person with the text and the presentation of one
/// they may open. Everyone who sees a search may copy it.
///
/// # Errors
/// `resource.not-found` for a search the person does not see, `request.invalid` for a name that
/// cannot be one.
pub async fn save_copy<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SaveCopy,
) -> Result<Authorized<SavedSearchView>, AppError> {
    let actor = ctx.inbox_owner(&input.session_token).await?;
    let original = ctx.open_search(&actor, input.search).await?.stored;
    let name = input.name.unwrap_or_else(|| original.search.name.clone());
    let presentation = Presentation::of(&original.search, &original.columns);
    let new = ctx.new_search(&actor, &name, original.search.text, presentation)?;
    ctx.store()
        .record_change(
            &new.event,
            inserting::<S>(new.search.clone(), new.columns.clone()),
        )
        .await
        .map_err(|error| change_failed::<S>(&error, Stops::NONE))?;
    ctx.new_view(&actor, new).await
}
