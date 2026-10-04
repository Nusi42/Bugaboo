//! Looks up user-facing text in the message catalog (`messages/en.json`, UX-07).
//!
//! Minimal on purpose: the typed Rust lookup generated from the catalog replaces this
//! (M0-F01). Until then every sentence still lives only in the catalog.

use std::collections::BTreeMap;
use std::sync::OnceLock;

const CATALOG: &str = include_str!("../../../messages/en.json");

fn catalog() -> &'static BTreeMap<String, String> {
    static PARSED: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    PARSED.get_or_init(|| serde_json::from_str(CATALOG).unwrap_or_default())
}

/// The text for `key` with each `{name}` replaced by its value. An unknown key gives the key
/// itself, so a mistake stays visible without stopping the program.
pub(crate) fn t(key: &str, values: &[(&str, &str)]) -> String {
    let mut text = catalog()
        .get(key)
        .cloned()
        .unwrap_or_else(|| key.to_owned());
    for (name, value) in values {
        text = text.replace(&format!("{{{name}}}"), value);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalog_is_readable_and_placeholders_are_filled() {
        assert!(!catalog().is_empty());
        assert_eq!(
            t("server.port-busy", &[("port", "47200")]),
            "Port 47200 is busy or not allowed. Choose another port."
        );
        assert_eq!(t("no.such-key", &[]), "no.such-key");
    }
}
