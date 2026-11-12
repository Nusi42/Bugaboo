//! The style sheets of own themes (M8-Q8, ADR-0026 update M8): `/themes/{id}.css?v=<version>` for a
//! saved theme and `/themes/{id}/draft.css?v=<n>` for the editor's own draft. Bugaboo serves them
//! from its own origin, so the content security policy stays `style-src 'self'`; each holds custom
//! properties and nothing else, written by the one writer of `bb-core`.
//!
//! An address with the version the sheet has now is kept like a hashed asset, but only by the
//! browser of the person (`private`): a sheet is for the people who may use the theme. A changed
//! theme has a new address; an address with another version is asked again every time. The sheet
//! of a draft is never kept: its `v` only gives the preview a new address when the draft changes.

use axum::extract::State;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse as _, Response};
use bb_api_words::OWN_THEME_SHEETS;
use bb_app::themes::{ThemeSheet, ThemeSheetQuery};
use bb_core::id::ThemeId;
use serde::Deserialize;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::theme_named;
use crate::api::auth::{AuthHandle, Info, NotFoundAnswer, NotSignedInAnswer, Session};
use crate::errors::{ApiError, ApiPath, ApiQuery};

/// The ending of the address of a saved theme's sheet.
const SHEET_ENDING: &str = ".css";
/// The content type of a style sheet.
const CSS: &str = "text/css; charset=utf-8";
/// A sheet whose address names the version it has: kept by the person's browser like a hashed
/// asset, never by a shared cache.
const CACHE_THIS_VERSION: &str = "private, max-age=31536000, immutable";
/// A sheet asked for with another version, or none: asked again every time.
const CACHE_ASK_AGAIN: &str = "private, no-cache";
/// The sheet of a draft: never kept.
const CACHE_NEVER: &str = "private, no-store";

/// The address of the style sheet of a saved own theme at `version`: what a page links.
pub(crate) fn sheet_address(theme: ThemeId, version: i64) -> String {
    format!("{OWN_THEME_SHEETS}{theme}{SHEET_ENDING}?v={version}")
}

/// The version an address asks for.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct SheetParams {
    /// The version of the theme, or of the draft, the address was made for.
    v: Option<i64>,
}

fn css_answer(sheet: ThemeSheet, cache: &'static str) -> Response {
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, HeaderValue::from_static(CSS)),
            (header::CACHE_CONTROL, HeaderValue::from_static(cache)),
        ],
        sheet.css,
    )
        .into_response()
}

/// How the sheet of a saved theme may be kept: like a hashed asset while the address names the
/// version it has, else asked for again.
fn saved_rule(sheet: &ThemeSheet, asked: Option<i64>) -> &'static str {
    if asked == Some(sheet.version) {
        CACHE_THIS_VERSION
    } else {
        CACHE_ASK_AGAIN
    }
}

/// The style sheet of a saved own theme: its variables under `data-own-theme`, for every person
/// who may use the theme.
#[utoipa::path(
    get,
    path = "/themes/{theme}",
    operation_id = "getThemeSheet",
    tag = "themes",
    params(("theme" = String, Path, description = "The ID of the theme followed by `.css`."), SheetParams),
    responses((status = 200, description = "The style sheet.", content_type = "text/css", body = String), NotSignedInAnswer, NotFoundAnswer)
)]
async fn get_theme_sheet(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(file): ApiPath<String>,
    ApiQuery(params): ApiQuery<SheetParams>,
) -> Result<Response, ApiError> {
    let theme = theme_named(
        &auth,
        &info,
        &session_token,
        file.strip_suffix(SHEET_ENDING).unwrap_or_default(),
    )
    .await?;
    let sheet = auth
        .api()?
        .theme_sheet(
            info,
            ThemeSheetQuery {
                session_token,
                theme,
            },
        )
        .await?;
    let rule = saved_rule(&sheet, params.v);
    Ok(css_answer(sheet, rule))
}

/// The style sheet of the person's own draft of a theme, for the preview of the editor: served to
/// nobody else.
#[utoipa::path(
    get,
    path = "/themes/{theme}/draft.css",
    operation_id = "getThemeDraftSheet",
    tag = "themes",
    params(("theme" = String, Path, description = "The ID of the theme."), SheetParams),
    responses((status = 200, description = "The style sheet of the draft.", content_type = "text/css", body = String), NotSignedInAnswer, NotFoundAnswer)
)]
async fn get_theme_draft_sheet(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(theme): ApiPath<String>,
) -> Result<Response, ApiError> {
    let theme = theme_named(&auth, &info, &session_token, &theme).await?;
    let sheet = auth
        .api()?
        .theme_draft_sheet(
            info,
            ThemeSheetQuery {
                session_token,
                theme,
            },
        )
        .await?;
    // A draft changes while it is edited: it is never kept, whatever its address says.
    Ok(css_answer(sheet, CACHE_NEVER))
}

/// The routes of the style sheets.
pub(crate) fn sheet_routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(get_theme_sheet))
        .routes(routes!(get_theme_draft_sheet))
}
