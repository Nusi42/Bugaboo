//! Reading own themes: the list a person may choose from, one theme with what is hard to read in
//! it, the file of a download, and the style sheets of a theme and of its draft (ADR-0026, update
//! M8). A theme somebody may not use is answered as if it did not exist.

use bb_core::ErrorCode;
use bb_core::edit::{DraftTarget, draft_cutoff};
use bb_core::settings::{self, SettingDefinition, SettingValue};
use bb_core::theme::{OwnValues, ThemeFile, ThemeKind, draft_style_sheet, style_sheet};
use bb_policy::{Authorized, Place};

use super::rules::{Need, editing_what, hard_pairs_of, sees_unsaved_for_everyone, view_of};
use super::types::{
    ThemeDetail, ThemeExport, ThemeList, ThemeQuery, ThemeSheet, ThemeSheetQuery, ThemesQuery,
};
use crate::auth::{AppError, AuthStore, Ctx, internal};

/// The sheet `write` makes of a theme with these values, or a failure: the values were checked when
/// they were stored, so a sheet that cannot be made is a fault of the server.
fn sheet_of(
    write: fn(&str, &OwnValues) -> Option<String>,
    theme: &str,
    values: &OwnValues,
    version: i64,
) -> Result<ThemeSheet, AppError> {
    let css = write(theme, values)
        .ok_or_else(|| AppError::Internal("a stored theme has values a sheet refuses".into()))?;
    Ok(ThemeSheet { css, version })
}

/// The settings that keep a choice of a theme.
fn theme_settings() -> Vec<&'static SettingDefinition> {
    vec![
        settings::APPEARANCE_LIGHT_THEME.definition(),
        settings::APPEARANCE_DARK_THEME.definition(),
    ]
}

/// The own themes listed to the person, by name: the saved themes for everyone and every theme
/// they made, saved or not (`ThemeView::theme.saved_at` tells), and for who holds *Change
/// organization settings* also the themes for everyone that were never saved. Only a saved theme
/// can be chosen.
///
/// # Errors
/// `auth.not-signed-in` without a session.
pub async fn themes<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ThemesQuery,
) -> Result<Authorized<ThemeList>, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let held = ctx.evaluation_of(&actor, Place::Organization).await?;
    let found = ctx
        .store()
        .themes_listed_for(actor.person.id, sees_unsaved_for_everyone(&held))
        .await
        .map_err(|error| internal::<S>(&error))?;
    let themes = found
        .into_iter()
        .map(|theme| view_of(theme, &held, actor.person.id))
        .collect();
    Ok(held.authorize(ThemeList { themes }))
}

/// One own theme, with the pairs of its saved values that are *Hard to read*.
///
/// # Errors
/// `resource.not-found` for a theme the person may not use.
pub async fn theme<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ThemeQuery,
) -> Result<Authorized<ThemeDetail>, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let (found, held) = ctx.theme_for(&actor, input.theme, Need::Use).await?;
    let hard_to_read = hard_pairs_of(found.extends.as_str(), &found.values)?;
    let view = view_of(found, &held, actor.person.id);
    let used_by = if view.may_change && view.theme.kind == ThemeKind::ForEveryone {
        let chosen = SettingValue::Text(view.choice.clone());
        Some(
            ctx.store()
                .people_choosing(theme_settings(), chosen)
                .await
                .map_err(|error| internal::<S>(&error))?,
        )
    } else {
        None
    };
    Ok(held.authorize(ThemeDetail {
        view,
        hard_to_read,
        used_by,
    }))
}

/// An own theme as a file: its name, the built-in theme it extends and its values, the shape an
/// upload takes.
///
/// # Errors
/// `resource.not-found` for a theme the person may not use.
pub async fn download_theme<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ThemeQuery,
) -> Result<Authorized<ThemeExport>, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let (found, held) = ctx.theme_for(&actor, input.theme, Need::Use).await?;
    Ok(held.authorize(ThemeExport(ThemeFile {
        name: found.name.as_str().to_owned(),
        extends: found.extends.as_str().to_owned(),
        values: found.values,
    })))
}

/// The style sheet of a saved own theme, for every person who may use it. It holds the stored
/// values and nothing of a person, so it leaves the use case as it is, like a downloaded file.
///
/// # Errors
/// `resource.not-found` for a theme the person may not use, or one that was never saved.
pub async fn theme_sheet<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ThemeSheetQuery,
) -> Result<ThemeSheet, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let (found, _) = ctx.theme_for(&actor, input.theme, Need::Use).await?;
    if found.saved_at.is_none() {
        return Err(ErrorCode::NotFound.into());
    }
    sheet_of(
        style_sheet,
        &found.id.to_string(),
        &found.values,
        found.version,
    )
}

/// The style sheet of the person's own draft of a theme, for the preview of the editor: nobody
/// else's draft is ever served, and a draft whose values a save would refuse has no sheet.
///
/// # Errors
/// `resource.not-found` for a theme the person may not use or has no draft of, or a draft with a
/// value an own theme may not set; `access.forbidden` for a theme they may not change.
pub async fn theme_draft_sheet<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: ThemeSheetQuery,
) -> Result<ThemeSheet, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let (found, _) = ctx.theme_for(&actor, input.theme, Need::Change).await?;
    let not_before = draft_cutoff(ctx.now())
        .ok_or_else(|| AppError::Internal("a keep period that cannot be written".to_owned()))?;
    let draft = ctx
        .store()
        .draft(
            actor.person.id,
            DraftTarget::Editing(editing_what(found.id)?),
            not_before,
        )
        .await
        .map_err(|error| internal::<S>(&error))?
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    let values = serde_json::from_str::<serde_json::Value>(&draft.text)
        .ok()
        .and_then(|json| OwnValues::from_json(&json).ok())
        .ok_or(AppError::Refused(ErrorCode::NotFound))?;
    sheet_of(
        draft_style_sheet,
        &found.id.to_string(),
        &values,
        draft.saved_at.as_microsecond(),
    )
}
