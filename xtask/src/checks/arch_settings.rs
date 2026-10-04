//! ARCH-SETTINGS (pattern part): settings are declared only in the settings registry and read only
//! through its typed handles (ADR-0018).
//!
//! Two patterns are found in the code of the product crates: a `settings!` declaration outside
//! `crates/bb-core/src/settings/`, and the key of a registered setting written as text anywhere
//! else (which would be a way around the registry). The part that watches storage comes with the
//! store (WP-05).

use std::collections::BTreeSet;

use super::{Check, Context, Finding};
use crate::rust_source::{Source, before_tests, read_product_sources};

/// The only place settings are declared.
const REGISTRY_FOLDER: &str = "crates/bb-core/src/settings/";

pub(crate) struct ArchSettings;

impl Check for ArchSettings {
    fn id(&self) -> &'static str {
        "ARCH-SETTINGS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        Ok(evaluate(&read_product_sources(context.repo)?))
    }
}

fn evaluate(sources: &[Source]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut keys = BTreeSet::new();
    for (path, text) in sources
        .iter()
        .filter(|(path, _)| path.starts_with(REGISTRY_FOLDER))
    {
        for key in declared_keys(text) {
            if !keys.insert(key.clone()) {
                findings.push(Finding::at(
                    path,
                    format!("The setting `{key}` is declared twice."),
                ));
            }
        }
    }
    for (path, text) in sources
        .iter()
        .filter(|(path, _)| !path.starts_with(REGISTRY_FOLDER))
    {
        let code = before_tests(text);
        for (index, line) in code.lines().enumerate() {
            let place = format!("{path}:{}", index + 1);
            if declares(line) {
                findings.push(Finding::at(
                    &place,
                    format!("Settings are declared only in {REGISTRY_FOLDER}; add this one there."),
                ));
            }
            for key in &keys {
                if line.contains(&format!("\"{key}\"")) {
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

/// Whether the line starts a `settings! { … }` declaration (not a comment about one).
fn declares(line: &str) -> bool {
    let code = line.trim_start();
    !code.starts_with("//") && (code.contains("settings! {") || code.contains("settings!{"))
}

/// The keys in `key: "…",` lines of a declaration.
fn declared_keys(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("key:")?.trim();
            let rest = rest.strip_prefix('"')?;
            Some(rest[..rest.find('"')?].to_owned())
        })
        .collect()
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

    const DECLARATION: &str =
        "settings! {\n    THEME: Text = {\n        key: \"appearance.theme\",\n    }\n}\n";

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
    fn the_settings_of_this_repository_pass() {
        let repo = crate::repo::Repo::discover().unwrap();
        let sources = read_product_sources(&repo).unwrap();
        assert!(found(&sources).is_empty(), "{}", found(&sources));
    }
}
