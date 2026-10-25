//! Asking the operating system (coding-standards §2, "No fixed paths"): where its folders are,
//! which account runs the program, what an account is called, where a program is. Nothing here
//! writes down a path of the system; when the system does not answer, the caller gets a plain
//! message and stops, and never guesses.

#[cfg(any(unix, test))]
use std::ffi::OsStr;
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
#[cfg(unix)]
pub(crate) fn account_name(number: u32) -> Option<String> {
    nix::unistd::User::from_uid(nix::unistd::Uid::from_raw(number))
        .ok()
        .flatten()
        .map(|account| account.name)
}

/// The program `name` in the first folder of the search path (`PATH`) that holds it, or `None`
/// when the search path is not set or holds no such file.
#[cfg(unix)]
pub(crate) fn program_on_path(name: &str) -> Option<PathBuf> {
    find_in(&std::env::var_os("PATH")?, name)
}

#[cfg(any(unix, test))]
fn find_in(search_path: &OsStr, name: &str) -> Option<PathBuf> {
    std::env::split_paths(search_path)
        .map(|folder| folder.join(name))
        .find(|candidate| candidate.is_file())
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

    #[test]
    fn ops_04_a_program_is_found_in_the_first_folder_of_the_search_path_that_holds_it() {
        let temp = tempfile::tempdir().unwrap();
        let (first, second) = (temp.path().join("a"), temp.path().join("b"));
        for folder in [&first, &second] {
            std::fs::create_dir(folder).unwrap();
        }
        std::fs::write(second.join("tool"), "").unwrap();
        let search = std::env::join_paths([&first, &second]).unwrap();
        assert_eq!(find_in(&search, "tool"), Some(second.join("tool")));
        assert_eq!(find_in(&search, "other"), None);
        assert_eq!(find_in(OsStr::new(""), "tool"), None);
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
        assert!(account_name(account_number()).is_some());
    }
}
