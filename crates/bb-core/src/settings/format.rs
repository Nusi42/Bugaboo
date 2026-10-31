//! The kinds of text a setting can hold beyond free text (ADR-0018): one choice out of a fixed
//! list, a time zone, a page of the app and the keys of the keyboard shortcuts. A format is part of
//! the definition in the registry, so the browser asks the registry what to show and the server
//! asks it what to accept, and nobody keeps a second list.

use serde_json::Value;

use crate::error_code::ErrorCode;
use crate::message::MessageKey;
use crate::time::zone;

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
/// The longest ID of a shortcut or of a form and the longest text of one key sequence, in characters.
const PLAIN_ID_LONGEST: usize = 40;
const SHORTCUT_SEQUENCE_LONGEST: usize = 40;
/// The most forms whose window size is remembered, and the largest side of a window in pixels.
const WINDOWS_MOST: usize = 64;
const WINDOW_SIDE_LARGEST: u64 = 10_000;

/// What kind of text a setting holds. Anything but [`Format::Free`] is checked when the value is
/// stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// Any text (within the limits of the setting).
    Free,
    /// One of a fixed list.
    Choices(&'static [Choice]),
    /// `device`, or a time zone of the list the server carries (`Europe/Berlin`, `UTC`).
    TimeZone,
    /// The address of a page of the app: starts with one slash and never names another website: no
    /// `//` anywhere, no `..` segment, no encoded slash, backslash or dot.
    PageAddress,
    /// The keys a person chose for shortcuts: a JSON object from the ID of a shortcut to the key
    /// sequences that do it. The browser knows which IDs exist; the server only keeps the shape
    /// and the size within limits.
    ShortcutKeys,
    /// The sizes a person gave the windows of the forms: a JSON object from the ID of a form to its
    /// width and height in whole pixels. The browser knows which forms exist; the server only keeps
    /// the shape and the size within limits.
    WindowSizes,
    /// A list of host names separated by commas, such as `images.example.com`: bare names, with an
    /// optional port, never an address with `https://` or a path.
    Hosts,
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
            Self::WindowSizes => are_window_sizes(text),
            Self::Hosts => are_hosts(text),
        }
    }

    /// The answer to a value this format does not accept: a plain sentence where one says what to
    /// do instead, otherwise an invalid request (the screen offers only values it accepts).
    #[must_use]
    pub const fn refusal(self) -> ErrorCode {
        match self {
            Self::TimeZone => ErrorCode::SettingUnknownTimeZone,
            Self::Hosts => ErrorCode::SettingHostsNotBare,
            Self::Free
            | Self::Choices(_)
            | Self::PageAddress
            | Self::ShortcutKeys
            | Self::WindowSizes => ErrorCode::InvalidRequest,
        }
    }

    /// The choices, for the format that has a list.
    #[must_use]
    pub const fn choices(self) -> &'static [Choice] {
        match self {
            Self::Choices(choices) => choices,
            Self::Free
            | Self::TimeZone
            | Self::PageAddress
            | Self::ShortcutKeys
            | Self::WindowSizes
            | Self::Hosts => &[],
        }
    }
}

/// `device`, or a zone of the list the server carries ([`zone::named`]): a name only shaped like
/// one (`Europe/Atlantis`) is refused (M2-F72).
fn is_time_zone(text: &str) -> bool {
    text == DEVICE || zone::named(text).is_some()
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

fn is_plain_id(text: &str) -> bool {
    text.starts_with(|c: char| c.is_ascii_lowercase())
        && text.len() <= PLAIN_ID_LONGEST
        && text
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn is_key_sequence(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= SHORTCUT_SEQUENCE_LONGEST
        && text.chars().all(|c| c.is_ascii_graphic() || c == ' ')
}

/// The most digits of a port.
const PORT_DIGITS_MOST: usize = 5;

/// A host name as a browser sends it (letters, digits, dots and dashes), with an optional port:
/// the one rule for a host of a setting (the registry) and for a host written into a policy of the
/// browser (`bb-api`).
#[must_use]
pub fn is_host(item: &str) -> bool {
    let (name, port) = item
        .split_once(':')
        .map_or((item, None), |(n, p)| (n, Some(p)));
    let name_ok = !name.is_empty()
        && !name.starts_with(['.', '-'])
        && !name.ends_with(['.', '-'])
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
    let port_ok = port.is_none_or(|port| {
        !port.is_empty()
            && port.len() <= PORT_DIGITS_MOST
            && port.chars().all(|c| c.is_ascii_digit())
    });
    name_ok && port_ok
}

/// Every item of the list is a bare host name. The list is read the way every list of the registry
/// is read (`list_items`).
fn are_hosts(text: &str) -> bool {
    super::list_items(text).all(is_host)
}

fn are_shortcut_keys(text: &str) -> bool {
    let Ok(Value::Object(shortcuts)) = serde_json::from_str::<Value>(text) else {
        return false;
    };
    shortcuts.len() <= SHORTCUTS_MOST
        && shortcuts.iter().all(|(id, keys)| {
            is_plain_id(id)
                && keys.as_array().is_some_and(|keys| {
                    keys.len() <= SHORTCUT_KEYS_MOST
                        && keys
                            .iter()
                            .all(|key| key.as_str().is_some_and(is_key_sequence))
                })
        })
}

fn is_window_size(size: &Value) -> bool {
    let Some(sides) = size.as_object() else {
        return false;
    };
    sides.len() == 2
        && ["width", "height"].iter().all(|side| {
            sides
                .get(*side)
                .and_then(Value::as_u64)
                .is_some_and(|pixels| (1..=WINDOW_SIDE_LARGEST).contains(&pixels))
        })
}

fn are_window_sizes(text: &str) -> bool {
    let Ok(Value::Object(forms)) = serde_json::from_str::<Value>(text) else {
        return false;
    };
    forms.len() <= WINDOWS_MOST
        && forms
            .iter()
            .all(|(id, size)| is_plain_id(id) && is_window_size(size))
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
    fn m2_f72_a_time_zone_is_device_or_a_zone_the_server_knows() {
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
            "Europe/Atlantis",
            "europe/berlin",
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
        assert_eq!(
            Format::TimeZone.refusal(),
            ErrorCode::SettingUnknownTimeZone
        );
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
    fn sec_13_hosts_are_bare_names_with_an_optional_port() {
        for good in [
            "",
            "images.example.com",
            "images.example.com, cdn.example.com:8443",
            "localhost",
        ] {
            assert!(Format::Hosts.accepts(good), "{good}");
        }
        for bad in [
            "https://images.example.com",
            "images.example.com/path",
            "user@images.example.com",
            "*.example.com",
            ".example.com",
            "example.com:",
            "example.com:port",
            "exa mple.com",
            "images.example.com;evil.example",
        ] {
            assert!(!Format::Hosts.accepts(bad), "{bad}");
        }
    }

    #[test]
    fn ux_6_3_window_sizes_are_one_object_from_a_form_to_a_size_within_limits() {
        let accepts = |text: &str| Format::WindowSizes.accepts(text);
        assert!(accepts("{}"));
        assert!(accepts(r#"{"new-issue":{"width":800,"height":600}}"#));
        for bad in [
            "",
            "[]",
            "null",
            r#"{"New Issue":{"width":800,"height":600}}"#,
            r#"{"new-issue":{"width":800}}"#,
            r#"{"new-issue":{"width":"wide","height":600}}"#,
            r#"{"new-issue":{"width":0,"height":600}}"#,
            r#"{"new-issue":{"width":800.5,"height":600}}"#,
            r#"{"new-issue":{"width":99999,"height":600}}"#,
            r#"{"new-issue":{"width":800,"height":600,"depth":1}}"#,
            r#"{"new-issue":800}"#,
        ] {
            assert!(!accepts(bad), "{bad}");
        }
        let many: Vec<String> = (0..=WINDOWS_MOST)
            .map(|n| format!("\"form-{n}\":{{\"width\":800,\"height\":600}}"))
            .collect();
        assert!(!accepts(&format!("{{{}}}", many.join(","))));
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
