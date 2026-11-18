//! *About* below `/api/v1/about` (COMP-01, COMP-02, ADR-0047 decision 3): the version, the license,
//! the parts list by name, version and license, and the three files of the build: the parts list
//! in CycloneDX and in SPDX, and the third-party notices.
//!
//! The files are the ones `cargo xtask build` writes into `about-files/` before it compiles the
//! program, taken in by `rust-embed` like the browser app. A build made another way (a plain
//! `cargo build`) has none, and the answer says so by listing no parts and no files. Signed-in
//! people may read it: the version is not shown to strangers.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse as _, Response};
use bb_app::auth::AuthenticateQuery;
use rust_embed::RustEmbed;
use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::auth::{AuthHandle, Info, NotFoundAnswer, NotSignedInAnswer, Session, answer};
use super::files::{FileBytes, download_headers};
use crate::errors::{ApiError, ApiPath};
use bb_core::error_code::ErrorCode;

/// What `cargo xtask build` writes before the program is compiled.
#[derive(RustEmbed)]
#[folder = "about-files"]
#[allow_missing = true]
struct BuildFiles;

/// The kinds of file of the build, as the address names them, with the end of the file's name.
#[derive(Clone, Copy)]
enum Kind {
    CycloneDx,
    Spdx,
    Notices,
}

impl Kind {
    const ALL: [Self; 3] = [Self::CycloneDx, Self::Spdx, Self::Notices];

    /// The word in the address.
    const fn word(self) -> &'static str {
        match self {
            Self::CycloneDx => "cyclonedx",
            Self::Spdx => "spdx",
            Self::Notices => "notices",
        }
    }

    const fn name_ends_with(self) -> &'static str {
        match self {
            Self::CycloneDx => ".cdx.json",
            Self::Spdx => ".spdx.json",
            Self::Notices => "-notices.txt",
        }
    }

    const fn media_type(self) -> &'static str {
        match self {
            Self::CycloneDx | Self::Spdx => "application/json",
            Self::Notices => "text/plain; charset=utf-8",
        }
    }

    fn named(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.word() == word)
    }

    /// The embedded file of this kind: its name and bytes.
    fn file(self) -> Option<(String, Vec<u8>)> {
        let name = BuildFiles::iter().find(|name| name.ends_with(self.name_ends_with()))?;
        let bytes = BuildFiles::get(&name)?.data.into_owned();
        Some((name.into_owned(), bytes))
    }
}

/// One part of the program.
#[derive(Serialize, ToSchema)]
pub(crate) struct PartBody {
    /// The name of the crate or package, or the path of the file.
    name: String,
    /// The version; empty for a file.
    version: String,
    /// The license, as an SPDX identifier or expression; empty when none is declared.
    license: String,
}

/// A file of the build that can be downloaded.
#[derive(Serialize, ToSchema)]
pub(crate) struct AboutFileBody {
    /// Which file: `cyclonedx`, `spdx` or `notices`.
    kind: String,
    /// The name the file is saved under.
    name: String,
    /// The address to download it from.
    address: String,
}

/// What this program is and what it is made of.
#[derive(Serialize, ToSchema)]
pub(crate) struct AboutBody {
    /// The version of the program.
    version: String,
    /// The license of the program, as an SPDX identifier.
    license: String,
    /// The parts the program is made of, in the order of the parts list. Empty when this build
    /// carries no parts list.
    parts: Vec<PartBody>,
    /// The files of the build: the parts list in two formats and the third-party notices.
    files: Vec<AboutFileBody>,
}

/// The license of a CycloneDX component, as text.
fn license_of(component: &Value) -> String {
    let first = component.pointer("/licenses/0");
    first
        .and_then(|choice| {
            choice
                .pointer("/license/id")
                .or_else(|| choice.pointer("/license/name"))
                .or_else(|| choice.get("expression"))
        })
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// The parts named in the text of a CycloneDX file; none when it cannot be read.
fn parts_of(cyclonedx: &[u8]) -> Vec<PartBody> {
    let Ok(document) = serde_json::from_slice::<Value>(cyclonedx) else {
        return Vec::new();
    };
    let text = |component: &Value, key: &str| {
        component
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    document
        .get("components")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|component| PartBody {
            name: text(component, "name"),
            version: text(component, "version"),
            license: license_of(component),
        })
        .collect()
}

fn about() -> AboutBody {
    let parts = Kind::CycloneDx
        .file()
        .map(|(_, bytes)| parts_of(&bytes))
        .unwrap_or_default();
    let files = Kind::ALL
        .into_iter()
        .filter_map(|kind| {
            kind.file().map(|(name, _)| AboutFileBody {
                kind: kind.word().to_owned(),
                name,
                address: format!("/api/v1/about/files/{}", kind.word()),
            })
        })
        .collect();
    AboutBody {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        license: env!("CARGO_PKG_LICENSE").to_owned(),
        parts,
        files,
    }
}

/// The version, the license and the parts of the program, and where to download its parts list
/// and third-party notices. Needs a signed-in person, nothing more.
#[utoipa::path(
    get,
    path = "/api/v1/about",
    operation_id = "getAbout",
    tag = "about",
    responses((status = 200, description = "What the program is made of.", body = AboutBody), NotSignedInAnswer)
)]
async fn get_about(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
) -> Result<Response, ApiError> {
    auth.api()?
        .authenticate(info, AuthenticateQuery { session_token })
        .await?;
    Ok(answer(about()))
}

/// One file of the build: the parts list as CycloneDX (`cyclonedx`) or SPDX (`spdx`) JSON, or the
/// third-party notices (`notices`). Needs a signed-in person, nothing more.
#[utoipa::path(
    get,
    path = "/api/v1/about/files/{kind}",
    operation_id = "downloadAboutFile",
    tag = "about",
    params(("kind" = String, Path, description = "Which file: `cyclonedx`, `spdx` or `notices`.")),
    responses(
        (status = 200, description = "The file.", content_type = "application/octet-stream", body = FileBytes),
        NotSignedInAnswer,
        NotFoundAnswer
    )
)]
async fn download_about_file(
    State(auth): State<AuthHandle>,
    Info(info): Info,
    Session(session_token): Session,
    ApiPath(kind): ApiPath<String>,
) -> Result<Response, ApiError> {
    auth.api()?
        .authenticate(info, AuthenticateQuery { session_token })
        .await?;
    let (kind, (name, bytes)) = Kind::named(&kind)
        .and_then(|kind| kind.file().map(|file| (kind, file)))
        .ok_or_else(|| ApiError::new(ErrorCode::NotFound))?;
    let headers = download_headers(&name, kind.media_type(), bytes.len() as u64);
    Ok((StatusCode::OK, headers, bytes).into_response())
}

pub(crate) fn routes<S>() -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AuthHandle: axum::extract::FromRef<S>,
{
    OpenApiRouter::new()
        .routes(routes!(get_about))
        .routes(routes!(download_about_file))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_parts_are_read_from_a_cyclonedx_file() {
        let text = br#"{"components":[
            {"name":"serde","version":"1.0.2","licenses":[{"expression":"MIT OR Apache-2.0"}]},
            {"name":"react","version":"19","licenses":[{"license":{"id":"MIT"}}]},
            {"name":"odd","version":"1"}]}"#;
        let parts = parts_of(text);
        let seen: Vec<_> = parts
            .iter()
            .map(|part| (part.name.as_str(), part.license.as_str()))
            .collect();
        assert_eq!(
            seen,
            [
                ("serde", "MIT OR Apache-2.0"),
                ("react", "MIT"),
                ("odd", "")
            ]
        );
        assert!(parts_of(b"not json").is_empty());
    }

    #[test]
    fn the_answer_names_this_version_and_license() {
        let body = about();
        assert_eq!(body.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(body.license, "AGPL-3.0-or-later");
    }

    #[test]
    fn the_words_of_the_addresses_find_their_kinds() {
        assert!(Kind::named("spdx").is_some());
        assert!(Kind::named("../secret").is_none());
    }
}
