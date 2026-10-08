//! THEME-ONLY: the look of the app lives in `design/theme.kdl` and nowhere else (ADR-0026).
//!
//! In the browser app and its packages there is
//!
//! - no `style=` (and no `.style.` or `setAttribute("style", …)`): a value set from code bypasses
//!   the tokens, and a style that is put on the page at run time needs a hole in the content
//!   security policy (`style-src 'self'`);
//! - no `className`: a component does not name classes. It puts `data-part`, `data-variant` and
//!   `data-state` on its elements (`partOf`), and React Aria's own `data-*` state attributes are
//!   styled by the rules of the theme file. React Aria's default class names are allowed because
//!   nothing writes them;
//! - no CSS file but the one that `cargo xtask gen theme` writes, so no `*.module.css`, and no
//!   `import` of one.
//!
//! The components that have not been moved yet stand in `tools/checks/theme-only-exemptions.toml`,
//! each with the follow-up that moves it. An entry whose files no longer break the rule fails, so
//! the list shrinks to nothing as the components move.

use std::path::Path;

use serde::Deserialize;

use super::exemptions::{self, Used};
use super::web_source::{code_of, is_app_source};
use super::{Check, Context, Finding};
use crate::docs::text::has_extension;
use crate::generate::all_paths;
use crate::generate::theme::STYLE_SHEET;
use crate::source::{Kind, Source};

/// The components that still use classes or CSS, each with the follow-up that moves it.
const EXEMPTIONS_PATH: &str = "tools/checks/theme-only-exemptions.toml";

/// Ways to put a style on an element from code.
const INLINE: [&str; 3] = [" style=", ".style.", "setAttribute(\"style\""];

pub(crate) struct ThemeOnly;

impl Check for ThemeOnly {
    fn id(&self) -> &'static str {
        "THEME-ONLY"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let sources = context.sources(self.id(), Kind::WebAndStyles)?;
        let exemptions: Exemptions = exemptions::load(context.repo, EXEMPTIONS_PATH)?;
        Ok(evaluate(&sources, &exemptions.exempt))
    }
}

/// The file of components that stand for now.
#[derive(Deserialize)]
struct Exemptions {
    #[serde(default)]
    exempt: Vec<Exempt>,
}

/// A file, or every file below a folder (the path ends with `/`), or every file whose name starts
/// with the path (`web/packages/ui/src/Card.` is `Card.tsx` and `Card.module.css`).
#[derive(Deserialize)]
struct Exempt {
    path: String,
    reason: String,
}

impl exemptions::Exemption for Exempt {
    fn name(&self) -> String {
        self.path.clone()
    }

    fn reason(&self) -> &str {
        &self.reason
    }

    fn unused(&self) -> String {
        "covers no file that breaks the rule any more. Remove it.".to_owned()
    }
}

fn evaluate(sources: &[Source], exemptions: &[Exempt]) -> Vec<Finding> {
    let generated = all_paths();
    let mut used = Used::default();
    let mut findings = Vec::new();
    for (path, text) in sources
        .iter()
        .filter(|(path, _)| is_app_source(path) && !generated.contains(&path.as_str()))
    {
        let found = scan(path, text);
        if found.is_empty() {
            continue;
        }
        let covering: Vec<usize> = exemptions
            .iter()
            .enumerate()
            .filter(|(_, exempt)| path.starts_with(&exempt.path))
            .map(|(index, _)| index)
            .collect();
        if covering.is_empty() {
            findings.extend(found);
        }
        for index in covering {
            used.mark(index);
        }
    }
    findings.extend(exemptions::judge(EXEMPTIONS_PATH, exemptions, &used));
    findings.extend(exemptions::follow_up_findings(EXEMPTIONS_PATH, exemptions));
    findings
}

/// What breaks the rule in one file.
fn scan(path: &str, text: &str) -> Vec<Finding> {
    if has_extension(path, "css") {
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
    if let Some(way) = INLINE.iter().find(|way| code.contains(**way)) {
        return Some(format!(
            "`{}` puts a style on an element from code. Write a `part` rule with tokens in design/theme.kdl; a style the choices of the person cannot reach is a bug (ADR-0026).",
            way.trim()
        ));
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

/// Whether the line imports a `.css` file other than the generated sheet.
fn imports_css(code: &str) -> bool {
    let trimmed = code.trim_start();
    if !trimmed.starts_with("import") {
        return false;
    }
    let sheet = Path::new(STYLE_SHEET)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    trimmed.split(['"', '\'']).any(|part| {
        Path::new(part)
            .extension()
            .is_some_and(|ending| ending == "css")
            && !part.ends_with(sheet)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(path: &str, text: &str) -> Vec<String> {
        let sources = [(path.to_owned(), text.to_owned())];
        evaluate(&sources, &[])
            .into_iter()
            .map(|finding| finding.message)
            .collect()
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

    #[test]
    fn an_exemption_covers_a_folder_or_a_name_and_a_stale_one_fails() {
        let sources = [
            (
                "web/app/src/features/shell/Shell.module.css".to_owned(),
                ".a {}".to_owned(),
            ),
            (
                "web/packages/ui/src/Card.tsx".to_owned(),
                "<div className={x} />".to_owned(),
            ),
        ];
        let exempt = |path: &str| Exempt {
            path: path.to_owned(),
            reason: "Moves with the shell (M2C-F90), package C-06.".to_owned(),
        };
        let covering = [
            exempt("web/app/src/features/shell/"),
            exempt("web/packages/ui/src/Card."),
        ];
        assert!(evaluate(&sources, &covering).is_empty());
        let stale = [
            exempt("web/app/src/features/shell/"),
            exempt("web/packages/ui/src/Card."),
            exempt("web/app/src/features/gone/"),
        ];
        let messages: Vec<String> = evaluate(&sources, &stale)
            .into_iter()
            .map(|finding| finding.message)
            .collect();
        assert_eq!(messages.len(), 1, "{messages:?}");
        assert!(messages[0].contains("features/gone/"), "{messages:?}");
    }

    #[test]
    fn an_exemption_without_a_follow_up_fails() {
        let exempt = Exempt {
            path: "web/app/src/features/shell/".to_owned(),
            reason: "Moves later, when somebody has time for it.".to_owned(),
        };
        let sources = [(
            "web/app/src/features/shell/Shell.module.css".to_owned(),
            ".a {}".to_owned(),
        )];
        let messages: Vec<String> = evaluate(&sources, &[exempt])
            .into_iter()
            .map(|finding| finding.message)
            .collect();
        assert!(
            messages.iter().any(|message| message.contains("follow-up")),
            "{messages:?}"
        );
    }
}
