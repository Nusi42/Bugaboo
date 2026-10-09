//! SEC-RICHTEXT: text written by people is rendered only by `bb-markdown` (SEC-13, ADR-0011,
//! coding-standards §4).
//!
//! One renderer means one place to make safe, and a preview that is what others see. This check
//! holds that in these ways:
//!
//! - no crate of the product but `bb-markdown` depends on a Markdown or HTML library, and
//!   `bb-markdown` itself uses only the Markdown parser in `[dependencies]` (an HTML parser may be
//!   a `[dev-dependencies]` of its tests, as the oracle that reads the HTML the writer made);
//!   `xtask` turns the Markdown docs into pages and is not the product;
//! - the browser app has no Markdown or HTML library either (no second renderer): the names are
//!   matched as families (`remark`, `remark-gfm`, `@types/dompurify`, …);
//! - HTML is set from a string in one file only, `markdown/policy.ts` of the UI package, which
//!   `MarkdownView` uses: no `innerHTML`, `outerHTML`, `insertAdjacentHTML`, `setHTMLUnsafe`,
//!   `parseHTMLUnsafe`, `srcdoc`, `createContextualFragment`, `DOMParser` or
//!   `dangerouslySetInnerHTML` anywhere else;
//! - what `MarkdownView` takes is `SafeHtml`, and only the WebAssembly renderer and the API client
//!   make one (`markSafeHtml` is named nowhere else, also not in an import, and `as SafeHtml` is
//!   written nowhere else);
//! - the name of the Trusted Types policy is written once (`bb-api-words`): the content security
//!   policy takes it from there, and the browser code from the generated client, never as a text;
//! - the corpus tests exist: the native one in `bb-markdown` and the one that runs the same files
//!   through the WebAssembly build. They run in CI like every test.

use serde_json::Value;

use super::web_source::code_of;
use super::{Check, Context, Finding};
use crate::metadata::{self, Package};
use crate::source::{Kind, Source, read_text};

/// The crate that renders.
const RENDERER: &str = "bb-markdown";
/// The one library the renderer uses in `[dependencies]`.
const RENDERER_PARSER: &str = "pulldown-cmark";
/// Crates that are not the product.
const NOT_THE_PRODUCT: [&str; 1] = ["xtask"];
/// Families of Rust libraries that parse or clean Markdown or HTML: a name is in a family when it
/// is the word or starts with the word and a dash or an underscore.
const RUST_LIBRARIES: [&str; 14] = [
    "pulldown-cmark",
    "pulldown_cmark",
    "ammonia",
    "comrak",
    "markdown",
    "html5ever",
    "markup5ever",
    "html5gum",
    "scraper",
    "kuchikiki",
    "lol_html",
    "lol-html",
    "tl",
    "select",
];
/// The same for the browser app.
const WEB_LIBRARIES: [&str; 22] = [
    "marked",
    "markdown-it",
    "showdown",
    "snarkdown",
    "micromark",
    "remark",
    "rehype",
    "unified",
    "mdast",
    "hast",
    "react-markdown",
    "prosemirror-markdown",
    "turndown",
    "sanitize-html",
    "html-react-parser",
    "htmlparser2",
    "parse5",
    "xss",
    "interweave",
    "html-to-react",
    "domhandler",
    "cheerio",
];
/// Words that name a library wherever they stand in the name (`isomorphic-dompurify`).
const WEB_LIBRARY_WORDS: [&str; 1] = ["dompurify"];
/// The tables of a `package.json` that list packages.
const PACKAGE_TABLES: [&str; 4] = [
    "dependencies",
    "devDependencies",
    "peerDependencies",
    "optionalDependencies",
];

/// The one file that sets HTML from a string.
const POLICY_FILE: &str = "web/packages/ui/src/markdown/policy.ts";
/// What sets HTML from a string, or runs it.
const HTML_SETTERS: [&str; 9] = [
    "innerHTML",
    "outerHTML",
    "insertAdjacentHTML",
    "setHTMLUnsafe",
    "parseHTMLUnsafe",
    "srcdoc",
    "createContextualFragment",
    "DOMParser",
    "dangerouslySetInnerHTML",
];
/// The call that says a string is HTML that `bb-markdown` made, and the code that may call it.
const MARK_SAFE: &str = "markSafeHtml";
/// A cast is the other way to make one.
const CAST_SAFE: &str = "as SafeHtml";
const MARK_SAFE_FOLDERS: [&str; 2] = ["web/packages/wasm/src/", "web/packages/api-client/src/"];
/// The name of the policy in the browser code: imported from the generated client, never defined.
const POLICY_NAME: &str = "trustedTypesPolicyName";
const CLIENT_PACKAGE: &str = "@bugaboo/api-client";
/// The header of the server.
const HEADERS_FILE: &str = "crates/bb-api/src/headers.rs";
const TRUSTED_TYPES_REQUIRED: &str = "require-trusted-types-for 'script'";
/// The directive that allows the policy, as it is written in the header, and what must follow it:
/// the macro that is the one definition of the name.
const TRUSTED_TYPES_ALLOWED: &str = "trusted-types \"";
const POLICY_MACRO_CALL: &str = "bb_api_words::trusted_types_policy!()";

/// What must be in a file: the corpus tests, and the folder of the corpus they read.
const MUST_HAVE: [(&str, &str, &str); 5] = [
    (
        "crates/bb-api/src/headers.rs",
        "fn sec_13_the_csp_allows_exactly_the_one_trusted_types_policy",
        "the test that holds the built content security policy to exactly one policy",
    ),
    (
        "crates/bb-markdown/tests/render.rs",
        "fn sec_13_the_sanitizer_corpus_renders_every_attack_as_text",
        "the test that proves every attack renders as text (SEC-13)",
    ),
    (
        "crates/bb-markdown/tests/render.rs",
        "tests/corpus",
        "the folder of the corpus",
    ),
    (
        "web/tooling/markdown-corpus.test.ts",
        "sec_13_server_and_browser_render_the_same_html",
        "the test that proves the browser renders the corpus as the server does",
    ),
    (
        "web/tooling/markdown-corpus.test.ts",
        "crates/bb-markdown/tests/corpus",
        "the folder of the same corpus",
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
        findings.extend(safe_html_findings(&web));
        findings.extend(policy_findings(
            &read_text(repo, POLICY_FILE)?,
            &read_text(repo, HEADERS_FILE)?,
        ));
        let read = |file: &str| read_text(repo, file).unwrap_or_default();
        findings.extend(must_have_findings(read));
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

/// Whether `name` is in one of the families: the word itself, or the word and a dash or an
/// underscore and more (`remark-gfm`), also for a scoped name (`@types/dompurify`).
fn in_family(name: &str, families: &[&str]) -> bool {
    let bare = name.rsplit('/').next().unwrap_or(name);
    families.iter().any(|family| {
        bare == *family
            || bare
                .strip_prefix(family)
                .is_some_and(|rest| rest.starts_with(['-', '_']))
    })
}

fn rust_dependency_findings(packages: &[Package]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for package in packages
        .iter()
        .filter(|p| !NOT_THE_PRODUCT.contains(&p.name.as_str()))
    {
        let is_renderer = package.name == RENDERER;
        for dependency in package.dependencies.iter().filter(|d| {
            in_family(&d.name, &RUST_LIBRARIES)
                // The renderer has its parser, and an HTML parser as the oracle of its tests.
                && !(is_renderer && (d.name == RENDERER_PARSER || d.kind.as_deref() == Some("dev")))
        }) {
            findings.push(Finding::at(
                &package.name,
                format!(
                    "depends on `{}`. Text written by people is rendered only by `{RENDERER}` (SEC-RICHTEXT, ADR-0011): call it instead of using a second library. (`{RENDERER}` itself may have an HTML parser only in `[dev-dependencies]`.)",
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
        let Ok(manifest) = serde_json::from_str::<Value>(text) else {
            continue;
        };
        for table in PACKAGE_TABLES {
            let Some(packages) = manifest.get(table).and_then(Value::as_object) else {
                continue;
            };
            for name in packages.keys().filter(|name| {
                in_family(name, &WEB_LIBRARIES)
                    || WEB_LIBRARY_WORDS.iter().any(|word| name.contains(word))
            }) {
                findings.push(Finding::at(
                    path,
                    format!(
                        "uses `{name}`. There is no other renderer in the browser: the preview is `bb-markdown` in WebAssembly (SEC-RICHTEXT, ADR-0011)."
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
                        "`{setter}` makes or runs HTML from a string. Only `MarkdownView` shows HTML, through the policy in {POLICY_FILE} (SEC-RICHTEXT)."
                    ),
                ));
            }
        }
    }
    findings
}

/// Only the code that gets HTML from `bb-markdown` may say that a string is safe HTML.
fn safe_html_findings(sources: &[Source]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (path, text) in sources.iter().filter(|(path, _)| {
        !path.contains(".test.")
            && !MARK_SAFE_FOLDERS
                .iter()
                .any(|folder| path.starts_with(folder))
    }) {
        for (number, line) in text.lines().enumerate() {
            let code = code_of(line);
            if code.contains(MARK_SAFE) || code.contains(CAST_SAFE) {
                findings.push(Finding::at(
                    format!("{path}:{}", number + 1),
                    format!(
                        "`{MARK_SAFE}` and `{CAST_SAFE}` say a string is HTML that `bb-markdown` made. Only the WebAssembly renderer and the API client may (SEC-RICHTEXT); ask them for a `SafeHtml` instead."
                    ),
                ));
            }
        }
    }
    findings
}

/// The name of the policy is written once: the browser code imports it and the header takes it
/// from the same macro.
fn policy_findings(policy_source: &str, headers_source: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let imports = policy_source.contains(CLIENT_PACKAGE) && policy_source.contains(POLICY_NAME);
    let defines = policy_source.contains(&format!("const {POLICY_NAME}"));
    if !imports || defines {
        findings.push(Finding::at(
            POLICY_FILE,
            format!(
                "The policy must be made with `{POLICY_NAME}` imported from `{CLIENT_PACKAGE}`, not with a name written here: the name is defined once, in `bb-api-words`."
            ),
        ));
    }
    if !headers_source.contains(TRUSTED_TYPES_REQUIRED) {
        findings.push(Finding::at(
            HEADERS_FILE,
            format!("The content security policy must say `{TRUSTED_TYPES_REQUIRED}`."),
        ));
    }
    let directive_is_exact = headers_source
        .split_once(TRUSTED_TYPES_ALLOWED)
        .is_some_and(|(_, after)| {
            after
                .trim_start_matches([',', ' ', '\n'])
                .strip_prefix(POLICY_MACRO_CALL)
                // The macro is the last part: nothing but the end of the `concat!` follows it.
                .is_some_and(|rest| rest.trim_start().starts_with(')'))
        });
    if !directive_is_exact {
        findings.push(Finding::at(
            HEADERS_FILE,
            format!(
                "The `trusted-types` directive of the content security policy must end with `{POLICY_MACRO_CALL}`, so it allows exactly the one policy and no other, and the name is not written a second time."
            ),
        ));
    }
    findings
}

fn must_have_findings(read: impl Fn(&str) -> String) -> Vec<Finding> {
    MUST_HAVE
        .iter()
        .filter(|(file, needle, _)| !read(file).contains(needle))
        .map(|(file, needle, why)| {
            Finding::at(*file, format!("`{needle}` must be there: it is {why}."))
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
        "import { trustedTypesPolicyName } from \"@bugaboo/api-client\";\nelement.innerHTML = x;\n";
    const HEADERS: &str = "require-trusted-types-for 'script'; \\\n trusted-types \",\n bb_api_words::trusted_types_policy!()
 )";

    #[test]
    fn good_example_the_renderer_the_policy_and_the_tests() {
        let mut renderer = package("bb-markdown", &["pulldown-cmark"]);
        renderer
            .dependencies
            .extend(
                package("x", &["html5ever"])
                    .dependencies
                    .into_iter()
                    .map(|mut d| {
                        d.kind = Some("dev".to_owned());
                        d
                    }),
            );
        assert!(
            rust_dependency_findings(&[
                renderer,
                package("xtask", &["pulldown-cmark"]),
                package("bb-api", &["bb-core"]),
            ])
            .is_empty()
        );
        let manifest = source(
            "package.json",
            "{\"devDependencies\": {\"vite\": \"1\", \"html-validate\": \"1\"}}",
        );
        assert!(web_dependency_findings(&manifest).is_empty());
        assert!(html_setter_findings(&source(POLICY_FILE, POLICY)).is_empty());
        assert!(
            html_setter_findings(&source("web/app/src/a.ts", "// innerHTML in a comment\n"))
                .is_empty()
        );
        assert!(
            safe_html_findings(&source(
                "web/packages/wasm/src/index.ts",
                "markSafeHtml(render(t));"
            ))
            .is_empty()
        );
        assert!(
            safe_html_findings(&source("web/packages/ui/src/a.test.tsx", "markSafeHtml(x)"))
                .is_empty()
        );
        assert!(policy_findings(POLICY, HEADERS).is_empty());
        let all: String = MUST_HAVE.iter().map(|(_, needle, _)| *needle).collect();
        assert!(must_have_findings(|_| all.clone()).is_empty());
    }

    #[test]
    fn bad_example_a_second_renderer_in_rust() {
        let found = rust_dependency_findings(&[package(
            "bb-api",
            &[
                "pulldown-cmark",
                "ammonia",
                "html5ever",
                "markup5ever_rcdom",
            ],
        )]);
        assert_eq!(found.len(), 4);
        // The renderer itself must not have an HTML parser outside its tests.
        assert_eq!(
            rust_dependency_findings(&[package("bb-markdown", &["pulldown-cmark", "ammonia"])])
                .len(),
            1
        );
    }

    #[test]
    fn bad_example_a_second_renderer_in_the_browser() {
        let manifest = source(
            "web/app/package.json",
            "{\"dependencies\": {\"marked\": \"1\", \"isomorphic-dompurify\": \"1\", \"remark-gfm\": \"1\", \"html-react-parser\": \"1\", \"@types/dompurify\": \"1\"}}",
        );
        assert_eq!(web_dependency_findings(&manifest).len(), 5);
    }

    #[test]
    fn bad_example_html_set_from_a_string_outside_the_policy_file() {
        for line in [
            "box.innerHTML = html;",
            "<div dangerouslySetInnerHTML={{ __html: html }} />",
            "box.insertAdjacentHTML('beforeend', html);",
            "new DOMParser().parseFromString(html, 'text/html');",
            "box.setHTMLUnsafe(html);",
            "Document.parseHTMLUnsafe(html);",
            "<iframe srcdoc={html} />",
        ] {
            assert_eq!(
                html_setter_findings(&source("web/app/src/a.tsx", line)).len(),
                1,
                "{line}"
            );
        }
    }

    #[test]
    fn bad_example_a_string_called_safe_html_by_somebody_else() {
        assert_eq!(
            safe_html_findings(&source("web/app/src/a.tsx", "const h = markSafeHtml(x);")).len(),
            1
        );
        for line in [
            "import { markSafeHtml as m } from \"@bugaboo/api-client\";",
            "const h = ({ html: s }) as SafeHtml;",
        ] {
            assert_eq!(
                safe_html_findings(&source("web/app/src/a.tsx", line)).len(),
                1,
                "{line}"
            );
        }
    }

    #[test]
    fn bad_example_the_policy_name_is_written_twice_or_the_header_is_not_exact() {
        // A name written in the browser code.
        assert_eq!(
            policy_findings(
                "import { trustedTypesPolicyName } from \"@bugaboo/api-client\";\nconst trustedTypesPolicyName = \"x\";",
                HEADERS
            )
            .len(),
            1
        );
        // A header that says the name as a text, or another name after it.
        for header in [
            "require-trusted-types-for 'script'; trusted-types bugaboo-markdown",
            "require-trusted-types-for 'script'; trusted-types \", other",
            "require-trusted-types-for 'script'; trusted-types \", bb_api_words::trusted_types_policy!(), \" other 'allow-duplicates'",
        ] {
            assert_eq!(policy_findings(POLICY, header).len(), 1, "{header}");
        }
        assert_eq!(policy_findings(POLICY, "default-src 'self'").len(), 2);
    }

    #[test]
    fn bad_example_a_missing_corpus_test() {
        assert_eq!(must_have_findings(|_| String::new()).len(), MUST_HAVE.len());
    }
}
