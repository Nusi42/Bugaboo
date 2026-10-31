//! `cargo xtask gen time-vectors`: the test vectors that hold the worked-out *Time left* of
//! `bb-core` and its SQL expression in the search compiler together (ADR-0035, decision 4; the
//! second approved pair of coding-standards §1).
//!
//! The answers are what `bb_core::time::left` gives; the two-database test
//! `time_03_the_compiled_time_left_matches_the_vectors` of `bb-store` reads the file and runs
//! every case through the compiler. DOC-GENERATED fails when the file is not what the code gives.

use super::Generated;

/// Where the vectors are written, relative to the repository root.
pub(crate) const OUTPUT: &str = "tools/testdata/time-left.json";

/// The file, as it should be.
pub(crate) fn generate() -> anyhow::Result<Vec<Generated>> {
    let text = bb_core::time::left::time_left_file()
        .map_err(|problem| anyhow::anyhow!("the vectors cannot be made: {problem}"))?;
    Ok(vec![Generated::new(OUTPUT, text)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_vectors_are_json_with_the_cases_of_the_rule() {
        let files = generate().unwrap_or_default();
        let parsed: serde_json::Value = serde_json::from_str(&files[0].content).unwrap_or_default();
        assert!(
            parsed["cases"]
                .as_array()
                .is_some_and(|cases| cases.len() > 5)
        );
    }
}
