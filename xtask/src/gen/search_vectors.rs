//! `cargo xtask gen search-vectors`: the test vectors that hold the browser's module and the
//! server's code together (QRY-03).
//!
//! The texts, the canonical texts, the mistakes and the suggestions are what the Rust code makes of
//! the texts of `bb_searchlang::vectors`; the unit tests of the browser run the WebAssembly module on
//! the same texts and check that it answers the same. DOC-GENERATED fails when the file is not what
//! the code gives.

use anyhow::Context as _;

use super::Generated;
use super::api::format_with_oxfmt;
use crate::repo::Repo;

/// Where the vectors are written, relative to the repository root.
pub(crate) const OUTPUT: &str = "tools/testdata/search-vectors.json";

/// The file, as it should be.
pub(crate) fn generate(repo: &Repo) -> anyhow::Result<Vec<Generated>> {
    let text = bb_searchlang::vectors::file().context("the vectors cannot be written as JSON")?;
    Ok(vec![Generated::new(
        OUTPUT,
        format_with_oxfmt(repo, OUTPUT, &text)?,
    )])
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_vectors_are_json_with_cases_and_places() {
        let text = bb_searchlang::vectors::file().unwrap_or_default();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
        assert!(
            parsed["cases"]
                .as_array()
                .is_some_and(|cases| cases.len() > 40)
        );
        assert!(
            parsed["places"]
                .as_array()
                .is_some_and(|places| places.len() > 20)
        );
        assert!(parsed["catalogue"]["fields"].as_array().is_some());
    }
}
