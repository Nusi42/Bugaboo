//! The members of a backup and the one check of their names (ADR-0046, decision 1).
//!
//! The names are fixed: the manifest, the copy of the database, the copy of the replay list, and
//! the files under `files/`, each at the path it has in the `files/` folder of the data folder. A
//! restore writes a member only where its name says, so a name is read once, here, and refused
//! when it could lead out of the restore's own folder: no `..` or `.`, no absolute path or drive,
//! no backslash, no empty part, and no member that is a link or a folder.

use core::fmt;

/// The manifest: the one description of what the backup holds.
pub const MANIFEST: &str = "manifest.json";
/// The copy of a SQLite database.
pub const DATABASE_SQLITE: &str = "database.sqlite";
/// The copy of a PostgreSQL database: the rows of every table.
pub const DATABASE_ROWS: &str = "database.rows";
/// The copy of the replay list (ADR-0044, decision 8).
pub const REPLAY_LIST: &str = "replay.list";
/// The folder of the files the database points to, inside the archive.
pub const FILES: &str = "files";

/// What kind of entry the archive says a member is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryKind {
    /// A plain file: the only kind a backup holds.
    File,
    /// A symbolic or hard link.
    Link,
    /// A folder.
    Folder,
    /// Anything else (a device, a pipe, an extension of the format).
    Other,
}

/// Why a member's name is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameProblem {
    /// The name is not one a backup holds.
    Unknown,
    /// The name starts at the root of a drive or of the file system.
    Absolute,
    /// A part of the name is `..` or `.`, or empty.
    LeavesFolder,
    /// The name has a backslash, a colon or a control character, which a system could read as a
    /// separator or a drive.
    Unsafe,
    /// The member is a link, a folder or another special entry.
    NotAFile,
}

impl fmt::Display for NameProblem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Unknown => "unknown",
            Self::Absolute => "absolute",
            Self::LeavesFolder => "leaves-folder",
            Self::Unsafe => "unsafe",
            Self::NotAFile => "not-a-file",
        })
    }
}

/// A member's name that passed the check: one of the fixed names, or a path under `files/`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MemberName(String);

impl MemberName {
    /// Checks the name of an entry of the archive and the kind of the entry.
    ///
    /// # Errors
    /// The [`NameProblem`] of a name that could leave the restore's folder, or of an entry that is
    /// not a plain file.
    pub fn check(name: &str, kind: EntryKind) -> Result<Self, NameProblem> {
        if kind != EntryKind::File {
            return Err(NameProblem::NotAFile);
        }
        Self::parse(name)
    }

    /// Checks a name as the manifest lists it.
    ///
    /// # Errors
    /// The [`NameProblem`] of a name that could leave the restore's folder or is not one a backup
    /// holds.
    pub fn parse(name: &str) -> Result<Self, NameProblem> {
        if name.starts_with('/') || name.as_bytes().get(1) == Some(&b':') {
            return Err(NameProblem::Absolute);
        }
        if name
            .chars()
            .any(|c| c == '\\' || c == ':' || c.is_control())
        {
            return Err(NameProblem::Unsafe);
        }
        if name
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err(NameProblem::LeavesFolder);
        }
        let fixed = [MANIFEST, DATABASE_SQLITE, DATABASE_ROWS, REPLAY_LIST];
        let in_files = name
            .strip_prefix(FILES)
            .and_then(|rest| rest.strip_prefix('/'))
            .is_some();
        if fixed.contains(&name) || in_files {
            Ok(Self(name.to_owned()))
        } else {
            Err(NameProblem::Unknown)
        }
    }

    /// The member of a file at `path` in the `files/` folder (parts separated by `/`).
    ///
    /// # Errors
    /// The [`NameProblem`] of a path that could leave the folder.
    pub fn of_file(path: &str) -> Result<Self, NameProblem> {
        Self::parse(&format!("{FILES}/{path}"))
    }

    /// The name as the archive holds it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// For a member under `files/`: the parts of its path inside the `files/` folder, each checked
    /// to stay inside it. `None` for the fixed members.
    #[must_use]
    pub fn file_parts(&self) -> Option<Vec<&str>> {
        self.0
            .strip_prefix(FILES)
            .and_then(|rest| rest.strip_prefix('/'))
            .map(|rest| rest.split('/').collect())
    }
}

impl fmt::Display for MemberName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_member_name_that_leaves_the_folder_is_refused() {
        let cases: &[(&str, EntryKind, Result<(), NameProblem>)] = &[
            (MANIFEST, EntryKind::File, Ok(())),
            (DATABASE_SQLITE, EntryKind::File, Ok(())),
            (DATABASE_ROWS, EntryKind::File, Ok(())),
            (REPLAY_LIST, EntryKind::File, Ok(())),
            ("files/content/ab/abcdef", EntryKind::File, Ok(())),
            (
                "../manifest.json",
                EntryKind::File,
                Err(NameProblem::LeavesFolder),
            ),
            (
                "files/../../keys/master.key",
                EntryKind::File,
                Err(NameProblem::LeavesFolder),
            ),
            ("files/./a", EntryKind::File, Err(NameProblem::LeavesFolder)),
            ("files//a", EntryKind::File, Err(NameProblem::LeavesFolder)),
            ("files/", EntryKind::File, Err(NameProblem::LeavesFolder)),
            ("/etc/passwd", EntryKind::File, Err(NameProblem::Absolute)),
            ("/files/a", EntryKind::File, Err(NameProblem::Absolute)),
            ("C:/Windows/a", EntryKind::File, Err(NameProblem::Absolute)),
            ("c:a", EntryKind::File, Err(NameProblem::Absolute)),
            ("files\\..\\a", EntryKind::File, Err(NameProblem::Unsafe)),
            ("files/a:stream", EntryKind::File, Err(NameProblem::Unsafe)),
            ("files/a\u{0}b", EntryKind::File, Err(NameProblem::Unsafe)),
            (
                "keys/master.key",
                EntryKind::File,
                Err(NameProblem::Unknown),
            ),
            ("filesx/a", EntryKind::File, Err(NameProblem::Unknown)),
            ("files/link", EntryKind::Link, Err(NameProblem::NotAFile)),
            (
                "files/folder",
                EntryKind::Folder,
                Err(NameProblem::NotAFile),
            ),
            ("files/pipe", EntryKind::Other, Err(NameProblem::NotAFile)),
        ];
        for (name, kind, expected) in cases {
            let found = MemberName::check(name, *kind).map(|_| ());
            assert_eq!(&found, expected, "{name:?} as {kind:?}");
        }
    }

    #[test]
    fn a_file_member_names_its_place_in_the_files_folder() {
        let member = MemberName::of_file("content/ab/abcdef").unwrap();
        assert_eq!(member.as_str(), "files/content/ab/abcdef");
        assert_eq!(member.file_parts(), Some(vec!["content", "ab", "abcdef"]));
        assert_eq!(MemberName::parse(MANIFEST).unwrap().file_parts(), None);
        assert_eq!(MemberName::of_file("../x"), Err(NameProblem::LeavesFolder));
    }
}
