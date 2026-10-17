//! Reading the files the checks look at: which files a check covers, how they are read, and the
//! rules every reader shares (line endings, whole words). Every check and generator asks here.
//!
//! The files a check covers are written once, as the `files` globs of its entry in the check
//! catalogue (`tools/checks/catalog.toml`); the code never lists them again (coding-standards §1).

use std::fs;
use std::path::Path;

use anyhow::Context as _;

use crate::catalog::Entry;
use crate::repo::Repo;

/// One source file: its repository-relative path and its text, with `\n` line endings.
pub(crate) type Source = (String, String);

/// Check examples live here; the normal runs of the checks must not see them.
pub(crate) const FIXTURES_FOLDER: &str = "tools/checks/fixtures";

/// The lock file of npm: written by npm, never read or formatted by us.
const LOCK_FILE: &str = "package-lock.json";

/// Gives text `\n` line endings, wherever it was read from.
pub(crate) fn normalize(text: &str) -> String {
    text.replace("\r\n", "\n")
}

/// The text in the bytes of a file, with `\n` line endings, or `None` for a file that is no text
/// (it holds NUL bytes that are no UTF-16). UTF-16 with or without a byte order mark (what
/// PowerShell writes with `>`) is decoded, a UTF-8 mark is dropped, and bytes that are no UTF-8
/// are read as Windows-1252 (what German Excel writes, and Latin-1 for the most part), so `Müller`
/// is found in them too. Every check that looks into a file of unknown kind reads it here.
pub(crate) fn decode_text(bytes: &[u8]) -> Option<String> {
    let utf16 = |rest: &[u8], unit: fn([u8; 2]) -> u16| {
        let units: Vec<u16> = rest
            .chunks_exact(2)
            .map(|pair| unit([pair[0], pair[1]]))
            .collect();
        String::from_utf16_lossy(&units)
    };
    let text = match bytes {
        [0xFF, 0xFE, rest @ ..] => utf16(rest, u16::from_le_bytes),
        [0xFE, 0xFF, rest @ ..] => utf16(rest, u16::from_be_bytes),
        _ if is_utf16_without_mark(bytes, true) => utf16(bytes, u16::from_le_bytes),
        _ if is_utf16_without_mark(bytes, false) => utf16(bytes, u16::from_be_bytes),
        _ if bytes.contains(&0) => return None,
        [0xEF, 0xBB, 0xBF, rest @ ..] => String::from_utf8_lossy(rest).into_owned(),
        _ => std::str::from_utf8(bytes).map_or_else(
            |_| bytes.iter().map(|b| windows_1252(*b)).collect(),
            str::to_owned,
        ),
    };
    Some(normalize(&text))
}

/// The characters of Windows-1252 for the bytes 0x80 to 0x9F; the five bytes it leaves open stay
/// the control characters of Latin-1.
const WINDOWS_1252_HIGH: [char; 32] = [
    '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8D}', 'Ž', '\u{8F}',
    '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9D}', 'ž', 'Ÿ',
];

fn windows_1252(byte: u8) -> char {
    match byte {
        0x80..=0x9F => WINDOWS_1252_HIGH[usize::from(byte - 0x80)],
        _ => char::from(byte),
    }
}

/// Whether the bytes look like UTF-16 text without a mark: letters of one half of every pair are
/// NUL (the second byte of each pair for little endian), and the other half never is.
fn is_utf16_without_mark(bytes: &[u8], little_endian: bool) -> bool {
    let pairs = bytes.len() / 2;
    let (zeros, others) = bytes
        .chunks_exact(2)
        .map(|pair| {
            if little_endian {
                (pair[1], pair[0])
            } else {
                (pair[0], pair[1])
            }
        })
        .fold((0_usize, 0_usize), |(zeros, others), (high, low)| {
            (
                zeros + usize::from(high == 0),
                others + usize::from(low == 0),
            )
        });
    pairs >= 2 && zeros * 3 >= pairs && others == 0
}

/// The shortest run of characters that [`printable_runs`] reports.
const MIN_RUN: usize = 4;

/// The runs of printable ASCII in the bytes of a file that is no text, one per line, and the runs
/// of UTF-16 letters (an ASCII letter and a NUL byte): what the tool `strings` finds.
pub(crate) fn printable_runs(bytes: &[u8]) -> String {
    let printable = |byte: u8| (0x20..=0x7E).contains(&byte) || byte == b'\t';
    let mut runs: Vec<String> = Vec::new();
    for (start, step) in [(0, 1), (0, 2), (1, 2)] {
        let mut run = String::new();
        for at in (start..bytes.len()).step_by(step) {
            let wide_ok = step == 1 || bytes.get(at + 1) == Some(&0);
            if printable(bytes[at]) && wide_ok {
                run.push(char::from(bytes[at]));
            } else {
                if run.len() >= MIN_RUN {
                    runs.push(std::mem::take(&mut run));
                }
                run.clear();
            }
        }
        if run.len() >= MIN_RUN {
            runs.push(run);
        }
    }
    runs.join("\n")
}

/// Reads a text file (path relative to the repository root) with `\n` line endings.
pub(crate) fn read_text(repo: &Repo, relative: &str) -> anyhow::Result<String> {
    let path = repo.path(relative);
    let text =
        fs::read_to_string(&path).with_context(|| format!("could not read {}", path.display()))?;
    Ok(normalize(&text))
}

/// The kinds of code the checks read.
#[derive(Clone, Copy)]
pub(crate) enum Kind {
    /// The Rust files of the product crates, except integration tests and their examples.
    Rust,
    /// The TypeScript files of the browser app and its packages.
    Web,
    /// Like [`Kind::Web`], and the style sheets too.
    WebAndStyles,
    /// The HTML pages of the documents and the mockups.
    DocPages,
}

impl Kind {
    fn folder(self) -> &'static str {
        match self {
            Self::Rust => "crates/",
            Self::Web | Self::WebAndStyles => "web/",
            Self::DocPages => "docs/",
        }
    }

    fn endings(self) -> &'static [&'static str] {
        match self {
            Self::Rust => &["rs"],
            Self::Web => &["ts", "tsx"],
            Self::WebAndStyles => &["ts", "tsx", "css", "scss", "sass", "less"],
            Self::DocPages => &["html"],
        }
    }

    /// The folders whose files are never product sources.
    fn skipped(self) -> &'static [&'static str] {
        match self {
            Self::Rust => &["tests"],
            Self::Web | Self::WebAndStyles => &["node_modules", "dist", "fixtures"],
            Self::DocPages => &[],
        }
    }

    /// The files that are never product sources: the tests of a module, in a file of their own next
    /// to it (`router.rs` and `router/tests.rs`).
    fn skipped_files(self) -> &'static [&'static str] {
        match self {
            Self::Rust => &["tests.rs"],
            Self::Web | Self::WebAndStyles | Self::DocPages => &[],
        }
    }

    fn wants(self, path: &str) -> bool {
        path.starts_with(self.folder())
            && Path::new(path)
                .extension()
                .is_some_and(|ending| self.endings().iter().any(|wanted| ending == *wanted))
            && !path.split('/').any(|part| self.skipped().contains(&part))
            && !path
                .rsplit('/')
                .next()
                .is_some_and(|file| self.skipped_files().contains(&file))
    }
}

/// The files of the repository that the entry covers (all of them without an entry).
fn covered(repo: &Repo, entry: Option<&Entry>) -> anyhow::Result<Vec<String>> {
    let files = repo.all_files()?;
    match entry {
        Some(entry) => entry.matching(&files),
        None => Ok(files),
    }
}

/// The code of `kind` that the entry covers, read with `\n` line endings. Without an entry, all
/// of that kind (for the generators, which are not a check).
pub(crate) fn read_sources(
    repo: &Repo,
    entry: Option<&Entry>,
    kind: Kind,
) -> anyhow::Result<Vec<Source>> {
    let mut sources = Vec::new();
    for file in covered(repo, entry)? {
        if kind.wants(&file) {
            let text = read_text(repo, &file)?;
            sources.push((file, text));
        }
    }
    Ok(sources)
}

/// The files for a tool to run on: the ones the entry covers among `changed` (the files being
/// edited or committed), or among all files of the repository when `changed` is `None`. Fixtures
/// of the checks and the npm lock file are never among them.
pub(crate) fn selected(
    repo: &Repo,
    entry: &Entry,
    changed: Option<&[String]>,
) -> anyhow::Result<Vec<String>> {
    let candidates = match changed {
        Some(changed) => changed.to_vec(),
        None => repo.all_files()?,
    };
    let mut files = entry.matching(&candidates)?;
    files.retain(|file| {
        !file.starts_with(FIXTURES_FOLDER)
            && !file.ends_with(LOCK_FILE)
            && repo.path(file).is_file()
    });
    files.sort();
    files.dedup();
    Ok(files)
}

/// Whether `c` can be part of a name in code.
pub(crate) fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The byte positions where `word` stands in `text` as a whole word, not as part of a longer name.
pub(crate) fn word_positions<'a>(text: &'a str, word: &'a str) -> impl Iterator<Item = usize> + 'a {
    text.match_indices(word).filter_map(move |(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + word.len()..].chars().next();
        (!before.is_some_and(is_word_char) && !after.is_some_and(is_word_char)).then_some(at)
    })
}

/// Whether `word` is in `text` as a whole word.
pub(crate) fn contains_word(text: &str, word: &str) -> bool {
    word_positions(text, word).next().is_some()
}

/// The byte positions where `word` is named in code: as a whole word, not as a method of a value
/// (after a dot) and not as the definition of a function (after `fn`).
pub(crate) fn name_uses<'a>(code: &'a str, word: &'a str) -> impl Iterator<Item = usize> + 'a {
    word_positions(code, word).filter(move |at| {
        let before = &code[..*at];
        !before.ends_with('.') && !before.trim_end().ends_with("fn")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_word_is_whole_only_between_non_name_characters() {
        assert!(contains_word("let x = Role::new();", "Role"));
        assert!(contains_word("Role", "Role"));
        assert!(!contains_word("StartingRole", "Role"));
        assert!(!contains_word("Role_id", "Role"));
        assert!(!contains_word("Roles", "Role"));
        assert!(contains_word("Roles and Role", "Role"));
    }

    #[test]
    fn a_name_is_used_but_neither_a_method_nor_a_definition() {
        let uses = |code: &str| name_uses(code, "answer").count();
        assert_eq!(uses("answer(x)"), 1);
        assert_eq!(uses("self.answer(x)"), 0);
        assert_eq!(uses("fn answer(x)"), 0);
        assert_eq!(uses("fn  answer(x)"), 0);
        assert_eq!(uses("answer_all(x)"), 0);
        assert_eq!(uses("a.b(); answer(x)"), 1);
    }

    fn utf16(text: &str, mark: &[u8], unit: fn(u16) -> [u8; 2]) -> Vec<u8> {
        mark.iter()
            .copied()
            .chain(text.encode_utf16().flat_map(unit))
            .collect()
    }

    #[test]
    fn text_is_decoded_whatever_way_it_was_written() {
        let marked = utf16("a\r\nb", &[0xFF, 0xFE], u16::to_le_bytes);
        assert_eq!(decode_text(&marked).as_deref(), Some("a\nb"));
        let big = utf16("ab", &[0xFE, 0xFF], u16::to_be_bytes);
        assert_eq!(decode_text(&big).as_deref(), Some("ab"));
        // UTF-16 without a mark, little and big endian.
        let bare = utf16("hello world", &[], u16::to_le_bytes);
        assert_eq!(decode_text(&bare).as_deref(), Some("hello world"));
        let bare_big = utf16("hello world", &[], u16::to_be_bytes);
        assert_eq!(decode_text(&bare_big).as_deref(), Some("hello world"));
        assert_eq!(decode_text(b"\xEF\xBB\xBFab").as_deref(), Some("ab"));
        // Windows-1252 (German Excel): the umlaut and the euro sign come out right.
        assert_eq!(
            decode_text(b"M\xFCller \x80").as_deref(),
            Some("M\u{FC}ller \u{20AC}")
        );
        assert_eq!(decode_text(b"PNG\0data"), None);
    }

    #[test]
    fn the_printable_runs_of_a_file_that_is_no_text_are_found() {
        let mut bytes = b"\x89PNG\0\x01ab\x02Hello there\0\x03".to_vec();
        bytes.extend(utf16("wide text", &[], u16::to_le_bytes));
        bytes.extend(b"\0\x01");
        let runs = printable_runs(&bytes);
        assert!(runs.contains("Hello there"), "{runs}");
        assert!(runs.contains("wide text"), "{runs}");
        // Runs shorter than four characters are noise.
        assert!(!runs.lines().any(|run| run == "ab"), "{runs}");
    }

    #[test]
    fn line_endings_become_new_lines() {
        assert_eq!(normalize("a\r\nb\r\n"), "a\nb\n");
        assert_eq!(normalize("a\nb\n"), "a\nb\n");
    }

    #[test]
    fn a_kind_wants_its_files_in_its_folder_and_not_the_tests() {
        assert!(Kind::Rust.wants("crates/bb-core/src/lib.rs"));
        assert!(!Kind::Rust.wants("crates/bb-core/tests/x.rs"));
        // The tests of a module in a file of their own are not product code either.
        assert!(!Kind::Rust.wants("crates/bb-api/src/router/tests.rs"));
        assert!(Kind::Rust.wants("crates/bb-api/src/router.rs"));
        assert!(!Kind::Rust.wants("web/app/src/x.ts"));
        assert!(Kind::Web.wants("web/app/src/x.tsx"));
        assert!(!Kind::Web.wants("web/app/src/x.css"));
        assert!(Kind::WebAndStyles.wants("web/app/src/x.css"));
        assert!(!Kind::Web.wants("web/node_modules/p/x.ts"));
    }
}
