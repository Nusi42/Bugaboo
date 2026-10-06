//! The kinds of text a setting can hold beyond free text (ADR-0018): one choice out of a fixed
//! list, a time zone, a page of the app and the keys of the keyboard shortcuts. A format is part of
//! the definition in the registry, so the browser asks the registry what to show and the server
//! asks it what to accept, and nobody keeps a second list.

use serde_json::Value;

use crate::message::MessageKey;

/// One value a setting with a fixed list can have, and the words that name it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Choice {
    value: &'static str,
    label: MessageKey,
}

impl Choice {
    /// The choice stored as `value` and shown as the message `label`.
    #[must_use]
    pub const fn new(value: &'static str, label: MessageKey) -> Self {
        Self { value, label }
    }

    /// The text that is stored.
    #[must_use]
    pub const fn value(&self) -> &'static str {
        self.value
    }

    /// The message that names the choice for people.
    #[must_use]
    pub const fn label(&self) -> MessageKey {
        self.label
    }
}

/// The value that means "whatever my device says": the time zone, the formats and the first day
/// of the week of the browser.
pub const DEVICE: &str = "device";

/// The longest address of a page that can be a start page, in characters.
const PAGE_ADDRESS_LONGEST: usize = 200;
/// The most shortcuts whose keys can be changed, and the most keys one shortcut can have.
const SHORTCUTS_MOST: usize = 64;
const SHORTCUT_KEYS_MOST: usize = 4;
/// The longest ID of a shortcut and the longest text of one key sequence, in characters.
const SHORTCUT_ID_LONGEST: usize = 40;
const SHORTCUT_SEQUENCE_LONGEST: usize = 40;
/// The longest time zone name and the longest part of one, in characters.
const ZONE_LONGEST: usize = 64;
const ZONE_PART_LONGEST: usize = 32;
/// The most parts a time zone name has (`America/Argentina/Buenos_Aires`).
const ZONE_PARTS_MOST: usize = 3;

/// What kind of text a setting holds. Anything but [`Format::Free`] is checked when the value is
/// stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// Any text (within the limits of the setting).
    Free,
    /// One of a fixed list.
    Choices(&'static [Choice]),
    /// `device`, or a time zone of the IANA list (`Europe/Berlin`, `UTC`).
    TimeZone,
    /// The address of a page of the app: starts with one slash and never names another website: no
    /// `//` anywhere, no `..` segment, no encoded slash, backslash or dot.
    PageAddress,
    /// The keys a person chose for shortcuts: a JSON object from the ID of a shortcut to the key
    /// sequences that do it. The browser knows which IDs exist; the server only keeps the shape
    /// and the size within limits.
    ShortcutKeys,
}

impl Format {
    /// Whether `text` is a value of this format.
    #[must_use]
    pub fn accepts(self, text: &str) -> bool {
        match self {
            Self::Free => true,
            Self::Choices(choices) => choices.iter().any(|choice| choice.value == text),
            Self::TimeZone => is_time_zone(text),
            Self::PageAddress => is_page_address(text),
            Self::ShortcutKeys => are_shortcut_keys(text),
        }
    }

    /// The choices, for the format that has a list.
    #[must_use]
    pub const fn choices(self) -> &'static [Choice] {
        match self {
            Self::Choices(choices) => choices,
            Self::Free | Self::TimeZone | Self::PageAddress | Self::ShortcutKeys => &[],
        }
    }
}

fn is_time_zone(text: &str) -> bool {
    if text == DEVICE || text == "UTC" {
        return true;
    }
    let parts: Vec<&str> = text.split('/').collect();
    text.len() <= ZONE_LONGEST
        && (2..=ZONE_PARTS_MOST).contains(&parts.len())
        && parts.iter().all(|part| {
            part.starts_with(|c: char| c.is_ascii_alphabetic())
                && part.len() <= ZONE_PART_LONGEST
                && part
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '+'))
        })
}

/// Percent-encoded characters that change what an address means once something decodes it: a
/// slash, a backslash and a dot (`%2e%2e` is `..`).
const ENCODED_SEPARATORS: [&str; 3] = ["%2f", "%5c", "%2e"];

fn is_page_address(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let path = text.split(['?', '#']).next().unwrap_or_default();
    text.starts_with('/')
        && !text.contains("//")
        && !path.split('/').any(|segment| segment == "..")
        && !ENCODED_SEPARATORS
            .iter()
            .any(|encoded| lower.contains(encoded))
        && text.len() <= PAGE_ADDRESS_LONGEST
        && text.chars().all(|c| {
            c.is_ascii_alphanumeric()
                || matches!(c, '/' | '_' | '-' | '.' | '~' | '%' | '?' | '=' | '&' | '#')
        })
}

fn is_shortcut_id(text: &str) -> bool {
    text.starts_with(|c: char| c.is_ascii_lowercase())
        && text.len() <= SHORTCUT_ID_LONGEST
        && text
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn is_key_sequence(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= SHORTCUT_SEQUENCE_LONGEST
        && text.chars().all(|c| c.is_ascii_graphic() || c == ' ')
}

fn are_shortcut_keys(text: &str) -> bool {
    let Ok(Value::Object(shortcuts)) = serde_json::from_str::<Value>(text) else {
        return false;
    };
    shortcuts.len() <= SHORTCUTS_MOST
        && shortcuts.iter().all(|(id, keys)| {
            is_shortcut_id(id)
                && keys.as_array().is_some_and(|keys| {
                    keys.len() <= SHORTCUT_KEYS_MOST
                        && keys
                            .iter()
                            .all(|key| key.as_str().is_some_and(is_key_sequence))
                })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message;

    const CHOICES: &[Choice] = &[
        Choice::new("light", message::THEME_DAYLIGHT),
        Choice::new("dark", message::THEME_DUSK),
    ];

    #[test]
    fn cfg_04_a_choice_must_be_on_the_list() {
        let format = Format::Choices(CHOICES);
        assert!(format.accepts("light"));
        assert!(format.accepts("dark"));
        assert!(!format.accepts("Light"));
        assert!(!format.accepts(""));
        assert_eq!(format.choices().len(), 2);
        assert!(Format::Free.choices().is_empty());
        assert_eq!(CHOICES[0].value(), "light");
        assert_eq!(CHOICES[0].label(), message::THEME_DAYLIGHT);
    }

    #[test]
    fn cfg_04_a_time_zone_is_device_utc_or_a_name_of_the_iana_list() {
        for good in [
            "device",
            "UTC",
            "Europe/Berlin",
            "America/Argentina/Buenos_Aires",
            "Etc/GMT+5",
            "America/Port-au-Prince",
        ] {
            assert!(Format::TimeZone.accepts(good), "{good}");
        }
        for bad in [
            "",
            "Berlin",
            "/Europe/Berlin",
            "Europe/",
            "Europe//Berlin",
            "Europe/Berlin Mitte",
            "../etc/passwd",
            "A/B/C/D",
            "1Europe/Berlin",
            &format!("Europe/{}", "x".repeat(40)),
        ] {
            assert!(!Format::TimeZone.accepts(bad), "{bad}");
        }
    }

    #[test]
    fn cfg_04_a_start_page_is_a_page_of_the_app_and_never_another_website() {
        for good in [
            "/",
            "/time",
            "/p/WEB/issues?show=board&s=1",
            "/me/appearance",
            "/p/WEB/issues.2024",
            "/a?q=..",
        ] {
            assert!(Format::PageAddress.accepts(good), "{good}");
        }
        for bad in [
            "",
            "time",
            "//evil.test",
            "https://evil.test/",
            "/\\evil.test",
            "/a b",
            "/a\nb",
            "/javascript:alert(1)",
            "/a//evil.test",
            "/a?next=//evil.test",
            "/p/../admin",
            "/..",
            "/a/..?x=1",
            "/a%2Fb",
            "/a%5cb",
            "/%2e%2e/admin",
            &format!("/{}", "a".repeat(200)),
        ] {
            assert!(!Format::PageAddress.accepts(bad), "{bad}");
        }
    }

    #[test]
    fn cfg_04_shortcut_keys_are_one_object_of_lists_within_limits() {
        let accepts = |text: &str| Format::ShortcutKeys.accepts(text);
        assert!(accepts("{}"));
        assert!(accepts(r#"{"go-home":["g h","Mod+h"],"undo":[]}"#));
        for bad in [
            "",
            "[]",
            "null",
            r#"{"Go Home":["g h"]}"#,
            r#"{"go-home":"g h"}"#,
            r#"{"go-home":[1]}"#,
            r#"{"go-home":[""]}"#,
            r#"{"go-home":["a","b","c","d","e"]}"#,
            r#"{"go-home":["tab\there"]}"#,
        ] {
            assert!(!accepts(bad), "{bad}");
        }
        let many: Vec<String> = (0..=SHORTCUTS_MOST)
            .map(|n| format!("\"s{n}\":[\"x\"]"))
            .collect();
        assert!(!accepts(&format!("{{{}}}", many.join(","))));
    }
}
