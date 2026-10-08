//! THEME-ONLY: the look of the app lives in `design/theme.kdl` and nowhere else (ADR-0026).
//!
//! In the browser app and its packages there is
//!
//! - no `style=` and no other way to put a style or a class on the page from code (`.style`,
//!   `setAttribute("style" or "class", …)`, `classList`, `<style>`, `createElement("style")`,
//!   `CSSStyleSheet`, `insertRule`, `adoptedStyleSheets`, `attributeStyleMap`, a `style: {…}`
//!   object): a value set from code bypasses
//!   the tokens, and a style that is put on the page at run time needs a hole in the content
//!   security policy (`style-src 'self'`);
//! - no `className`: a component does not name classes. It puts `data-part`, `data-variant` and
//!   `data-state` on its elements (`partOf`), and React Aria's own `data-*` state attributes are
//!   styled by the rules of the theme file. React Aria's default class names are allowed because
//!   nothing writes them;
//! - no style file but the one that `cargo xtask gen theme` writes (no `*.module.css`, `.scss`,
//!   `.sass`, `.less`), and no
//!   `import`, `import()` or `require` of one, with or without `?inline` or `?raw` after the name.
//!
//! Nothing is exempt: the components moved onto the theme file in package C-06 (M2C-F90).

use std::path::Path;

use super::web_source::{code_of, is_app_source};
use super::{Check, Context, Finding};
use crate::docs::text::has_extension;
use crate::generate::all_paths;
use crate::generate::theme::STYLE_SHEET;
use crate::source::{Kind, Source};

/// Ways to put a style on an element, or a style sheet on the page, from code: the text to find
/// and what it is.
const FROM_CODE: [(&str, &str); 11] = [
    (" style=", "`style=`"),
    (".style.", "`.style.`"),
    (".style =", "`.style =`"),
    (".style=", "`.style=`"),
    (".style[", "`.style[`"),
    ("<style", "a `<style` element"),
    ("classList", "`classList`"),
    ("CSSStyleSheet", "a `CSSStyleSheet`"),
    ("insertRule", "`insertRule`"),
    ("adoptedStyleSheets", "`adoptedStyleSheets`"),
    ("attributeStyleMap", "`attributeStyleMap`"),
];

/// The attributes that `setAttribute` and `createElement` may not be given.
const STYLE_ATTRIBUTES: [&str; 2] = ["style", "class"];

/// The endings of the style sheets of every kind.
const STYLE_FILES: [&str; 4] = ["css", "scss", "sass", "less"];

pub(crate) struct ThemeOnly;

impl Check for ThemeOnly {
    fn id(&self) -> &'static str {
        "THEME-ONLY"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let sources = context.sources(self.id(), Kind::WebAndStyles)?;
        Ok(evaluate(&sources))
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

/// What breaks the rule in one file.
fn scan(path: &str, text: &str) -> Vec<Finding> {
    if STYLE_FILES.iter().any(|ending| has_extension(path, ending)) {
        return vec![Finding::at(
            path,
            format!(
                "A CSS file of its own. All styles are rules of the theme file: write them as `part` rules in design/theme.kdl and run `cargo xtask gen theme`, which writes {STYLE_SHEET}."
            ),
        )];
    }
    let mut findings = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let code = code_of(line);
        if let Some(problem) = problem_in(code) {
            findings.push(Finding::at(format!("{path}:{}", index + 1), problem));
        }
    }
    findings
}

fn problem_in(code: &str) -> Option<String> {
    if let Some((_, what)) = FROM_CODE.iter().find(|(text, _)| code.contains(text)) {
        return Some(format!(
            "{what} puts a style or a class on the page from code. Write a `part` rule with tokens in design/theme.kdl; a style the choices of the person cannot reach is a bug (ADR-0026)."
        ));
    }
    if let Some(call) = names_style_attribute(code) {
        return Some(format!(
            "`{call}` puts a style or a class on the page from code. Write a `part` rule with tokens in design/theme.kdl (ADR-0026)."
        ));
    }
    if has_style_object(code) {
        return Some(
            "A `style:` object handed to an element. Write a `part` rule with tokens in design/theme.kdl (ADR-0026)."
                .to_owned(),
        );
    }
    if contains_word(code, "className") {
        return Some(
            "`className`: a component does not name classes. Put `{...partOf(part.name)}` on the element and style the part in design/theme.kdl (ADR-0026).".to_owned(),
        );
    }
    if imports_css(code) {
        return Some(format!(
            "Imports a CSS file. The only style sheet is {STYLE_SHEET}, made from design/theme.kdl (ADR-0026)."
        ));
    }
    None
}

/// Whether `word` stands in the code as a whole word.
fn contains_word(code: &str, word: &str) -> bool {
    code.match_indices(word).any(|(at, _)| {
        let before = code[..at].chars().next_back();
        let after = code[at + word.len()..].chars().next();
        let part_of_name = |c: char| c.is_alphanumeric() || c == '_';
        !before.is_some_and(part_of_name) && !after.is_some_and(part_of_name)
    })
}

/// `setAttribute("style", …)`, `setAttribute('class', …)` and `createElement("style")`, written
/// with any of the three quotes and spaces after the bracket: the call that was found.
fn names_style_attribute(code: &str) -> Option<String> {
    ["setAttribute(", "createElement("].iter().find_map(|call| {
        code.match_indices(call).find_map(|(at, _)| {
            let rest = code[at + call.len()..].trim_start();
            let quote = rest
                .chars()
                .next()
                .filter(|c| matches!(c, '"' | '\'' | '`'))?;
            let name = rest[1..].split(quote).next()?;
            let bad = if *call == "createElement(" {
                name == "style"
            } else {
                STYLE_ATTRIBUTES.contains(&name)
            };
            bad.then(|| format!("{call}{quote}{name}{quote}…"))
        })
    })
}

/// A `style: {` or `style: [`: an object of styles given to an element (a `style:` that holds
/// anything else, like the option of `Intl.ListFormat`, is not one).
fn has_style_object(code: &str) -> bool {
    code.match_indices("style").any(|(at, word)| {
        let before = code[..at].chars().next_back();
        let after = code[at + word.len()..].trim_start();
        !before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.')
            && after
                .strip_prefix(':')
                .is_some_and(|value| value.trim_start().starts_with(['{', '[']))
    })
}

/// Whether the line imports (or `import()`s, or `require`s) a style sheet other than the generated
/// one, whatever follows the file name (`?inline`, `?raw`, `#hash`).
fn imports_css(code: &str) -> bool {
    let trimmed = code.trim_start();
    let loads = trimmed.starts_with("import")
        || trimmed.starts_with("export")
        || code.contains("import(")
        || code.contains("require(");
    if !loads {
        return false;
    }
    let sheet = Path::new(STYLE_SHEET)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    code.split(['"', '\'', '`']).any(|part| {
        let file = part.split(['?', '#']).next().unwrap_or(part);
        STYLE_FILES.iter().any(|ending| has_extension(file, ending)) && !file.ends_with(sheet)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(path: &str, text: &str) -> Vec<String> {
        let sources = [(path.to_owned(), text.to_owned())];
        evaluate(&sources)
            .into_iter()
            .map(|finding| finding.message)
            .collect()
    }

    #[test]
    fn bad_example_the_other_ways_to_put_a_style_on_the_page() {
        for bad in [
            "node.classList.add(\"x\");",
            "node.setAttribute('style', value);",
            "node.setAttribute( \"class\", value);",
            "node.setAttribute(`style`, value);",
            "node.style = value;",
            "node.style=value;",
            "const top = node.style[name];",
            "const sheet = new CSSStyleSheet();",
            "sheet.insertRule(rule);",
            "document.adoptedStyleSheets = [sheet];",
            "const tag = document.createElement(\"style\");",
            "node.attributeStyleMap.set(\"color\", x);",
            "const props = { style: { color: x } };",
            "return <style>{css}</style>;",
            "import sheet from \"./a.css?inline\";",
            "import raw from \"./a.css?raw\";",
            "const sheet = await import(\"./a.css\");",
            "import \"./a.scss\";",
            "import \"./a.sass\";",
            "import \"./a.less\";",
            "const css = require(\"./a.css\");",
        ] {
            assert!(!run("web/app/src/a.ts", bad).is_empty(), "{bad}");
        }
        for path in [
            "web/app/src/a.scss",
            "web/app/src/a.sass",
            "web/app/src/a.less",
        ] {
            assert!(!run(path, "a { b: c }").is_empty(), "{path}");
        }
    }

    #[test]
    fn good_example_things_that_only_look_like_a_style() {
        for good in [
            "new Intl.ListFormat(locale, { style: \"long\" })",
            "const lifestyle = 1;",
            "node.setAttribute(\"data-part\", name);",
            "document.createElement(\"div\");",
            "import \"@bugaboo/ui/theme.css?v=1\";",
            "const rule = insert(rule);",
        ] {
            let found = run("web/app/src/a.ts", good);
            assert!(found.is_empty(), "{good}: {found:?}");
        }
    }

    #[test]
    fn bad_example() {
        for (path, bad) in [
            (
                "web/app/src/a.tsx",
                "return <p style={{ margin: 4 }}>x</p>;",
            ),
            ("web/app/src/a.ts", "element.style.color = \"x\";"),
            (
                "web/app/src/a.ts",
                "element.setAttribute(\"style\", \"x\");",
            ),
            ("web/app/src/a.tsx", "return <p className=\"stack\">x</p>;"),
            (
                "web/app/src/a.tsx",
                "return <p className={styles.text}>x</p>;",
            ),
            (
                "web/app/src/a.tsx",
                "return <p className={`row ${wide}`}>x</p>;",
            ),
            (
                "web/app/src/a.tsx",
                "import styles from \"./Page.module.css\";",
            ),
            ("web/app/src/a.tsx", "import \"./extra.css\";"),
            ("web/app/src/Page.module.css", ".stack { display: flex; }"),
            ("web/app/src/global.css", "body { margin: 0; }"),
        ] {
            assert!(!run(path, bad).is_empty(), "{path}: {bad}");
        }
    }

    #[test]
    fn good_example() {
        for (path, good) in [
            (
                "web/app/src/a.tsx",
                "return <p {...partOf(part.muted)}>x</p>;",
            ),
            ("web/app/src/a.tsx", "import \"@bugaboo/ui/theme.css\";"),
            ("web/app/src/a.ts", "const link = `#${id}`;"),
            ("web/app/src/a.ts", "// the old className was removed"),
            ("web/app/src/a.ts", "const classNames = names;"),
            ("web/app/src/a.ts", "const styleName = \"quiet\";"),
            // Tests, tooling and the end-to-end specs are not the app.
            ("web/app/e2e/a.spec.ts", "await page.locator('[style]');"),
            ("web/tooling/a.ts", "element.style.color = \"x\";"),
            ("web/app/src/a.test.tsx", "<p className=\"x\" />"),
        ] {
            let found = run(path, good);
            assert!(found.is_empty(), "{path}: {good}: {found:?}");
        }
    }

    #[test]
    fn the_generated_sheet_is_the_one_css_file() {
        assert!(run(STYLE_SHEET, "body { color: red; }").is_empty());
    }
}
