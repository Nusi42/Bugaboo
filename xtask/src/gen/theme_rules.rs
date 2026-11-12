//! The contrast rules of the base theme file, written as data into `bb-core` (ADR-0026, update M8),
//! so the server and the theme editor read the rules that the check of the built-in files reads.

use anyhow::Context as _;

use super::theme_file::ThemeFile;
use bb_core::contrast::Rules;

/// Where the rules are written.
pub(super) const OUTPUT: &str = "crates/bb-core/src/contrast/base_rules.rs";

/// The source of `base_rules.rs`, before `rustfmt`: the rules as one JSON text that `base_rules`
/// reads (a list of this size as Rust expressions would be one function of a thousand lines).
pub(super) fn rust(file: &ThemeFile, generated_by: &str) -> anyhow::Result<String> {
    let rules = Rules {
        contrast: file.contrast.clone(),
        distinct: file.distinct.clone(),
    };
    let json = serde_json::to_string_pretty(&rules).context("the rules are not JSON")?;
    anyhow::ensure!(!json.contains("\"#"), "a rule holds a quote and a hash");
    Ok(format!(
        "//! {generated_by}\n//!\n//! The contrast rules of the base theme file (`contrast` and `distinct`).\n\nuse super::Rules;\n\nconst RULES: &str = r#\"{json}\"#;\n\n/// The rules of the base theme file.\n///\n/// # Errors\n///\n/// Never in practice: the text is written by the generator and a test reads it.\npub fn base_rules() -> Result<Rules, serde_json::Error> {{\n    serde_json::from_str(RULES)\n}}\n"
    ))
}
