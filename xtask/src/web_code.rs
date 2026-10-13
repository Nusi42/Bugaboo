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
}

/// The code with its comments (`//` and `/* */`) replaced by spaces; strings and template
/// literals stay as they are, so a `//` inside `"https://…"` is not a comment. New lines stay,
/// so line numbers still fit.
pub(crate) fn strip_comments(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut inside = Inside::Code;
    let mut at = 0;
    while at < chars.len() {
        let c = chars[at];
        let next = chars.get(at + 1).copied();
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
            Inside::Code => {
                inside = match c {
                    '"' | '\'' => Inside::Quoted(c),
                    '`' => Inside::Template,
                    _ => Inside::Code,
                };
            }
            Inside::Quoted(_) | Inside::Template if c == '\\' => {
                out.push(c);
                out.extend(next);
                at += 2;
                continue;
            }
            Inside::Quoted(quote) if c == quote || c == '\n' => inside = Inside::Code,
            Inside::Template if c == '`' => inside = Inside::Code,
            Inside::Quoted(_) | Inside::Template => {}
        }
        out.push(c);
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
    fn an_escaped_quote_does_not_end_a_string() {
        let out = strip_comments(r#"a("it\"s // not a comment"); // gone"#);
        assert!(out.contains("// not a comment"));
        assert!(!out.contains("gone"));
    }
}
