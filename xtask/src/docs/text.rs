//! Small text helpers shared by the docs builder and the documentation checks.

/// Whether a file name ends in `.extension`, ignoring case.
pub(crate) fn has_extension(name: &str, extension: &str) -> bool {
    std::path::Path::new(name)
        .extension()
        .is_some_and(|found| found.eq_ignore_ascii_case(extension))
}

/// Escapes text for the inside of an HTML element (quotes stay as they are).
pub(crate) fn escape_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Escapes text for an HTML attribute value.
pub(crate) fn escape_attribute(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            other => out.push(other),
        }
    }
    out
}

/// Words in `text` that look like catalogue IDs (`ARCH-DEPS`, or a family such as `FMT-*`).
///
/// Feature IDs (`SEC-06`), decision records (`ADR-0014`) and similar codes ending in digits are
/// not catalogue IDs and are skipped.
pub(crate) fn check_id_tokens(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '*'))
        .filter(|word| is_check_id(word))
        .map(str::to_owned)
        .collect()
}

fn is_check_id(word: &str) -> bool {
    let segments: Vec<&str> = word.split('-').collect();
    let [first, rest @ ..] = segments.as_slice() else {
        return false;
    };
    let first_ok = first.len() >= 2 && first.bytes().all(|b| b.is_ascii_uppercase());
    let rest_ok = !rest.is_empty()
        && rest.iter().all(|segment| {
            *segment == "*"
                || (!segment.is_empty()
                    && segment
                        .bytes()
                        .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit()))
        });
    let last_is_number = rest
        .last()
        .is_some_and(|last| last.bytes().all(|b| b.is_ascii_digit()));
    first_ok && rest_ok && !last_is_number
}

/// Whether `id` is named by `token`, which is an exact ID or a family like `FMT-*`.
pub(crate) fn token_matches(token: &str, id: &str) -> bool {
    match token.strip_suffix('*') {
        Some(prefix) => id.starts_with(prefix),
        None => token == id,
    }
}

/// Feature IDs (`SEC-06`, `ISS-01a`) found in `text`, in order, with duplicates.
pub(crate) fn feature_ids(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut start = 0;
    while start < bytes.len() {
        let at_boundary = start == 0 || !is_word_byte(bytes[start - 1]);
        if at_boundary && let Some(end) = feature_id_end(bytes, start) {
            found.push(text[start..end].to_owned());
            start = end;
        } else {
            start += 1;
        }
    }
    found
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'
}

/// If a feature ID starts at `start`, the index just after it.
fn feature_id_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut at = start;
    while bytes.get(at).is_some_and(u8::is_ascii_uppercase) {
        at += 1;
    }
    if !(2..=6).contains(&(at - start)) || &bytes[start..at] == b"ADR" || &bytes[start..at] == b"WP"
    {
        return None;
    }
    if bytes.get(at) != Some(&b'-') {
        return None;
    }
    at += 1;
    let digits = at;
    while bytes.get(at).is_some_and(u8::is_ascii_digit) {
        at += 1;
    }
    if at - digits != 2 {
        return None;
    }
    if bytes.get(at).is_some_and(u8::is_ascii_lowercase) {
        at += 1;
    }
    if bytes.get(at).is_some_and(|byte| is_word_byte(*byte)) {
        return None;
    }
    Some(at)
}

/// The lowercase words of a text.
fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// Whether the text says `review` in one of the two accepted forms, with a reason of at least two
/// words: `review: <reason>` or `review (<reason>)`.
pub(crate) fn says_review_with_reason(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.match_indices("review").any(|(at, _)| {
        let starts_word = !lower[..at]
            .chars()
            .next_back()
            .is_some_and(char::is_alphanumeric);
        let after = &lower[at + "review".len()..];
        let reason = if let Some(rest) = after.strip_prefix(':') {
            rest
        } else if let Some(rest) = after.strip_prefix(" (") {
            rest.split(')').next().unwrap_or("")
        } else {
            return false;
        };
        starts_word && words(reason).len() >= 2
    })
}

/// Whether the text names a catalogue ID (or family like `FMT-*`), or says `review` with a reason.
/// This is what "this rule names its check" means everywhere (META-RULE-TRACE, DOC-ADR-ENFORCED).
pub(crate) fn traces(text: &str, catalog_ids: &[&str]) -> bool {
    says_review_with_reason(text)
        || check_id_tokens(text)
            .iter()
            .any(|token| catalog_ids.iter().any(|id| token_matches(token, id)))
}

/// Whether the text states a rule: "must", "shall", "has to", "may only" and the like.
pub(crate) fn states_rule(text: &str) -> bool {
    let words = words(text);
    words.iter().enumerate().any(|(at, word)| {
        let next = words.get(at + 1).map(String::as_str);
        let after = words.get(at + 2).map(String::as_str);
        match word.as_str() {
            "must" | "shall" => true,
            "has" | "have" | "needs" => next == Some("to"),
            "may" => matches!(next, Some("only" | "not")),
            "is" | "are" => next == Some("required") && after == Some("to"),
            _ => false,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_are_recognised_in_all_their_forms() {
        for rule in [
            "It must work",
            "It shall work",
            "It has to work",
            "They have to",
            "Everyone needs to",
            "It may only be",
            "It may not be",
            "It is required to",
            "They are required to",
        ] {
            assert!(states_rule(rule), "{rule}");
        }
        for plain in ["Never do that", "It may be fine", "The mustard has a top"] {
            assert!(!states_rule(plain), "{plain}");
        }
    }

    #[test]
    fn review_needs_a_reason() {
        assert!(traces("review: checked at every milestone", &[]));
        assert!(traces("review (a design choice)", &[]));
        assert!(!traces("review", &[]));
        assert!(!traces("review, compliance.md", &[]));
        assert!(!traces("review decides what is fair", &[]));
        assert!(!traces("review (x)", &[]));
        assert!(!traces("a preview of it, soon", &[]));
        assert!(traces("see ARCH-DEPS", &["ARCH-DEPS"]));
        assert!(!traces("see ARCH-NOPE", &["ARCH-DEPS"]));
    }

    #[test]
    fn finds_check_ids_and_families() {
        let found = check_id_tokens("ARCH-DEPS, FMT-*; `SEC-06` ADR-0014 review x-ray UTF-8");
        assert_eq!(found, ["ARCH-DEPS", "FMT-*"]);
    }

    #[test]
    fn families_match_by_prefix() {
        assert!(token_matches("FMT-*", "FMT-RUST"));
        assert!(!token_matches("FMT-*", "LINT-RUST"));
        assert!(token_matches("ARCH-DEPS", "ARCH-DEPS"));
    }

    #[test]
    fn finds_feature_ids_with_letter_suffix_only() {
        let found = feature_ids("SEC-06, ISS-01a and ADR-0014, WP-04; ISS-014");
        assert_eq!(found, ["SEC-06", "ISS-01a"]);
    }
}
