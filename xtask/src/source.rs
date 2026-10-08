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
}

impl Kind {
    fn folder(self) -> &'static str {
        match self {
            Self::Rust => "crates/",
            Self::Web | Self::WebAndStyles => "web/",
        }
    }

    fn endings(self) -> &'static [&'static str] {
        match self {
            Self::Rust => &["rs"],
            Self::Web => &["ts", "tsx"],
            Self::WebAndStyles => &["ts", "tsx", "css"],
        }
    }

    /// The folders whose files are never product sources.
    fn skipped(self) -> &'static [&'static str] {
        match self {
            Self::Rust => &["tests"],
            Self::Web | Self::WebAndStyles => &["node_modules", "dist", "fixtures"],
        }
    }

    fn wants(self, path: &str) -> bool {
        path.starts_with(self.folder())
            && Path::new(path)
                .extension()
                .is_some_and(|ending| self.endings().iter().any(|wanted| ending == *wanted))
            && !path.split('/').any(|part| self.skipped().contains(&part))
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

    #[test]
    fn line_endings_become_new_lines() {
        assert_eq!(normalize("a\r\nb\r\n"), "a\nb\n");
        assert_eq!(normalize("a\nb\n"), "a\nb\n");
    }

    #[test]
    fn a_kind_wants_its_files_in_its_folder_and_not_the_tests() {
        assert!(Kind::Rust.wants("crates/bb-core/src/lib.rs"));
        assert!(!Kind::Rust.wants("crates/bb-core/tests/x.rs"));
        assert!(!Kind::Rust.wants("web/app/src/x.ts"));
        assert!(Kind::Web.wants("web/app/src/x.tsx"));
        assert!(!Kind::Web.wants("web/app/src/x.css"));
        assert!(Kind::WebAndStyles.wants("web/app/src/x.css"));
        assert!(!Kind::Web.wants("web/node_modules/p/x.ts"));
    }
}
