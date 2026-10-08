//! The rule for the short texts people type: names of people, groups, roles, projects and access
//! keys, and the labels of passkeys. Defined once, so every use case that takes such a text keeps
//! it the same way.

/// The longest name of a person, group, role, project or access key, in characters.
pub const NAME_MAX: usize = 100;

/// The longest label of a passkey, in characters.
pub const LABEL_MAX: usize = 64;

/// The text as it is kept: trimmed, not empty, at most `longest` characters and without control
/// characters. `None` for a text that breaks one of these.
#[must_use]
pub fn clean(text: &str, longest: usize) -> Option<&str> {
    let text = text.trim();
    let fine =
        !text.is_empty() && text.chars().count() <= longest && !text.chars().any(char::is_control);
    fine.then_some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_text_is_trimmed_and_kept_when_it_is_plain_and_not_too_long() {
        assert_eq!(clean("  Ada Lovelace ", NAME_MAX), Some("Ada Lovelace"));
        assert_eq!(
            clean(&"é".repeat(NAME_MAX), NAME_MAX).map(str::len),
            Some(2 * NAME_MAX)
        );
    }

    #[test]
    fn an_empty_long_or_control_text_is_refused() {
        assert_eq!(clean("", NAME_MAX), None);
        assert_eq!(clean("   ", NAME_MAX), None);
        assert_eq!(clean(&"a".repeat(NAME_MAX + 1), NAME_MAX), None);
        assert_eq!(clean("two\nlines", NAME_MAX), None);
        assert_eq!(clean("tab\tinside", LABEL_MAX), None);
        assert_eq!(clean(&"a".repeat(LABEL_MAX + 1), LABEL_MAX), None);
    }
}
