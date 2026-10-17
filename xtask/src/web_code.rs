//! Reading TypeScript as code for the checks and generators that look at the words of a string.
//! Reading the files is [`crate::source`]; the same job for Rust is [`crate::rust_source`].

/// What the scanner is inside of.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Inside {
    Code,
    /// A string in this quote (`"` or `'`); it ends at the line end too, like a bad string.
    Quoted(char),
    /// A template literal (backticks); it may span lines.
    Template,
    /// A regular expression literal (`/…/`); the flag says whether the scanner is inside a
    /// character class `[…]`, where a `/` does not end it. It ends at the line end too.
    Regex {
        in_class: bool,
    },
}

/// Words after which a `/` starts a regular expression and is not a division.
const BEFORE_REGEX: [&str; 8] = ["return", "typeof", "case", "do", "else", "in", "of", "void"];

/// Whether a `/` that comes after `before` (the code so far) starts a regular expression: it does
/// after an operator or an opening bracket or at the start, and it is a division after a name, a
/// number or a closing bracket.
fn starts_regex(before: &str) -> bool {
    let trimmed = before.trim_end();
    let Some(last) = trimmed.chars().next_back() else {
        return true;
    };
    if last.is_alphanumeric() || last == '_' || last == '$' {
        let word_start = trimmed
            .rfind(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
            .map_or(0, |at| at + 1);
        return BEFORE_REGEX.contains(&&trimmed[word_start..]);
    }
    !matches!(last, ')' | ']' | '}' | '"' | '\'' | '`')
}

/// The code with its comments (`//` and `/* */`) replaced by spaces; strings, template
/// literals and regular expression literals stay as they are, so a `//` inside `"https://…"` or
/// in `/\/\//` is not a comment. A `/` is told from a division by what comes before it
/// ([`starts_regex`]); after a `)` or a `}` it counts as a division. New lines stay,
/// so line numbers still fit.
pub(crate) fn strip_comments(text: &str) -> String {
    blank_out(text, Strings::Keep)
}

/// The code without its comments and without what is inside strings, template literals and
/// regular expression literals (their quotes stay): what is left is only code, for the checks
/// that read the names it declares.
pub(crate) fn strip(text: &str) -> String {
    blank_out(text, Strings::Blank)
}

/// What [`blank_out`] does with the inside of strings, template literals and regular expressions.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Strings {
    Keep,
    Blank,
}

fn blank_out(text: &str, strings: Strings) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut inside = Inside::Code;
    let mut at = 0;
    while at < chars.len() {
        let c = chars[at];
        let next = chars.get(at + 1).copied();
        let before = inside;
        match inside {
            Inside::Code if c == '/' && next == Some('/') => {
                while at < chars.len() && chars[at] != '\n' {
                    out.push(' ');
                    at += 1;
                }
                continue;
            }
            Inside::Code if c == '/' && next == Some('*') => {
                out.push_str("  ");
                at += 2;
                while at < chars.len() && !(chars[at] == '*' && chars.get(at + 1) == Some(&'/')) {
                    out.push(if chars[at] == '\n' { '\n' } else { ' ' });
                    at += 1;
                }
                if at < chars.len() {
                    out.push_str("  ");
                    at += 2;
                }
                continue;
            }
            Inside::Code if c == '/' && starts_regex(&out) => {
                inside = Inside::Regex { in_class: false }
            }
            Inside::Code => {
                inside = match c {
                    '"' | '\'' => Inside::Quoted(c),
                    '`' => Inside::Template,
                    _ => Inside::Code,
                };
            }
            Inside::Quoted(_) | Inside::Template | Inside::Regex { .. } if c == '\\' => {
                let blanked = strings == Strings::Blank;
                out.push(if blanked { ' ' } else { c });
                out.extend(next.map(|n| if blanked && n != '\n' { ' ' } else { n }));
                at += 2;
                continue;
            }
            Inside::Quoted(quote) if c == quote || c == '\n' => inside = Inside::Code,
            Inside::Template if c == '`' => inside = Inside::Code,
            Inside::Regex { .. } if c == '\n' => inside = Inside::Code,
            Inside::Regex { in_class: false } if c == '/' => inside = Inside::Code,
            Inside::Regex { in_class } if c == '[' || c == ']' => {
                inside = Inside::Regex {
                    in_class: c == '[' || (in_class && c != ']'),
                };
            }
            Inside::Quoted(_) | Inside::Template | Inside::Regex { .. } => {}
        }
        // Inside a string that goes on, the characters are blanked; its quotes stay.
        let inside_string = before != Inside::Code && inside != Inside::Code;
        out.push(if strings == Strings::Blank && inside_string && c != '\n' {
            ' '
        } else {
            c
        });
        at += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_are_blanked_and_strings_stay() {
        let code =
            "a(\"x.y\"); // t(\"c.d\")\n/* t(\"e.f\")\n still */ b('g.h');\nu(\"http://z\"); `//k`";
        let out = strip_comments(code);
        assert_eq!(out.len(), code.len());
        assert_eq!(out.lines().count(), code.lines().count());
        assert!(out.contains("\"x.y\"") && out.contains("'g.h'"));
        assert!(!out.contains("c.d") && !out.contains("e.f") && !out.contains("still"));
        assert!(out.contains("\"http://z\"") && out.contains("`//k`"));
    }

    #[test]
    fn strip_leaves_only_code() {
        let code =
            "const a = \"function x() {}\"; // c\nlet b = `t ${1} \\` more`; let r = /ab\\//;";
        let out = strip(code);
        assert_eq!(out.len(), code.len());
        assert!(out.contains("const a = \"") && out.contains("let b = `"));
        assert!(!out.contains("function") && !out.contains("more") && !out.contains("// c"));
        assert!(out.contains("let r = /"));
    }

    #[test]
    fn a_regular_expression_is_not_a_comment() {
        let code = r"const r = /\/\//; // gone
x.split(/[/]x/); // gone too
const q = a / b; // c";
        let out = strip_comments(code);
        assert!(out.contains(r"/\/\//;") && out.contains("/[/]x/"));
        assert!(!out.contains("gone") && !out.contains("// c"));
        assert!(out.contains("a / b;"));
        assert_eq!(out.len(), code.len());
    }

    #[test]
    fn a_slash_after_a_closing_bracket_is_a_division() {
        let out = strip_comments(
            "f(a) / 2; // gone
return /x/.test(y); // gone",
        );
        assert!(!out.contains("gone"));
        assert!(out.contains("/x/.test(y);"));
    }

    #[test]
    fn an_escaped_quote_does_not_end_a_string() {
        let out = strip_comments(r#"a("it\"s // not a comment"); // gone"#);
        assert!(out.contains("// not a comment"));
        assert!(!out.contains("gone"));
    }
}
