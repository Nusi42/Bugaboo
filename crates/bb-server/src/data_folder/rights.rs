//! What is too open in a data folder that already exists (SEC-16, follow-up M1-F65).
//!
//! The folders that Bugaboo makes are private from the start, and `keys/`, `tls/` and `logs/` are
//! made private again at every start. The rest of an existing folder is only looked at: the data
//! folder itself, the database file, `files/`, `backups/` and the setup code file. A person may
//! have set their rights on purpose (a group that backs the folder up), so Bugaboo reports what
//! other accounts on this computer can open and changes nothing.
//!
//! *Too open* means: on Linux, a group or world permission bit is set; on Windows, the access
//! list gives something to everybody, to all users or to all signed-in accounts of the computer.
//! When the rights cannot be read (the system tool fails, or its answer cannot be understood) the
//! part is *unknown*, never private: a check that cannot look must not say all is well.

use std::path::{Path, PathBuf};

use bb_app::status::{Access, DataFolderRights};
use bb_core::message::{self, Message, MessageKey};

use super::{BACKUPS, DATABASE_COMPANIONS, DataFolder, FILES};

impl DataFolder {
    /// Which parts of the folder other accounts on this computer can open. Parts that are not
    /// there are private. Looking changes nothing.
    pub(crate) fn rights(&self) -> DataFolderRights {
        self.rights_by(&access_of)
    }

    /// The same, with the way of looking at one path given (the tests make one that fails).
    fn rights_by(&self, look: &dyn Fn(&Path) -> Access) -> DataFolderRights {
        let database = self.database_file();
        let database_access = [""]
            .into_iter()
            .chain(DATABASE_COMPANIONS)
            .map(|suffix| {
                let mut name = database.clone().into_os_string();
                name.push(suffix);
                look(&PathBuf::from(name))
            })
            .fold(Access::Private, Access::worse);
        DataFolderRights {
            folder: look(self.root()),
            database: database_access,
            files: look(&self.root().join(FILES)),
            backups: look(&self.root().join(BACKUPS)),
            setup_code_file: look(&self.setup_code_file()),
        }
    }

    /// One sentence for each part that `rights` calls too open or cannot check, with a short name
    /// of the part for the log. The sentence names the part and its place and says what to do;
    /// Bugaboo changes nothing.
    pub(crate) fn open_parts(&self, rights: DataFolderRights) -> Vec<(&'static str, String)> {
        let parts: [(Access, &'static str, MessageKey, PathBuf); 5] = [
            (
                rights.folder,
                "data folder",
                message::DATA_FOLDER_PART_FOLDER,
                self.root().to_path_buf(),
            ),
            (
                rights.database,
                "database",
                message::DATA_FOLDER_PART_DATABASE,
                self.database_file(),
            ),
            (
                rights.files,
                "files",
                message::DATA_FOLDER_PART_FILES,
                self.root().join(FILES),
            ),
            (
                rights.backups,
                "backups",
                message::DATA_FOLDER_PART_BACKUPS,
                self.root().join(BACKUPS),
            ),
            (
                rights.setup_code_file,
                "setup code file",
                message::DATA_FOLDER_PART_SETUP_CODE_FILE,
                self.setup_code_file(),
            ),
        ];
        parts
            .into_iter()
            .filter_map(|(access, name, part, path)| {
                let part = Message::from(part).render();
                let sentence = match access {
                    Access::Private => return None,
                    Access::Open => message::data_folder_open(part, path.display()),
                    Access::Unknown => message::data_folder_unknown(part, path.display()),
                };
                Some((name, sentence.render()))
            })
            .collect()
    }
}

/// How open `path` is. A path that is not there is private.
fn access_of(path: &Path) -> Access {
    if path.exists() {
        file_access(path)
    } else {
        Access::Private
    }
}

/// Linux: a group or world bit is set.
#[cfg(unix)]
fn file_access(path: &Path) -> Access {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::metadata(path).map_or(Access::Unknown, |data| {
        Access::of(data.permissions().mode() & 0o077 != 0)
    })
}

/// Windows: the access list gives something to everybody. What cannot be read is unknown.
#[cfg(windows)]
fn file_access(path: &Path) -> Access {
    access_list(path).map_or(Access::Unknown, |sddl| sddl_access(&sddl))
}

/// The access list of `path` as the text `icacls /save` writes (SDDL), found by asking the system
/// tool, which writes it to a file in a folder of its own that goes away afterwards.
#[cfg(windows)]
fn access_list(path: &Path) -> Option<String> {
    let folder = tempfile::tempdir().ok()?;
    let saved = folder.path().join("access-list.txt");
    let output = std::process::Command::new(super::system_tool("icacls.exe"))
        .arg(path)
        .arg("/save")
        .arg(&saved)
        .output()
        .ok()?;
    let bytes = std::fs::read(&saved).ok();
    if let Err(error) = folder.close() {
        tracing::debug!(cause = %error, "the temporary access list could not be removed");
    }
    if !output.status.success() {
        return None;
    }
    // The tool writes UTF-16 with a byte order mark.
    let units: Vec<u16> = bytes?
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    Some(String::from_utf16_lossy(&units))
}

/// Anything else: nothing is known about access lists.
#[cfg(not(any(unix, windows)))]
fn file_access(_path: &Path) -> Access {
    Access::Unknown
}

/// The accounts that stand for many people: everybody, all users of the computer, all accounts
/// that signed in, anybody who comes over the network or sits at the computer, guests. Given as
/// the short names SDDL uses and as SIDs. Domain Users and Domain Guests (`S-1-5-21-…-513` and
/// `-514`) are recognised by their ending.
#[cfg(any(windows, test))]
const BROAD: [&str; 14] = [
    "WD",
    "BU",
    "AU",
    "IU",
    "NU",
    "BG",
    "AN",
    "S-1-1-0",
    "S-1-5-32-545",
    "S-1-5-11",
    "S-1-5-4",
    "S-1-5-2",
    "S-1-5-7",
    "S-1-5-32-546",
];

#[cfg(any(windows, test))]
fn is_broad(who: &str) -> bool {
    BROAD.contains(&who)
        || (who.starts_with("S-1-5-21-") && (who.ends_with("-513") || who.ends_with("-514")))
}

/// What the SDDL text (the last line `icacls /save` wrote: `D:PAI(A;;FA;;;SY)…`) says: open when
/// an entry allows something to one of the accounts that stand for many people, unknown when the
/// text has no access list at all. An entry that only passes on to what is made inside (inherit
/// only, `IO`) does not apply to the object itself, and a deny entry gives nothing.
#[cfg(any(windows, test))]
fn sddl_access(sddl: &str) -> Access {
    let Some(list) = sddl
        .lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty())
        .and_then(|line| line.strip_prefix("D:"))
    else {
        return Access::Unknown;
    };
    // Only the part before the system list (`S:`).
    let list = list.split("S:").next().unwrap_or_default();
    Access::of(list.split('(').skip(1).any(|entry| {
        let entry = entry.split(')').next().unwrap_or_default();
        let parts: Vec<&str> = entry.split(';').collect();
        let (Some(kind), Some(flags), Some(who)) = (parts.first(), parts.get(1), parts.get(5))
        else {
            return false;
        };
        matches!(*kind, "A" | "OA") && !flags.contains("IO") && is_broad(who)
    }))
}

#[cfg(test)]
mod tests {
    use super::super::ADMINISTRATORS_SID;
    use super::*;

    fn data(folder: &Path) -> DataFolder {
        DataFolder::prepare(&folder.join("data")).unwrap()
    }

    #[test]
    fn sec_16_the_access_list_of_a_folder_is_open_when_it_gives_something_to_everybody() {
        let private = format!(
            "C:\\data\nD:PAI(A;OICI;FA;;;S-1-5-21-1-2-3-1001)(A;OICI;FA;;;SY)(A;OICI;FA;;;{ADMINISTRATORS_SID})"
        );
        assert_eq!(sddl_access(&private), Access::Private);
        for who in [
            "WD",
            "BU",
            "AU",
            "S-1-1-0",
            "S-1-5-32-545",
            "S-1-5-11",
            "S-1-5-21-111-222-333-513",
        ] {
            let open = format!("D:\\data\nD:PAI(A;OICI;FA;;;SY)(A;OICI;0x1200a9;;;{who})");
            assert_eq!(sddl_access(&open), Access::Open, "{who}");
        }
        // A person of the domain is not everybody.
        let person = "x\nD:PAI(A;OICI;FA;;;S-1-5-21-111-222-333-1105)";
        assert_eq!(sddl_access(person), Access::Private);
        // A path on drive D must not be taken for the list.
        assert_eq!(
            sddl_access("D:\\data\nD:PAI(A;OICI;FA;;;SY)"),
            Access::Private
        );
        // Entries that give nothing to the object itself.
        for gives_nothing in [
            "x\nD:PAI(A;OICIIO;GR;;;WD)(A;OICI;FA;;;SY)",
            "x\nD:PAI(D;;FA;;;WD)(A;OICI;FA;;;SY)",
        ] {
            assert_eq!(sddl_access(gives_nothing), Access::Private);
        }
        // Only the access list counts, not the system list that can follow.
        assert_eq!(
            sddl_access("x\nD:PAI(A;OICI;FA;;;SY)S:AI(A;;FA;;;WD)"),
            Access::Private
        );
    }

    #[test]
    fn sec_16_rights_that_cannot_be_read_are_unknown_and_never_private() {
        // No text, no access list in the text: the answer is unknown.
        assert_eq!(sddl_access(""), Access::Unknown);
        assert_eq!(sddl_access("C:\\data\nsomething else"), Access::Unknown);
        // A look that fails for one part makes that part unknown, and the database unknown when
        // one of its files is.
        let temp = tempfile::tempdir().unwrap();
        let folder = data(temp.path());
        let broken = |path: &Path| {
            if path.ends_with("backups") || path.to_string_lossy().ends_with("bugaboo.db-wal") {
                Access::Unknown
            } else {
                Access::Private
            }
        };
        let rights = folder.rights_by(&broken);
        assert_eq!(rights.backups, Access::Unknown);
        assert_eq!(rights.database, Access::Unknown);
        assert_eq!(rights.files, Access::Private);
        assert!(!rights.any(), "unknown is not open");
        assert!(rights.any_unknown());
        // An open part wins over an unknown one in the same group, and both are told.
        let mixed = |path: &Path| {
            if path.ends_with("bugaboo.db") {
                Access::Open
            } else {
                Access::Unknown
            }
        };
        assert_eq!(folder.rights_by(&mixed).database, Access::Open);
        // Each is named with its place, and the unknown one says so.
        let told = folder.open_parts(rights);
        let names: Vec<&str> = told.iter().map(|(name, _)| *name).collect();
        assert_eq!(names, ["database", "backups"]);
        assert!(told[1].1.contains("could not be checked"), "{}", told[1].1);
        assert!(!told[1].1.contains("can be opened"), "{}", told[1].1);
        // Nothing is the same as private only when every part really was looked at.
        assert!(DataFolderRights::unknown().any_unknown());
    }

    #[test]
    fn sec_16_what_is_too_open_is_named_with_its_place_and_what_to_do() {
        let temp = tempfile::tempdir().unwrap();
        let folder = data(temp.path());
        assert!(folder.open_parts(DataFolderRights::default()).is_empty());
        let rights = DataFolderRights {
            database: Access::Open,
            backups: Access::Open,
            ..DataFolderRights::default()
        };
        let found = folder.open_parts(rights);
        let names: Vec<&str> = found.iter().map(|(name, _)| *name).collect();
        assert_eq!(names, ["database", "backups"]);
        let database = &found[0].1;
        assert!(database.starts_with("The database file "), "{database}");
        assert!(
            database.contains(&folder.database_file().display().to_string()),
            "{database}"
        );
        assert!(database.contains("leaves it as it is"), "{database}");
        assert!(
            database.contains("chmod") && database.contains("Security tab"),
            "{database}"
        );
    }

    #[test]
    fn sec_16_a_folder_that_bugaboo_made_is_not_reported() {
        let temp = tempfile::tempdir().unwrap();
        let folder = data(temp.path());
        assert_eq!(folder.rights(), DataFolderRights::default());
        // Parts that are not there are not open.
        std::fs::write(folder.database_file(), "x").unwrap();
        std::fs::write(folder.setup_code_file(), "x").unwrap();
        assert!(!folder.rights().any());
    }

    #[cfg(unix)]
    #[test]
    fn sec_16_a_too_open_data_folder_is_reported_at_start() {
        use std::os::unix::fs::PermissionsExt as _;
        let temp = tempfile::tempdir().unwrap();
        let folder = data(temp.path());
        let mode = |path: &Path, mode: u32| {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
        };
        std::fs::write(folder.database_file(), "x").unwrap();
        std::fs::write(folder.setup_code_file(), "x").unwrap();
        mode(&folder.database_file(), 0o600);
        mode(&folder.setup_code_file(), 0o600);

        // Somebody made the folders with the rights of an ordinary folder, and the database
        // readable by its group.
        mode(folder.root(), 0o755);
        mode(&folder.root().join("files"), 0o750);
        mode(&folder.root().join("backups"), 0o700);
        mode(&folder.database_file(), 0o640);
        mode(&folder.setup_code_file(), 0o604);
        assert_eq!(
            folder.rights(),
            DataFolderRights {
                folder: Access::Open,
                database: Access::Open,
                files: Access::Open,
                backups: Access::Private,
                setup_code_file: Access::Open,
            }
        );
        // The write-ahead log of the database is part of it.
        mode(&folder.database_file(), 0o600);
        let companion = folder.root().join("bugaboo.db-wal");
        std::fs::write(&companion, "x").unwrap();
        mode(&companion, 0o644);
        assert!(folder.rights().database.is_open());

        // Looking changed nothing.
        let after = |path: &Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(after(folder.root()), 0o755);
        assert_eq!(after(&folder.root().join("files")), 0o750);
        assert_eq!(after(&companion), 0o644);
    }

    #[cfg(windows)]
    #[test]
    fn sec_16_a_too_open_data_folder_is_reported_at_start_on_windows() {
        let temp = tempfile::tempdir().unwrap();
        let folder = data(temp.path());
        assert!(!folder.rights().any(), "{:?}", folder.rights());
        // The access list of the folder for attachments gives the users of the computer a right.
        let files = folder.root().join("files");
        let output = std::process::Command::new("icacls")
            .arg(&files)
            .args(["/grant", "*S-1-5-32-545:(OI)(CI)R"])
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(
            folder.rights(),
            DataFolderRights {
                files: Access::Open,
                ..DataFolderRights::default()
            }
        );
        // Looking changed nothing: the entry is still there.
        assert!(folder.rights().files.is_open());
    }
}
