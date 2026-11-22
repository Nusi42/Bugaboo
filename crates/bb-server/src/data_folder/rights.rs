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
                    Access::Open => open_sentence(&part, &path),
                    Access::Unknown => message::data_folder_unknown(message::DataFolderUnknown {
                        part: &part,
                        path: &path.display(),
                    }),
                };
                Some((name, sentence.render()))
            })
            .collect()
    }
}

/// The sentence of a part that other accounts can open, with the command that takes their rights
/// away: `chmod` here. It needs no `-R` for a folder: the check reads the rights of the folder
/// itself, and once other accounts cannot enter it they cannot reach anything inside, where what
/// Bugaboo makes is private already.
#[cfg(not(windows))]
fn open_sentence(part: &str, path: &Path) -> Message {
    message::data_folder_open(message::DataFolderOpen {
        part: &part,
        path: &path.display(),
    })
}

/// The same on Windows, with the account that runs Bugaboo.
#[cfg(windows)]
fn open_sentence(part: &str, path: &Path) -> Message {
    windows_open_sentence(part, path, super::current_sid().ok().as_deref())
}

/// The sentence on Windows for the account `sid`: the two `icacls` commands that keep only that
/// account, exactly as Bugaboo restricts what it makes itself (`restrict_arguments`), with the
/// grants of a folder or of a file. Without the account the sentence names no command and points
/// to the Security tab: the account of the person who types is the wrong one when a service runs
/// Bugaboo.
#[cfg(windows)]
fn windows_open_sentence(part: &str, path: &Path, sid: Option<&str>) -> Message {
    let Some(sid) = sid else {
        return message::data_folder_open_windows_unknown_account(
            message::DataFolderOpenWindowsUnknownAccount {
                part: &part,
                path: &path.display(),
            },
        );
    };
    let grants = if path.is_dir() {
        super::folder_grants(sid)
    } else {
        super::file_grants(sid)
    };
    let quoted = |word: &str| format!("\"{word}\"");
    let command = bb_core::access_list::restrict_arguments(&grants)
        .map(|arguments| {
            std::iter::once(format!("icacls {}", quoted(&path.display().to_string())))
                .chain(arguments.iter().map(|argument| {
                    if argument.starts_with('/') {
                        argument.clone()
                    } else {
                        quoted(argument)
                    }
                }))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .join(" && ");
    message::data_folder_open_windows(message::DataFolderOpenWindows {
        command: &command,
        part: &part,
        path: &path.display(),
    })
}

/// The mode bits on Linux that make a part of the data folder too open: any right of the group or
/// of everybody.
const OPEN_BITS: u32 = 0o077;

/// How open `path` is. A path that is not there is private.
fn access_of(path: &Path) -> Access {
    access_with(path, OPEN_BITS)
}

/// How open `path` is to other accounts on this computer: on Linux one of `open_bits` is set in
/// its mode, on Windows its access list gives something to an account that stands for many
/// people. A path that is not there is private; what cannot be read is unknown. The one reading
/// of these rights, for the data folder and for the settings file and the HTTPS key.
pub(crate) fn access_with(path: &Path, open_bits: u32) -> Access {
    if path.exists() {
        file_access(path, open_bits)
    } else {
        Access::Private
    }
}

/// Linux: one of `open_bits` is set.
#[cfg(unix)]
fn file_access(path: &Path, open_bits: u32) -> Access {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::metadata(path).map_or(Access::Unknown, |data| {
        Access::of(data.permissions().mode() & open_bits != 0)
    })
}

/// Windows: the access list gives something to everybody. What cannot be read is unknown.
#[cfg(windows)]
fn file_access(path: &Path, _open_bits: u32) -> Access {
    access_list(path).map_or(Access::Unknown, |sddl| sddl_access(&sddl))
}

/// The access list of `path` as the text `icacls /save` writes (SDDL), found by asking the system
/// tool, which writes it to a file in a folder of its own that goes away afterwards.
#[cfg(windows)]
pub(crate) fn access_list(path: &Path) -> Option<String> {
    let folder = tempfile::tempdir().ok()?;
    let saved = folder.path().join("access-list.txt");
    let output = std::process::Command::new(crate::system::system_tool("icacls.exe").ok()?)
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
    Some(bb_core::access_list::saved_text(&bytes?))
}

/// The owners (SIDs) of `paths`, in their order, found by asking Windows PowerShell once for all
/// of them (`icacls` does not show owners); `None` for each one whose owner cannot be read.
#[cfg(windows)]
pub(crate) fn owners(paths: &[PathBuf]) -> Vec<Option<String>> {
    use base64ct::Encoding as _;
    let texts: Vec<String> = paths
        .iter()
        .map(|path| path.display().to_string())
        .collect();
    let script = bb_core::access_list::owners_script(&texts);
    // The script goes in encoded as PowerShell expects (UTF-16, base64), so no quoting of the
    // command line can change it.
    let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let output = crate::system::system_tool(POWERSHELL)
        .ok()
        .and_then(|program| {
            std::process::Command::new(program)
                .args(POWERSHELL_ARGUMENTS)
                .arg(base64ct::Base64::encode_string(&bytes))
                .output()
                .ok()
        });
    output.filter(|output| output.status.success()).map_or_else(
        || vec![None; paths.len()],
        |output| {
            bb_core::access_list::owners_in(&String::from_utf8_lossy(&output.stdout), paths.len())
        },
    )
}

/// Windows PowerShell, in the system folder of Windows.
#[cfg(windows)]
const POWERSHELL: &str = r"WindowsPowerShell\v1.0\powershell.exe";

/// How Windows PowerShell is started: no profile, nothing asked, the script given encoded next.
#[cfg(windows)]
const POWERSHELL_ARGUMENTS: [&str; 3] = ["-NoProfile", "-NonInteractive", "-EncodedCommand"];

/// Anything else: nothing is known about access lists.
#[cfg(not(any(unix, windows)))]
fn file_access(_path: &Path, _open_bits: u32) -> Access {
    Access::Unknown
}

/// What the SDDL text says: open when an entry allows something to one of the accounts that stand
/// for many people, unknown when the text has no access list at all.
#[cfg(any(windows, test))]
fn sddl_access(sddl: &str) -> Access {
    bb_core::access_list::open_to_many(sddl).map_or(Access::Unknown, Access::of)
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
        // The exact command that keeps only the account that runs Bugaboo, for a file and for a
        // folder (whose rights the files made in it later take over).
        let backups = &found[1].1;
        if cfg!(windows) {
            // As Bugaboo restricts what it makes itself: every entry added by hand goes first.
            let path = folder.database_file().display().to_string();
            let line = format!(
                "icacls \"{path}\" /reset && icacls \"{path}\" /inheritance:r /grant:r \"*"
            );
            assert!(database.contains(&line), "{database}");
            // A file keeps reading for the administrators, a folder hands its rights on.
            assert!(
                database.ends_with(&format!(":F\" /grant:r \"*{ADMINISTRATORS_SID}:R\"")),
                "{database}"
            );
            assert!(backups.ends_with(":(OI)(CI)F\""), "{backups}");
        } else {
            let path = folder.database_file().display().to_string();
            assert!(
                database.ends_with(&format!("chmod go-rwx '{path}'")),
                "{database}"
            );
            assert!(backups.contains("chmod go-rwx '"), "{backups}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn sec_16_without_the_account_no_command_names_a_guessed_one() {
        let temp = tempfile::tempdir().unwrap();
        let said = windows_open_sentence("The data folder", temp.path(), None).render();
        assert!(said.contains("Security tab"), "{said}");
        assert!(!said.contains("icacls"), "{said}");
        assert!(!said.contains("%USERNAME%"), "{said}");
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
