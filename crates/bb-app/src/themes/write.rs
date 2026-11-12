//! Changing own themes: making one, saving its values under the edit lock, keeping a copy, making
//! one from an uploaded file, and taking one away (THM-04, ADR-0026 update M8). Every change is an
//! event (ARCH-EVENTS); the limits of the number of themes are checked in the change that adds one.

use bb_core::ErrorCode;
use std::sync::Arc;

use bb_core::event::{DomainEvent, FieldChange, RecordKind};
use bb_core::id::{ThemeId, UserId};
use bb_core::technical::TechnicalName;
use bb_core::theme::{
    FileProblem, OwnTheme, OwnValues, PERSONAL_THEMES_MOST, THEME_UPLOAD_MOST,
    THEMES_FOR_EVERYONE_MOST, ThemeFile, ThemeKind, ThemeName,
};
use bb_core::version::{FIRST_VERSION, Version, counted_up};
use bb_policy::{Authorized, Evaluation, Place};
use serde_json::{Value, json};

use super::rules::{
    NamedPair, Need, built_in_of, editing_what, hard_pairs, name_of, refuse_hard_for_everyone,
    values_of, view_of,
};
use super::types::{
    CopyTheme, CreateTheme, DeleteTheme, SaveTheme, ThemeDetail, ThemeView, UploadTheme,
};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, Done, by, entity, set_to};
use crate::editing::{TextSave, Written, let_go_for_delete};
use crate::member::theme as member;
use crate::projects::Why;

/// The values of a theme as an event writes them.
fn summary(values: &OwnValues) -> Value {
    serde_json::to_value(values).unwrap_or(Value::Null)
}

/// The most themes of `kind` there may be, and the answer when there are.
const fn limit_of(kind: ThemeKind) -> (usize, ErrorCode) {
    match kind {
        ThemeKind::ForEveryone => (THEMES_FOR_EVERYONE_MOST, ErrorCode::ThemesForEveryoneMost),
        ThemeKind::Personal => (PERSONAL_THEMES_MOST, ErrorCode::PersonalThemesMost),
    }
}

/// A new theme to add.
struct NewTheme {
    kind: ThemeKind,
    name: ThemeName,
    extends: TechnicalName,
    values: OwnValues,
    /// Saved at once (a copy, an upload), or listed nowhere until its first save (a new theme).
    saved: bool,
}

/// A theme to add, with the event that adds it.
struct Adding {
    theme: OwnTheme,
    event: DomainEvent,
}

/// What adding a theme that is saved at once needs: what the actor holds, the theme with its
/// event, and the pairs that are hard to read in it (the warning of a personal theme).
struct AddingSaved {
    held: Arc<Evaluation>,
    adding: Adding,
    hard_to_read: Vec<NamedPair>,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The theme `new` of the actor, and the event that adds it.
    fn adding(&self, actor: &Actor, new: NewTheme) -> Result<Adding, AppError> {
        let now = self.now();
        let theme = OwnTheme {
            id: ThemeId::generate(&mut self.ids())
                .map_err(|error| AppError::Internal(error.to_string()))?,
            kind: new.kind,
            maker: actor.person.id,
            name: new.name,
            extends: new.extends,
            values: new.values,
            saved_at: new.saved.then_some(now),
            created_at: now,
            updated_at: now,
            version: FIRST_VERSION,
        };
        let event = self.event(
            by(actor.person.id),
            entity(RecordKind::Theme, theme.id),
            vec![
                set_to(member::KIND, json!(theme.kind.code())),
                set_to(member::NAME, json!(theme.name.as_str())),
                set_to(member::EXTENDS, json!(theme.extends.as_str())),
                set_to(member::VALUES, summary(&theme.values)),
            ],
        )?;
        Ok(Adding { theme, event })
    }

    /// A theme that is saved at once (a copy, an upload), checked like a save: a theme for everyone
    /// with a pair that is *Hard to read* is refused, and a personal one is saved with the pairs
    /// named.
    async fn adding_saved(
        &self,
        actor: &Actor,
        kind: ThemeKind,
        name: ThemeName,
        extends: &str,
        values: OwnValues,
    ) -> Result<AddingSaved, AppError> {
        let held = self.may_create_theme(actor, kind).await?;
        let base = built_in_of(extends)?;
        let hard_to_read = refuse_hard_for_everyone(kind, hard_pairs(base.id, &values)?)?;
        let adding = self.adding(
            actor,
            NewTheme {
                kind,
                name,
                extends: technical(base.id)?,
                values,
                saved: true,
            },
        )?;
        Ok(AddingSaved {
            held,
            adding,
            hard_to_read,
        })
    }
}

/// Inside the change that adds `theme`: refuses it when the limit of its kind is reached (a theme
/// that was never saved counts too), and adds it otherwise.
async fn insert_within_limit<S: AuthStore>(
    transaction: &mut S::Transaction,
    theme: OwnTheme,
    stopped: Why,
) -> Result<(), S::Error> {
    let (most, refusal) = limit_of(theme.kind);
    let count = match theme.kind {
        ThemeKind::ForEveryone => S::count_themes_for_everyone_in(transaction).await?,
        ThemeKind::Personal => S::count_personal_themes_of_in(transaction, theme.maker).await?,
    };
    if usize::try_from(count).is_ok_and(|count| count >= most) {
        return Err(stopped.stop::<S>(refusal));
    }
    S::insert_own_theme(transaction, &theme).await
}

/// The answer of a theme that was added at once.
fn added(saved: AddingSaved, person: UserId) -> Authorized<ThemeDetail> {
    let view = view_of(saved.adding.theme, &saved.held, person);
    saved.held.authorize(ThemeDetail {
        view,
        hard_to_read: saved.hard_to_read,
    })
}

fn technical(id: &str) -> Result<TechnicalName, AppError> {
    TechnicalName::new(id).map_err(|error| AppError::Internal(error.to_string()))
}

/// Makes a new own theme that extends a built-in theme and changes nothing yet. It is listed
/// nowhere until its first save; it counts against the limit of its kind from now on.
///
/// # Errors
/// `request.invalid` for a name that is empty or too long or a built-in theme that does not exist,
/// `access.forbidden` for a theme for everyone without *Change organization settings*,
/// `theme.too-many-for-everyone` or `theme.too-many-personal` at the limit.
pub async fn create_theme<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CreateTheme,
) -> Result<Authorized<ThemeView>, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let held = ctx.may_create_theme(&actor, input.kind).await?;
    let base = built_in_of(&input.extends)?;
    let adding = ctx.adding(
        &actor,
        NewTheme {
            kind: input.kind,
            name: name_of(&input.name)?,
            extends: technical(base.id)?,
            values: OwnValues::default(),
            saved: false,
        },
    )?;
    let why = Why::default();
    let (stopped, theme) = (why.clone(), adding.theme.clone());
    ctx.store()
        .record_change(&adding.event, move |transaction| {
            Box::pin(insert_within_limit::<S>(transaction, theme, stopped))
        })
        .await
        .map_err(|error| why.said_or_failed::<S>(&error))?;
    Ok(held.authorize(view_of(adding.theme, &held, actor.person.id)))
}

/// Saves the name and the values of an own theme under the edit lock the person holds. A theme for
/// everyone with a pair that is *Hard to read* is refused, naming the pairs; a personal one is
/// saved, and the answer names them. The first save lists the theme.
///
/// # Errors
/// `resource.not-found` for a theme the person may not use, `access.forbidden` for one they may not
/// change, `request.invalid` for a name that is empty or too long, `theme.value-refused` naming each
/// value an own theme may not set, `theme.hard-to-read` naming the pairs of a theme for everyone;
/// `edit.lock-lost`, `edit.taken-over` and `edit.version-changed` for a lock that is gone, was
/// taken over, or a theme that changed since (the values are kept as the person's draft).
pub async fn save_theme<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: SaveTheme,
) -> Result<Authorized<ThemeDetail>, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let (stored, held) = ctx.theme_for(&actor, input.theme, Need::Change).await?;
    let name = name_of(&input.name)?;
    let values = values_of(&input.values)?;
    let hard_to_read =
        refuse_hard_for_everyone(stored.kind, hard_pairs(stored.extends.as_str(), &values)?)?;
    let what = editing_what(stored.id)?;
    let event = ctx.event(
        by(actor.person.id),
        what.record.clone(),
        vec![
            FieldChange {
                field: member::NAME.to_owned(),
                before: Some(json!(stored.name.as_str())),
                after: Some(json!(name.as_str())),
            },
            FieldChange {
                field: member::VALUES.to_owned(),
                before: Some(summary(&stored.values)),
                after: Some(summary(&values)),
            },
            counted_up(input.version),
        ],
    )?;
    let now = ctx.now();
    let written = OwnTheme {
        name,
        values,
        saved_at: stored.saved_at.or(Some(now)),
        updated_at: now,
        ..stored
    };
    let save = TextSave {
        what,
        token: input.lock_token,
        text: input.values.to_string(),
        version: input.version,
    };
    let (version, row) = (input.version, written.clone());
    ctx.save_text(
        actor.person.id,
        Place::Organization,
        &save,
        &event,
        move |transaction| {
            Box::pin(async move {
                if S::save_own_theme(transaction, &row, Version::new(version)).await? {
                    Ok(Written::Saved(()))
                } else {
                    Ok(Written::Stale)
                }
            })
        },
    )
    .await?;
    let view = view_of(
        OwnTheme {
            version: version + 1,
            ..written
        },
        &held,
        actor.person.id,
    );
    Ok(held.authorize(ThemeDetail { view, hard_to_read }))
}

/// Keeps an own theme the person may use as a new one of theirs (*Save a copy*), saved at once.
///
/// # Errors
/// `resource.not-found` for a theme the person may not use, `request.invalid` for a name that is
/// empty or too long, `access.forbidden` for a copy for everyone without *Change organization
/// settings*, `theme.hard-to-read` for a copy for everyone with a pair that is hard to read, the
/// answers of a limit.
pub async fn copy_theme<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: CopyTheme,
) -> Result<Authorized<ThemeDetail>, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let (source, _) = ctx.theme_for(&actor, input.theme, Need::Use).await?;
    let name = name_of(&input.name)?;
    let saved = ctx
        .adding_saved(
            &actor,
            input.kind,
            name,
            source.extends.as_str(),
            source.values,
        )
        .await?;
    let why = Why::default();
    let (stopped, theme) = (why.clone(), saved.adding.theme.clone());
    ctx.store()
        .record_change(&saved.adding.event, move |transaction| {
            Box::pin(insert_within_limit::<S>(transaction, theme, stopped))
        })
        .await
        .map_err(|error| why.said_or_failed::<S>(&error))?;
    Ok(added(saved, actor.person.id))
}

/// Makes an own theme from a file in the shape of the export (`name`, `extends`, `values`), checked
/// like a save and saved at once.
///
/// # Errors
/// `request.too-large` for a file over [`THEME_UPLOAD_MOST`] bytes, `request.invalid` for a file
/// that is no such JSON object, a name that is empty or too long or a built-in theme that does not
/// exist, `theme.value-refused` naming each part or value that is not allowed, and the answers of a
/// copy.
pub async fn upload_theme<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: UploadTheme,
) -> Result<Authorized<ThemeDetail>, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    if input.file.len() > THEME_UPLOAD_MOST {
        return Err(ErrorCode::TooLarge.into());
    }
    let file = serde_json::from_slice::<Value>(&input.file)
        .map_err(|_| AppError::Refused(ErrorCode::InvalidRequest))?;
    let upload = ThemeFile::read(&file).map_err(|problem| match problem {
        FileProblem::NotATheme => AppError::Refused(ErrorCode::InvalidRequest),
        FileProblem::Values(found) => AppError::ThemeValuesRefused(found),
    })?;
    let saved = ctx
        .adding_saved(
            &actor,
            input.kind,
            name_of(&upload.name)?,
            &upload.extends,
            upload.values,
        )
        .await?;
    let why = Why::default();
    let (stopped, theme) = (why.clone(), saved.adding.theme.clone());
    ctx.store()
        .record_change(&saved.adding.event, move |transaction| {
            Box::pin(insert_within_limit::<S>(transaction, theme, stopped))
        })
        .await
        .map_err(|error| why.said_or_failed::<S>(&error))?;
    Ok(added(saved, actor.person.id))
}

/// Takes an own theme away. Its edit lock, if the person holds it, goes in the same change; a
/// choice of it counts as the built-in theme it extended from now on.
///
/// # Errors
/// `resource.not-found` for a theme the person may not use, `access.forbidden` for one they may not
/// change, `edit.version-changed` for a theme that changed since the person looked at it,
/// `edit.in-progress` while somebody else holds its edit lock.
pub async fn delete_theme<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: DeleteTheme,
) -> Result<Done, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let (stored, _) = ctx.theme_for(&actor, input.theme, Need::Change).await?;
    let what = editing_what(stored.id)?;
    let record = what.record.clone();
    let event = ctx.event(
        by(actor.person.id),
        record.clone(),
        vec![
            FieldChange {
                field: member::NAME.to_owned(),
                before: Some(json!(stored.name.as_str())),
                after: None,
            },
            FieldChange {
                field: member::VALUES.to_owned(),
                before: Some(summary(&stored.values)),
                after: None,
            },
            counted_up(input.version),
        ],
    )?;
    let why = Why::default();
    let stopped = why.clone();
    let (id, version, me, now) = (stored.id, input.version, actor.person.id, ctx.now());
    ctx.store()
        .record_change(&event, move |transaction| {
            Box::pin(async move {
                let_go_for_delete::<S>(transaction, &what, me, now, &stopped).await?;
                if S::delete_own_theme(transaction, id, Version::new(version)).await? {
                    Ok(())
                } else {
                    Err(stopped.stop::<S>(ErrorCode::VersionChanged))
                }
            })
        })
        .await
        .map_err(|error| {
            why.answer::<S>(&error, ErrorCode::VersionChanged).about(
                &record,
                Place::Organization,
                actor.person.id,
            )
        })?;
    Ok(Done)
}
