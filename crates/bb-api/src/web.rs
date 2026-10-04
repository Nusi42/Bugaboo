//! The browser app: where its files come from and how they are looked up.

use std::path::{Path, PathBuf};

use rust_embed::RustEmbed;

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
    let full = folder.join(path);
    // Directories (and anything else that is not a plain file) are never served.
    if !tokio::fs::metadata(&full).await.ok()?.is_file() {
        return None;
    }
    tokio::fs::read(full).await.ok()
}

/// Device names Windows treats as devices in any folder, with or without an extension.
const WINDOWS_RESERVED: [&str; 4] = ["CON", "PRN", "AUX", "NUL"];

fn is_windows_reserved(part: &str) -> bool {
    let stem = part.split('.').next().unwrap_or_default().trim_end();
    let stem = stem.to_ascii_uppercase();
    let numbered = |prefix: &str| {
        stem.strip_prefix(prefix)
            .is_some_and(|n| matches!(n.as_bytes(), [b'1'..=b'9']))
    };
    WINDOWS_RESERVED.contains(&stem.as_str()) || numbered("COM") || numbered("LPT")
}

/// Windows lets `INDEX~1.HTM` stand for `index.html` inside a folder (an "8.3 short name"). Such
/// a name never belongs to the app, so it is refused (M0-F08): a stem of at most eight
/// characters that ends in `~` and digits.
fn is_short_name(part: &str) -> bool {
    let stem = part.rsplit_once('.').map_or(part, |(stem, _)| stem);
    let digits = stem.len() - stem.trim_end_matches(|c: char| c.is_ascii_digit()).len();
    digits > 0 && stem.len() <= 8 && stem[..stem.len() - digits].ends_with('~')
}

/// A path is safe if every part is a plain name: no `..`, no separators or stream markers,
/// no names Windows treats specially. Applies on every platform so behavior is the same.
fn is_safe(path: &str) -> bool {
    !path.is_empty()
        && path.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && !part.contains(['\\', ':', '\0'])
                && !part.ends_with(['.', ' '])
                && !is_windows_reserved(part)
                && !is_short_name(part)
        })
}

/// Turns `%XX` escapes (exactly two hex digits) into the bytes they stand for.
/// `None` if an escape is malformed or the result is not UTF-8.
pub(crate) fn decode_path(path: &str) -> Option<String> {
    let bytes = path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        if byte == b'%' {
            let high = hex_digit(*bytes.get(index + 1)?)?;
            let low = hex_digit(*bytes.get(index + 2)?)?;
            decoded.push(high * 16 + low);
            index += 3;
        } else {
            decoded.push(byte);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

/// `u8::from_str_radix` would also accept a sign (`%+1`), so the digits are read by hand.
fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn content_type(path: &str) -> String {
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    if mime.type_() == mime_guess::mime::TEXT {
        format!("{mime}; charset=utf-8")
    } else {
        mime.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sec_11_safe_paths_accept_plain_names_only() {
        for good in [
            "index.html",
            "assets/app-1a2b.js",
            "my file.txt",
            "console.log",
            "COM10",
            "app~name.js",
            "a~1b.txt",
            "waytoolong~1.txt",
        ] {
            assert!(is_safe(good), "{good}");
        }
        for bad in [
            "",
            "..",
            "a/../b",
            "./a",
            "a//b",
            "a\\b",
            "c:x",
            "a.",
            "a ",
            "x\0y",
            "CON",
            "con",
            "Aux.txt",
            "nul.tar.gz",
            "PRN",
            "com1",
            "COM9.js",
            "lpt1",
            "LPT9.x",
            "dir/NUL",
            "con .txt",
            "INDEX~1.HTM",
            "index~1.html",
            "assets/APP-1A~2.JS",
            "PROGRA~1",
        ] {
            assert!(!is_safe(bad), "{bad}");
        }
    }

    #[test]
    fn sec_11_percent_escapes_are_decoded_before_the_safety_check() {
        assert_eq!(decode_path("my%20file.txt").as_deref(), Some("my file.txt"));
        assert_eq!(decode_path("%2e%2e/x").as_deref(), Some("../x"));
        assert_eq!(decode_path("..%5Cx.js").as_deref(), Some("..\\x.js"));
        assert_eq!(decode_path("%C3%A4").as_deref(), Some("\u{e4}"));
        assert_eq!(decode_path("%"), None);
        assert_eq!(decode_path("%zz"), None);
        assert_eq!(decode_path("%FF"), None);
        // A sign is not a hex digit (M0-F08).
        assert_eq!(decode_path("%+1"), None);
        assert_eq!(decode_path("%-1"), None);
        assert_eq!(decode_path("a%2"), None);
        assert_eq!(decode_path("%4A").as_deref(), Some("J"));
    }
}
