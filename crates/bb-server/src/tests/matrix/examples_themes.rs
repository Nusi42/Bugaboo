//! The example requests of the routes of own themes and their style sheets (THM-04).
//!
//! The routes that read or change a theme work on a theme for everyone that the administrator of
//! the matrix saved: every signed-in person may use it, and changing it needs *Change organization
//! settings*. A save carries a token of a lock nobody holds: an actor who is let in is told that
//! the lock is lost, which is not a refusal of the permission. The routes that make a theme make a
//! personal one, which every signed-in person may make.

use bb_app::themes::{ThemesQuery, UploadTheme};
use bb_core::theme::ThemeKind;
use bb_testing::Failure;
use serde_json::json;

use super::examples::Request;
use super::routes::Fixture;
use crate::sign_in_tests::rig::Rig;

/// A token that holds no lock.
const NOBODYS_LOCK: &str = "5b0f6a3e-2d1c-4e8a-9f77-0a1b2c3d4e5f";

/// A saved theme for everyone, made by the administrator of the matrix when there is none: its ID
/// and its version.
async fn a_theme(fixture: &Fixture<'_>) -> Result<(String, i64), Failure> {
    let api = fixture.rig().api();
    let session = fixture.admin_session().to_owned();
    let listed = api
        .themes(
            Rig::info(),
            ThemesQuery {
                session_token: session.clone(),
            },
        )
        .await
        .map_err(|error| error.to_string())?
        .into_value();
    if let Some(found) = listed
        .themes
        .iter()
        .find(|view| view.theme.kind == ThemeKind::ForEveryone)
    {
        return Ok((found.theme.id.to_string(), found.theme.version));
    }
    let file = json!({"name": "Company look", "extends": "dusk", "values": {}});
    let saved = api
        .upload_theme(
            Rig::info(),
            UploadTheme {
                session_token: session,
                kind: ThemeKind::ForEveryone,
                file: file.to_string().into_bytes(),
            },
        )
        .await
        .map_err(|error| error.to_string())?
        .into_value();
    Ok((saved.view.theme.id.to_string(), saved.view.theme.version))
}

pub(super) async fn theme_request(
    fixture: &Fixture<'_>,
    operation: &str,
) -> Result<Option<Request>, Failure> {
    let personal = json!({"name": "Mine", "kind": "personal"});
    Ok(Some(match operation {
        "listThemes" => ("GET", "/api/v1/themes".into(), None),
        "createTheme" => (
            "POST",
            "/api/v1/themes".into(),
            Some(json!({"name": "Mine", "kind": "personal", "extends": "dusk"})),
        ),
        "uploadTheme" => (
            "POST",
            "/api/v1/themes/upload?kind=personal".into(),
            Some(json!({"name": "Mine", "extends": "dusk", "values": {}})),
        ),
        "getTheme" | "downloadTheme" | "getThemeSheet" | "getThemeDraftSheet" | "copyTheme" => {
            let (theme, version) = a_theme(fixture).await?;
            match operation {
                "getTheme" => ("GET", format!("/api/v1/themes/{theme}"), None),
                "downloadTheme" => ("GET", format!("/api/v1/themes/{theme}/file"), None),
                "getThemeSheet" => ("GET", format!("/themes/{theme}.css?v={version}"), None),
                "getThemeDraftSheet" => ("GET", format!("/themes/{theme}/draft.css"), None),
                _ => (
                    "POST",
                    format!("/api/v1/themes/{theme}/copy"),
                    Some(personal),
                ),
            }
        }
        "saveTheme" => {
            let (theme, version) = a_theme(fixture).await?;
            (
                "PUT",
                format!("/api/v1/themes/{theme}"),
                Some(json!({
                    "version": version,
                    "lock_token": NOBODYS_LOCK,
                    "name": "Company look",
                    "values": {},
                })),
            )
        }
        "deleteTheme" => {
            let (theme, version) = a_theme(fixture).await?;
            (
                "DELETE",
                format!("/api/v1/themes/{theme}?version={version}"),
                None,
            )
        }
        _ => return Ok(None),
    }))
}
