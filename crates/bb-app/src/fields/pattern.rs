//! The pattern of a rule (CF-03): checked by an engine that cannot run away. The engine is the
//! `regex` crate, whose matching time grows with the length of the text and never blows up; the
//! pattern has a length cap (`bb_core::own_fields::PATTERN_MAX`) and the compiled program a size
//! cap, so a pattern cannot cost more than a small, fixed amount (SEC-PATTERNS).

use bb_core::own_fields_value::PatternEngine;
use regex::RegexBuilder;

/// The most memory one compiled pattern may take, in bytes.
const PROGRAM_BYTES_MAX: usize = 100_000;

/// A pattern as a regular expression that must match the whole text.
fn compile(pattern: &str) -> Option<regex::Regex> {
    RegexBuilder::new(&format!("^(?:{pattern})$"))
        .size_limit(PROGRAM_BYTES_MAX)
        .dfa_size_limit(PROGRAM_BYTES_MAX * 10)
        .build()
        .ok()
}

/// Whether the pattern can be used: it compiles within the size cap.
pub(crate) fn is_usable(pattern: &str) -> bool {
    compile(pattern).is_some()
}

/// The linear-time engine of the server.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Linear;

impl PatternEngine for Linear {
    fn matches(&self, pattern: &str, text: &str) -> bool {
        // A pattern that does not compile was refused when the rule was made; a stored one that
        // no longer does lets nothing through.
        compile(pattern).is_some_and(|program| program.is_match(text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cf_03_a_pattern_matches_the_whole_text_and_a_bad_one_is_refused() {
        assert!(Linear.matches("[A-Z]{3}-[0-9]+", "ABC-12"));
        assert!(!Linear.matches("[A-Z]{3}-[0-9]+", "xABC-12"));
        assert!(!Linear.matches("[A-Z]{3}-[0-9]+", "ABC-12x"));
        assert!(!is_usable("("));
        // Look-around and back-references are not part of a linear-time engine.
        assert!(!is_usable("(?=a)a"));
        assert!(!is_usable(r"(a)\1"));
    }

    #[test]
    fn cf_03_a_pattern_that_would_blow_up_in_other_engines_is_cheap_here() {
        let text = "a".repeat(5_000) + "!";
        let started = std::time::Instant::now();
        assert!(!Linear.matches("(a+)+$", &text));
        assert!(started.elapsed().as_secs() < 2);
        // A huge counted repeat is refused by the size cap, not run.
        assert!(!is_usable("((((a{1000}){1000}){1000}){1000})"));
    }
}
