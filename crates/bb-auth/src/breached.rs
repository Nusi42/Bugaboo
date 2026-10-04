//! The bundled list of breached passwords (AUTH-01). No network call, ever.
//!
//! `data/breached-passwords.txt` holds passwords of 12 characters or more (shorter ones are
//! refused by the length rule anyway) that appear in public password leaks, in lower case, one
//! per line, sorted bytewise, so a lookup is a binary search. Source and licence are in the third
//! party register (`docs/third-party/README.md`, "Data"). A password is looked up in lower case,
//! so changing the capitals of a leaked password does not help.

use std::sync::OnceLock;

/// The list, built into the program.
const LIST: &str = include_str!("../data/breached-passwords.txt");

fn lines() -> &'static [&'static str] {
    static LINES: OnceLock<Vec<&'static str>> = OnceLock::new();
    LINES.get_or_init(|| LIST.lines().collect())
}

/// Whether `password` is on the list.
pub(crate) fn is_breached(password: &str) -> bool {
    let lower = password.to_lowercase();
    lines().binary_search(&lower.as_str()).is_ok()
}

/// How many passwords the list holds.
#[cfg(test)]
pub(crate) fn len() -> usize {
    lines().len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_01_the_list_is_sorted_so_the_binary_search_finds_every_entry() {
        assert!(len() > 50_000);
        assert!(lines().windows(2).all(|pair| pair[0] < pair[1]));
        for line in lines().iter().step_by(997) {
            assert!(is_breached(line), "{line}");
        }
    }

    #[test]
    fn auth_01_known_leaked_passwords_are_found_whatever_the_capitals() {
        for leaked in ["123456789012", "QWERTYUIOPASDF", "1qaz2wsx3edc"] {
            assert!(is_breached(leaked), "{leaked}");
        }
        assert!(!is_breached("correct horse battery staple"));
        assert!(!is_breached(""));
    }

    #[test]
    fn auth_01_the_list_has_no_short_entries_or_blank_lines() {
        assert!(lines().iter().all(|line| line.chars().count() >= 12));
    }
}
