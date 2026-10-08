//! ARCH-WEB-FEATURES: the folders of the browser app depend on each other only through their
//! public entry (architecture §4, ADR-0009).
//!
//! * A feature (`web/app/src/features/<name>/`) is reached from outside only through its
//!   `index.ts`; the files inside it stay private, so a feature can change inside without
//!   breaking others.
//! * Shared code (`web/app/src/shared/`) never depends on a feature: features use shared code,
//!   not the other way round.
//! * The shared packages (`web/packages/`) never depend on the app.

use super::web_source::code_of;
use super::{Check, Context, Finding};
use crate::source::{Kind, Source};

const FEATURES: &str = "web/app/src/features/";
const SHARED: &str = "web/app/src/shared/";
const APP: &str = "web/app/";
const PACKAGES: &str = "web/packages/";
/// The files that are a feature's public entry.
const ENTRIES: [&str; 3] = ["index.ts", "index.tsx", "index"];

pub(crate) struct ArchWebFeatures;

impl Check for ArchWebFeatures {
    fn id(&self) -> &'static str {
        "ARCH-WEB-FEATURES"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        Ok(evaluate(&context.sources(self.id(), Kind::Web)?))
    }
}

fn evaluate(sources: &[Source]) -> Vec<Finding> {
    sources
        .iter()
        .flat_map(|(path, text)| scan(path, text))
        .collect()
}

fn scan(path: &str, text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let Some(module) = imported(code_of(line)) else {
            continue;
        };
        if !module.starts_with('.') {
            continue;
        }
        let target = resolve(path, module);
        if let Some(problem) = problem_with(path, &target) {
            findings.push(Finding::at(format!("{path}:{}", index + 1), problem));
        }
    }
    findings
}

/// What is wrong with the file at `path` importing `target` (both repository-relative), if anything.
fn problem_with(path: &str, target: &str) -> Option<String> {
    if let Some(inside) = target.strip_prefix(FEATURES) {
        let (feature, rest) = inside.split_once('/').unwrap_or((inside, ""));
        let same_feature = path
            .strip_prefix(FEATURES)
            .is_some_and(|own| own.split('/').next() == Some(feature));
        if path.starts_with(SHARED) {
            return Some(format!(
                "Shared code imports the feature `{feature}`. Features use shared code, never the other way round."
            ));
        }
        if !same_feature && !ENTRIES.contains(&rest) {
            return Some(format!(
                "`{target}` is inside the feature `{feature}`. Import from `{FEATURES}{feature}/index.ts`, the feature's public entry."
            ));
        }
    }
    if path.starts_with(PACKAGES) && target.starts_with(APP) {
        return Some(format!(
            "A shared package imports `{target}` from the app. The app uses the packages, never the other way round."
        ));
    }
    None
}

/// The module named by `import … from "x"`, `export … from "x"`, `import "x"` or `import("x")`.
fn imported(code: &str) -> Option<&str> {
    let after = code
        .split_once(" from ")
        .map(|(_, rest)| rest)
        .or_else(|| code.trim_start().strip_prefix("import "))
        .or_else(|| code.split_once("import(").map(|(_, rest)| rest))?;
    let rest = after.trim_start().trim_start_matches('(');
    let quote = rest
        .chars()
        .next()
        .filter(|c| matches!(c, '"' | '\'' | '`'))?;
    let inner = &rest[1..];
    inner.split_once(quote).map(|(name, _)| name)
}

/// The repository-relative path a relative import of `module` from the file `from` leads to.
fn resolve(from: &str, module: &str) -> String {
    let mut parts: Vec<&str> = from.split('/').collect();
    parts.pop();
    for step in module.split('/') {
        match step {
            "." | "" => {}
            ".." => {
                parts.pop();
            }
            name => parts.push(name),
        }
    }
    parts.join("/")
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
        // Reaching into another feature.
        let messages = run(
            "web/app/src/features/home/HomePage.tsx",
            "import { Card } from \"../account/Card.tsx\";",
        );
        assert!(
            messages.iter().any(|m| m.contains("public entry")),
            "{messages:?}"
        );
        // The routes reach into a feature's files.
        assert!(
            !run(
                "web/app/src/routes/app-pages.tsx",
                "import { Page } from \"../features/shell/Page.tsx\";",
            )
            .is_empty()
        );
        // Shared code depends on a feature, even through its entry.
        let messages = run(
            "web/app/src/shared/session.ts",
            "import { Frame } from \"../features/shell/index.ts\";",
        );
        assert!(
            messages.iter().any(|m| m.contains("Shared code")),
            "{messages:?}"
        );
        // A package depends on the app.
        assert!(
            !run(
                "web/packages/ui/src/Button.tsx",
                "import { paths } from \"../../../app/src/shared/paths.ts\";",
            )
            .is_empty()
        );
    }

    #[test]
    fn good_example() {
        for (path, line) in [
            (
                "web/app/src/routes/layouts.tsx",
                "} from \"../features/shell/index.ts\";",
            ),
            (
                "web/app/src/features/setup/SaferStep.tsx",
                "import { CodeAppSection } from \"../account/index.ts\";",
            ),
            (
                "web/app/src/features/shell/AppLayout.tsx",
                "import styles from \"./AppLayout.module.css\";",
            ),
            (
                "web/app/src/features/shell/AppLayout.tsx",
                "import { Sidebar } from \"./sidebar/Sidebar.tsx\";",
            ),
            (
                "web/app/src/features/account/Row.tsx",
                "import { api } from \"../../shared/api.ts\";",
            ),
            (
                "web/app/src/routes/app-pages.tsx",
                "component: lazyRouteComponent(() => import(\"../features/admin/index.ts\"), \"Page\"),",
            ),
            (
                "web/packages/ui/src/Link.tsx",
                "import { cx } from \"./cx.ts\";",
            ),
        ] {
            let found = run(path, line);
            assert!(found.is_empty(), "{path}: {line}: {found:?}");
        }
    }

    #[test]
    fn resolving_follows_the_dots() {
        assert_eq!(
            resolve("web/app/src/routes/a.tsx", "../features/x/index.ts"),
            "web/app/src/features/x/index.ts"
        );
        assert_eq!(
            resolve("web/app/src/features/x/a.ts", "./b/c.ts"),
            "web/app/src/features/x/b/c.ts"
        );
    }
}
