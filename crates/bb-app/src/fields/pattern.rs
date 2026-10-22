//! The pattern of a rule (CF-03): checked by an engine that cannot run away. The engine is the
//! `regex` crate, whose matching time grows with the length of the text and never blows up; the
//! pattern has a length cap (`bb_core::own_fields::PATTERN_MAX`) and the compiled program a size
//! cap, so a pattern cannot cost more than a small, fixed amount (SEC-PATTERNS).

use crate::auth::locked;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use bb_core::own_fields_value::PatternEngine;
use regex::RegexBuilder;

/// The most memory one compiled pattern may take, in bytes.
const PROGRAM_BYTES_MAX: usize = 100_000;

/// How many compiled patterns are kept. The patterns are those of the field definitions, so a few
/// hundred is more than an organization has; a full cache is emptied rather than grown.
const CACHE_MAX: usize = 256;

/// The patterns compiled so far, by their text (`None` for one that does not compile), so a value
/// of a field is not checked by compiling its pattern again.
static COMPILED: LazyLock<Mutex<HashMap<String, Option<regex::Regex>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// A pattern as a regular expression that must match the whole text.
fn compile(pattern: &str) -> Option<regex::Regex> {
    RegexBuilder::new(&format!("^(?:{pattern})$"))
        .size_limit(PROGRAM_BYTES_MAX)
        .dfa_size_limit(PROGRAM_BYTES_MAX * 10)
        .build()
        .ok()
}

/// The pattern compiled, from the cache when it was compiled before.
fn compiled(pattern: &str) -> Option<regex::Regex> {
    let mut cache = locked(&COMPILED);
    if let Some(found) = cache.get(pattern) {
        return found.clone();
    }
    let built = compile(pattern);
    if cache.len() >= CACHE_MAX {
        cache.clear();
    }
    cache.insert(pattern.to_owned(), built.clone());
    built
}

/// Whether the pattern can be used: it compiles within the size cap.
pub(crate) fn is_usable(pattern: &str) -> bool {
    compiled(pattern).is_some()
}

/// The linear-time engine of the server.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Linear;

impl PatternEngine for Linear {
    fn matches(&self, pattern: &str, text: &str) -> bool {
        // A pattern that does not compile was refused when the rule was made; a stored one that
        // no longer does lets nothing through.
        compiled(pattern).is_some_and(|program| program.is_match(text))
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
    fn cf_03_the_cache_of_compiled_patterns_does_not_grow_without_end() {
        for number in 0..CACHE_MAX * 2 {
            assert!(Linear.matches(&format!("a{number}"), &format!("a{number}")));
            assert!(Linear.matches(&format!("a{number}"), &format!("a{number}")));
        }
        let kept = locked(&COMPILED).len();
        assert!(kept <= CACHE_MAX, "{kept} patterns kept");
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
