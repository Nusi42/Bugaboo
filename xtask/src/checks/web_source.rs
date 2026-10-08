//! Reading a line of TypeScript as code, for the checks that look at the browser app
//! (ARCH-WEB-API, LINT-API-TYPES, SEC-COOKIES, STY-TOKENS, THEME-ONLY). The files come from `crate::source`.

/// The files that are not the app: the end-to-end specs, the tooling and the tests.
const NOT_THE_APP: [&str; 3] = ["/e2e/", "web/tooling/", ".test."];

/// Whether the file is part of the browser app or its packages (not a test or a tool).
pub(super) fn is_app_source(path: &str) -> bool {
    !NOT_THE_APP.iter().any(|part| path.contains(part))
}

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
    fn tests_tooling_and_end_to_end_specs_are_not_the_app() {
        assert!(is_app_source("web/app/src/a.tsx"));
        assert!(!is_app_source("web/app/e2e/a.spec.ts"));
        assert!(!is_app_source("web/tooling/a.ts"));
        assert!(!is_app_source("web/packages/ui/src/a.test.ts"));
    }

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
