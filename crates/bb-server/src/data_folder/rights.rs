//! What is too open in a data folder that already exists (SEC-16, follow-up M1-F65).
//!
//! The folders that Bugaboo makes are private from the start, and `keys/`, `tls/` and `logs/` are
//! made private again at every start. The rest of an existing folder is only looked at: the data
//! folder itself, the database file, `files/`, `backups/` and the setup code file. A person may
//! have made or moved them on purpose (a group that backs the folder up), so Bugaboo reports what
//! other accounts on this computer can open and changes nothing.
//!
//! *Too open* means: on Linux, a group or world permission bit is set; on Windows, the access
//! list gives something to everybody, to all users or to all signed-in accounts of the computer.

use std::path::{Path, PathBuf};

use bb_app::status::{Access, DataFolderRights};
use bb_core::message::{self, Message, MessageKey};

use super::{BACKUPS, DATABASE_COMPANIONS, DataFolder, FILES};

impl DataFolder {
    /// Which parts of the folder other accounts on this computer can open. Parts that are not
    /// there are not open. Looking changes nothing.
    pub(crate) fn rights(&self) -> DataFolderRights {
        let database = self.database_file();
        let database_open = [""].into_iter().chain(DATABASE_COMPANIONS).any(|suffix| {
            let mut name = database.clone().into_os_string();
            name.push(suffix);
            too_open(&PathBuf::from(name))
        });
        DataFolderRights {
            folder: Access::of(too_open(self.root())),
            database: Access::of(database_open),
            files: Access::of(too_open(&self.root().join(FILES))),
            backups: Access::of(too_open(&self.root().join(BACKUPS))),
            setup_code_file: Access::of(too_open(&self.setup_code_file())),
        }
    }
}

impl DataFolder {
    /// One sentence for each part that `rights` calls too open, with a short name of the part for
    /// the log. The sentence names the part and its place and says what to do; Bugaboo changes
    /// nothing.
    pub(crate) fn open_parts(&self, rights: DataFolderRights) -> Vec<(&'static str, String)> {
        let parts: [(bool, &'static str, MessageKey, PathBuf); 5] = [
            (
                rights.folder.is_open(),
                "data folder",
                message::DATA_FOLDER_PART_FOLDER,
                self.root().to_path_buf(),
            ),
            (
                rights.database.is_open(),
                "database",
                message::DATA_FOLDER_PART_DATABASE,
                self.database_file(),
            ),
            (
                rights.files.is_open(),
                "files",
                message::DATA_FOLDER_PART_FILES,
                self.root().join(FILES),
            ),
            (
                rights.backups.is_open(),
                "backups",
                message::DATA_FOLDER_PART_BACKUPS,
                self.root().join(BACKUPS),
            ),
            (
                rights.setup_code_file.is_open(),
                "setup code file",
                message::DATA_FOLDER_PART_SETUP_CODE_FILE,
                self.setup_code_file(),
            ),
        ];
        parts
            .into_iter()
            .filter(|(open, ..)| *open)
            .map(|(_, name, part, path)| {
                let sentence =
                    message::data_folder_open(Message::from(part).render(), path.display());
                (name, sentence.render())
            })
            .collect()
    }
}

/// Whether other accounts can open `path`. A path that is not there is not open.
fn too_open(path: &Path) -> bool {
    path.exists() && open_to_everybody(path)
}

/// Linux: a group or world bit is set.
#[cfg(unix)]
fn open_to_everybody(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::metadata(path).is_ok_and(|data| data.permissions().mode() & 0o077 != 0)
}

/// Windows: the access list gives something to everybody. What cannot be read is not reported.
#[cfg(windows)]
fn open_to_everybody(path: &Path) -> bool {
    access_list(path).is_some_and(|sddl| grants_to_everybody(&sddl))
}

/// The access list of `path` as the text `icacls /save` writes (SDDL), found by asking the system
/// tool, which writes it to a file of its own in the temporary folder.
#[cfg(windows)]
fn access_list(path: &Path) -> Option<String> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let saved = std::env::temp_dir().join(format!(
        "bugaboo-access-list-{}-{}.txt",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let output = std::process::Command::new(super::system_tool("icacls.exe"))
        .arg(path)
        .arg("/save")
        .arg(&saved)
        .output()
        .ok()?;
    let bytes = std::fs::read(&saved).ok();
    let _ = std::fs::remove_file(&saved);
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
fn open_to_everybody(_path: &Path) -> bool {
    false
}

/// The accounts that stand for many people: everybody, all users of the computer, all accounts
/// that signed in, anybody who comes over the network or sits at the computer, guests. Given as
/// the short names SDDL uses and as SIDs.
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

/// Whether the SDDL text (the last line `icacls /save` wrote: `D:PAI(A;;FA;;;SY)…`) has an entry
/// that allows something to one of the accounts that stand for many people. An entry that only
/// passes on to what is made inside (inherit only, `IO`) does not apply to the object itself, and
/// a deny entry gives nothing.
#[cfg(any(windows, test))]
fn grants_to_everybody(sddl: &str) -> bool {
    let Some(list) = sddl
        .lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty())
        .and_then(|line| line.strip_prefix("D:"))
    else {
        return false;
    };
    // Only the part before the system list (`S:`).
    let list = list.split("S:").next().unwrap_or_default();
    list.split('(').skip(1).any(|entry| {
        let entry = entry.split(')').next().unwrap_or_default();
        let parts: Vec<&str> = entry.split(';').collect();
        let (Some(kind), Some(flags), Some(who)) = (parts.first(), parts.get(1), parts.get(5))
        else {
            return false;
        };
        matches!(*kind, "A" | "OA") && !flags.contains("IO") && BROAD.contains(who)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(folder: &Path) -> DataFolder {
        DataFolder::prepare(&folder.join("data")).unwrap()
    }

    #[test]
    fn sec_16_the_access_list_of_a_folder_is_open_when_it_gives_something_to_everybody() {
        let private = "C:\\data\nD:PAI(A;OICI;FA;;;S-1-5-21-1-2-3-1001)(A;OICI;FA;;;SY)(A;OICI;FA;;;S-1-5-32-544)";
        assert!(!grants_to_everybody(private));
        for who in ["WD", "BU", "AU", "S-1-1-0", "S-1-5-32-545", "S-1-5-11"] {
            let open = format!("D:\\data\nD:PAI(A;OICI;FA;;;SY)(A;OICI;0x1200a9;;;{who})");
            assert!(grants_to_everybody(&open), "{who}");
        }
        // A path on drive D must not be taken for the list.
        assert!(!grants_to_everybody("D:\\data\nD:PAI(A;OICI;FA;;;SY)"));
        // Entries that give nothing to the object itself.
        assert!(!grants_to_everybody(
            "x\nD:PAI(A;OICIIO;GR;;;WD)(A;OICI;FA;;;SY)"
        ));
        assert!(!grants_to_everybody("x\nD:PAI(D;;FA;;;WD)(A;OICI;FA;;;SY)"));
        // Only the access list counts, not the system list that can follow.
        assert!(!grants_to_everybody(
            "x\nD:PAI(A;OICI;FA;;;SY)S:AI(A;;FA;;;WD)"
        ));
        assert!(!grants_to_everybody(""));
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
