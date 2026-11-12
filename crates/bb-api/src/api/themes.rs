//! The routes of own themes (THM-03, THM-04, ADR-0026 update M8): `/api/v1/themes` for the themes a
//! person may use, make, save under the edit lock, copy, download, upload and take away, and the
//! style sheets of a theme and of the draft of its editor at `/themes/…`, which a page links.
//!
//! A theme for everyone is used by every signed-in person of the organization and changed with
//! *Change organization settings*; a personal theme is its maker's alone. Anything else is answered
//! as a theme that does not exist. The use cases are in `bb-app`, behind
//! [`AuthApi`](bb_app::auth::AuthApi).

use axum::body::Bytes;
use axum::extract::State;
use axum::response::Response;
use bb_app::auth::RequestInfo;
use bb_app::themes::{
    CopyTheme, CreateTheme, DeleteTheme, SaveTheme, ThemeDetail, ThemeExport, ThemeList,
    ThemeQuery, ThemeView, ThemesQuery, UploadTheme,
};
use bb_core::id::{LockToken, ThemeId};
use bb_core::theme::{
    PERSONAL_THEMES_MOST, ReadabilitySummary, THEMES_FOR_EVERYONE_MOST, ThemeKind,
};
use bb_policy::Masked;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{
    AuthHandle, ConflictAnswer, ForbiddenAnswer, Info, InvalidAnswer, NotFoundAnswer,
    NotSignedInAnswer, Session, answer_authorized, done,
};
use super::{body_id, path_id};
use crate::conflict::HardPairBody;
use crate::errors::{ApiError, ApiJson, ApiPath, ApiQuery};

mod sheets;

pub(crate) use sheets::{sheet_address, sheet_routes};

/// The theme an address names. A text that names none is answered after who asks is known, so an
/// access key is turned away as a key before it learns anything about the address.
pub(crate) async fn theme_named(
    auth: &AuthHandle,
    info: &RequestInfo,
    session_token: &str,
    text: &str,
) -> Result<ThemeId, ApiError> {
    auth.after_who_asks(info, session_token, path_id::<ThemeId>(text))
        .await
}

/// The most themes a person can be offered: every theme for everyone and all of their own.
const THEMES_LISTED_MOST: usize = THEMES_FOR_EVERYONE_MOST + PERSONAL_THEMES_MOST;

/// An own theme as a person who may use it sees it.
#[derive(Serialize, ToSchema)]
pub(crate) struct ThemeBody {
    /// The ID.
    id: String,
    /// What it is called.
    name: String,
    /// For everyone or personal.
    kind: ThemeKind,
    /// The ID of the built-in theme it extends.
    extends: String,
    /// Whether it is a light theme (it extends a light built-in theme).
    light: bool,
    /// The value the settings *Light theme* or *Dark theme* keep when the person chooses it.
    choice: String,
    /// The address of its style sheet, which changes with every save. Absent while it was never
    /// saved.
    #[serde(skip_serializing_if = "Option::is_none")]
    sheet: Option<String>,
    /// Whether the person may change it and take it away.
    may_change: bool,
    /// Whether it was saved at least once. A theme never saved is listed to its maker (and, for
    /// everyone, to who holds *Change organization settings*) and cannot be chosen.
    saved: bool,
    /// Counts up with every save; a change and a deletion send it back.
    version: i64,
    /// How many of its pairs of colors are *Easy*, *OK* and *Hard to read*, as saved.
    readability: ReadabilityBody,
}

/// How many pairs of colors of a theme are *Easy*, *OK* and *Hard to read*; a pair counts once, at
/// its worst rating.
#[derive(Serialize, ToSchema)]
pub(crate) struct ReadabilityBody {
    /// The pairs that are *Easy*.
    easy: u32,
    /// The pairs that are *OK*.
    ok: u32,
    /// The pairs that are *Hard to read*.
    hard_to_read: u32,
    /// The colors to change to mend them.
    colors_hard_to_read: u32,
}

impl From<ReadabilitySummary> for ReadabilityBody {
    fn from(summary: ReadabilitySummary) -> Self {
        Self {
            easy: summary.easy,
            ok: summary.ok,
            hard_to_read: summary.hard_to_read,
            colors_hard_to_read: summary.colors_hard_to_read,
        }
    }
}

impl From<&ThemeView> for ThemeBody {
    fn from(view: &ThemeView) -> Self {
        let theme = &view.theme;
        let saved = theme.saved_at.is_some();
        Self {
            id: theme.id.to_string(),
            name: theme.name.as_str().to_owned(),
            kind: theme.kind,
            extends: theme.extends.as_str().to_owned(),
            light: view.light,
            choice: view.choice.clone(),
            sheet: saved.then(|| sheet_address(theme.id, theme.version)),
            may_change: view.may_change,
            saved,
            version: theme.version,
            readability: view.readability.into(),
        }
    }
}

impl Masked for ThemeBody {}

/// The own themes a person may use.
#[derive(Serialize, ToSchema)]
pub(crate) struct ThemeListBody {
    /// The themes for everyone and the person's own personal themes, by name.
    themes: Vec<ThemeBody>,
    /// Whether the list was cut at its cap; the limits of the number of themes keep it below.
    truncated: bool,
}

impl Masked for ThemeListBody {}

impl From<&ThemeList> for ThemeListBody {
    fn from(list: &ThemeList) -> Self {
        Self {
            themes: list
                .themes
                .iter()
                .take(THEMES_LISTED_MOST)
                .map(ThemeBody::from)
                .collect(),
            truncated: list.themes.len() > THEMES_LISTED_MOST,
        }
    }
}

/// An own theme with its values and the pairs that are hard to read.
#[derive(Serialize, ToSchema)]
pub(crate) struct ThemeDetailBody {
    /// The theme.
    theme: ThemeBody,
    /// The values it changes of the built-in theme it extends: `colors` (each with `value` and,
    /// optionally, `more`), `shadow` (at most two layers), `radii` and `fonts`.
    #[schema(value_type = bb_core::theme::OwnValues)]
    values: serde_json::Value,
    /// The pairs of its colors that are *Hard to read*: the warning of a personal theme. A theme
    /// for everyone with any is never saved.
    hard_to_read: Vec<HardPairBody>,
    /// How many people chose it themselves: only for a theme for everyone, and only for who may
    /// change it, so the question before it is deleted can say how many people it reaches.
    #[serde(skip_serializing_if = "Option::is_none")]
    used_by: Option<u64>,
    /// Whether it is the organization's choice of a light or a dark theme, which everyone who keeps
    /// that choice follows: for who may change it, beside `used_by`.
    #[serde(skip_serializing_if = "Option::is_none")]
    organization_choice: Option<bool>,
}

impl Masked for ThemeDetailBody {}

impl From<&ThemeDetail> for ThemeDetailBody {
    fn from(detail: &ThemeDetail) -> Self {
        Self {
            theme: ThemeBody::from(&detail.view),
            values: serde_json::to_value(&detail.view.theme.values)
                .unwrap_or(serde_json::Value::Null),
            hard_to_read: detail.hard_to_read.iter().map(HardPairBody::from).collect(),
            used_by: detail.used_by,
            organization_choice: detail.organization_choice,
        }
    }
}

/// An own theme as a file: what a download gives and an upload takes.
#[derive(Serialize, Deserialize, ToSchema)]
pub(crate) struct ThemeFileBody {
    /// What the theme is called.
    name: String,
    /// The ID of the built-in theme it extends.
    extends: String,
    /// The values it changes, as in a theme.
    #[schema(value_type = bb_core::theme::OwnValues)]
    values: serde_json::Value,
}

/// A new own theme.
#[derive(Deserialize, ToSchema)]
pub(crate) struct NewThemeBody {
    /// What it is called.
    name: String,
    /// For everyone (needs *Change organization settings*) or personal.
    kind: ThemeKind,
    /// The ID of the built-in theme it extends.
    extends: String,
}

/// The values of an own theme to save.
#[derive(Deserialize, ToSchema)]
pub(crate) struct SaveThemeBody {
    /// The version the person started from.
    version: i64,
    /// The token the edit lock was taken with.
    lock_token: String,
    /// What it is called now.
    name: String,
    /// The values it changes, in the shape of `values` of a theme.
    #[schema(value_type = bb_core::theme::OwnValues)]
    values: serde_json::Value,
}

/// A copy of an own theme to keep.
#[derive(Deserialize, ToSchema)]
pub(crate) struct CopyThemeBody {
    /// What the copy is called.
    name: String,
    /// For everyone (needs *Change organization settings*) or personal.
    kind: ThemeKind,
}

/// What an uploaded theme becomes.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct UploadThemeParams {
    /// For everyone (needs *Change organization settings*) or personal.
    kind: ThemeKind,
}

/// The version of the theme to take away.
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct DeleteThemeParams {
    /// The version the person looked at.
    version: i64,
}

/// The own themes the person may use: those for everyone and their own personal ones.
#[utoipa::path(
    get,
    path = "/api/v1/themes",
    operation_id = "listThemes",
    tag = "themes",
    responses((status = 200, description = "The themes, by name.", body = ThemeListBody), NotSignedInAnswer)
)]
async fn list_themes(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    let list = auth
        .api()?
        .themes(info, ThemesQuery { session_token })
        .await?;
    answer_authorized(&list.map(|list| ThemeListBody::from(&list)))
}

/// Makes a new own theme that extends a built-in theme. It cannot be chosen until its first save.
/// A theme for everyone needs *Change organization settings*.
#[utoipa::path(
    post,
    path = "/api/v1/themes",
    operation_id = "createTheme",
    tag = "themes",
    request_body = NewThemeBody,
    responses((status = 200, description = "The new theme.", body = ThemeBody), InvalidAnswer, NotSignedInAnswer, ForbiddenAnswer, ConflictAnswer)
)]
async fn create_theme(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiJson(body): ApiJson<NewThemeBody>,
) -> Result<Response, ApiError> {
    let theme = auth
        .api()?
        .create_theme(
            info,
            CreateTheme {
                session_token,
                name: body.name,
                kind: body.kind,
                extends: body.extends,
            },
        )
        .await?;
    answer_authorized(&theme.map(|view| ThemeBody::from(&view)))
}

/// One own theme with its values and the pairs that are hard to read.
#[utoipa::path(
    get,
    path = "/api/v1/themes/{theme}",
    operation_id = "getTheme",
    tag = "themes",
    params(("theme" = String, Path, description = "The ID of the theme.")),
    responses((status = 200, description = "The theme.", body = ThemeDetailBody), NotSignedInAnswer, NotFoundAnswer)
)]
async fn get_theme(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(theme): ApiPath<String>,
) -> Result<Response, ApiError> {
    let theme = theme_named(&auth, &info, &session_token, &theme).await?;
    let detail = auth
        .api()?
        .theme(
            info,
            ThemeQuery {
                session_token,
                theme,
            },
        )
        .await?;
    answer_authorized(&detail.map(|detail| ThemeDetailBody::from(&detail)))
}

/// Saves the name and the values of an own theme, under the edit lock the person holds. A theme
/// for everyone with a pair that is hard to read is refused (`theme.hard-to-read`, naming the
/// pairs); a personal one is saved, and the answer names them.
#[utoipa::path(
    put,
    path = "/api/v1/themes/{theme}",
    operation_id = "saveTheme",
    tag = "themes",
    params(("theme" = String, Path, description = "The ID of the theme.")),
    request_body = SaveThemeBody,
    responses((status = 200, description = "The theme as it is now.", body = ThemeDetailBody), InvalidAnswer, NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer, ConflictAnswer)
)]
async fn save_theme(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(theme): ApiPath<String>,
    ApiJson(body): ApiJson<SaveThemeBody>,
) -> Result<Response, ApiError> {
    let theme = theme_named(&auth, &info, &session_token, &theme).await?;
    let detail = auth
        .api()?
        .save_theme(
            info,
            SaveTheme {
                session_token,
                theme,
                name: body.name,
                values: body.values,
                version: body.version,
                lock_token: body_id::<LockToken>(&body.lock_token)?,
            },
        )
        .await?;
    answer_authorized(&detail.map(|detail| ThemeDetailBody::from(&detail)))
}

/// Takes an own theme away. A choice of it counts as the built-in theme it extended.
#[utoipa::path(
    delete,
    path = "/api/v1/themes/{theme}",
    operation_id = "deleteTheme",
    tag = "themes",
    params(("theme" = String, Path, description = "The ID of the theme."), DeleteThemeParams),
    responses((status = 204, description = "The theme is gone."), NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer, ConflictAnswer)
)]
async fn delete_theme(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(theme): ApiPath<String>,
    ApiQuery(params): ApiQuery<DeleteThemeParams>,
) -> Result<Response, ApiError> {
    let theme = theme_named(&auth, &info, &session_token, &theme).await?;
    auth.api()?
        .delete_theme(
            info,
            DeleteTheme {
                session_token,
                theme,
                version: params.version,
            },
        )
        .await?;
    Ok(done())
}

/// Keeps an own theme the person may use as a new one (*Save a copy*), saved at once.
#[utoipa::path(
    post,
    path = "/api/v1/themes/{theme}/copy",
    operation_id = "copyTheme",
    tag = "themes",
    params(("theme" = String, Path, description = "The ID of the theme that is copied.")),
    request_body = CopyThemeBody,
    responses((status = 200, description = "The copy.", body = ThemeDetailBody), InvalidAnswer, NotSignedInAnswer, ForbiddenAnswer, NotFoundAnswer, ConflictAnswer)
)]
async fn copy_theme(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(theme): ApiPath<String>,
    ApiJson(body): ApiJson<CopyThemeBody>,
) -> Result<Response, ApiError> {
    let theme = theme_named(&auth, &info, &session_token, &theme).await?;
    let detail = auth
        .api()?
        .copy_theme(
            info,
            CopyTheme {
                session_token,
                theme,
                name: body.name,
                kind: body.kind,
            },
        )
        .await?;
    answer_authorized(&detail.map(|detail| ThemeDetailBody::from(&detail)))
}

/// An own theme as a file, in the shape an upload takes.
#[utoipa::path(
    get,
    path = "/api/v1/themes/{theme}/file",
    operation_id = "downloadTheme",
    tag = "themes",
    params(("theme" = String, Path, description = "The ID of the theme.")),
    responses((status = 200, description = "The theme as a file.", body = ThemeFileBody), NotSignedInAnswer, NotFoundAnswer)
)]
async fn download_theme(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(theme): ApiPath<String>,
) -> Result<Response, ApiError> {
    let theme = theme_named(&auth, &info, &session_token, &theme).await?;
    let file: bb_policy::Authorized<ThemeExport> = auth
        .api()?
        .download_theme(
            info,
            ThemeQuery {
                session_token,
                theme,
            },
        )
        .await?;
    answer_authorized(&file)
}

/// Makes an own theme from a file in the shape of a download (at most 64 KiB), checked like a
/// save and saved at once: a part or a value that is not allowed is refused with its name.
#[utoipa::path(
    post,
    path = "/api/v1/themes/upload",
    operation_id = "uploadTheme",
    tag = "themes",
    params(UploadThemeParams),
    request_body = ThemeFileBody,
    responses((status = 200, description = "The new theme.", body = ThemeDetailBody), InvalidAnswer, NotSignedInAnswer, ForbiddenAnswer, ConflictAnswer, (status = 413, description = "The file is bigger than 64 KiB."))
)]
async fn upload_theme(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiQuery(params): ApiQuery<UploadThemeParams>,
    file: Bytes,
) -> Result<Response, ApiError> {
    let detail = auth
        .api()?
        .upload_theme(
            info,
            UploadTheme {
                session_token,
                kind: params.kind,
                file: file.to_vec(),
            },
        )
        .await?;
    answer_authorized(&detail.map(|detail| ThemeDetailBody::from(&detail)))
}

/// The routes of own themes below `/api/v1`.
pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(list_themes, create_theme))
        .routes(routes!(get_theme, save_theme, delete_theme))
        .routes(routes!(copy_theme))
        .routes(routes!(download_theme))
        .routes(routes!(upload_theme))
        .merge(sheet_routes())
}
