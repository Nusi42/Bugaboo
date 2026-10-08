//! Reading Rust code as code: what is left of the text when its comments (and, for most
//! checks, its strings) are blanked out. Reading the files is [`crate::source`].

/// What [`blank_out`] does with strings.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Strings {
    Blank,
    Keep,
}

/// The code with comments and the insides of strings and character literals replaced by spaces
/// (new lines stay, so line numbers still fit).
pub(crate) fn strip(text: &str) -> String {
    blank_out(text, Strings::Blank)
}

/// The code with comments and character literals replaced by spaces, and strings kept (the checks
/// that look at the words of a string need them). A raw string is kept as an ordinary string:
/// its `r#` and `#` become spaces and a quote or backslash inside it a space, so one scan of
/// strings reads both. Positions and new lines stay.
pub(crate) fn strip_comments(text: &str) -> String {
    blank_out(text, Strings::Keep)
}

fn blank_out(text: &str, strings: Strings) -> String {
    let keep = strings == Strings::Keep;
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let blank = |c: char| if c == '\n' { '\n' } else { ' ' };
    let mut at = 0;
    while at < chars.len() {
        let c = chars[at];
        let next = chars.get(at + 1).copied();
        if c == '/' && next == Some('/') {
            while at < chars.len() && chars[at] != '\n' {
                out.push(' ');
                at += 1;
            }
        } else if c == '/' && next == Some('*') {
            let mut depth = 0;
            while at < chars.len() {
                if chars[at] == '/' && chars.get(at + 1) == Some(&'*') {
                    depth += 1;
                    out.push_str("  ");
                    at += 2;
                } else if chars[at] == '*' && chars.get(at + 1) == Some(&'/') {
                    depth -= 1;
                    out.push_str("  ");
                    at += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    out.push(blank(chars[at]));
                    at += 1;
                }
            }
        } else if c == 'r'
            && matches!(next, Some('"' | '#'))
            && raw_string_hashes(&chars, at).is_some()
        {
            let hashes = raw_string_hashes(&chars, at).unwrap_or(0);
            let end_marker: Vec<char> = std::iter::once('"')
                .chain(std::iter::repeat_n('#', hashes))
                .collect();
            let quote = if keep { '"' } else { ' ' };
            out.push_str(&" ".repeat(1 + hashes));
            out.push(quote);
            at += 1 + hashes + 1;
            while at < chars.len()
                && chars[at..]
                    .iter()
                    .take(end_marker.len())
                    .ne(end_marker.iter())
            {
                let inner = chars[at];
                out.push(if keep && !matches!(inner, '"' | '\\') {
                    inner
                } else {
                    blank(inner)
                });
                at += 1;
            }
            let rest = end_marker.len().min(chars.len() - at);
            if rest > 0 {
                out.push(quote);
                out.push_str(&" ".repeat(rest - 1));
            }
            at += rest;
        } else if c == '"' {
            out.push(if keep { '"' } else { ' ' });
            at += 1;
            while at < chars.len() && chars[at] != '"' {
                if chars[at] == '\\' && at + 1 < chars.len() {
                    out.push(if keep { '\\' } else { ' ' });
                    at += 1;
                }
                out.push(if keep { chars[at] } else { blank(chars[at]) });
                at += 1;
            }
            out.push(if keep { '"' } else { ' ' });
            at += 1;
        } else if c == '\'' && is_char_literal(&chars, at) {
            out.push(' ');
            at += 1;
            while at < chars.len() && chars[at] != '\'' {
                if chars[at] == '\\' {
                    out.push(' ');
                    at += 1;
                }
                out.push(' ');
                at += 1;
            }
            out.push(' ');
            at += 1;
        } else {
            out.push(c);
            at += 1;
        }
    }
    out
}

/// The code with every white space taken out (so a call that was split over lines reads as one),
/// and the line (from 1) each remaining character stood on.
pub(crate) fn squeezed(code: &str) -> (String, Vec<usize>) {
    let mut text = String::new();
    let mut lines = Vec::new();
    for (index, line) in code.lines().enumerate() {
        for character in line.chars().filter(|c| !c.is_whitespace()) {
            text.push(character);
            // One entry for each byte, so a byte position finds its line.
            lines.extend(std::iter::repeat_n(index + 1, character.len_utf8()));
        }
    }
    (text, lines)
}

/// How many `#` follow the `r` of a raw string that starts at `at`, if it is one.
fn raw_string_hashes(chars: &[char], at: usize) -> Option<usize> {
    let hashes = chars[at + 1..].iter().take_while(|c| **c == '#').count();
    (chars.get(at + 1 + hashes) == Some(&'"')).then_some(hashes)
}

/// Whether the `'` at `at` starts a character literal (and not a lifetime).
fn is_char_literal(chars: &[char], at: usize) -> bool {
    matches!(
        (chars.get(at + 1), chars.get(at + 2)),
        (Some('\\'), _) | (Some(_), Some('\''))
    )
}

/// The text with every item behind `#[cfg(test)]` blanked out (new lines stay, so line numbers still
/// fit), and the rest of the file kept: a unit test module is usually the last item of a file,
/// but code that comes after it is code that ships. Comments and strings are read as such, so a
/// brace in a text does not end an item early.
pub(crate) fn without_tests(text: &str) -> String {
    const ATTRIBUTE: &str = "#[cfg(test)]";
    let code: Vec<char> = strip(text).chars().collect();
    let marker: Vec<char> = ATTRIBUTE.chars().collect();
    let mut cut = vec![false; code.len()];
    let mut at = 0;
    while at + marker.len() <= code.len() {
        if code[at..at + marker.len()] == marker[..] {
            let end = item_end(&code, at + marker.len());
            for flag in &mut cut[at..end] {
                *flag = true;
            }
            at = end.max(at + marker.len());
        } else {
            at += 1;
        }
    }
    text.chars()
        .zip(cut)
        .map(|(c, gone)| if gone && c != '\n' { ' ' } else { c })
        .collect()
}

/// Where the item that starts at `from` (after its attribute) ends: after the `;` or after the
/// braces that close it, whichever comes first at the top level. More attributes in between are
/// part of the item.
fn item_end(code: &[char], from: usize) -> usize {
    let mut at = from;
    // More attributes.
    loop {
        while code.get(at).is_some_and(|c| c.is_whitespace()) {
            at += 1;
        }
        if code.get(at) == Some(&'#') && code.get(at + 1) == Some(&'[') {
            let mut depth = 0_usize;
            while let Some(c) = code.get(at) {
                at += 1;
                match c {
                    '[' => depth += 1,
                    ']' => {
                        depth = depth.saturating_sub(1);
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        } else {
            break;
        }
    }
    let mut depth = 0_i32;
    while let Some(c) = code.get(at) {
        at += 1;
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' => depth -= 1,
            '}' => {
                depth -= 1;
                if depth <= 0 {
                    return at;
                }
            }
            ';' if depth <= 0 => return at,
            _ => {}
        }
    }
    at
}

/// The text from the first `open` bracket in `text` (`(`, `[` or `{`) up to the bracket that
/// closes it, brackets included, or `None` when it never closes. Brackets inside a string do not
/// count, so this reads code with its strings kept as well as blanked code. The one way the checks
/// cut a call, an attribute or a body out of code.
pub(crate) fn balanced(text: &str, open: char) -> Option<&str> {
    let close = match open {
        '(' => ')',
        '[' => ']',
        _ => '}',
    };
    let mut start = None;
    let mut depth = 0_usize;
    let mut in_string = false;
    let mut escaped = false;
    for (at, character) in text.char_indices() {
        if in_string {
            match (escaped, character) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '"') => in_string = false,
                _ => {}
            }
        } else if character == '"' {
            in_string = true;
        } else if character == open {
            start.get_or_insert(at);
            depth += 1;
        } else if character == close && start.is_some() {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return start.map(|from| &text[from..at + close.len_utf8()]);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bracket_is_matched_with_its_partner_and_strings_do_not_count() {
        assert_eq!(
            balanced("f(a, b(c), \")\") rest", '('),
            Some("(a, b(c), \")\")")
        );
        assert_eq!(balanced("x { a { b } c } d", '{'), Some("{ a { b } c }"));
        assert_eq!(balanced("v[0][1]", '['), Some("[0]"));
        assert_eq!(balanced("f(a, (b", '('), None, "never closes");
        assert_eq!(balanced("no bracket", '('), None);
    }

    #[test]
    fn the_unit_tests_are_left_out_and_the_rest_of_the_file_stays() {
        let text = "fn a() {}
#[cfg(test)]
mod tests {
    fn t() { let s = \"}\"; }
}
fn after() {}
";
        let code = without_tests(text);
        assert!(code.contains("fn a()"));
        assert!(
            code.contains("fn after()"),
            "code after the tests ships: {code:?}"
        );
        assert!(!code.contains("fn t()"));
        assert_eq!(code.lines().count(), text.lines().count(), "new lines stay");
        assert_eq!(
            without_tests(
                "fn a() {}
"
            ),
            "fn a() {}
"
        );
    }

    #[test]
    fn a_test_item_that_is_not_a_module_is_left_out_too() {
        let code = without_tests(
            "#[cfg(test)]
use a::b;
fn kept() {}
#[cfg(test)]
#[inline]
fn helper() { { } }
fn also() {}
",
        );
        assert!(code.contains("fn kept()") && code.contains("fn also()"));
        assert!(!code.contains("use a::b") && !code.contains("fn helper"));
    }

    #[test]
    fn strings_comments_and_lifetimes_are_told_apart() {
        let code = strip(
            "let a = 'x'; let b: &'a str = \"WWW // d\"; // QQQ
/* ZZZ */ keep",
        );
        assert!(code.contains("let a"));
        assert!(code.contains("&'a str"));
        for gone in ["WWW", "QQQ", "ZZZ", "'x'"] {
            assert!(!code.contains(gone), "{gone} in {code:?}");
        }
        assert!(code.contains("keep"));
        assert_eq!(code.lines().count(), 2, "new lines stay");
    }

    #[test]
    fn comments_go_but_strings_stay_and_raw_strings_read_as_ordinary_ones() {
        let text = "let a = \"keep // this\"; // gone
/* gone */ let b = r#\"raw \"x\" y\"#;
let c = 'q';";
        let code = strip_comments(text);
        assert!(code.contains("\"keep // this\""), "{code:?}");
        assert!(!code.contains("gone") && !code.contains("'q'"), "{code:?}");
        assert!(code.contains("\"raw  x  y\""), "{code:?}");
        assert_eq!(code.len(), text.len(), "positions stay");
        assert_eq!(code.lines().count(), text.lines().count());
    }
}
