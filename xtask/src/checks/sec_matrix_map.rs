//! SEC-MATRIX, the relation map (ISS-16, PRIV-12, PLAN-08, UX design §4.15 *Security*): the matrix
//! has a `map` line for every starting role and each kind of map, and each says that nothing of the
//! hidden issue, and nothing reached only through it, was on the map; beside it the two property
//! tests that try random graphs exist, are declared and are not switched off.

use bb_api_words::sec_matrix::{self as words, RECORD_PATH};
use bb_policy::StartingRole;

use super::Finding;
use super::sec_matrix::said_lines;

/// The file of the property tests of the map.
pub(super) const PROPERTY_PATH: &str = "crates/bb-app/src/tests/relation_map_sight.rs";
/// The module list that declares it.
pub(super) const TESTS_PATH: &str = "crates/bb-app/src/tests/mod.rs";
/// The module of the property tests in that list.
const PROPERTY_MODULE: &str = "mod relation_map_sight;";
/// The property tests that must be there.
const PROPERTY_TESTS: [&str; 2] = [
    "iss_16_no_chain_passes_through_a_hidden_issue",
    "iss_16_the_map_is_the_map_of_the_issues_the_reader_sees",
];
/// What switches a test off.
const SWITCHED_OFF: &str = "#[ignore";

/// What the record and the property tests say of the relation map. `tests` is the file of the
/// property tests and the module list, when they could be read.
pub(super) fn map_findings(record: &str, tests: Option<(&str, &str)>) -> Vec<Finding> {
    let mut found = Vec::new();
    let lines = said_lines(record, words::MAP_LINE);
    for kind in words::MAP_KINDS {
        for role in StartingRole::ALL {
            let actor = words::role_actor(role.code());
            let place = format!("The `{kind}` map of `{actor}`");
            let Some((_, said)) = lines
                .iter()
                .find(|(head, _)| *head == format!("{kind} {actor}"))
            else {
                found.push(Finding::at(
                    RECORD_PATH,
                    format!("{place} is missing from the record: run the matrix."),
                ));
                continue;
            };
            let mut wanted = vec!["hidden"];
            if kind == words::MAP_OF_ISSUE {
                wanted.push("behind");
            }
            for name in wanted {
                if said.get(name).map(String::as_str) != Some(words::ABSENT) {
                    found.push(Finding::at(
                        RECORD_PATH,
                        format!(
                            "{place} must say `{name}={}`: a map never shows or walks through an \
                             issue the reader cannot see (ISS-16).",
                            words::ABSENT
                        ),
                    ));
                }
            }
        }
    }
    let Some((property, modules)) = tests else {
        found.push(Finding::at(
            PROPERTY_PATH,
            "The property tests of the relation map are missing.",
        ));
        return found;
    };
    if !modules.lines().any(|line| line.trim() == PROPERTY_MODULE) {
        found.push(Finding::at(
            TESTS_PATH,
            format!("`{PROPERTY_MODULE}` is not declared, so its property tests do not run."),
        ));
    }
    for test in PROPERTY_TESTS {
        if !property.contains(&format!("fn {test}(")) {
            found.push(Finding::at(
                PROPERTY_PATH,
                format!("The property test `{test}` is missing."),
            ));
        }
    }
    if property.contains(SWITCHED_OFF) {
        found.push(Finding::at(
            PROPERTY_PATH,
            "A property test of the relation map is switched off.",
        ));
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(hidden: &str) -> String {
        StartingRole::ALL
            .iter()
            .map(|role| {
                let actor = words::role_actor(role.code());
                format!(
                    "map issue {actor} : hidden={hidden} behind=absent\nmap search {actor} : hidden=absent\n"
                )
            })
            .collect::<Vec<_>>()
            .concat()
    }

    fn property() -> String {
        PROPERTY_TESTS
            .iter()
            .map(|test| format!("    #[test]\n    fn {test}(case in case()) {{}}\n"))
            .collect::<Vec<_>>()
            .concat()
    }

    fn messages(record: &str, tests: Option<(&str, &str)>) -> Vec<String> {
        map_findings(record, tests)
            .into_iter()
            .map(|finding| finding.message)
            .collect()
    }

    #[test]
    fn good_example_every_role_sees_nothing_hidden_and_the_property_tests_run() {
        let found = messages(&record(words::ABSENT), Some((&property(), PROPERTY_MODULE)));
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn bad_example_a_map_that_walks_through_a_hidden_issue() {
        let found = messages(
            &record(words::PRESENT),
            Some((&property(), PROPERTY_MODULE)),
        );
        assert_eq!(found.len(), StartingRole::ALL.len(), "{found:?}");
        assert!(found[0].contains("must say `hidden=absent`"), "{found:?}");
        let missing = record(words::ABSENT).replacen("map search", "gone", 1);
        let found = messages(&missing, Some((&property(), PROPERTY_MODULE)));
        assert!(found[0].contains("missing from the record"), "{found:?}");
    }

    #[test]
    fn bad_example_property_tests_that_are_missing_or_switched_off() {
        let good = record(words::ABSENT);
        assert_eq!(messages(&good, None).len(), 1);
        let one = property().replace(PROPERTY_TESTS[0], "another_test");
        let found = messages(&good, Some((&one, PROPERTY_MODULE)));
        assert!(found[0].contains(PROPERTY_TESTS[0]), "{found:?}");
        let off = format!("#[ignore = \"slow\"]\n{}", property());
        let found = messages(&good, Some((&off, PROPERTY_MODULE)));
        assert!(found[0].contains("switched off"), "{found:?}");
        let found = messages(&good, Some((&property(), "")));
        assert!(found[0].contains("not declared"), "{found:?}");
    }
}
