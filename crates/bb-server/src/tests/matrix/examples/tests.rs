use std::collections::BTreeSet;

use super::*;

#[test]
fn every_route_has_one_row_only() {
    let operations: BTreeSet<&str> = EXAMPLES.iter().map(|example| example.operation).collect();
    assert_eq!(operations.len(), EXAMPLES.len());
}

#[test]
fn only_the_routes_that_list_what_is_seen_may_answer_a_person_without_the_permission() {
    let filtering: Vec<&str> = EXAMPLES
        .iter()
        .filter(|example| example.has(Quirk::FiltersWhatIsSeen))
        .map(|example| example.operation)
        .collect();
    assert_eq!(
        filtering,
        ["listProjects"],
        "M2-F30: one route, with its reason"
    );
}
