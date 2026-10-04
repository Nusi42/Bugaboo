//! Reading the TypeScript sources of the browser app for the checks that look at code
//! (ARCH-WEB-API, LINT-API-TYPES, SEC-COOKIES).

use std::path::Path;

use anyhow::Context as _;

use crate::repo::Repo;

/// One TypeScript file: its repository-relative path and its text.
pub(super) type WebSource = (String, String);

/// The folders whose files are never product sources.
const SKIPPED: [&str; 3] = ["node_modules", "dist", "fixtures"];

/// Every `.ts` and `.tsx` file below `web/`.
pub(super) fn read_web_sources(repo: &Repo) -> anyhow::Result<Vec<WebSource>> {
    let mut sources = Vec::new();
    for file in repo.all_files()? {
        let is_source = file.starts_with("web/")
            && Path::new(&file)
                .extension()
                .is_some_and(|ext| ext == "ts" || ext == "tsx")
            && !file.split('/').any(|part| SKIPPED.contains(&part));
        if is_source {
            let text = std::fs::read_to_string(repo.path(&file))
                .with_context(|| format!("could not read {file}"))?;
            sources.push((file, text.replace("\r\n", "\n")));
        }
    }
    Ok(sources)
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
