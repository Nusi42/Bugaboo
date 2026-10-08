//! SEC-COOKIES: cookies are set in one place, and every cookie follows the policy
//! (COMP-13, PRIV-01, architecture §7).
//!
//! The table in `crates/bb-api/src/cookies.rs` is the only place that may set a cookie. This check
//! reads the table (every name starts with `__Host-`, every cookie says what it is for) and finds
//! cookie writes anywhere else: a `Set-Cookie` header in Rust, a cookie library, or a write to
//! `document.cookie` in the browser app.

use anyhow::Context as _;

use super::web_source::code_of;
use super::{Check, Context, Finding};
use crate::generate::api::{CookieTable, parse_cookies};
use crate::generate::messages::{SOURCE_LOCALE_PATH, parse_pairs};
use crate::generate::messages_rust::constant_name;
use crate::rust_source::{strip_comments, without_tests};
use crate::source::{Kind, Source};

/// The one file that may set cookies.
const COOKIE_MODULE: &str = "crates/bb-api/src/cookies.rs";
/// The required start of every cookie name (the browser then insists on `Secure`, `Path=/` and no
/// `Domain`).
const NAME_START: &str = "__Host-";
/// How a Rust file writes a cookie. Lower case; the text searched is lower-cased too.
const RUST_WRITES: [&str; 6] = [
    "set_cookie",
    "set-cookie",
    "cookiejar",
    "cookie::build",
    "add_cookie",
    "cookie::new",
];

pub(crate) struct SecCookies;

impl Check for SecCookies {
    fn id(&self) -> &'static str {
        "SEC-COOKIES"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let rust = context.sources(self.id(), Kind::Rust)?;
        let web = context.sources(self.id(), Kind::Web)?;
        let catalog = std::fs::read_to_string(context.repo.path(SOURCE_LOCALE_PATH))
            .with_context(|| format!("could not read {SOURCE_LOCALE_PATH}"))?;
        let constants = parse_pairs(&catalog)?
            .iter()
            .map(|(key, _)| constant_name(key))
            .collect::<Vec<_>>();
        Ok(evaluate(&rust, &web, &constants))
    }
}

fn evaluate(rust: &[Source], web: &[Source], message_constants: &[String]) -> Vec<Finding> {
    let mut findings = Vec::new();
    match rust.iter().find(|(path, _)| path == COOKIE_MODULE) {
        None => findings.push(Finding::at(COOKIE_MODULE, "The cookie module is missing.")),
        Some((_, text)) => match parse_cookies(text) {
            Ok(table) => findings.extend(check_table(&table, message_constants)),
            Err(error) => findings.push(Finding::at(COOKIE_MODULE, format!("{error:#}"))),
        },
    }
    for (path, text) in rust.iter().filter(|(path, _)| path != COOKIE_MODULE) {
        findings.extend(rust_writes(path, &strip_comments(&without_tests(text))));
    }
    for (path, text) in web {
        findings.extend(browser_writes(path, text));
    }
    findings
}

fn check_table(table: &CookieTable, message_constants: &[String]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for cookie in &table.cookies {
        let place = format!("{COOKIE_MODULE}: {}", cookie.variant);
        if !cookie.name.starts_with(NAME_START) {
            findings.push(Finding::at(
                &place,
                format!(
                    "The cookie `{}` must start with `{NAME_START}`: it keeps the cookie to this address, over HTTPS only.",
                    cookie.name
                ),
            ));
        }
        if !message_constants.contains(&cookie.purpose) {
            findings.push(Finding::at(
                &place,
                format!(
                    "`{}` is not a message in {SOURCE_LOCALE_PATH}; every cookie says what it is for.",
                    cookie.purpose
                ),
            ));
        }
    }
    if table.cookies.is_empty() {
        findings.push(Finding::at(COOKIE_MODULE, "The cookie table is empty."));
    }
    findings
}

fn rust_writes(path: &str, code: &str) -> Vec<Finding> {
    code.lines()
        .enumerate()
        .filter(|(_, line)| {
            let lower = line.to_lowercase();
            RUST_WRITES.iter().any(|w| lower.contains(w))
        })
        .map(|(index, _)| {
            Finding::at(
                format!("{path}:{}", index + 1),
                format!("A cookie is set outside {COOKIE_MODULE}. Add it to the cookie table and set it with `Cookie::set`."),
            )
        })
        .collect()
}

fn browser_writes(path: &str, text: &str) -> Vec<Finding> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| writes_document_cookie(code_of(line)))
        .map(|(index, _)| {
            Finding::at(
                format!("{path}:{}", index + 1),
                "A cookie is written in the browser. Only the server sets cookies (cookie module in bb-api).",
            )
        })
        .collect()
}

/// `document.cookie = …` (not `==`, `===`; reading is fine).
fn writes_document_cookie(code: &str) -> bool {
    code.match_indices("document.cookie").any(|(at, found)| {
        let rest = code[at + found.len()..].trim_start();
        rest.starts_with('=') && !rest.starts_with("==") || rest.starts_with("+=")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: &str = "pub const CSRF_HEADER: &str = \"x-csrf-token\";\ncookies! {\n    Session => (\"__Host-bb_session\", true, Lax, message::COOKIE_SESSION_PURPOSE),\n}\n";

    fn module(text: &str) -> Source {
        (COOKIE_MODULE.to_owned(), text.to_owned())
    }

    fn run(rust: &[Source], web: &[Source]) -> Vec<String> {
        let constants = vec!["COOKIE_SESSION_PURPOSE".to_owned()];
        evaluate(rust, web, &constants)
            .into_iter()
            .map(|finding| finding.message)
            .collect()
    }

    #[test]
    fn good_example() {
        let rust = [
            module(TABLE),
            (
                "crates/bb-api/src/router.rs".to_owned(),
                "// Set-Cookie is only in cookies.rs\nfn a() {}\n#[cfg(test)]\nmod t { const X: &str = \"set-cookie\"; }".to_owned(),
            ),
        ];
        let web = [(
            "web/app/src/a.ts".to_owned(),
            "const all = document.cookie;\nif (document.cookie === \"\") {}\n".to_owned(),
        )];
        assert_eq!(run(&rust, &web), Vec::<String>::new());
    }

    #[test]
    fn bad_example() {
        let rust = [
            module(
                "cookies! {\n    A => (\"session\", true, Lax, message::COOKIE_MISSING),\n}\n\
                 pub const CSRF_HEADER: &str = \"x\";",
            ),
            (
                "crates/bb-api/src/api.rs".to_owned(),
                "headers.insert(SET_COOKIE, value);\nlet jar = CookieJar::new();\n".to_owned(),
            ),
        ];
        let web = [(
            "web/app/src/a.ts".to_owned(),
            "document.cookie = \"a=b\";\ndocument.cookie += \"c\";\n".to_owned(),
        )];
        let found = run(&rust, &web).join("\n");
        assert!(found.contains("must start with `__Host-`"), "{found}");
        assert!(found.contains("COOKIE_MISSING"), "{found}");
        assert_eq!(found.matches("set outside").count(), 2, "{found}");
        assert_eq!(
            found.matches("written in the browser").count(),
            2,
            "{found}"
        );
    }

    #[test]
    fn a_missing_module_is_a_finding() {
        assert!(run(&[], &[]).join("").contains("missing"));
    }
}
