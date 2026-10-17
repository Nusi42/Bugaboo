//! Handles (M3-F70, M4-Q9, ADR-0033 §6): a person's short unique name for mentions, `@ana.brandt`.
//!
//! The handle is made from the name when the account is made, is unique, and can be changed in
//! *My settings*. A mention stores the person and not the handle, so a changed handle never breaks
//! an old comment: the handle a person had before stays with them, so the `@name` in an old text
//! still names them, and nobody else can take it. A handle is people information and follows *Who
//! sees people*.

use jiff::Timestamp;

use crate::comment::{HANDLE_MAX, handle_of};
use crate::id::UserId;
use crate::personal::Entity;

/// The handle of a person whose name has no letter or digit to make one from.
pub const FALLBACK: &str = "person";

/// How many handles are tried for one name (`ana.brandt`, `ana.brandt.2`, …) before giving up,
/// which would mean something other than two people sharing a name.
pub const CANDIDATES_MAX: u32 = 1000;

/// The handle a name stands for, before it is made unique.
#[must_use]
pub fn base_of(name: &str) -> String {
    handle_of(name).unwrap_or_else(|| FALLBACK.to_owned())
}

/// The `n`-th handle to try for a base: the base itself, then `base.2`, `base.3` …, cut so that it
/// is never longer than a handle may be.
#[must_use]
pub fn candidate(base: &str, n: u32) -> String {
    if n <= 1 {
        return base.to_owned();
    }
    let suffix = format!(".{n}");
    let room = HANDLE_MAX.saturating_sub(suffix.chars().count());
    let cut: String = base.chars().take(room).collect();
    format!("{}{suffix}", cut.trim_end_matches(['.', '-']))
}

/// Whether the text is a handle as it is written: it is its own normal form, so it has no capital,
/// no space, no leading or trailing dot or dash, and is not too long.
#[must_use]
pub fn is_handle(text: &str) -> bool {
    handle_of(text).as_deref() == Some(text)
}

/// A handle of a person (ADR-0033 §6): the one they have now, or one they had before.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct PersonHandle {
    /// The person.
    #[personal(category = "identity", basis = "contract")]
    pub person: UserId,
    /// The handle, unique among all handles, former ones included. Made from the name, so it can hold the name.
    #[personal(category = "identity", basis = "contract")]
    pub handle: String,
    /// When it was made or last taken up.
    #[not_personal]
    pub changed_at: Timestamp,
    /// Whether it is the handle the person has now. The others are former handles, which still
    /// name the person in old texts and stay out of everybody else's reach.
    #[not_personal]
    pub current: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_handle_is_made_from_the_name_and_a_name_without_letters_gets_the_fallback() {
        assert_eq!(base_of("Ana Brandt"), "ana.brandt");
        assert_eq!(base_of("---"), FALLBACK);
    }

    #[test]
    fn two_people_with_one_name_get_numbered_handles_that_are_still_handles() {
        let base = base_of("Ana Brandt");
        assert_eq!(candidate(&base, 1), "ana.brandt");
        assert_eq!(candidate(&base, 2), "ana.brandt.2");
        assert!(is_handle(&candidate(&base, 12)));
        let long = base_of(&"ab ".repeat(100));
        let numbered = candidate(&long, 10);
        assert!(numbered.chars().count() <= HANDLE_MAX);
        assert!(numbered.ends_with(".10"));
        assert!(is_handle(&numbered));
    }

    #[test]
    fn only_a_handle_in_its_normal_form_is_a_handle() {
        for good in ["ana", "ana.brandt", "a-b", "ana.2"] {
            assert!(is_handle(good), "{good}");
        }
        for bad in [
            "",
            "Ana",
            "ana brandt",
            ".ana",
            "ana.",
            "-ana",
            "an@",
            "ana..b",
        ] {
            assert!(!is_handle(bad), "{bad}");
        }
    }
}
