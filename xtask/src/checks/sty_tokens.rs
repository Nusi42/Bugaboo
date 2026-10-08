//! STY-TOKENS: colors come from design tokens only (ADR-0010, ADR-0026, coding-standards §4).
//!
//! In `design/theme.kdl` a color value is written once, in the `color` token that names it; no rule
//! of a component holds one (`#5b3fd9`, `rgb(…)`, `hsl(…)` and the like). In the TypeScript and
//! the remaining CSS of the browser app there are none written out either. A rule uses
//! `$accent`, `$fg`, … so that the theme and the choices of the person (spacing, text size) reach
//! every component. Styles from code (`style=`) are THEME-ONLY's, sizes and fonts in the rules are
//! LINT-CSS's. The generated files are the one place outside the theme file that holds raw values.

use super::web_source::{code_of, is_app_source};
use super::{Check, Context, Finding};
use crate::docs::text::has_extension;
use crate::generate::all_paths;
use crate::generate::theme_file::{Item, THEME_PATH, ThemeFile};
use crate::source::{Kind, Source};

/// Functions that write a color value.
const COLOR_FUNCTIONS: [&str; 9] = [
    "rgb", "rgba", "hsl", "hsla", "hwb", "lab", "lch", "oklab", "oklch",
];

pub(crate) struct StyTokens;

impl Check for StyTokens {
    fn id(&self) -> &'static str {
        "STY-TOKENS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let sources = context.sources(self.id(), Kind::WebAndStyles)?;
        let mut findings = evaluate(&sources);
        findings.extend(theme_findings(&ThemeFile::load(context.repo)?));
        Ok(findings)
    }
}

fn evaluate(sources: &[Source]) -> Vec<Finding> {
    let generated = all_paths();
    sources
        .iter()
        .filter(|(path, _)| is_app_source(path) && !generated.contains(&path.as_str()))
        .flat_map(|(path, text)| scan(path, text))
        .collect()
}

/// The colors written out in the rules, the mixins and the keyframes of the theme file.
fn theme_findings(file: &ThemeFile) -> Vec<Finding> {
    let mut findings = Vec::new();
    let in_rules = file.rules.iter().map(|rule| &rule.items);
    let in_mixins = file.mixins.iter().map(|mixin| &mixin.items);
    for items in in_rules.chain(in_mixins) {
        check_items(items, &mut findings);
    }
    for keyframes in &file.keyframes {
        for (_, declarations) in &keyframes.frames {
            for declaration in declarations {
                check_value(&declaration.value, declaration.line, &mut findings);
            }
        }
    }
    findings
}

fn check_items(items: &[Item], findings: &mut Vec<Finding>) {
    for item in items {
        match item {
            Item::Declaration(declaration) => {
                check_value(&declaration.value, declaration.line, findings);
            }
            Item::Media { items, .. } => check_items(items, findings),
            Item::Use { .. } => {}
        }
    }
}

fn check_value(value: &str, line: usize, findings: &mut Vec<Finding>) {
    if let Some(problem) = problem_in(value, true) {
        findings.push(Finding::at(format!("{THEME_PATH}:{line}"), problem));
    }
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

fn problem_in(code: &str, in_rule: bool) -> Option<String> {
    let how = if in_rule {
        "Define it as a `color` token in design/theme.kdl and write `$name` here."
    } else {
        "Use a token: `var(--accent)`, `var(--fg)`, … (design/theme.kdl)."
    };
    if let Some(color) = hex_color(code) {
        return Some(format!("`{color}` is a color written out. {how}"));
    }
    color_function(code).map(|function| format!("`{function}(` writes a color out. {how}"))
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

    fn in_theme(rule: &str) -> Vec<String> {
        let repo = crate::repo::Repo::discover().unwrap();
        let text = crate::source::read_text(&repo, THEME_PATH).unwrap();
        let file = ThemeFile::parse(&format!("{text}\n{rule}\n")).unwrap();
        theme_findings(&file)
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
        ] {
            assert!(!run(path, bad).is_empty(), "{path}: {bad}");
        }
        assert!(!in_theme("part \"zz\" {\n  color \"#5b3fd9\"\n}").is_empty());
        assert!(!in_theme("part \"zz\" {\n  background \"rgb(0 0 0 / 0.5)\"\n}").is_empty());
        assert!(!in_theme("mixin \"zz\" {\n  color \"#fff\"\n}").is_empty());
        assert!(!in_theme("part \"zz\" {\n  forced {\n    color \"#fff\"\n  }\n}").is_empty());
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
            ("web/app/src/a.ts", "const label = transform(value);"),
            ("web/app/src/a.ts", "const hsl = 3;"),
        ] {
            let found = run(path, good);
            assert!(found.is_empty(), "{path}: {good}: {found:?}");
        }
        assert!(
            in_theme("part \"zz\" {\n  color \"$fg\"\n  outline \"$ring-width solid $accent\"\n}")
                .is_empty()
        );
    }

    #[test]
    fn the_theme_file_of_the_repository_holds_no_color_in_a_rule() {
        let repo = crate::repo::Repo::discover().unwrap();
        let file = ThemeFile::load(&repo).unwrap();
        let found: Vec<String> = theme_findings(&file)
            .into_iter()
            .map(|finding| finding.message)
            .collect();
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn the_generated_sheet_and_the_tests_are_not_checked() {
        let bad = "  --accent: #5b3fd9;";
        assert!(run("web/packages/ui/src/theme.css", bad).is_empty());
        assert!(run("web/app/e2e/a.spec.ts", "const c = \"#ffffff\";").is_empty());
        assert!(!run("web/packages/ui/src/other.css", bad).is_empty());
    }
}
