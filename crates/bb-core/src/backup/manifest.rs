//! The manifest of a backup (`manifest.json`, ADR-0046 decision 1): the version of Bugaboo, the
//! migrations applied to the database with their checksums, the kind of database, the moment, the
//! head of the security log, and the size and SHA-256 of every member.
//!
//! A restore reads the manifest first and checks it here before it changes anything: a format it
//! does not know, a name that leaves the folder, a member missing or one too many, a migration the
//! program does not know or one that differs, and a member whose bytes do not match are all
//! refused (decision 4).

use std::collections::HashSet;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use super::DatabaseKind;
use super::member::{self, MemberName, NameProblem};
use crate::hex;
use crate::security_log::Digest;

/// The format of the manifest this program writes and reads. A backup of a newer format is
/// refused, whatever else it holds.
pub const FORMAT: u32 = 1;

/// The one description of a backup.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// The format of this manifest ([`FORMAT`]).
    pub format: u32,
    /// The version of Bugaboo that made the backup.
    pub version: String,
    /// The kind of database the copy is of.
    pub database: DatabaseKind,
    /// The moment of the copy: every change saved before it is in the backup.
    pub moment: Timestamp,
    /// The migrations applied to the database at the moment of the copy, oldest first.
    pub migrations: Vec<AppliedMigration>,
    /// The newest entry of the security log in the copy, if it has one.
    pub security_log_head: Option<LogHead>,
    /// Every member of the archive but the manifest itself.
    pub members: Vec<Member>,
}

/// A migration applied to the database of a backup.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppliedMigration {
    /// Its number.
    pub version: i64,
    /// Its name.
    pub name: String,
    /// The checksum of its steps, as 64 hex digits.
    pub checksum: String,
}

/// The newest entry of the security log in a backup.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogHead {
    /// Its place in the chain.
    pub seq: i64,
    /// Its hash, as 64 hex digits.
    pub hash: String,
}

impl LogHead {
    /// The head of the log at `seq` with `hash`.
    #[must_use]
    pub fn new(seq: i64, hash: &Digest) -> Self {
        Self {
            seq,
            hash: hex::encode(hash.as_bytes()),
        }
    }

    /// The hash, when it is 64 hex digits (checked when the manifest is read).
    #[must_use]
    pub fn digest(&self) -> Option<Digest> {
        hex::decode(&self.hash).map(Digest::new)
    }
}

/// One member of the archive.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Member {
    /// Its name inside the archive.
    pub name: String,
    /// Its size in bytes.
    pub size: u64,
    /// The SHA-256 of its bytes, as 64 hex digits.
    pub sha256: String,
}

impl Member {
    /// The member `name` with these bytes counted.
    #[must_use]
    pub fn new(name: &MemberName, size: u64, sha256: &Digest) -> Self {
        Self {
            name: name.as_str().to_owned(),
            size,
            sha256: hex::encode(sha256.as_bytes()),
        }
    }
}

/// A migration of the program, to hold the migrations of a backup against.
#[derive(Clone, Copy, Debug)]
pub struct KnownMigration<'a> {
    /// Its number.
    pub version: i64,
    /// Its name.
    pub name: &'a str,
    /// The checksum of its steps.
    pub checksum: Digest,
}

/// Why a backup is refused. A refused backup changes nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BackupProblem {
    /// The archive or its manifest cannot be read.
    Unreadable,
    /// The manifest is of a newer format than this program knows.
    NewerFormat(u32),
    /// A member's name could leave the restore's folder, or is not one a backup holds.
    BadName(String, NameProblem),
    /// A member the manifest lists is not in the archive.
    Missing(String),
    /// The archive holds a member the manifest does not list, or one twice.
    Extra(String),
    /// A member's bytes do not match its size or checksum.
    Damaged(String),
    /// The backup holds a migration this program does not know: a newer Bugaboo made it.
    NewerMigration(i64),
    /// A migration of the backup differs from the migration of the same number in this program.
    ChangedMigration(i64),
    /// The backup is of another kind of database than the one it would be restored into.
    OtherDatabase(DatabaseKind),
}

/// Whether `text` is 64 hex digits.
fn is_digest(text: &str) -> bool {
    hex::decode::<{ Digest::LENGTH }>(text).is_some()
}

impl Manifest {
    /// Reads a manifest and checks what it says on its own: its format, the names of its members
    /// (each once, the copy of its kind of database among them) and the form of its checksums.
    ///
    /// # Errors
    /// The [`BackupProblem`] of the first thing that does not fit.
    pub fn read(bytes: &[u8]) -> Result<Self, BackupProblem> {
        #[derive(Deserialize)]
        struct Format {
            format: u32,
        }
        let format: Format =
            serde_json::from_slice(bytes).map_err(|_| BackupProblem::Unreadable)?;
        if format.format > FORMAT {
            return Err(BackupProblem::NewerFormat(format.format));
        }
        let manifest: Self =
            serde_json::from_slice(bytes).map_err(|_| BackupProblem::Unreadable)?;
        manifest.check_form()?;
        Ok(manifest)
    }

    fn check_form(&self) -> Result<(), BackupProblem> {
        let mut seen: Vec<&str> = Vec::with_capacity(self.members.len());
        let mut folded = HashSet::with_capacity(self.members.len());
        for listed in &self.members {
            MemberName::parse(&listed.name)
                .map_err(|problem| BackupProblem::BadName(listed.name.clone(), problem))?;
            // Two names that differ only in case would be one file on Windows.
            if listed.name == member::MANIFEST || !folded.insert(member::folded(&listed.name)) {
                return Err(BackupProblem::Extra(listed.name.clone()));
            }
            if !is_digest(&listed.sha256) {
                return Err(BackupProblem::Unreadable);
            }
            seen.push(&listed.name);
        }
        let database = self.database.member();
        if !seen.contains(&database) {
            return Err(BackupProblem::Missing(database.to_owned()));
        }
        let other = [member::DATABASE_SQLITE, member::DATABASE_ROWS]
            .into_iter()
            .find(|name| *name != database && seen.contains(name));
        if let Some(other) = other {
            return Err(BackupProblem::Extra(other.to_owned()));
        }
        let well_formed = self
            .migrations
            .iter()
            .all(|migration| is_digest(&migration.checksum))
            && self
                .security_log_head
                .as_ref()
                .is_none_or(|head| head.digest().is_some());
        if well_formed {
            Ok(())
        } else {
            Err(BackupProblem::Unreadable)
        }
    }

    /// The manifest as the archive holds it.
    ///
    /// # Errors
    /// Never in practice; JSON of these types always writes.
    pub fn to_json(&self) -> Result<Vec<u8>, BackupProblem> {
        serde_json::to_vec_pretty(self).map_err(|_| BackupProblem::Unreadable)
    }

    /// The member listed under `name`.
    #[must_use]
    pub fn member(&self, name: &str) -> Option<&Member> {
        self.members.iter().find(|listed| listed.name == name)
    }

    /// Checks the migrations of the backup against the program's: every one must be known, with
    /// the same name and checksum. Migrations the backup lacks are fine: a restore runs them.
    ///
    /// # Errors
    /// [`BackupProblem::NewerMigration`] or [`BackupProblem::ChangedMigration`].
    pub fn check_migrations(&self, known: &[KnownMigration<'_>]) -> Result<(), BackupProblem> {
        for applied in &self.migrations {
            let Some(mine) = known.iter().find(|mine| mine.version == applied.version) else {
                return Err(BackupProblem::NewerMigration(applied.version));
            };
            if mine.name != applied.name
                || hex::encode(mine.checksum.as_bytes()) != applied.checksum
            {
                return Err(BackupProblem::ChangedMigration(applied.version));
            }
        }
        Ok(())
    }

    /// Checks that the backup is of the kind of database it is restored into.
    ///
    /// # Errors
    /// [`BackupProblem::OtherDatabase`] with the kind of the backup.
    pub fn check_database(&self, here: DatabaseKind) -> Result<(), BackupProblem> {
        if self.database == here {
            Ok(())
        } else {
            Err(BackupProblem::OtherDatabase(self.database))
        }
    }

    /// Checks one member of the archive against the manifest: it must be listed, and its bytes
    /// must have the listed size and checksum.
    ///
    /// # Errors
    /// [`BackupProblem::Extra`] for a member that is not listed, [`BackupProblem::Damaged`] for
    /// bytes that do not match.
    pub fn check_member(
        &self,
        name: &MemberName,
        size: u64,
        sha256: &Digest,
    ) -> Result<(), BackupProblem> {
        let listed = self
            .member(name.as_str())
            .ok_or_else(|| BackupProblem::Extra(name.to_string()))?;
        if listed.size == size && listed.sha256 == hex::encode(sha256.as_bytes()) {
            Ok(())
        } else {
            Err(BackupProblem::Damaged(name.to_string()))
        }
    }

    /// The first member the manifest lists that is not among `found`.
    ///
    /// # Errors
    /// [`BackupProblem::Missing`] with its name.
    pub fn check_complete<'a>(
        &self,
        found: impl IntoIterator<Item = &'a MemberName>,
    ) -> Result<(), BackupProblem> {
        let found: Vec<&str> = found.into_iter().map(MemberName::as_str).collect();
        match self
            .members
            .iter()
            .find(|listed| !found.contains(&listed.name.as_str()))
        {
            Some(missing) => Err(BackupProblem::Missing(missing.name.clone())),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: u8) -> Digest {
        Digest::new([byte; 32])
    }

    fn manifest() -> Manifest {
        Manifest {
            format: FORMAT,
            version: "0.5.1".to_owned(),
            database: DatabaseKind::Sqlite,
            moment: Timestamp::from_second(1_767_225_600).unwrap(),
            migrations: vec![AppliedMigration {
                version: 1,
                name: "initial".to_owned(),
                checksum: hex::encode(digest(1).as_bytes()),
            }],
            security_log_head: Some(LogHead::new(4, &digest(9))),
            members: vec![
                Member::new(
                    &MemberName::parse(member::DATABASE_SQLITE).unwrap(),
                    10,
                    &digest(2),
                ),
                Member::new(
                    &MemberName::of_file("content/ab/ab").unwrap(),
                    3,
                    &digest(3),
                ),
            ],
        }
    }

    #[test]
    fn a_manifest_reads_back_what_was_written() {
        let written = manifest();
        let read = Manifest::read(&written.to_json().unwrap()).unwrap();
        assert_eq!(read, written);
        assert_eq!(read.security_log_head.unwrap().digest(), Some(digest(9)));
    }

    #[test]
    fn a_manifest_refuses_a_newer_format_an_unknown_migration_and_changed_bytes() {
        // A newer format is refused before anything else is read.
        let newer = br#"{"format": 2, "anything": true}"#;
        assert_eq!(Manifest::read(newer), Err(BackupProblem::NewerFormat(2)));
        assert_eq!(Manifest::read(b"not json"), Err(BackupProblem::Unreadable));
        // A migration the program does not know, or one that differs.
        let known = [KnownMigration {
            version: 1,
            name: "initial",
            checksum: digest(1),
        }];
        let mut newer = manifest();
        newer.migrations.push(AppliedMigration {
            version: 2,
            name: "later".to_owned(),
            checksum: hex::encode(digest(5).as_bytes()),
        });
        assert_eq!(
            newer.check_migrations(&known),
            Err(BackupProblem::NewerMigration(2))
        );
        let changed = [KnownMigration {
            checksum: digest(7),
            ..known[0]
        }];
        assert_eq!(
            manifest().check_migrations(&changed),
            Err(BackupProblem::ChangedMigration(1))
        );
        assert_eq!(manifest().check_migrations(&known), Ok(()));
        // A member whose bytes changed.
        let database = MemberName::parse(member::DATABASE_SQLITE).unwrap();
        assert_eq!(manifest().check_member(&database, 10, &digest(2)), Ok(()));
        assert_eq!(
            manifest().check_member(&database, 10, &digest(4)),
            Err(BackupProblem::Damaged(member::DATABASE_SQLITE.to_owned()))
        );
        assert_eq!(
            manifest().check_member(&database, 11, &digest(2)),
            Err(BackupProblem::Damaged(member::DATABASE_SQLITE.to_owned()))
        );
        // A member that is missing or not listed.
        assert_eq!(
            manifest().check_complete([&database]),
            Err(BackupProblem::Missing("files/content/ab/ab".to_owned()))
        );
        let replay = MemberName::parse(member::REPLAY_LIST).unwrap();
        assert_eq!(
            manifest().check_member(&replay, 0, &digest(0)),
            Err(BackupProblem::Extra(member::REPLAY_LIST.to_owned()))
        );
        // Another kind of database.
        assert_eq!(
            manifest().check_database(DatabaseKind::Postgres),
            Err(BackupProblem::OtherDatabase(DatabaseKind::Sqlite))
        );
    }

    #[test]
    fn a_manifest_that_lists_a_bad_name_or_no_database_is_refused() {
        let mut bad = manifest();
        bad.members[1].name = "files/../keys/master.key".to_owned();
        assert_eq!(
            Manifest::read(&bad.to_json().unwrap()),
            Err(BackupProblem::BadName(
                "files/../keys/master.key".to_owned(),
                NameProblem::LeavesFolder
            ))
        );
        let mut twice = manifest();
        twice.members.push(twice.members[1].clone());
        assert_eq!(
            Manifest::read(&twice.to_json().unwrap()),
            Err(BackupProblem::Extra("files/content/ab/ab".to_owned()))
        );
        let mut in_case = manifest();
        let mut upper = in_case.members[1].clone();
        upper.name = "files/content/AB/ab".to_owned();
        in_case.members.push(upper);
        assert_eq!(
            Manifest::read(&in_case.to_json().unwrap()),
            Err(BackupProblem::Extra("files/content/AB/ab".to_owned()))
        );
        let mut without = manifest();
        without.members.remove(0);
        assert_eq!(
            Manifest::read(&without.to_json().unwrap()),
            Err(BackupProblem::Missing(member::DATABASE_SQLITE.to_owned()))
        );
        let mut both = manifest();
        both.members.push(Member::new(
            &MemberName::parse(member::DATABASE_ROWS).unwrap(),
            1,
            &digest(1),
        ));
        assert_eq!(
            Manifest::read(&both.to_json().unwrap()),
            Err(BackupProblem::Extra(member::DATABASE_ROWS.to_owned()))
        );
        let mut short = manifest();
        short.members[0].sha256 = "abc".to_owned();
        assert_eq!(
            Manifest::read(&short.to_json().unwrap()),
            Err(BackupProblem::Unreadable)
        );
    }
}
