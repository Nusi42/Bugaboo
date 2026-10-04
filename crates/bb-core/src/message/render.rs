//! Fills in an ICU message (`Hello {name}`, plural, select) with English rules.
//!
//! The server only needs this for the few texts it prints itself and for error titles. The
//! browser formats messages with `intl-messageformat`; both read the same catalog. This reader
//! understands what `xtask`'s catalog check (`icu.rs`) accepts, and nothing else.

use super::Value;

/// The values a message is filled with.
type Arguments<'a> = &'a [(&'static str, Value)];

/// The text of `template` with every placeholder replaced. A placeholder with no value is empty.
pub(super) fn render(template: &str, arguments: Arguments<'_>) -> String {
    let chars: Vec<char> = template.chars().collect();
    let mut out = String::new();
    read(&chars, 0, arguments, None, &mut out);
    out
}

/// Reads up to the end, or up to the `}` that closes a plural or select option; returns where it
/// stopped. `count` is what `#` stands for inside a plural option.
fn read(
    chars: &[char],
    start: usize,
    arguments: Arguments<'_>,
    count: Option<i64>,
    out: &mut String,
) -> usize {
    let mut index = start;
    while index < chars.len() {
        match chars[index] {
            '}' => return index,
            '\'' => index = quoted(chars, index, out),
            '{' => index = argument(chars, index + 1, arguments, out),
            '#' => {
                match count {
                    Some(number) => out.push_str(&number.to_string()),
                    None => out.push('#'),
                }
                index += 1;
            }
            other => {
                out.push(other);
                index += 1;
            }
        }
    }
    index
}

/// ICU apostrophes: `''` is one apostrophe, `'{'` is a literal brace, a lone `'` is itself.
fn quoted(chars: &[char], index: usize, out: &mut String) -> usize {
    match chars.get(index + 1) {
        Some('\'') => {
            out.push('\'');
            index + 2
        }
        Some('{' | '}') => {
            let mut cursor = index + 1;
            while cursor < chars.len() && chars[cursor] != '\'' {
                out.push(chars[cursor]);
                cursor += 1;
            }
            (cursor + 1).min(chars.len())
        }
        _ => {
            out.push('\'');
            index + 1
        }
    }
}

/// A word up to the next `,` or `}`, trimmed; returns it and the index of the stop.
fn word(chars: &[char], start: usize) -> (String, usize) {
    let mut index = start;
    while index < chars.len() && chars[index] != ',' && chars[index] != '}' {
        index += 1;
    }
    (
        chars[start..index]
            .iter()
            .collect::<String>()
            .trim()
            .to_owned(),
        index,
    )
}

/// One `{name}` or `{name, kind, …}`; `start` is just after the `{`. Returns the index after it.
fn argument(chars: &[char], start: usize, arguments: Arguments<'_>, out: &mut String) -> usize {
    let (name, mut index) = word(chars, start);
    let value = arguments
        .iter()
        .find(|(key, _)| *key == name)
        .map(|(_, v)| v);
    if chars.get(index) == Some(&',') {
        let (kind, after) = word(chars, index + 1);
        index = after;
        if matches!(kind.as_str(), "plural" | "select" | "selectordinal")
            && chars.get(index) == Some(&',')
        {
            let (chosen, after) = options(chars, index + 1, arguments, value, kind == "select");
            out.push_str(&chosen);
            return after + 1;
        }
        if chars.get(index) == Some(&',') {
            // A style such as `{count, number, percent}`; English output ignores it.
            index = word(chars, index + 1).1;
        }
    }
    if let Some(value) = value {
        out.push_str(&value.to_text());
    }
    index + 1
}

/// Reads `selector {text} selector {text} …` up to the closing `}` and returns the chosen text
/// and the index of that `}`.
fn options(
    chars: &[char],
    start: usize,
    arguments: Arguments<'_>,
    value: Option<&Value>,
    by_word: bool,
) -> (String, usize) {
    let number = value.and_then(Value::as_number);
    let text = value.map(Value::to_text).unwrap_or_default();
    let mut index = start;
    let mut chosen: Option<String> = None;
    let mut other = String::new();
    loop {
        while chars.get(index).is_some_and(|c| c.is_whitespace()) {
            index += 1;
        }
        if index >= chars.len() || chars[index] == '}' {
            break;
        }
        let begin = index;
        while chars
            .get(index)
            .is_some_and(|c| !c.is_whitespace() && *c != '{')
        {
            index += 1;
        }
        let selector: String = chars[begin..index].iter().collect();
        while chars.get(index).is_some_and(|c| c.is_whitespace()) {
            index += 1;
        }
        let mut body = String::new();
        index = read(
            chars,
            index + 1,
            arguments,
            if by_word { None } else { number },
            &mut body,
        ) + 1;
        if selector == "other" {
            other = body;
        } else if chosen.is_none() && matches_selector(&selector, by_word, &text, number) {
            chosen = Some(body);
        }
    }
    (chosen.unwrap_or(other), index)
}

fn matches_selector(selector: &str, by_word: bool, text: &str, number: Option<i64>) -> bool {
    if by_word {
        return selector == text;
    }
    match (selector, number) {
        ("one", Some(n)) => n == 1,
        (exact, Some(n)) => exact.strip_prefix('=').and_then(|v| v.parse::<i64>().ok()) == Some(n),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(name: &'static str, value: &str) -> (&'static str, Value) {
        (name, Value::Text(value.to_owned()))
    }

    #[test]
    fn plain_text_and_placeholders() {
        assert_eq!(render("Hello", &[]), "Hello");
        assert_eq!(render("Hi {name}!", &[text("name", "Ana")]), "Hi Ana!");
        assert_eq!(render("Hi {name}!", &[]), "Hi !");
        assert_eq!(
            render(
                "{n} of {total, number}",
                &[("n", Value::Number(2)), ("total", Value::Number(5))]
            ),
            "2 of 5"
        );
    }

    #[test]
    fn plural_picks_the_english_form() {
        let message = "{n, plural, =0 {no issues} one {# issue} other {# issues}}";
        assert_eq!(render(message, &[("n", Value::Number(0))]), "no issues");
        assert_eq!(render(message, &[("n", Value::Number(1))]), "1 issue");
        assert_eq!(render(message, &[("n", Value::Number(7))]), "7 issues");
    }

    #[test]
    fn select_picks_the_word_or_other() {
        let message = "{kind, select, bug {A bug} other {Something}}";
        assert_eq!(render(message, &[text("kind", "bug")]), "A bug");
        assert_eq!(render(message, &[text("kind", "idea")]), "Something");
    }

    #[test]
    fn apostrophes_quote_braces() {
        assert_eq!(
            render("It''s '{'not'}' a {x}", &[text("x", "test")]),
            "It's {not} a test"
        );
        assert_eq!(render("Don't", &[]), "Don't");
    }

    #[test]
    fn nested_placeholders_in_options() {
        let message = "{n, plural, one {{who} has # issue} other {{who} has # issues}}";
        assert_eq!(
            render(message, &[("n", Value::Number(3)), text("who", "Ana")]),
            "Ana has 3 issues"
        );
    }
}
