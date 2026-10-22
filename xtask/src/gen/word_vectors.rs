//! `cargo xtask gen word-vectors`: the test vectors that hold the two writings of a value together
//! (ADR-0033, the amendment about value wording).
//!
//! The expected words are what the server writes, taken from `bb_core::wording::vectors`; the
//! browser test reads the file and checks that the screen writes the same. DOC-GENERATED fails
//! when the file is not what the code gives.

use super::Generated;

/// Where the vectors are written, relative to the repository root.
pub(crate) const OUTPUT: &str = "tools/testdata/value-words.json";

/// The file, as it should be.
pub(crate) fn generate() -> anyhow::Result<Vec<Generated>> {
    let text = bb_core::wording::vectors::value_words_file()
        .map_err(|problem| anyhow::anyhow!("the vectors cannot be made: {problem}"))?;
    Ok(vec![Generated::new(OUTPUT, text)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_vectors_are_json_with_the_cases_of_the_server() {
        let files = generate().unwrap_or_default();
        let parsed: serde_json::Value = serde_json::from_str(&files[0].content).unwrap_or_default();
        assert!(
            parsed["cases"]
                .as_array()
                .is_some_and(|cases| cases.len() > 20)
        );
        assert!(
            parsed["lists"]
                .as_array()
                .is_some_and(|lists| lists.len() == 5)
        );
    }
}
