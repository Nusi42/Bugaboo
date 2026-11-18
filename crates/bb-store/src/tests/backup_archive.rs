//! The backup file: what is written reads back, and a crafted or damaged file is refused before
//! anything outside the folder of the restore is written (ADR-0046, decision 1).

use bb_core::backup::DatabaseKind;
use bb_core::backup::manifest::{BackupProblem, FORMAT, Manifest};
use bb_core::backup::member::NameProblem;
use jiff::Timestamp;

use super::*;

fn manifest() -> Manifest {
    Manifest {
        format: FORMAT,
        version: "0.6.0".to_owned(),
        database: DatabaseKind::Sqlite,
        moment: Timestamp::from_second(1_767_225_600).unwrap(),
        migrations: Vec::new(),
        security_log_head: None,
        members: Vec::new(),
    }
}

/// An archive with a database and one file, written into `folder`.
fn written(folder: &Path) -> PathBuf {
    let database = folder.join("db");
    std::fs::write(&database, b"the rows").unwrap();
    let file = folder.join("file");
    std::fs::write(&file, b"the bytes of an attachment").unwrap();
    let archive = folder.join("a.backup");
    let sources = [
        Source {
            name: MemberName::parse(member::DATABASE_SQLITE).unwrap(),
            path: database,
        },
        Source {
            name: MemberName::of_file("content/ab/abcd").unwrap(),
            path: file,
        },
    ];
    write(&archive, &mut manifest(), &sources).unwrap();
    archive
}

fn refusal(result: Result<Manifest, StoreError>) -> BackupProblem {
    match result {
        Err(StoreError::BackupRefused(problem)) => problem,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn a_backup_file_reads_back_what_was_written() {
    let folder = tempfile::tempdir().unwrap();
    let archive = written(folder.path());
    let (size, digest) = measure(&archive).unwrap();
    assert!(size > 0);
    assert_ne!(digest, Digest::new([0; 32]));
    let into = folder.path().join("out");
    std::fs::create_dir(&into).unwrap();
    let read = read(&archive, &into).unwrap();
    assert_eq!(read.members.len(), 2);
    let file = extracted(&into, &MemberName::of_file("content/ab/abcd").unwrap());
    assert_eq!(std::fs::read(file).unwrap(), b"the bytes of an attachment");
}

#[test]
fn a_changed_byte_is_refused() {
    let folder = tempfile::tempdir().unwrap();
    let archive = written(folder.path());
    let mut bytes = std::fs::read(&archive).unwrap();
    let at = bytes
        .windows(8)
        .position(|window| window == b"the rows")
        .unwrap();
    bytes[at] = b'T';
    std::fs::write(&archive, bytes).unwrap();
    let into = folder.path().join("out");
    std::fs::create_dir(&into).unwrap();
    assert_eq!(
        refusal(read(&archive, &into)),
        BackupProblem::Damaged(member::DATABASE_SQLITE.to_owned())
    );
}

/// An archive with the right manifest first and then an entry with a name and kind written
/// straight into its header, as somebody who crafts a file would.
fn crafted(folder: &Path, name: &str, kind: EntryType) -> PathBuf {
    let archive = folder.join("crafted.backup");
    let file = File::create(&archive).unwrap();
    let mut builder = Builder::new(file);
    // The manifest lists what a backup must hold, so the entry after it is read.
    let mut listed = manifest();
    listed.members.push(Member::new(
        &MemberName::parse(member::DATABASE_SQLITE).unwrap(),
        4,
        &Digest::new([0; 32]),
    ));
    let json = listed.to_json().unwrap();
    let mut head = header(json.len() as u64, 0);
    builder
        .append_data(&mut head, member::MANIFEST, json.as_slice())
        .unwrap();
    let mut head = header(4, 0);
    head.set_entry_type(kind);
    head.as_old_mut().name[..name.len()].copy_from_slice(name.as_bytes());
    head.set_cksum();
    builder.append(&head, &b"evil"[..]).unwrap();
    builder.finish().unwrap();
    archive
}

#[test]
fn ops_07_a_name_that_leaves_the_folder_is_refused() {
    let cases = [
        ("../escaped", EntryType::Regular, NameProblem::LeavesFolder),
        (
            "files/../../escaped",
            EntryType::Regular,
            NameProblem::LeavesFolder,
        ),
        ("/tmp/escaped", EntryType::Regular, NameProblem::Absolute),
        ("files/link", EntryType::Symlink, NameProblem::NotAFile),
        ("files/hard", EntryType::Link, NameProblem::NotAFile),
    ];
    for (name, kind, problem) in cases {
        let folder = tempfile::tempdir().unwrap();
        let archive = crafted(folder.path(), name, kind);
        let into = folder.path().join("restore").join("inside");
        std::fs::create_dir_all(&into).unwrap();
        assert_eq!(
            refusal(read(&archive, &into)),
            BackupProblem::BadName(name.to_owned(), problem),
            "{name}"
        );
        // Nothing was written, inside the folder or beside it.
        assert_eq!(std::fs::read_dir(&into).unwrap().count(), 0, "{name}");
        assert!(!folder.path().join("restore").join("escaped").exists());
        assert!(!folder.path().join("escaped").exists());
    }
}
