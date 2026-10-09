//! SEC-RICHTEXT: text written by people is rendered only by `bb-markdown` (SEC-13, ADR-0011,
//! coding-standards §4).
//!
//! One renderer means one place to make safe, and a preview that is what others see. This check
//! holds that in five ways:
//!
//! - no crate of the product but `bb-markdown` depends on a Markdown or HTML library (`xtask` turns
//!   the Markdown docs into pages and is not the product);
//! - the browser app has no Markdown or HTML library either (no second renderer);
//! - HTML is set from a string in one file only, `markdown/policy.ts` of the UI package, which
//!   `MarkdownView` uses: no `innerHTML`, `outerHTML`, `insertAdjacentHTML`,
//!   `createContextualFragment`, `DOMParser` or `dangerouslySetInnerHTML` anywhere else;
//! - the name of the Trusted Types policy in that file is the one the content security policy of
//!   the server names (`trusted-types <name>`), so a browser lets exactly that policy make HTML;
//! - the corpus tests exist: the native one in `bb-markdown` and the one that runs the same files
//!   through the WebAssembly build. They run in CI like every test.

use super::web_source::code_of;
use super::{Check, Context, Finding};
use crate::metadata::{self, Package};
use crate::source::{Kind, Source, read_text};

/// The crate that renders.
const RENDERER: &str = "bb-markdown";
/// Crates that are not the product.
const NOT_THE_PRODUCT: [&str; 1] = ["xtask"];
/// Libraries that parse Markdown or HTML, or clean it. Only `bb-markdown` may use the ones that
/// parse Markdown.
const RUST_LIBRARIES: [&str; 8] = [
    "pulldown-cmark",
    "ammonia",
    "comrak",
    "markdown",
    "html5ever",
    "markup5ever",
    "scraper",
    "lol_html",
];
/// The same for the browser app.
const WEB_LIBRARIES: [&str; 12] = [
    "marked",
    "markdown-it",
    "showdown",
    "snarkdown",
    "micromark",
    "remark",
    "react-markdown",
    "rehype",
    "dompurify",
    "sanitize-html",
    "mdast",
    "prosemirror-markdown",
];

/// The one file that sets HTML from a string.
const POLICY_FILE: &str = "web/packages/ui/src/markdown/policy.ts";
/// What sets HTML from a string.
const HTML_SETTERS: [&str; 6] = [
    "innerHTML",
    "outerHTML",
    "insertAdjacentHTML",
    "createContextualFragment",
    "DOMParser",
    "dangerouslySetInnerHTML",
];
/// The line of the policy file that names the policy, and the directives of the header.
const POLICY_NAME_MARK: &str = "trustedTypesPolicyName =";
const HEADERS_FILE: &str = "crates/bb-api/src/headers.rs";
const TRUSTED_TYPES_REQUIRED: &str = "require-trusted-types-for 'script'";
const TRUSTED_TYPES_ALLOWED: &str = "trusted-types ";

/// The tests that must exist, with the file and the name.
const CORPUS_TESTS: [(&str, &str); 2] = [
    (
        "crates/bb-markdown/tests/render.rs",
        "fn sec_13_the_sanitizer_corpus_renders_every_attack_as_text",
    ),
    (
        "web/tooling/markdown-corpus.test.ts",
        "sec_13_server_and_browser_render_the_same_html",
    ),
];

pub(crate) struct SecRichtext;

impl Check for SecRichtext {
    fn id(&self) -> &'static str {
        "SEC-RICHTEXT"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let repo = context.repo;
        let web = context.everything().sources(self.id(), Kind::Web)?;
        let mut findings = rust_dependency_findings(&metadata::load(repo.root())?);
        findings.extend(web_dependency_findings(&package_manifests(context)?));
        findings.extend(html_setter_findings(&web));
        findings.extend(policy_findings(
            &read_text(repo, POLICY_FILE)?,
            &read_text(repo, HEADERS_FILE)?,
        ));
        let missing = |file: &str| read_text(repo, file).unwrap_or_default();
        findings.extend(test_findings(missing));
        Ok(findings)
    }
}

/// The `package.json` files of the root and of the browser packages, with their text.
fn package_manifests(context: &Context<'_>) -> anyhow::Result<Vec<Source>> {
    let mut manifests = Vec::new();
    for file in context.repo.all_files()? {
        let is_manifest = file == "package.json"
            || (file.starts_with("web/") && file.ends_with("/package.json"))
                && !file.contains("node_modules");
        if is_manifest {
            let text = read_text(context.repo, &file)?;
            manifests.push((file, text));
        }
    }
    Ok(manifests)
}

fn rust_dependency_findings(packages: &[Package]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for package in packages
        .iter()
        .filter(|p| p.name != RENDERER && !NOT_THE_PRODUCT.contains(&p.name.as_str()))
    {
        for dependency in package
            .dependencies
            .iter()
            .filter(|d| RUST_LIBRARIES.contains(&d.name.as_str()))
        {
            findings.push(Finding::at(
                &package.name,
                format!(
                    "depends on `{}`. Text written by people is rendered only by `{RENDERER}` (SEC-RICHTEXT, ADR-0011): call it instead of using a second library.",
                    dependency.name
                ),
            ));
        }
    }
    findings
}

fn web_dependency_findings(manifests: &[Source]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (path, text) in manifests {
        for library in WEB_LIBRARIES {
            if text.contains(&format!("\"{library}\":")) {
                findings.push(Finding::at(
                    path,
                    format!(
                        "uses `{library}`. There is no other renderer in the browser: the preview is `bb-markdown` in WebAssembly (SEC-RICHTEXT, ADR-0011)."
                    ),
                ));
            }
        }
    }
    findings
}

fn html_setter_findings(sources: &[Source]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (path, text) in sources.iter().filter(|(path, _)| path != POLICY_FILE) {
        for (number, line) in text.lines().enumerate() {
            let code = code_of(line);
            for setter in HTML_SETTERS.iter().filter(|setter| code.contains(*setter)) {
                findings.push(Finding::at(
                    format!("{path}:{}", number + 1),
                    format!(
                        "`{setter}` makes HTML from a string. Only `MarkdownView` shows HTML, through the policy in {POLICY_FILE} (SEC-RICHTEXT)."
                    ),
                ));
            }
        }
    }
    findings
}

/// The policy named in the file of the policy must be the one the server allows.
fn policy_findings(policy_source: &str, headers_source: &str) -> Vec<Finding> {
    let name = policy_source
        .lines()
        .find(|line| line.contains(POLICY_NAME_MARK))
        .and_then(|line| line.split('"').nth(1));
    let Some(name) = name else {
        return vec![Finding::at(
            POLICY_FILE,
            "The file must name the policy: `export const trustedTypesPolicyName = \"…\"`.",
        )];
    };
    let mut findings = Vec::new();
    if !headers_source.contains(TRUSTED_TYPES_REQUIRED) {
        findings.push(Finding::at(
            HEADERS_FILE,
            format!("The content security policy must say `{TRUSTED_TYPES_REQUIRED}`."),
        ));
    }
    let allowed = format!("{TRUSTED_TYPES_ALLOWED}{name}");
    if !headers_source.contains(&allowed) {
        findings.push(Finding::at(
            HEADERS_FILE,
            format!("The content security policy must say `{allowed}`, the policy of {POLICY_FILE}, or the browser refuses the HTML of MarkdownView."),
        ));
    }
    findings
}

fn test_findings(read: impl Fn(&str) -> String) -> Vec<Finding> {
    CORPUS_TESTS
        .iter()
        .filter(|(file, name)| !read(file).contains(name))
        .map(|(file, name)| {
            Finding::at(
                *file,
                format!("The test `{name}` must exist: it is the proof that every attack renders as text (SEC-13)."),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::fixtures::package;

    fn source(path: &str, text: &str) -> Vec<Source> {
        vec![(path.to_owned(), text.to_owned())]
    }

    const POLICY: &str =
        "export const trustedTypesPolicyName = \"bugaboo-markdown\";\nelement.innerHTML = x;\n";
    const HEADERS: &str = "require-trusted-types-for 'script'; trusted-types bugaboo-markdown";

    #[test]
    fn good_example_the_renderer_the_policy_and_the_tests() {
        assert!(
            rust_dependency_findings(&[
                package("bb-markdown", &["pulldown-cmark"]),
                package("xtask", &["pulldown-cmark"]),
                package("bb-api", &["bb-core"]),
            ])
            .is_empty()
        );
        assert!(web_dependency_findings(&source("package.json", "{\"vite\": \"1\"}")).is_empty());
        assert!(html_setter_findings(&source(POLICY_FILE, POLICY)).is_empty());
        assert!(
            html_setter_findings(&source("web/app/src/a.ts", "// innerHTML in a comment\n"))
                .is_empty()
        );
        assert!(policy_findings(POLICY, HEADERS).is_empty());
        assert!(
            test_findings(|_| CORPUS_TESTS.iter().map(|(_, n)| *n).collect::<String>()).is_empty()
        );
    }

    #[test]
    fn bad_example_a_second_renderer_in_rust() {
        let found = rust_dependency_findings(&[package("bb-api", &["pulldown-cmark", "ammonia"])]);
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn bad_example_a_second_renderer_in_the_browser() {
        let manifest = source(
            "web/app/package.json",
            "{\"dependencies\": {\"marked\": \"1\", \"dompurify\": \"1\"}}",
        );
        assert_eq!(web_dependency_findings(&manifest).len(), 2);
    }

    #[test]
    fn bad_example_html_set_from_a_string_outside_the_policy_file() {
        for line in [
            "box.innerHTML = html;",
            "<div dangerouslySetInnerHTML={{ __html: html }} />",
            "box.insertAdjacentHTML('beforeend', html);",
            "new DOMParser().parseFromString(html, 'text/html');",
        ] {
            assert_eq!(
                html_setter_findings(&source("web/app/src/a.tsx", line)).len(),
                1,
                "{line}"
            );
        }
    }

    #[test]
    fn bad_example_the_header_does_not_allow_the_policy() {
        assert_eq!(
            policy_findings(
                POLICY,
                "require-trusted-types-for 'script'; trusted-types other"
            )
            .len(),
            1
        );
        assert_eq!(policy_findings(POLICY, "default-src 'self'").len(), 2);
        assert_eq!(policy_findings("const x = 1;", HEADERS).len(), 1);
    }

    #[test]
    fn bad_example_a_missing_corpus_test() {
        assert_eq!(test_findings(|_| String::new()).len(), 2);
    }
}
