//! Asking the operating system (coding-standards §2, "No fixed paths"): where its folders are,
//! which account runs the program, what an account is called, where a program is. Nothing here
//! writes down a path of the system; when the system does not answer, the caller gets a plain
//! message and stops, and never guesses.

#[cfg(unix)]
use std::ffi::OsStr;
#[cfg(unix)]
use std::path::Path;
use std::path::PathBuf;

#[cfg(any(windows, test))]
use bb_core::message::{self, Message};

/// The program `program` in the system folder of Windows (`icacls.exe`, `whoami.exe`), where
/// Windows itself says that folder is.
#[cfg(windows)]
pub(crate) fn system_tool(program: &str) -> Result<PathBuf, Message> {
    in_system_folder(
        known_folders::get_known_folder_path(known_folders::KnownFolder::System),
        program,
    )
}

/// The program in the folder the system named, or the message when it named none.
#[cfg(any(windows, test))]
fn in_system_folder(named: Option<PathBuf>, program: &str) -> Result<PathBuf, Message> {
    named
        .filter(|folder| folder.is_absolute())
        .map(|folder| folder.join(program))
        .ok_or_else(|| message::system_folder_unknown(program))
}

/// The number of the account that runs this program.
#[cfg(unix)]
pub(crate) fn account_number() -> u32 {
    nix::unistd::geteuid().as_raw()
}

/// The name of the account with this number, from the account database of the system.
#[cfg(target_os = "linux")]
pub(crate) fn account_name(number: u32) -> Option<String> {
    nix::unistd::User::from_uid(nix::unistd::Uid::from_raw(number))
        .ok()
        .flatten()
        .map(|account| account.name)
}

/// Whether the account database of the system has an account with this name.
#[cfg(unix)]
pub(crate) fn account_exists(name: &str) -> bool {
    nix::unistd::User::from_name(name).is_ok_and(|account| account.is_some())
}

/// The program `name` in the first absolute folder of the search path (`PATH`) that holds a file
/// of that name which can run, or `None` when there is none.
#[cfg(unix)]
pub(crate) fn program_on_path(name: &str) -> Option<PathBuf> {
    program_in(&std::env::var_os("PATH")?, name)
}

#[cfg(unix)]
fn program_in(search_path: &OsStr, name: &str) -> Option<PathBuf> {
    bb_core::system_answer::find_in(search_path, name, is_runnable)
}

/// Whether `path` is a file that somebody may run (an execute bit is set).
#[cfg(unix)]
fn is_runnable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::metadata(path)
        .is_ok_and(|data| data.is_file() && data.permissions().mode() & 0o111 != 0)
}

/// What a program printed when it ended well, or `None` when it could not be started or failed.
#[cfg(unix)]
pub(crate) fn answer_of(program: &str, arguments: &[&str]) -> Option<String> {
    let output = std::process::Command::new(program)
        .args(arguments)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ops_04_a_system_folder_that_windows_names_is_used_for_the_program() {
        let folder = std::env::temp_dir();
        assert_eq!(
            in_system_folder(Some(folder.clone()), "icacls.exe").unwrap(),
            folder.join("icacls.exe")
        );
    }

    #[test]
    fn ops_04_a_system_folder_that_windows_does_not_name_stops_with_a_plain_message() {
        let said = in_system_folder(None, "icacls.exe").unwrap_err().render();
        assert!(said.contains("does not guess"), "{said}");
        assert!(said.contains("icacls.exe"), "{said}");
        // An answer that is no full path is no answer.
        assert!(in_system_folder(Some(PathBuf::from("System32")), "icacls.exe").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn ops_04_only_a_file_that_can_run_in_an_absolute_folder_is_a_program() {
        use std::os::unix::fs::PermissionsExt as _;
        let temp = tempfile::tempdir().unwrap();
        let folder = temp.path().join("bin");
        std::fs::create_dir(&folder).unwrap();
        let tool = folder.join("tool");
        std::fs::write(&tool, "").unwrap();
        let search =
            std::env::join_paths([OsStr::new(""), OsStr::new("."), folder.as_os_str()]).unwrap();
        std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(program_in(&search, "tool"), None, "no execute bit");
        std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(program_in(&search, "tool"), Some(tool));
        // An empty entry first, as in ":/x", is the folder the program runs in and is skipped.
        assert_eq!(program_in(OsStr::new(":/nowhere"), "tool"), None);
    }

    #[cfg(unix)]
    #[test]
    fn ops_04_the_account_of_the_program_is_the_owner_of_what_it_makes() {
        use std::os::unix::fs::MetadataExt as _;
        let temp = tempfile::tempdir().unwrap();
        assert_eq!(
            std::fs::metadata(temp.path()).unwrap().uid(),
            account_number()
        );
        #[cfg(target_os = "linux")]
        {
            assert!(account_name(account_number()).is_some());
            assert_eq!(account_name(0).as_deref(), Some("root"));
            assert!(account_exists("root"));
        }
        assert!(!account_exists("bb-no-such-account-x"));
    }
}
