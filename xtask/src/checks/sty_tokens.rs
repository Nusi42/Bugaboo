//! STY-TOKENS: styles come from design tokens only (ADR-0010, coding-standards §4).
//!
//! In the TypeScript and CSS of the browser app there are no color values written out (`#5b3fd9`,
//! `rgb(…)`, `hsl(…)` and the like) and no inline styles (`style={…}`, `element.style.x = …`,
//! `setAttribute("style", …)`). Colors, sizes, fonts and radii are `var(--token)`; the theme and
//! the choices of the person (spacing, text size) reach every component through the tokens, which
//! an inline value would bypass. Stylelint (LINT-CSS) checks sizes in CSS Modules; this check also
//! covers the TypeScript and the colors everywhere. The generated token files are the one place
//! that holds raw values.

use super::web_source::code_of;
use super::{Check, Context, Finding};
use crate::docs::text::has_extension;
use crate::source::{Kind, Source};

/// The generated files that hold the raw values, and the tests and tooling that are not the app.
const EXEMPT: [&str; 5] = [
    "web/packages/ui/src/tokens.css",
    "web/packages/ui/src/tokens.ts",
    "/e2e/",
    "web/tooling/",
    ".test.",
];

/// Functions that write a color value.
const COLOR_FUNCTIONS: [&str; 9] = [
    "rgb", "rgba", "hsl", "hsla", "hwb", "lab", "lch", "oklab", "oklch",
];

/// Ways to put a style on an element from code.
const INLINE: [&str; 3] = [" style=", ".style.", "setAttribute(\"style\""];

pub(crate) struct StyTokens;

impl Check for StyTokens {
    fn id(&self) -> &'static str {
        "STY-TOKENS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let sources = context.sources(self.id(), Kind::WebAndStyles)?;
        Ok(evaluate(&sources))
    }
}

fn evaluate(sources: &[Source]) -> Vec<Finding> {
    sources
        .iter()
        .filter(|(path, _)| !EXEMPT.iter().any(|part| path.contains(part)))
        .flat_map(|(path, text)| scan(path, text))
        .collect()
}

fn scan(path: &str, text: &str) -> Vec<Finding> {
    let css = has_extension(path, "css");
    let mut findings = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let code = if css { css_code(line) } else { code_of(line) };
        if let Some(problem) = problem_in(code, css) {
            findings.push(Finding::at(format!("{path}:{}", index + 1), problem));
        }
    }
    findings
}

/// A line of CSS without a `/* … */` comment that sits on it.
fn css_code(line: &str) -> &str {
    let trimmed = line.trim_start();
    if trimmed.starts_with("/*") || trimmed.starts_with('*') {
        return "";
    }
    line.split("/*").next().unwrap_or(line)
}

fn problem_in(code: &str, css: bool) -> Option<String> {
    if let Some(color) = hex_color(code) {
        return Some(format!(
            "`{color}` is a color written out. Use a token: `var(--accent)`, `var(--fg)`, … (design/tokens.json)."
        ));
    }
    if let Some(function) = color_function(code) {
        return Some(format!(
            "`{function}(` writes a color out. Use a token: `var(--accent)`, `var(--fg)`, … (design/tokens.json)."
        ));
    }
    if !css && let Some(way) = INLINE.iter().find(|way| code.contains(**way)) {
        return Some(format!(
            "`{}` puts a style on an element from code. Use a CSS Module with tokens; a style the choices of the person cannot reach is a bug.",
            way.trim()
        ));
    }
    None
}

/// A `#rgb`, `#rgba`, `#rrggbb` or `#rrggbbaa` color in the code.
fn hex_color(code: &str) -> Option<&str> {
    code.match_indices('#').find_map(|(at, _)| {
        let rest = &code[at + 1..];
        let digits = rest.chars().take_while(char::is_ascii_hexdigit).count();
        let ends_here = rest[digits..]
            .chars()
            .next()
            .is_none_or(|next| !(next.is_alphanumeric() || next == '_' || next == '-'));
        (matches!(digits, 3 | 4 | 6 | 8) && ends_here).then(|| &code[at..at + 1 + digits])
    })
}

/// The name of a color function that is called in the code, such as `rgb`.
fn color_function(code: &str) -> Option<&'static str> {
    COLOR_FUNCTIONS.iter().copied().find(|name| {
        let call = format!("{name}(");
        code.match_indices(&call).any(|(at, _)| {
            code[..at].chars().last().is_none_or(|before| {
                !(before.is_alphanumeric() || matches!(before, '_' | '-' | '.'))
            })
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(path: &str, text: &str) -> Vec<String> {
        evaluate(&[(path.to_owned(), text.to_owned())])
            .into_iter()
            .map(|finding| finding.message)
            .collect()
    }

    #[test]
    fn bad_example() {
        for (path, bad) in [
            ("web/app/src/a.tsx", "const color = \"#5b3fd9\";"),
            ("web/app/src/a.ts", "const fill = \"#FFF\";"),
            ("web/app/src/a.css", "  color: #5b3fd9;"),
            ("web/app/src/a.css", "  background: rgba(0, 0, 0, 0.5);"),
            ("web/app/src/a.ts", "const c = `oklch(0.5 0.1 200)`;"),
            (
                "web/app/src/a.tsx",
                "return <p style={{ margin: 4 }}>x</p>;",
            ),
            ("web/app/src/a.ts", "element.style.color = \"x\";"),
            (
                "web/app/src/a.ts",
                "element.setAttribute(\"style\", \"x\");",
            ),
        ] {
            assert!(!run(path, bad).is_empty(), "{path}: {bad}");
        }
    }

    #[test]
    fn good_example() {
        for (path, good) in [
            ("web/app/src/a.css", "  color: var(--fg);"),
            ("web/app/src/a.css", "  outline: 2px solid Highlight;"),
            ("web/app/src/a.ts", "const link = `#${id}`;"),
            ("web/app/src/a.ts", "const anchor = \"#content\";"),
            ("web/app/src/a.ts", "// the old color was #5b3fd9"),
            ("web/app/src/a.css", "/* not #5b3fd9 any more */"),
            (
                "web/app/src/a.tsx",
                "return <p className={styles.text}>x</p>;",
            ),
            ("web/app/src/a.ts", "const label = transform(value);"),
            ("web/app/src/a.ts", "const hsl = 3;"),
        ] {
            let found = run(path, good);
            assert!(found.is_empty(), "{path}: {good}: {found:?}");
        }
    }

    #[test]
    fn the_generated_tokens_and_the_tests_are_not_checked() {
        let bad = "  --accent: #5b3fd9;";
        assert!(run("web/packages/ui/src/tokens.css", bad).is_empty());
        assert!(run("web/app/e2e/a.spec.ts", "const c = \"#ffffff\";").is_empty());
        assert!(!run("web/packages/ui/src/other.css", bad).is_empty());
    }
}
