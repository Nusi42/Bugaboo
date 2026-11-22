//! Who besides the system may change the program of the server and its settings file, and who
//! may read the settings file and the private key of an HTTPS certificate of the operator's own
//! (SEC-16).
//!
//! Whoever can change the program, a library beside it, a folder above it or the settings file
//! runs code as the account of the server, a service included. `bugaboo service install` refuses
//! such a place; the start and the status page report it, and change nothing.
//!
//! *Can change* means: on Linux, the owner is not root (or the account that runs Bugaboo) or the
//! group or everybody may write; on Windows, the access list allows a right that writes, deletes
//! or takes over to an account that is not the system, the administrators of the computer or the
//! installer of Windows (or the account that runs Bugaboo). What cannot be read is unknown, never
//! closed.

use std::path::{Path, PathBuf};

use bb_app::status::{Access, HostRights};
use bb_core::message::{self, Message, MessageKey};

use crate::system::{lies_in, real_path};

/// What a path is to the file that is judged: the file itself, the folder that holds it (where a
/// library beside a program is loaded from), a folder further up (where only replacing a name
/// counts), or the root of the drive (which cannot itself be deleted or renamed).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Part {
    File,
    Folder,
    Above,
    Root,
}

/// Whose rights are fine besides those of the system and the administrators.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Besides {
    /// Nobody: what a service will run as its own account.
    Nobody,
    /// The account that runs this program, which may change its own files.
    ThisAccount,
}

/// What looking at a chain of paths found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// Nobody else can change any of them.
    Closed,
    /// Somebody else can change this one.
    Open(PathBuf),
    /// The rights of this one could not be read.
    Unknown(PathBuf),
}

impl Verdict {
    fn access(&self) -> Access {
        match self {
            Self::Closed => Access::Private,
            Self::Open(_) => Access::Open,
            Self::Unknown(_) => Access::Unknown,
        }
    }

    /// The one that needs more attention, as [`Access::worse`].
    fn worse(self, other: Self) -> Self {
        match (&self, &other) {
            (Self::Open(_), _) => self,
            (_, Self::Open(_)) => other,
            (Self::Unknown(_), _) => self,
            (_, Self::Unknown(_)) => other,
            _ => Self::Closed,
        }
    }
}

/// The paths a program relies on: itself, its folder, and every folder above it.
pub(crate) fn program_chain(program: &Path) -> Vec<(PathBuf, Part)> {
    let program = real_path(program);
    let mut chain = vec![(program.clone(), Part::File)];
    let mut folders = program.ancestors().skip(1);
    if let Some(folder) = folders.next() {
        chain.push((folder.to_path_buf(), Part::Folder));
    }
    chain.extend(folders.map(|folder| {
        let part = if folder.parent().is_none() {
            Part::Root
        } else {
            Part::Above
        };
        (folder.to_path_buf(), part)
    }));
    chain
}

/// The paths a settings file relies on: itself and its folder.
pub(crate) fn settings_chain(config: &Path) -> Vec<(PathBuf, Part)> {
    let mut chain = vec![(real_path(config), Part::File)];
    chain.extend(settings_folder(config));
    chain
}

/// The folder of a settings file, whose file the install gives its own access list.
pub(crate) fn settings_folder(config: &Path) -> Option<(PathBuf, Part)> {
    real_path(config)
        .parent()
        .map(|folder| (folder.to_path_buf(), Part::Folder))
}

/// The first path of `chain` that somebody else can change, by the rights of the system. When
/// the account that runs this program cannot be named, only the system and the administrators are
/// trusted: the answer errs on the side of *open*. Every path of the chain is a real path (links
/// and, on Windows, junctions followed), so the parents of what is really run are judged.
pub(crate) fn first_changeable(chain: &[(PathBuf, Part)], besides: Besides) -> Verdict {
    let also = also_trusted(besides);
    let owners = owners_of(chain);
    first_changeable_by(chain, &|path, part| {
        changeable(path, part, also.as_ref(), &owners)
    })
}

/// The same, with the way of looking at one path given (the tests).
fn first_changeable_by(
    chain: &[(PathBuf, Part)],
    look: &dyn Fn(&Path, Part) -> Option<bool>,
) -> Verdict {
    let mut verdict = Verdict::Closed;
    for (path, part) in chain {
        match look(path, *part) {
            Some(false) => {}
            Some(true) => return Verdict::Open(path.clone()),
            None => verdict = verdict.worse(Verdict::Unknown(path.clone())),
        }
    }
    verdict
}

/// Whether the program (or a settings file) lies in the data folder, which the account of a
/// service may change. Both are compared by their real paths, on Windows without regard to case.
pub(crate) fn in_data_folder(program: &Path, data_dir: &Path) -> bool {
    lies_in(&real_path(program), &real_path(data_dir))
}

/// Why a path is not trusted on Linux.
#[cfg(any(unix, test))]
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Untrusted {
    /// The group or everybody may write to it, so they could replace what root relies on.
    OthersCanWrite,
    /// It belongs to somebody but root (or the account that may own it), who could replace it.
    NotRoots,
    /// It cannot be looked at, so nothing is known about who may change it.
    #[cfg(unix)]
    Unreadable,
}

/// Whether a path with this mode and owner (user number) is safe to rely on: root, or `also`,
/// owns it and neither the group nor everybody may write to it.
#[cfg(any(unix, test))]
pub(crate) fn trust_of(mode: u32, owner: u32, also: Option<u32>) -> Option<Untrusted> {
    if mode & 0o022 != 0 {
        Some(Untrusted::OthersCanWrite)
    } else if owner != 0 && Some(owner) != also {
        Some(Untrusted::NotRoots)
    } else {
        None
    }
}

/// Linux: the reason the path is not trusted as root's, if it is not. A path that cannot be
/// looked at is not trusted either.
#[cfg(unix)]
pub(crate) fn untrusted(path: &Path, also: Option<u32>) -> Option<Untrusted> {
    use std::os::unix::fs::MetadataExt as _;
    match std::fs::metadata(path) {
        Ok(data) => trust_of(data.mode(), data.uid(), also),
        Err(_) => Some(Untrusted::Unreadable),
    }
}

#[cfg(all(test, not(unix)))]
pub(crate) fn untrusted(_path: &Path, _also: Option<u32>) -> Option<Untrusted> {
    None
}

/// The account whose rights are fine besides those of the system: Linux names it by its number,
/// Windows by its SID. `None` for nobody, and when it cannot be named.
#[cfg(unix)]
type Also = Option<u32>;
#[cfg(not(unix))]
type Also = Option<String>;

fn also_trusted(besides: Besides) -> Also {
    match besides {
        Besides::Nobody => None,
        #[cfg(unix)]
        Besides::ThisAccount => Some(crate::system::account_number()),
        #[cfg(windows)]
        Besides::ThisAccount => crate::data_folder::current_sid().ok(),
        #[cfg(not(any(unix, windows)))]
        Besides::ThisAccount => None,
    }
}

/// Who owns each path of a chain, where the owner is not in what the rights of a path already say:
/// on Windows, whose owner may always change the access list of what it owns. On Linux the owner
/// is read with the mode ([`untrusted`]), so nothing more is held.
struct Owners {
    /// The SID of the owner of each path whose owner could be read.
    #[cfg(windows)]
    of: std::collections::HashMap<PathBuf, String>,
}

/// The owners of the paths of `chain`, read once for the whole chain (one run of the system tool).
/// A path whose owner cannot be read is missing, so it is unknown.
#[cfg(windows)]
fn owners_of(chain: &[(PathBuf, Part)]) -> Owners {
    let paths: Vec<PathBuf> = chain.iter().map(|(path, _)| path.clone()).collect();
    let owners = crate::data_folder::rights::owners(&paths);
    Owners {
        of: paths
            .into_iter()
            .zip(owners)
            .filter_map(|(path, owner)| owner.map(|owner| (path, owner)))
            .collect(),
    }
}

#[cfg(not(windows))]
const fn owners_of(_chain: &[(PathBuf, Part)]) -> Owners {
    Owners {}
}

/// Linux: whether somebody else can change `path`; the kind of path does not matter, the owner
/// and the bits of writing say it all.
#[cfg(unix)]
fn changeable(path: &Path, _part: Part, also: Option<&u32>, _owners: &Owners) -> Option<bool> {
    match untrusted(path, also.copied()) {
        None => Some(false),
        Some(Untrusted::Unreadable) => None,
        Some(_) => Some(true),
    }
}

/// Windows: whether the access list allows a right that changes `path` as `part` to an account
/// that is not trusted, or an owner that is not trusted may change the access list itself.
#[cfg(windows)]
fn changeable(path: &Path, part: Part, also: Option<&String>, owners: &Owners) -> Option<bool> {
    use bb_core::access_list::{SYSTEM_ACCOUNTS, changeable_by_others, rights};
    let trusted: Vec<&str> = SYSTEM_ACCOUNTS
        .iter()
        .copied()
        .chain(also.map(String::as_str))
        .collect();
    let wanted = match part {
        Part::File => rights::CHANGE_FILE,
        Part::Folder => rights::CHANGE_FOLDER,
        Part::Above => rights::REPLACE_BELOW,
        Part::Root => rights::REPLACE_IN_ROOT,
    };
    changeable_by_others(
        &crate::data_folder::rights::access_list(path)?,
        owners.of.get(path)?,
        &trusted,
        wanted,
    )
}

#[cfg(not(any(unix, windows)))]
fn changeable(_path: &Path, _part: Part, _also: Option<&String>, _owners: &Owners) -> Option<bool> {
    None
}

/// What accounts that stand for many people may do with a file that is to be read only by the
/// server, as a verdict on `path`: the same reading as for the data folder, with the mode bits
/// that say too much on Linux.
fn readable_by_many(path: &Path, open_bits: u32) -> Verdict {
    match crate::data_folder::rights::access_with(path, open_bits) {
        Access::Private => Verdict::Closed,
        Access::Open => Verdict::Open(path.to_path_buf()),
        Access::Unknown => Verdict::Unknown(path.to_path_buf()),
    }
}

/// The mode bits of a settings file that say too much: everybody may read it, or the group may
/// change it (the group of a service may read it).
const SETTINGS_OPEN_BITS: u32 = 0o026;
/// The mode bits of a private key that say too much: anybody but its owner may do anything.
const KEY_OPEN_BITS: u32 = 0o077;

/// The files of a running server whose rights are looked at.
pub(crate) struct HostFiles<'a> {
    pub(crate) program: &'a Path,
    pub(crate) config: Option<&'a Path>,
    pub(crate) data_dir: &'a Path,
    pub(crate) tls_key: Option<&'a Path>,
}

/// What looking at the files of a running server found, with the path to name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HostLook {
    program: Verdict,
    settings_file: Verdict,
    tls_key: Verdict,
}

impl HostLook {
    /// The rights, as the status page shows them.
    pub(crate) fn rights(&self) -> HostRights {
        HostRights {
            program: self.program.access(),
            settings_file: self.settings_file.access(),
            tls_key: self.tls_key.access(),
            preview_child: None,
        }
    }

    /// One sentence for each part that is too open or cannot be checked, with a short name of the
    /// part for the log.
    pub(crate) fn open_parts(&self) -> Vec<(&'static str, String)> {
        let parts: [(&Verdict, &'static str, MessageKey); 3] = [
            (&self.program, "program", message::HOST_PART_PROGRAM),
            (
                &self.settings_file,
                "settings file",
                message::HOST_PART_SETTINGS_FILE,
            ),
            (&self.tls_key, "tls key", message::HOST_PART_TLS_KEY),
        ];
        parts
            .into_iter()
            .filter_map(|(verdict, name, part)| {
                let part = Message::from(part).render();
                let sentence = match verdict {
                    Verdict::Closed => return None,
                    Verdict::Open(path) => message::host_open(message::HostOpen {
                        part: &part,
                        path: &path.display(),
                    }),
                    Verdict::Unknown(path) => message::host_unknown(message::HostUnknown {
                        part: &part,
                        path: &path.display(),
                    }),
                };
                Some((name, sentence.render()))
            })
            .collect()
    }
}

/// Looks at the files of a running server. Looking changes nothing.
pub(crate) fn look(files: &HostFiles<'_>) -> HostLook {
    let program = if in_data_folder(files.program, files.data_dir) {
        Verdict::Open(files.program.to_path_buf())
    } else {
        first_changeable(&program_chain(files.program), Besides::ThisAccount)
    };
    let settings_file = files.config.map_or(Verdict::Closed, |config| {
        readable_by_many(config, SETTINGS_OPEN_BITS).worse(first_changeable(
            &settings_chain(config),
            Besides::ThisAccount,
        ))
    });
    let tls_key = files
        .tls_key
        .map_or(Verdict::Closed, |key| readable_by_many(key, KEY_OPEN_BITS));
    HostLook {
        program,
        settings_file,
        tls_key,
    }
}

#[cfg(test)]
mod tests;
