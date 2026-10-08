//! ARCH-SETTINGS (pattern part): settings are declared only in the settings registry and read only
//! through its typed handles (ADR-0018).
//!
//! Two patterns are found in the code of the product crates and the browser app: a `settings!` declaration outside
//! `crates/bb-core/src/settings/`, and the key of a registered setting written as text anywhere
//! else (which would be a way around the registry).
//!
//! Migrations are left out of the second pattern: they are frozen text that names the keys of their
//! time (a setting that was split in two, a key a migration carries values over to), and they can
//! use no handle of a registry that changes after them.
//!
//! The storage part: the table of setting values (`SettingRow` in `bb-store`) is read and written
//! in `crates/bb-store/src/settings.rs` only, which checks every key, level and type against the
//! registry. Its name appearing anywhere else (except the schema and the migrations that create
//! it) is a way around the registry.

use std::collections::BTreeSet;

use super::{Check, Context, Finding};
use crate::rust_registry::declared_keys;
use crate::rust_source::strip_comments;
use crate::rust_source::without_tests;
use crate::source::{Kind, Source};

/// The only place settings are declared.
pub(crate) const REGISTRY_FOLDER: &str = "crates/bb-core/src/settings/";
/// The file of the registry, which the browser client's keys, choices and defaults are made from.
pub(crate) const CATALOG_PATH: &str = "crates/bb-core/src/settings/catalog.rs";
/// Where the migrations are: they name the keys of their time as text, on purpose.
const MIGRATIONS_FOLDER: &str = "crates/bb-store/src/migrations/";

/// The name of the table of setting values in the schema, and the table itself as it is written
/// in text (the name of the table is `setting`).
const STORAGE_TABLE: &str = "SettingRow";
const STORAGE_TABLE_NAME: &str = "\"setting\"";
/// What turns a text into a table name in a query.
const TABLE_NAME_CALLS: [&str; 6] = [
    "Alias::new",
    ".from(",
    ".table(",
    "into_table(",
    "from_table(",
    "Iden",
];
/// The files that may name the table: the repository, the schema and the migrations.
const STORAGE_FILES: [&str; 3] = [
    "crates/bb-store/src/settings.rs",
    "crates/bb-store/src/schema.rs",
    "crates/bb-store/src/migrations/",
];

pub(crate) struct ArchSettings;

impl Check for ArchSettings {
    fn id(&self) -> &'static str {
        "ARCH-SETTINGS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let mut sources = context.sources(self.id(), Kind::Rust)?;
        sources.extend(
            context
                .sources(self.id(), Kind::Web)?
                .into_iter()
                .filter(|(path, _)| !NOT_APP_CODE.iter().any(|marker| path.contains(marker))),
        );
        Ok(evaluate(&sources))
    }
}

/// Web files that are not the app's own code: tests, end-to-end tests and generated files. The app
/// must not read settings by key text either.
const NOT_APP_CODE: [&str; 4] = [".test.", ".spec.", "/e2e/", "generated."];

/// What comes before a string that names a message of the catalog, not a setting: messages and
/// settings are separate namespaces, so a message may have the same text as the key of a setting
/// (`appearance.larger-text` is a message and a setting).
const MESSAGE_CALLS: [&str; 3] = ["t(", "isMessageKey(", "MessageKey::new("];

/// Whether `line` has `key` as the whole text of a string: in double, single or backtick quotes, or
/// in a raw string (`r#"key"#`). A string that is the argument of a call that looks up a message
/// does not count.
fn has_key_literal(line: &str, key: &str) -> bool {
    let is_quote = |c: char| matches!(c, '"' | '\x27' | '`');
    line.match_indices(key).any(|(at, _)| {
        let before = &line[..at];
        let opens_a_message = before.strip_suffix(is_quote).is_some_and(|call| {
            MESSAGE_CALLS.iter().any(|name| {
                call.strip_suffix(name)
                    .is_some_and(|rest| !rest.ends_with(|c: char| c.is_alphanumeric() || c == '_'))
            })
        });
        !opens_a_message
            && before.chars().next_back().is_some_and(is_quote)
            && line[at + key.len()..].chars().next().is_some_and(is_quote)
    })
}

/// Code outside the settings repository that names the table of setting values.
fn storage_findings(sources: &[Source]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (path, text) in sources.iter().filter(|(path, _)| {
        path.starts_with("crates/") && !STORAGE_FILES.iter().any(|file| path.starts_with(file))
    }) {
        for (index, line) in strip_comments(text).lines().enumerate() {
            let by_name = line.contains(STORAGE_TABLE_NAME)
                && TABLE_NAME_CALLS.iter().any(|call| line.contains(call));
            if line.contains(STORAGE_TABLE) || by_name {
                findings.push(Finding::at(
                    format!("{path}:{}", index + 1),
                    format!(
                        "The table of setting values (`{STORAGE_TABLE}`) is read and written in \
                         {} only, which checks the registry. Use `Database::set_setting`, \
                         `stored_setting` or `resolve_setting`.",
                        STORAGE_FILES[0]
                    ),
                ));
            }
        }
    }
    findings
}

fn evaluate(sources: &[Source]) -> Vec<Finding> {
    let mut findings = storage_findings(sources);
    let mut keys = BTreeSet::new();
    for (path, text) in sources
        .iter()
        .filter(|(path, _)| path.starts_with(REGISTRY_FOLDER) && path.ends_with("/catalog.rs"))
    {
        match declared_keys(text) {
            Ok(found) => {
                for key in found {
                    if !keys.insert(key.clone()) {
                        findings.push(Finding::at(
                            path,
                            format!("The setting `{key}` is declared twice."),
                        ));
                    }
                }
            }
            Err(error) => findings.push(Finding::at(
                path,
                format!("The registry cannot be read: {error:#}"),
            )),
        }
    }
    for (path, text) in sources.iter().filter(|(path, _)| {
        !path.starts_with(REGISTRY_FOLDER)
            && !path.starts_with(MIGRATIONS_FOLDER)
            && !path.contains("generated.")
    }) {
        let code = strip_comments(&without_tests(text));
        for (index, line) in code.lines().enumerate() {
            let place = format!("{path}:{}", index + 1);
            if declares(line) {
                findings.push(Finding::at(
                    &place,
                    format!("Settings are declared only in {REGISTRY_FOLDER}; add this one there."),
                ));
            }
            for key in &keys {
                if has_key_literal(line, key) {
                    findings.push(Finding::at(
                        &place,
                        format!("The key of the setting `{key}` is written as text. Read it through its handle in `bb_core::settings`."),
                    ));
                }
            }
        }
    }
    findings
}

/// Whether the line starts a `settings! { … }` declaration.
fn declares(line: &str) -> bool {
    line.contains("settings! {") || line.contains("settings!{")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry(text: &str) -> Source {
        (format!("{REGISTRY_FOLDER}catalog.rs"), text.to_owned())
    }

    fn found(sources: &[Source]) -> String {
        evaluate(sources)
            .into_iter()
            .map(|f| f.message)
            .collect::<Vec<_>>()
            .join("\n")
    }

    const DECLARATION: &str = "settings! {\n    THEME: Text = {\n        key: \"appearance.theme\",\n        default: \"device\",\n    }\n}\n";

    #[test]
    fn good_example() {
        let other = (
            "crates/bb-api/src/lib.rs".to_owned(),
            "let theme = settings::APPEARANCE_THEME;\n// settings! lives elsewhere\n".to_owned(),
        );
        assert!(found(&[registry(DECLARATION), other]).is_empty());
    }

    #[test]
    fn bad_example() {
        let other = (
            "crates/bb-api/src/lib.rs".to_owned(),
            "settings! {\n}\nlet theme = store.get(\"appearance.theme\");\n#[cfg(test)]\nmod tests { const K: &str = \"appearance.theme\"; }\n".to_owned(),
        );
        let report = found(&[registry(&format!("{DECLARATION}{DECLARATION}")), other]);
        assert!(report.contains("declared twice"), "{report}");
        assert!(report.contains("Settings are declared only in"), "{report}");
        assert!(report.contains("written as text"), "{report}");
        assert_eq!(
            report.matches("written as text").count(),
            1,
            "tests may name keys"
        );
    }

    #[test]
    fn good_example_a_migration_names_the_keys_of_its_time() {
        let migration = (
            "crates/bb-store/src/migrations/m0004_settings.rs".to_owned(),
            "const MODE: &str = \"appearance.theme\";
"
            .to_owned(),
        );
        assert!(found(&[registry(DECLARATION), migration]).is_empty());
        // Any other file of the store may not.
        let repository = (
            "crates/bb-store/src/jobs.rs".to_owned(),
            "const MODE: &str = \"appearance.theme\";
"
            .to_owned(),
        );
        assert!(found(&[registry(DECLARATION), repository]).contains("written as text"));
    }

    #[test]
    fn good_example_the_settings_repository_names_its_table() {
        let repository = (
            "crates/bb-store/src/settings.rs".to_owned(),
            "use crate::schema::SettingRow as Columns;\n".to_owned(),
        );
        let schema = (
            "crates/bb-store/src/schema.rs".to_owned(),
            "pub(crate) enum SettingRow { Table }\n".to_owned(),
        );
        assert!(found(&[registry(DECLARATION), repository, schema]).is_empty());
    }

    #[test]
    fn bad_example_other_code_names_the_table_as_text() {
        let around = (
            "crates/bb-store/src/jobs.rs".to_owned(),
            "select.from(Alias::new(\"setting\"));
let text = \"setting\";
"
            .to_owned(),
        );
        let report = found(&[registry(DECLARATION), around]);
        assert_eq!(
            report.matches("table of setting values").count(),
            1,
            "{report}"
        );
    }

    #[test]
    fn bad_example_other_code_reads_the_table_of_setting_values() {
        let around = (
            "crates/bb-store/src/jobs.rs".to_owned(),
            "// SettingRow is fine in a comment\nselect.from(SettingRow::Table);\n".to_owned(),
        );
        let report = found(&[registry(DECLARATION), around]);
        assert!(report.contains("table of setting values"), "{report}");
        assert_eq!(report.matches("table of setting values").count(), 1);
    }

    #[test]
    fn keys_are_found_in_every_kind_of_string_and_in_the_browser_code() {
        for line in [
            "get(\"appearance.theme\")",
            "get('appearance.theme')",
            "get(`appearance.theme`)",
            "get(r#\"appearance.theme\"#)",
        ] {
            assert!(has_key_literal(line, "appearance.theme"), "{line}");
        }
        assert!(!has_key_literal(
            "let a = \"appearance.theme.help\";",
            "appearance.theme"
        ));
        assert!(!has_key_literal("// appearance.theme", "appearance.theme"));
        // A message of the catalog may be called like a setting: they are separate namespaces.
        for line in [
            "t(\"appearance.theme\")",
            "label={t(\"appearance.theme\")}",
            "isMessageKey(\"appearance.theme\")",
            "MessageKey::new(\"appearance.theme\")",
        ] {
            assert!(!has_key_literal(line, "appearance.theme"), "{line}");
        }
        // A function whose name only ends like a message call is not one.
        assert!(has_key_literal(
            "fruit(\"appearance.theme\")",
            "appearance.theme"
        ));
        assert!(has_key_literal(
            "set(\"appearance.theme\")",
            "appearance.theme"
        ));
        let web = (
            "web/app/src/a.ts".to_owned(),
            "load('appearance.theme');\n".to_owned(),
        );
        assert!(found(&[registry(DECLARATION), web]).contains("written as text"));
    }

    #[test]
    fn good_example_the_browser_names_a_setting_through_the_generated_keys() {
        let keys = (
            "web/packages/api-client/src/setting-keys.generated.ts".to_owned(),
            "export const settingKeys = { appearanceMode: \"appearance.theme\" };
"
            .to_owned(),
        );
        let uses = (
            "web/app/src/a.ts".to_owned(),
            "load(settingKeys.appearanceMode);
const name = t(\"appearance.theme\");
"
            .to_owned(),
        );
        let generated_rust = (
            "crates/bb-core/src/message/generated.rs".to_owned(),
            "pub const X: MessageKey = MessageKey::new(\"appearance.theme\");
(\"appearance.theme\", \"x\"),
"
            .to_owned(),
        );
        assert!(found(&[registry(DECLARATION), keys, uses, generated_rust]).is_empty());
    }

    #[test]
    fn the_settings_of_this_repository_pass() {
        let repo = crate::repo::Repo::discover().unwrap();
        let sources = crate::source::read_sources(&repo, None, Kind::Rust).unwrap();
        assert!(found(&sources).is_empty(), "{}", found(&sources));
    }
}
