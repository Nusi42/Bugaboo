//! The secret key file is its owner's alone (SEC-18, ADR-0047 decision 2): a key without a
//! passphrase is kept by its place, so no other account of the computer may read it (the system
//! and the administrators of the computer, who can take over any file, aside). `keygen`
//! makes the file that way before anything goes into it; `sign` refuses a file other accounts may
//! read, and says how to close it.
//!
//! On Linux that is the mode `0600`. On Windows it is an access list with only the owner, set and
//! read with the system tools `whoami` and `icacls`, in the forms `bb_core::access_list` reads
//! and writes for the server's data folder too.

use std::fs;
use std::path::Path;

use anyhow::{Context as _, bail};

/// Makes the new file `path` for its owner only and gives it back, open for writing. A file that
/// is there already is never written over.
pub(super) fn create(path: &Path) -> anyhow::Result<fs::File> {
    let file = create_new(path).with_context(|| format!("could not create {}", path.display()))?;
    #[cfg(windows)]
    if let Err(error) = keep_for_owner_only(path) {
        drop(file);
        // An empty file that could not be closed to others is no key: it goes again.
        return Err(match fs::remove_file(path) {
            Ok(()) => error,
            Err(removed) => error.context(format!(
                "and the empty file {} could not be removed: {removed}",
                path.display()
            )),
        });
    }
    Ok(file)
}

/// Linux: the file is made with the mode `0600`, so it is never open to others, not even for a
/// moment.
#[cfg(unix)]
fn create_new(path: &Path) -> std::io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt as _;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(OWNER_ONLY_MODE)
        .open(path)
}

#[cfg(not(unix))]
fn create_new(path: &Path) -> std::io::Result<fs::File> {
    fs::File::create_new(path)
}

/// The mode of a file only its owner may read and write.
#[cfg(unix)]
const OWNER_ONLY_MODE: u32 = 0o600;

/// Whether a mode lets the group or everybody do anything with the file.
#[cfg(any(unix, test))]
pub(super) fn open_mode(mode: u32) -> bool {
    mode & 0o077 != 0
}

/// Refuses the secret key at `path` when other accounts may read it, with what to run to close
/// it. A file whose rights cannot be read is refused too: a check that cannot look must not say
/// all is well.
pub(super) fn refuse_open_to_others(path: &Path) -> anyhow::Result<()> {
    match open_to_others(path) {
        Some(false) => Ok(()),
        Some(true) => bail!(
            "the secret key {} can be read by other accounts of this computer. A key without a \
             passphrase is kept by its place (ADR-0047, decision 2): close it to them with {} and \
             sign again.",
            path.display(),
            how_to_close(path)
        ),
        None => bail!(
            "could not read who may read the secret key {}, so it is not used. Close it to every \
             account but yours with {} and sign again.",
            path.display(),
            how_to_close(path)
        ),
    }
}

/// Linux: a group or world bit is set.
#[cfg(unix)]
fn open_to_others(path: &Path) -> Option<bool> {
    use std::os::unix::fs::PermissionsExt as _;
    fs::metadata(path)
        .ok()
        .map(|data| open_mode(data.permissions().mode()))
}

#[cfg(unix)]
fn how_to_close(path: &Path) -> String {
    format!("`chmod {OWNER_ONLY_MODE:o} '{}'`", path.display())
}

/// Windows: the access list gives something to an account that is not the owner, the system or
/// the administrators of the computer, as the mode `0600` on Linux.
#[cfg(windows)]
fn open_to_others(path: &Path) -> Option<bool> {
    use bb_core::access_list::{SYSTEM_AND_ADMINISTRATORS, allows_others, rights};
    let owner = current_sid().ok()?;
    let trusted: Vec<&str> = SYSTEM_AND_ADMINISTRATORS
        .iter()
        .copied()
        .chain([owner.as_str()])
        .collect();
    allows_others(&access_list(path)?, &trusted, rights::ALL)
}

/// The two `icacls` commands that keep only the owner, as `keygen` runs them.
#[cfg(windows)]
fn how_to_close(path: &Path) -> String {
    let owner = current_sid().unwrap_or_else(|_| "<your account>".to_owned());
    bb_core::access_list::restrict_arguments(&[bb_core::access_list::owner_only(&owner)])
        .iter()
        .map(|arguments| format!("`icacls \"{}\" {}`", path.display(), arguments.join(" ")))
        .collect::<Vec<_>>()
        .join(" and ")
}

#[cfg(not(any(unix, windows)))]
fn open_to_others(_path: &Path) -> Option<bool> {
    None
}

#[cfg(not(any(unix, windows)))]
fn how_to_close(_path: &Path) -> String {
    "the tools of the system".to_owned()
}

/// The program that names the account running this one.
#[cfg(windows)]
const WHOAMI: &str = "whoami.exe";
/// The program that reads and sets access lists.
#[cfg(windows)]
const ICACLS: &str = "icacls.exe";

/// The SID of the account that runs this program.
#[cfg(windows)]
pub(super) fn current_sid() -> anyhow::Result<String> {
    let output = std::process::Command::new(WHOAMI)
        .args(bb_core::access_list::WHOAMI_ARGUMENTS)
        .output()
        .with_context(|| format!("could not run {WHOAMI}"))?;
    bb_core::access_list::sid_in_whoami(&String::from_utf8_lossy(&output.stdout))
        .with_context(|| format!("{WHOAMI} named no account"))
}

/// Windows: removes every entry of the access list of `path` and what it inherits, and grants
/// only the account that runs this program.
#[cfg(windows)]
fn keep_for_owner_only(path: &Path) -> anyhow::Result<()> {
    let owner = bb_core::access_list::owner_only(&current_sid()?);
    for arguments in bb_core::access_list::restrict_arguments(&[owner]) {
        let output = std::process::Command::new(ICACLS)
            .arg(path)
            .args(&arguments)
            .output()
            .with_context(|| format!("could not run {ICACLS}"))?;
        if !output.status.success() {
            bail!(
                "could not close {} to other accounts: {}",
                path.display(),
                String::from_utf8_lossy(&output.stdout).trim()
            );
        }
    }
    Ok(())
}

/// The access list of `path` as SDDL, as `icacls /save` writes it into a file of its own.
#[cfg(windows)]
pub(super) fn access_list(path: &Path) -> Option<String> {
    let saved = std::env::temp_dir().join(format!("bugaboo-key-access-{}.txt", std::process::id()));
    let ran = std::process::Command::new(ICACLS)
        .arg(path)
        .arg("/save")
        .arg(&saved)
        .output()
        .ok();
    let bytes = fs::read(&saved).ok();
    if let Err(error) = fs::remove_file(&saved) {
        crate::output::note(&format!("could not remove {}: {error}", saved.display()));
    }
    ran.filter(|output| output.status.success())?;
    Some(bb_core::access_list::saved_text(&bytes?))
}
