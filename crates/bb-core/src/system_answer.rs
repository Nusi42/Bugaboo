//! Reading what the operating system answers (coding-standards §2, "No fixed paths"): the text a
//! program printed, and the search path. The two rules live here once; the caller does the
//! looking at files (this crate does no I/O) and passes the question in.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// The folder a program named: exactly one absolute path in the text it printed (blank lines and
/// spaces around it do not count). Whether the folder exists is for the caller to ask.
#[must_use]
pub fn folder_in_answer(answer: &str) -> Option<PathBuf> {
    let mut lines = answer
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty());
    let folder = PathBuf::from(lines.next()?);
    (lines.next().is_none() && folder.is_absolute()).then_some(folder)
}

/// The file `name` in the first folder of the search path where `is_wanted` accepts it (the
/// caller asks the file system: a file that can run, for a program). A folder that is not
/// absolute (an empty entry, `.`) is skipped, so the answer never depends on the folder the
/// program happens to run in.
pub fn find_in(
    search_path: &OsStr,
    name: &str,
    is_wanted: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    std::env::split_paths(search_path)
        .filter(|folder| folder.is_absolute())
        .map(|folder| folder.join(name))
        .find(|candidate| is_wanted(candidate))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn absolute(name: &str) -> PathBuf {
        let root = if cfg!(windows) { "C:\\" } else { "/" };
        Path::new(root).join(name)
    }

    #[test]
    fn one_absolute_folder_in_an_answer_is_the_answer() {
        let folder = absolute("units");
        let said = format!("  {}\n\n", folder.display());
        assert_eq!(folder_in_answer(&said), Some(folder));
    }

    #[test]
    fn an_answer_that_is_empty_relative_or_several_lines_is_no_answer() {
        assert_eq!(folder_in_answer(""), None);
        assert_eq!(folder_in_answer("units"), None);
        let two = format!("{0}\n{0}\n", absolute("units").display());
        assert_eq!(folder_in_answer(&two), None);
    }

    #[test]
    fn a_program_is_found_in_the_first_folder_that_has_it() {
        let (first, second) = (absolute("a"), absolute("b"));
        let search = std::env::join_paths([&first, &second]).unwrap();
        let wanted = second.join("tool");
        assert_eq!(
            find_in(&search, "tool", |candidate| candidate == wanted),
            Some(wanted.clone())
        );
        assert_eq!(find_in(&search, "other", |_| false), None);
    }

    #[test]
    fn an_entry_that_is_empty_or_relative_is_never_searched() {
        let real = absolute("x");
        let search = std::env::join_paths([
            OsStr::new(""),
            OsStr::new("."),
            OsStr::new("bin"),
            real.as_os_str(),
        ])
        .unwrap();
        // Every candidate is accepted: the first one asked is the first absolute folder.
        assert_eq!(
            find_in(&search, "nologin", |_| true),
            Some(real.join("nologin"))
        );
        assert_eq!(
            find_in(OsStr::new(""), "nologin", |_| true),
            None,
            "an empty search path has no folder"
        );
    }
}
