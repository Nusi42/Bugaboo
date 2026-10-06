//! Reading the Rust source of the product crates, for the checks and generators that look at
//! code instead of running a tool on it.

use std::path::Path;

use anyhow::Context as _;

use crate::repo::Repo;

/// One Rust source file: its repository-relative path and its text.
pub(crate) type Source = (String, String);

/// Every Rust file of the product crates, except integration tests and their examples.
pub(crate) fn read_product_sources(repo: &Repo) -> anyhow::Result<Vec<Source>> {
    let mut sources = Vec::new();
    for file in repo.all_files()? {
        let is_product_code = file.starts_with("crates/")
            && Path::new(&file)
                .extension()
                .is_some_and(|extension| extension == "rs")
            && !file.split('/').any(|part| part == "tests");
        if is_product_code {
            let text = std::fs::read_to_string(repo.path(&file))
                .with_context(|| format!("could not read {file}"))?;
            sources.push((file, text.replace("\r\n", "\n")));
        }
    }
    Ok(sources)
}

/// The code with comments and the insides of strings and character literals replaced by spaces
/// (new lines stay, so line numbers still fit).
pub(crate) fn strip(text: &str) -> String {
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
            out.push(' ');
            at += 1 + hashes + 1;
            out.push_str(&" ".repeat(hashes + 1));
            while at < chars.len()
                && chars[at..]
                    .iter()
                    .take(end_marker.len())
                    .ne(end_marker.iter())
            {
                out.push(blank(chars[at]));
                at += 1;
            }
            let rest = end_marker.len().min(chars.len() - at);
            out.push_str(&" ".repeat(rest));
            at += rest;
        } else if c == '"' {
            out.push(' ');
            at += 1;
            while at < chars.len() && chars[at] != '"' {
                if chars[at] == '\\' && at + 1 < chars.len() {
                    out.push(' ');
                    at += 1;
                }
                out.push(blank(chars[at]));
                at += 1;
            }
            out.push(' ');
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
