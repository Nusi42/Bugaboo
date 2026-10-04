//! ARCH-ERRORS: every error code has a message in the catalog, and the API does not make up its
//! own error texts (architecture §7, UX §9).
//!
//! The codes are one table in `crates/bb-core/src/error_code.rs`: a line per code with its text,
//! its HTTP status and the constant of its message. This check reads that table and the catalog:
//! codes and messages are unique, shaped right, and the message exists and has no placeholder (an
//! error may never carry a name or a number that the person is not allowed to see). It also finds
//! error answers built from a text instead of an `ErrorCode`. The API part of the rule is
//! completed with the API skeleton (WP-06).

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Context as _;

use super::{Check, Context, Finding};
use crate::generate::icu::placeholders;
use crate::generate::messages::{SOURCE_LOCALE_PATH, parse_pairs};
use crate::generate::messages_rust::constant_name;
use crate::rust_source::{Source, before_tests, read_product_sources};

/// The table of codes.
const TABLE_PATH: &str = "crates/bb-core/src/error_code.rs";

/// The code that speaks HTTP; error answers there must come from an `ErrorCode`.
const HTTP_CODE: [&str; 2] = ["crates/bb-api/", "crates/bb-server/"];

/// The messages of error codes start with this word.
const KEY_PREFIX: &str = "error.";

pub(crate) struct ArchErrors;

impl Check for ArchErrors {
    fn id(&self) -> &'static str {
        "ARCH-ERRORS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let sources = read_product_sources(context.repo)?;
        let catalog = std::fs::read_to_string(context.repo.path(SOURCE_LOCALE_PATH))
            .with_context(|| format!("could not read {SOURCE_LOCALE_PATH}"))?;
        let messages = parse_pairs(&catalog)?;
        Ok(evaluate(&sources, &messages))
    }
}

/// One line of the table.
struct Entry {
    variant: String,
    code: String,
    status: u16,
    constant: String,
}

fn evaluate(sources: &[Source], messages: &[(String, String)]) -> Vec<Finding> {
    let mut findings = Vec::new();
    match sources.iter().find(|(path, _)| path == TABLE_PATH) {
        None => findings.push(Finding::at(
            TABLE_PATH,
            "The table of error codes is missing.",
        )),
        Some((_, text)) => match read_table(text) {
            Ok(table) => findings.extend(check_table(&table, messages)),
            Err(problem) => findings.push(Finding::at(
                TABLE_PATH,
                format!("The table of error codes {problem}."),
            )),
        },
    }
    for (path, text) in sources {
        if HTTP_CODE.iter().any(|folder| path.starts_with(folder)) {
            findings.extend(check_answers(path, before_tests(text)));
        }
    }
    findings
}

/// One line of the macro `error_codes! { Variant => ("code", 401, message::CONSTANT), … }`.
struct Row(Entry);

impl syn::parse::Parse for Row {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        syn::Attribute::parse_outer(input)?;
        let variant: syn::Ident = input.parse()?;
        input.parse::<syn::Token![=>]>()?;
        let inner;
        syn::parenthesized!(inner in input);
        let code: syn::LitStr = inner.parse()?;
        inner.parse::<syn::Token![,]>()?;
        let status: syn::LitInt = inner.parse()?;
        inner.parse::<syn::Token![,]>()?;
        let message: syn::Path = inner.parse()?;
        let constant = message
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .unwrap_or_default();
        Ok(Self(Entry {
            variant: variant.to_string(),
            code: code.value(),
            status: status.base10_parse()?,
            constant,
        }))
    }
}

/// The table, read as Rust. A table that cannot be read is an error, not an empty table.
fn read_table(text: &str) -> Result<Vec<Entry>, String> {
    let file = syn::parse_file(text).map_err(|error| format!("is not valid Rust: {error}"))?;
    let mut entries = Vec::new();
    let mut found = false;
    for item in &file.items {
        let syn::Item::Macro(item) = item else {
            continue;
        };
        if item
            .mac
            .path
            .segments
            .last()
            .is_none_or(|s| s.ident != "error_codes")
        {
            continue;
        }
        found = true;
        let rows = item
            .mac
            .parse_body_with(syn::punctuated::Punctuated::<Row, syn::Token![,]>::parse_terminated)
            .map_err(|error| format!("has a line that is not `Name => (\"code\", status, message::CONSTANT)`: {error}"))?;
        entries.extend(rows.into_iter().map(|row| row.0));
    }
    if found {
        Ok(entries)
    } else {
        Err("has no `error_codes! { … }` table".to_owned())
    }
}

fn check_table(table: &[Entry], messages: &[(String, String)]) -> Vec<Finding> {
    let mut findings = Vec::new();
    if table.is_empty() {
        findings.push(Finding::at(TABLE_PATH, "No error codes found; each line must look like `Name => (\"area.name\", 400, message::CONSTANT)`."));
    }
    let by_constant: BTreeMap<String, &(String, String)> = messages
        .iter()
        .map(|pair| (constant_name(&pair.0), pair))
        .collect();
    let mut codes = BTreeSet::new();
    let mut constants = BTreeSet::new();
    for entry in table {
        let place = format!("{TABLE_PATH}: {}", entry.variant);
        if !codes.insert(entry.code.as_str()) {
            findings.push(Finding::at(
                &place,
                format!("The code `{}` is used twice.", entry.code),
            ));
        }
        if !is_code(&entry.code) {
            findings.push(Finding::at(
                &place,
                format!(
                    "`{}` is not a code; use `area.name` in lower case with underscores.",
                    entry.code
                ),
            ));
        }
        if !(400..600).contains(&entry.status) {
            findings.push(Finding::at(
                &place,
                format!("{} is not an error status; use 400 to 599.", entry.status),
            ));
        }
        if !constants.insert(entry.constant.as_str()) {
            findings.push(Finding::at(
                &place,
                format!(
                    "`{}` is the message of two codes; each code needs its own.",
                    entry.constant
                ),
            ));
        }
        match by_constant.get(&entry.constant) {
            None => findings.push(Finding::at(
                &place,
                format!("`{}` is not a message without placeholders in {SOURCE_LOCALE_PATH}; add the message and run `cargo xtask gen messages`.", entry.constant),
            )),
            Some((key, text)) => {
                if !key.starts_with(KEY_PREFIX) {
                    findings.push(Finding::at(&place, format!("The message `{key}` must start with `{KEY_PREFIX}`.")));
                }
                if placeholders(text).is_ok_and(|found| !found.is_empty()) {
                    findings.push(Finding::at(&place, format!("The message `{key}` has a placeholder; an error message must not carry a value that may be hidden.")));
                }
            }
        }
    }
    findings
}

fn is_code(code: &str) -> bool {
    let word = |part: &str| {
        !part.is_empty()
            && part.starts_with(|c: char| c.is_ascii_lowercase())
            && part.chars().all(|c| c.is_ascii_lowercase() || c == '_')
    };
    code.split_once('.')
        .is_some_and(|(area, name)| word(area) && word(name))
}

/// Finds `(StatusCode::NOT_FOUND, "text")`: an error answer with a text written in the code.
fn check_answers(path: &str, code: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (index, line) in code.lines().enumerate() {
        let Some(at) = line.find("StatusCode::") else {
            continue;
        };
        let after = &line[at..];
        let Some(comma) = after.find(',') else {
            continue;
        };
        if after[comma + 1..].trim_start().starts_with('"') && is_error_status(&after[..comma]) {
            findings.push(Finding::at(
                format!("{path}:{}", index + 1),
                "An error answer with a text written in the code. Answer with a `Problem` made from an `ErrorCode`; its text comes from the message catalog.",
            ));
        }
    }
    findings
}

fn is_error_status(name: &str) -> bool {
    !name.contains("OK") && !name.contains("CREATED") && !name.contains("NO_CONTENT")
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: &str = "error_codes! {\n    /// Wrong.\n    WrongCredentials => (\"auth.wrong_credentials\", 401, message::ERROR_AUTH_WRONG),\n    /// Gone.\n    NotFound => (\"resource.not_found\", 404, message::ERROR_RESOURCE_NOT_FOUND),\n}\n";

    fn messages(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(k, t)| ((*k).to_owned(), (*t).to_owned()))
            .collect()
    }

    fn run(table: &str, pairs: &[(&str, &str)], api: &str) -> Vec<String> {
        let sources = vec![
            (TABLE_PATH.to_owned(), table.to_owned()),
            ("crates/bb-api/src/router.rs".to_owned(), api.to_owned()),
        ];
        evaluate(&sources, &messages(pairs))
            .into_iter()
            .map(|f| f.message)
            .collect()
    }

    const GOOD_MESSAGES: [(&str, &str); 2] = [
        ("error.auth.wrong", "No."),
        ("error.resource.not-found", "Not here."),
    ];

    #[test]
    fn good_example() {
        assert!(
            run(
                TABLE,
                &GOOD_MESSAGES,
                "let a = (StatusCode::OK, \"fine\");\nlet b = problem(ErrorCode::NotFound);\n"
            )
            .is_empty()
        );
    }

    #[test]
    fn bad_example() {
        let table = r#"error_codes! {
    A => ("auth.x", 401, message::ERROR_A),
    B => ("auth.x", 200, message::ERROR_A),
    C => ("Auth", 404, message::ERROR_MISSING),
    D => ("x.y", 404, message::ERROR_D),
}
"#;
        let found = run(
            table,
            &[("error.a", "No."), ("error.d", "Hello {name}")],
            "return (StatusCode::NOT_FOUND, \"Not found\");\n",
        )
        .join("\n");
        assert!(found.contains("used twice"), "{found}");
        assert!(found.contains("not an error status"), "{found}");
        assert!(found.contains("is not a code"), "{found}");
        assert!(found.contains("message of two codes"), "{found}");
        assert!(found.contains("ERROR_MISSING"), "{found}");
        assert!(found.contains("has a placeholder"), "{found}");
        assert!(found.contains("text written in the code"), "{found}");
    }

    #[test]
    fn a_missing_or_unreadable_table_is_an_error_not_an_empty_table() {
        let none = run(
            "fn unrelated() {}
",
            &GOOD_MESSAGES,
            "",
        )
        .join(
            "
",
        );
        assert!(none.contains("has no `error_codes!"), "{none}");
        let broken = run(
            "error_codes! { A => (\"x.y\" 401, m::A) }
",
            &GOOD_MESSAGES,
            "",
        )
        .join(
            "
",
        );
        assert!(broken.contains("has a line that is not"), "{broken}");
    }

    #[test]
    fn the_error_table_of_this_repository_passes() {
        let repo = crate::repo::Repo::discover().unwrap();
        let sources = read_product_sources(&repo).unwrap();
        let text = std::fs::read_to_string(repo.path(SOURCE_LOCALE_PATH)).unwrap();
        let found = evaluate(&sources, &parse_pairs(&text).unwrap());
        assert!(
            found.is_empty(),
            "{:?}",
            found.iter().map(|f| &f.message).collect::<Vec<_>>()
        );
    }
}
