//! The browser app: where its files come from and how they are looked up.

use std::path::{Path, PathBuf};

use rust_embed::RustEmbed;

/// What to tell the person when the browser app has not been built (`cargo xtask build`).
pub const WEB_APP_MISSING: &str =
    "The browser app is not built yet. Run `cargo xtask build`, or pass --web <folder>.";

/// The file every page address falls back to (single-page app).
pub(crate) const INDEX: &str = "index.html";

/// Release builds hold `web/app/dist` inside the binary. Debug builds read the same folder from
/// disk, so `cargo build` works before the web build exists (`allow_missing`).
#[derive(RustEmbed)]
#[folder = "../../web/app/dist"]
#[allow_missing = true]
struct Embedded;

/// Where the files of the browser app come from.
#[derive(Clone, Debug)]
pub enum WebSource {
    /// The build output compiled into the binary (read from disk in debug builds).
    Embedded,
    /// A folder on disk (`bugaboo serve --web <folder>`).
    Folder(PathBuf),
}

/// A file of the browser app.
pub(crate) struct Asset {
    pub(crate) bytes: Vec<u8>,
    pub(crate) content_type: String,
}

impl WebSource {
    /// Whether the app can be served at all (its start page exists).
    pub async fn has_index(&self) -> bool {
        self.load(INDEX).await.is_some()
    }

    /// Looks up a file by its path inside the app, such as `assets/app-1a2b.js`.
    /// Paths that could leave the app folder are never found.
    pub(crate) async fn load(&self, path: &str) -> Option<Asset> {
        if !is_safe(path) {
            return None;
        }
        let bytes = match self {
            Self::Embedded => Embedded::get(path)?.data.into_owned(),
            Self::Folder(folder) => read_file(folder, path).await?,
        };
        Some(Asset {
            bytes,
            content_type: content_type(path),
        })
    }
}

async fn read_file(folder: &Path, path: &str) -> Option<Vec<u8>> {
    tokio::fs::read(folder.join(path)).await.ok()
}

fn is_safe(path: &str) -> bool {
    !path.is_empty()
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != ".." && !part.contains(['\\', ':', '\0']))
}

fn content_type(path: &str) -> String {
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    if mime.type_() == mime_guess::mime::TEXT {
        format!("{mime}; charset=utf-8")
    } else {
        mime.to_string()
    }
}
