//! Reading a line of TypeScript as code, for the checks that look at the browser app
//! (ARCH-WEB-API, LINT-API-TYPES, SEC-COOKIES, STY-TOKENS). The files come from `crate::source`.

/// The code of a line: without a trailing `//` comment, and empty for comment lines.
pub(super) fn code_of(line: &str) -> &str {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") || trimmed.starts_with('*') || trimmed.starts_with("/*") {
        return "";
    }
    // A `//` inside a string (`"http://…"`) is not a comment; only cut after the last quote
    // before it is balanced. Lines are short, so a simple scan is enough.
    let mut quote: Option<char> = None;
    let mut previous = '\0';
    for (index, character) in line.char_indices() {
        match quote {
            Some(open) if character == open && previous != '\\' => quote = None,
            None if matches!(character, '"' | '\'' | '`') => quote = Some(character),
            None if character == '/' && previous == '/' => return &line[..index - 1],
            _ => {}
        }
        previous = character;
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_are_not_code_but_slashes_in_strings_are() {
        assert_eq!(code_of("// fetch('/api')"), "");
        assert_eq!(code_of(" * fetch(x)"), "");
        assert_eq!(code_of("let a = 1; // fetch(x)"), "let a = 1; ");
        assert_eq!(
            code_of("const u = \"http://x/y\"; // note"),
            "const u = \"http://x/y\"; "
        );
        assert_eq!(code_of("let a = 1;"), "let a = 1;");
    }
}
