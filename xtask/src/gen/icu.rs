//! Reads the placeholders of an ICU message (`Hello {name}`, plural, select, number, date).
//!
//! The message catalog check and the typed-keys generator both need to know which values a
//! message expects, so this is the one place that understands the format (coding-standards §1).

use std::collections::BTreeMap;

/// What kind of value a placeholder expects, from least to most specific.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ArgKind {
    /// `{name}`: text or a number.
    Text,
    /// `{kind, select, …}`: one of the listed words.
    Select,
    /// `{count, number}`, `{count, plural, …}`: a number.
    Number,
    /// `{when, date}`, `{when, time}`: a date.
    Date,
}

/// Placeholder names with their kinds, sorted by name.
pub(crate) type Placeholders = BTreeMap<String, ArgKind>;

/// Finds every placeholder of `message`, or says what is wrong with its braces.
pub(crate) fn placeholders(message: &str) -> Result<Placeholders, String> {
    let chars: Vec<char> = message.chars().collect();
    let mut found = Placeholders::new();
    let end = parse_message(&chars, 0, false, &mut found)?;
    if end < chars.len() {
        return Err("There is a closing `}` without a matching `{`.".to_owned());
    }
    Ok(found)
}

/// Parses text up to the end, or up to the `}` that closes a plural/select option.
/// Returns the index where it stopped.
fn parse_message(
    chars: &[char],
    start: usize,
    nested: bool,
    found: &mut Placeholders,
) -> Result<usize, String> {
    let mut index = start;
    while index < chars.len() {
        match chars[index] {
            '\'' => index = skip_quoted(chars, index),
            '{' => index = parse_argument(chars, index + 1, found)?,
            '}' => return Ok(index),
            _ => index += 1,
        }
    }
    if nested {
        return Err("A `{` is never closed.".to_owned());
    }
    Ok(index)
}

/// ICU apostrophes: `''` is one apostrophe, `'{'` is a literal brace, a lone `'` is itself.
fn skip_quoted(chars: &[char], index: usize) -> usize {
    match chars.get(index + 1) {
        Some('\'') => index + 2,
        Some('{' | '}') => {
            let mut cursor = index + 1;
            while cursor < chars.len() && chars[cursor] != '\'' {
                cursor += 1;
            }
            (cursor + 1).min(chars.len())
        }
        _ => index + 1,
    }
}

fn parse_argument(chars: &[char], start: usize, found: &mut Placeholders) -> Result<usize, String> {
    let (name, mut index, mut stop) = read_word(chars, start);
    if name.is_empty() {
        return Err("A `{` has no placeholder name.".to_owned());
    }
    let mut kind = ArgKind::Text;
    if stop == ',' {
        let (type_name, after, type_stop) = read_word(chars, index + 1);
        index = after;
        stop = type_stop;
        kind = match type_name.as_str() {
            "number" | "plural" | "selectordinal" | "spellout" | "ordinal" | "duration" => {
                ArgKind::Number
            }
            "date" | "time" => ArgKind::Date,
            "select" => ArgKind::Select,
            other => return Err(format!("`{{{name}, {other}}}` is not a known format.")),
        };
        if matches!(type_name.as_str(), "plural" | "selectordinal" | "select") {
            if stop != ',' {
                return Err(format!("`{{{name}, {type_name}}}` needs options."));
            }
            index = parse_options(chars, index + 1, found)?;
        } else if stop == ',' {
            // A style such as `{count, number, percent}`.
            let (_, after_style, style_stop) = read_word(chars, index + 1);
            index = after_style;
            stop = style_stop;
        }
    }
    if stop != '}' && chars.get(index) != Some(&'}') {
        return Err(format!("The placeholder `{{{name}` is never closed."));
    }
    remember(found, name, kind);
    Ok(index + 1)
}

/// Parses `selector {message} selector {message} …` and returns the index of the closing `}`.
fn parse_options(chars: &[char], start: usize, found: &mut Placeholders) -> Result<usize, String> {
    let mut index = start;
    let mut options = 0_usize;
    loop {
        while chars.get(index).is_some_and(|c| c.is_whitespace()) {
            index += 1;
        }
        match chars.get(index) {
            None => return Err("A plural or select is never closed.".to_owned()),
            Some('}') => break,
            Some(_) => {}
        }
        while chars
            .get(index)
            .is_some_and(|c| !c.is_whitespace() && *c != '{' && *c != '}')
        {
            index += 1;
        }
        while chars.get(index).is_some_and(|c| c.is_whitespace()) {
            index += 1;
        }
        if chars.get(index) != Some(&'{') {
            return Err("Every plural or select option needs its text in `{ }`.".to_owned());
        }
        let end = parse_message(chars, index + 1, true, found)?;
        if chars.get(end) != Some(&'}') {
            return Err("An option of a plural or select is never closed.".to_owned());
        }
        index = end + 1;
        options += 1;
    }
    if options == 0 {
        return Err("A plural or select has no options.".to_owned());
    }
    Ok(index)
}

/// Reads a trimmed word up to the next `,` or `}`; returns the word, the stop index and the stop.
fn read_word(chars: &[char], start: usize) -> (String, usize, char) {
    let mut index = start;
    let mut word = String::new();
    while let Some(&c) = chars.get(index) {
        if c == ',' || c == '}' {
            return (word.trim().to_owned(), index, c);
        }
        word.push(c);
        index += 1;
    }
    (word.trim().to_owned(), index, '\0')
}

fn remember(found: &mut Placeholders, name: String, kind: ArgKind) {
    let entry = found.entry(name).or_insert(kind);
    *entry = (*entry).max(kind);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_has_no_placeholders() {
        assert!(placeholders("Hello Bugaboo").unwrap().is_empty());
    }

    #[test]
    fn simple_and_nested_placeholders_are_found() {
        let found =
            placeholders("{name} has {count, plural, one {# issue by {owner}} other {# issues}}")
                .unwrap();
        let names: Vec<&str> = found.keys().map(String::as_str).collect();
        assert_eq!(names, ["count", "name", "owner"]);
        assert_eq!(found["count"], ArgKind::Number);
        assert_eq!(found["name"], ArgKind::Text);
    }

    #[test]
    fn quoted_braces_are_text() {
        assert!(
            placeholders("Use '{' and '}' like this")
                .unwrap()
                .is_empty()
        );
        assert!(placeholders("It''s fine").unwrap().is_empty());
    }

    #[test]
    fn broken_braces_are_reported() {
        assert!(placeholders("Hello {name").is_err());
        assert!(placeholders("Hello name}").is_err());
        assert!(placeholders("Hello {}").is_err());
        assert!(placeholders("{n, plural}").is_err());
        assert!(placeholders("{n, plural, one {x}").is_err());
    }

    #[test]
    fn date_and_style_are_understood() {
        let found = placeholders("{when, date, short} {n, number, percent}").unwrap();
        assert_eq!(found["when"], ArgKind::Date);
        assert_eq!(found["n"], ArgKind::Number);
    }
}
