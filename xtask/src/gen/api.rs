//! `cargo xtask gen api`: the API description, the browser client's types and the list of cookies
//! (ADR-0008, COMP-13).
//!
//! The one source is the code of `bb-api`. xtask does not link the product crates, so:
//!
//! - the OpenAPI document comes from the program itself (`bugaboo openapi`, built and run with
//!   cargo), and is written to `web/packages/api-client/openapi.json`;
//! - the TypeScript types are made from that document by `openapi-typescript` (a small script
//!   in the client package, run with Node) into `web/packages/api-client/src/schema.ts`;
//! - the cookie table (`crates/bb-api/src/cookies.rs`) is read as Rust source, like the
//!   personal-data marks, and written as the browser client's constants
//!   (`src/protocol.ts`) and as the user docs page "Cookies we use" (`userDocs/cookies.md`);
//! - the settings registry (`crates/bb-core/src/settings/catalog.rs`) is read as Rust source and
//!   written as the keys, the choices and the defaults of the settings
//!   (`src/setting-keys.generated.ts`);
//! - the page sizes (`crates/bb-core/src/paging.rs`) are read as Rust source and written as the
//!   browser client's `src/paging.generated.ts`;
//! - the times and sizes of editing (`crates/bb-core/src/edit.rs`) are written as
//!   `src/editing.generated.ts`;
//! - the scale of own-field decimals (`crates/bb-core/src/own_fields.rs`) is written as
//!   `src/fields.generated.ts`;
//! - the size of a part of an upload and of a megabyte (`crates/bb-core/src/files.rs`) are written
//!   as `src/files.generated.ts`;
//! - the words of the vocabularies the browser lists (the icons of issue types) are read from the
//!   API description into `src/words.generated.ts`;
//! - the names of the properties the server renders as HTML (the mark `x-safe-html`) are written
//!   as `src/safe-html.generated.ts`, and their TypeScript type is `SafeHtml`;
//! - the addresses of the app pages the server links to (`crates/bb-core/src/app_paths.rs`) are
//!   written as `src/app-paths.generated.ts`;
//! - the table of *Customize* and the numbers that the texts of notifications, signing up and
//!   handles say are read as Rust source into `src/notifications.generated.ts`;
//! - the name of the Trusted Types policy (`crates/bb-api-words`) is read as Rust source and
//!   written as `src/browser.generated.ts`.
//!
//! TypeScript and JSON go through the formatter (`oxfmt`), so the generated files already look
//! the way FMT-TS wants them.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context as _, bail};

use super::Generated;
use super::messages::{SOURCE_LOCALE_PATH, source_pairs};
use super::messages_rust::constant_name;
use super::privacy::table;
use crate::repo::Repo;
use crate::source::{Kind, Source, read_sources};
use crate::tools;

mod web_constants;

/// The OpenAPI document, relative to the repository root.
pub(crate) const SPEC_PATH: &str = "web/packages/api-client/openapi.json";
/// The TypeScript types made from the document.
pub(crate) const SCHEMA_PATH: &str = "web/packages/api-client/src/schema.ts";
/// Names the browser client needs from the server code.
pub(crate) const PROTOCOL_PATH: &str = "web/packages/api-client/src/protocol.ts";
/// The keys, choices and defaults of the settings of the registry, for the browser app
/// (ARCH-SETTINGS: the browser names a setting through these constants and never by writing its
/// key; the choices and defaults are never written a second time).
pub(crate) const SETTING_KEYS_PATH: &str = "web/packages/api-client/src/setting-keys.generated.ts";
/// The page sizes of the lists, for the browser app.
pub(crate) const PAGING_PATH: &str = "web/packages/api-client/src/paging.generated.ts";
/// The times and sizes of editing, for the browser app.
pub(crate) const EDITING_PATH: &str = "web/packages/api-client/src/editing.generated.ts";
/// The scale of own-field numbers, for the browser app.
pub(crate) const FIELDS_PATH: &str = "web/packages/api-client/src/fields.generated.ts";
/// The sizes of files the server and the browser both name.
pub(crate) const FILES_PATH: &str = "web/packages/api-client/src/files.generated.ts";
/// The names of the properties the server renders as HTML (mark `x-safe-html`).
pub(crate) const SAFE_HTML_PATH: &str = "web/packages/api-client/src/safe-html.generated.ts";
/// The words of the vocabularies the browser lists at run time.
pub(crate) const WORDS_PATH: &str = "web/packages/api-client/src/words.generated.ts";
/// Names the browser needs from the server code, such as the Trusted Types policy.
pub(crate) const BROWSER_NAMES_PATH: &str = "web/packages/api-client/src/browser.generated.ts";
/// The addresses of the app pages the server links to.
pub(crate) const APP_PATHS_PATH: &str = "web/packages/api-client/src/app-paths.generated.ts";
/// What the texts of notifications, signing up and handles say: numbers and the table of *Customize*.
pub(crate) const NOTIFICATIONS_PATH: &str =
    "web/packages/api-client/src/notifications.generated.ts";
/// The user docs page.
pub(crate) const COOKIES_PAGE: &str = "userDocs/cookies.md";

const COOKIES_SOURCE: &str = "crates/bb-api/src/cookies.rs";
const TYPES_SCRIPT: &str = "web/packages/api-client/scripts/openapi-types.mjs";
const FORMATTER_CONFIG: &str = ".oxfmtrc.json";
const TS_HEADER: &str = "// Generated by `cargo xtask gen api`. Do not edit by hand.\n\n";

/// The paths [`generate`] writes.
pub(crate) fn output_paths() -> Vec<&'static str> {
    vec![
        SPEC_PATH,
        SCHEMA_PATH,
        PROTOCOL_PATH,
        SETTING_KEYS_PATH,
        PAGING_PATH,
        EDITING_PATH,
        FILES_PATH,
        WORDS_PATH,
        SAFE_HTML_PATH,
        BROWSER_NAMES_PATH,
        APP_PATHS_PATH,
        NOTIFICATIONS_PATH,
        COOKIES_PAGE,
    ]
}

pub(crate) fn generate(repo: &Repo) -> anyhow::Result<Vec<Generated>> {
    let spec = openapi_text(repo)?;
    let sources = read_sources(repo, None, Kind::Rust)?;
    let cookies = read_cookies(&sources)?;
    let purposes = purposes(&source_pairs(repo)?);
    let types = openapi_types(repo, &spec)?;
    let protocol = protocol(&cookies)?;
    Ok(vec![
        Generated::new(SPEC_PATH, format_with_oxfmt(repo, SPEC_PATH, &spec)?),
        Generated::new(
            SCHEMA_PATH,
            format_with_oxfmt(repo, SCHEMA_PATH, &format!("{TS_HEADER}{types}"))?,
        ),
        Generated::new(
            PROTOCOL_PATH,
            format_with_oxfmt(repo, PROTOCOL_PATH, &protocol)?,
        ),
        Generated::new(
            SETTING_KEYS_PATH,
            format_with_oxfmt(
                repo,
                SETTING_KEYS_PATH,
                &web_constants::setting_keys(&sources)?,
            )?,
        ),
        Generated::new(
            PAGING_PATH,
            format_with_oxfmt(repo, PAGING_PATH, &web_constants::paging(&sources)?)?,
        ),
        Generated::new(
            EDITING_PATH,
            format_with_oxfmt(repo, EDITING_PATH, &web_constants::editing(&sources)?)?,
        ),
        Generated::new(
            FIELDS_PATH,
            format_with_oxfmt(repo, FIELDS_PATH, &web_constants::fields(&sources)?)?,
        ),
        Generated::new(
            FILES_PATH,
            format_with_oxfmt(repo, FILES_PATH, &web_constants::files(&sources)?)?,
        ),
        Generated::new(
            WORDS_PATH,
            format_with_oxfmt(repo, WORDS_PATH, &web_constants::words(&spec, &sources)?)?,
        ),
        Generated::new(
            SAFE_HTML_PATH,
            format_with_oxfmt(repo, SAFE_HTML_PATH, &web_constants::safe_html(&spec)?)?,
        ),
        Generated::new(
            BROWSER_NAMES_PATH,
            format_with_oxfmt(
                repo,
                BROWSER_NAMES_PATH,
                &web_constants::browser_names(&sources)?,
            )?,
        ),
        Generated::new(
            APP_PATHS_PATH,
            format_with_oxfmt(repo, APP_PATHS_PATH, &web_constants::app_paths(&sources)?)?,
        ),
        Generated::new(
            NOTIFICATIONS_PATH,
            format_with_oxfmt(
                repo,
                NOTIFICATIONS_PATH,
                &web_constants::notifications(&sources, &spec)?,
            )?,
        ),
        Generated::new(COOKIES_PAGE, cookies_page(&cookies.cookies, &purposes)?),
    ])
}

/// Asks the program for its API description. Only stdout is the document; cargo's own messages
/// go to stderr and are shown when the run fails.
fn openapi_text(repo: &Repo) -> anyhow::Result<String> {
    let out = Command::new("cargo")
        .args(["run", "--quiet", "--package", "bb-server", "--", "openapi"])
        .current_dir(repo.root())
        .output()
        .context("could not start cargo to get the API description")?;
    if !out.status.success() {
        bail!(
            "`bugaboo openapi` failed:\n{}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    String::from_utf8(out.stdout).context("the API description is not UTF-8 text")
}

/// Runs `program` with `input` on its standard input and returns its standard output.
fn pipe(program: &Path, args: &[&str], cwd: &Path, input: &str) -> anyhow::Result<String> {
    let mut child = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("could not start {}", program.display()))?;
    child
        .stdin
        .take()
        .context("no input pipe")?
        .write_all(input.as_bytes())
        .with_context(|| format!("could not send input to {}", program.display()))?;
    let out = child.wait_with_output()?;
    if !out.status.success() {
        bail!(
            "{} failed:\n{}{}",
            program.display(),
            String::from_utf8_lossy(&out.stdout).trim(),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    String::from_utf8(out.stdout).context("tool output is not UTF-8 text")
}

pub(crate) fn format_with_oxfmt(repo: &Repo, path: &str, text: &str) -> anyhow::Result<String> {
    let formatter = tools::find("oxfmt").map_err(|finding| anyhow::anyhow!(finding.message))?;
    pipe(
        &formatter,
        &["--config", FORMATTER_CONFIG, "--stdin-filepath", path],
        repo.root(),
        text,
    )
}

fn openapi_types(repo: &Repo, spec: &str) -> anyhow::Result<String> {
    pipe(
        Path::new("node"),
        &[TYPES_SCRIPT, bb_api_words::marks::SAFE_HTML],
        repo.root(),
        spec,
    )
}

/// What the browser client needs from `cookies.rs`.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CookieTable {
    pub(crate) cookies: Vec<CookieDef>,
    /// The value of `CSRF_HEADER`.
    pub(crate) csrf_header: String,
    /// The value of `BACKGROUND_HEADER`.
    pub(crate) background_header: String,
}

/// One cookie of the table.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CookieDef {
    pub(crate) variant: String,
    pub(crate) name: String,
    pub(crate) http_only: bool,
    pub(crate) same_site: String,
    /// The constant of the purpose message, like `COOKIE_SESSION_PURPOSE`.
    pub(crate) purpose: String,
}

struct Row(CookieDef);

impl syn::parse::Parse for Row {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        syn::Attribute::parse_outer(input)?;
        let variant: syn::Ident = input.parse()?;
        input.parse::<syn::Token![=>]>()?;
        let inner;
        syn::parenthesized!(inner in input);
        let name: syn::LitStr = inner.parse()?;
        inner.parse::<syn::Token![,]>()?;
        let http_only: syn::LitBool = inner.parse()?;
        inner.parse::<syn::Token![,]>()?;
        let same_site: syn::Ident = inner.parse()?;
        inner.parse::<syn::Token![,]>()?;
        let purpose: syn::Path = inner.parse()?;
        Ok(Self(CookieDef {
            variant: variant.to_string(),
            name: name.value(),
            http_only: http_only.value,
            same_site: same_site.to_string(),
            purpose: purpose
                .segments
                .last()
                .map(|segment| segment.ident.to_string())
                .unwrap_or_default(),
        }))
    }
}

/// The rows of `cookies! { … }` in the source.
pub(crate) fn read_cookies(sources: &[Source]) -> anyhow::Result<CookieTable> {
    let Some((_, text)) = sources.iter().find(|(path, _)| path == COOKIES_SOURCE) else {
        bail!("{COOKIES_SOURCE} is missing");
    };
    parse_cookies(text)
}

pub(crate) fn parse_cookies(text: &str) -> anyhow::Result<CookieTable> {
    let file = syn::parse_file(text).with_context(|| format!("{COOKIES_SOURCE} is not Rust"))?;
    let csrf_header = header_const(&file, "CSRF_HEADER")?;
    let background_header = header_const(&file, "BACKGROUND_HEADER")?;
    for item in &file.items {
        let syn::Item::Macro(item) = item else {
            continue;
        };
        if item
            .mac
            .path
            .segments
            .last()
            .is_none_or(|segment| segment.ident != "cookies")
        {
            continue;
        }
        let rows = item
            .mac
            .parse_body_with(syn::punctuated::Punctuated::<Row, syn::Token![,]>::parse_terminated)
            .map_err(|error| {
                anyhow::anyhow!(
                    "{COOKIES_SOURCE}: a line of the cookie table is not `Name => (\"__Host-name\", http_only, SameSite, message::PURPOSE)`: {error}"
                )
            })?;
        return Ok(CookieTable {
            cookies: rows.into_iter().map(|row| row.0).collect(),
            csrf_header,
            background_header,
        });
    }
    bail!("{COOKIES_SOURCE} has no `cookies! {{ … }}` table")
}

/// The text of `pub const NAME: &str = "…";`.
fn header_const(file: &syn::File, name: &str) -> anyhow::Result<String> {
    file.items
        .iter()
        .find_map(|item| match item {
            syn::Item::Const(item) if item.ident == name => match &*item.expr {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(text),
                    ..
                }) => Some(text.value()),
                _ => None,
            },
            _ => None,
        })
        .with_context(|| format!("{COOKIES_SOURCE} has no `const {name}: &str = \"…\"`"))
}

/// The English purpose texts by constant name.
fn purposes(messages: &[(String, String)]) -> BTreeMap<String, String> {
    messages
        .iter()
        .map(|(key, text)| (constant_name(key), text.clone()))
        .collect()
}

fn protocol(table: &CookieTable) -> anyhow::Result<String> {
    let csrf = table
        .cookies
        .iter()
        .find(|cookie| cookie.variant == "Csrf")
        .context("the cookie table has no `Csrf` cookie")?;
    Ok(format!(
        "{TS_HEADER}/** The cookie that holds the CSRF token. The page reads it and sends it back in {{@link CSRF_HEADER}}. */
export const CSRF_COOKIE = \"{}\";

/** The request header that carries the CSRF token on every change request. */
export const CSRF_HEADER = \"{}\";

/** The request header that marks a read nobody asked for (a count that refreshes itself): it does not keep a session alive. */
export const BACKGROUND_HEADER = \"{}\";
",
        csrf.name, table.csrf_header, table.background_header
    ))
}

fn cookies_page(
    cookies: &[CookieDef],
    purposes: &BTreeMap<String, String>,
) -> anyhow::Result<String> {
    let mut rows = Vec::new();
    for cookie in cookies {
        let purpose = purposes.get(&cookie.purpose).with_context(|| {
            format!(
                "the cookie `{}` names the message `{}`, which is not in {SOURCE_LOCALE_PATH}",
                cookie.name, cookie.purpose
            )
        })?;
        let reader = if cookie.http_only {
            "Only the server."
        } else {
            "Only the server and the Bugaboo page itself, which needs it to protect your changes."
        };
        let sent = match cookie.same_site.as_str() {
            "Strict" => "Only for requests that start on Bugaboo's own pages.",
            "Lax" => {
                "For requests on Bugaboo's own pages, and when you arrive from a link on another website."
            }
            other => bail!("unknown SameSite rule `{other}` on `{}`", cookie.name),
        };
        rows.push(vec![
            format!("`{}`", cookie.name),
            purpose.clone(),
            reader.to_owned(),
            sent.to_owned(),
        ]);
    }
    let mut out = String::from("# Cookies we use\n\n");
    out.push_str("GENERATED by `cargo xtask gen api` from the cookie table in the code (COMP-13). Do not edit this page by hand.\n\n");
    out.push_str("Bugaboo sets only the cookies it needs to work. They are not used to follow you around, to count visits or for advertising, and no other company sets cookies through Bugaboo. That is why Bugaboo shows no cookie banner (PRIV-01, COMP-13).\n\n");
    out.push_str(&table(
        &[
            "Cookie",
            "What it is for",
            "Who can read it",
            "When it is sent",
        ],
        &rows,
    ));
    out.push_str("\nThe names start with `__Host-`. This makes your browser keep the cookie to this one Bugaboo address and send it only over a secure (HTTPS) connection.\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: &str = "pub const CSRF_HEADER: &str = \"x-csrf-token\";
pub const BACKGROUND_HEADER: &str = \"x-background-read\";
cookies! {\n    /// One.\n    Session => (\"__Host-bb_session\", true, Lax, message::COOKIE_SESSION_PURPOSE),\n    Csrf => (\"__Host-bb_csrf\", false, Strict, message::COOKIE_CSRF_PURPOSE),\n}\n";

    #[test]
    fn the_cookie_table_is_read_from_the_source() {
        let table = parse_cookies(TABLE).unwrap();
        let cookies = &table.cookies;
        assert_eq!(table.csrf_header, "x-csrf-token");
        assert_eq!(cookies.len(), 2);
        assert_eq!(cookies[0].name, "__Host-bb_session");
        assert!(cookies[0].http_only);
        assert_eq!(cookies[1].same_site, "Strict");
        assert_eq!(cookies[1].purpose, "COOKIE_CSRF_PURPOSE");
        assert!(parse_cookies("fn x() {}").is_err());
        assert!(parse_cookies("cookies! { A => (1) }").is_err());
    }

    #[test]
    fn the_page_lists_every_cookie_with_its_purpose() {
        let cookies = parse_cookies(TABLE).unwrap().cookies;
        let purposes = BTreeMap::from([
            (
                "COOKIE_SESSION_PURPOSE".to_owned(),
                "Keeps you in.".to_owned(),
            ),
            ("COOKIE_CSRF_PURPOSE".to_owned(), "Protects you.".to_owned()),
        ]);
        let page = cookies_page(&cookies, &purposes).unwrap();
        assert!(page.contains("`__Host-bb_session`"));
        assert!(page.contains("Keeps you in."));
        assert!(page.contains("Only the server. "));
        let missing = cookies_page(&cookies, &BTreeMap::new());
        assert!(missing.is_err());
    }

    #[test]
    fn the_browser_gets_the_csrf_cookie_name() {
        let text = protocol(&parse_cookies(TABLE).unwrap()).unwrap();
        assert!(text.contains("CSRF_COOKIE = \"__Host-bb_csrf\""));
        assert!(text.contains("CSRF_HEADER = \"x-csrf-token\""));
    }
}
