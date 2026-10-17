//! The tests of permissions.

use std::collections::BTreeSet;

use super::*;
use crate::openapi::operations;

#[test]
fn adr_0019_a_route_that_saves_long_text_is_a_route() {
    for id in LONG_TEXT_ROUTES {
        assert!(declared(id).is_some(), "{id} is no route");
    }
}

#[test]
fn m1_f32_no_row_names_a_route_that_does_not_exist_or_a_route_twice() {
    let ids: BTreeSet<&str> = operations()
        .iter()
        .map(|operation| operation.id.as_str())
        .collect();
    for row in ROUTES {
        assert!(
            ids.contains(row.id),
            "{} is declared but is no route",
            row.id
        );
    }
    let rows: BTreeSet<&str> = ROUTES.iter().map(|row| row.id).collect();
    assert_eq!(
        rows.len(),
        ROUTES.len(),
        "an operation ID is declared twice"
    );
}

#[test]
fn perf_03_only_a_read_is_never_stored_and_every_read_is_ordinary_or_never_stored() {
    for operation in operations() {
        let reads = operation.method == Method::GET;
        assert!(
            reads || operation.kind != Answer::NeverStored,
            "{} {} is a change: its answer is never kept anyway",
            operation.method,
            operation.path
        );
        assert!(
            !reads || operation.kind != Answer::Secret,
            "GET {} hands out a secret: a read is ordinary or never stored",
            operation.path
        );
    }
}

#[test]
fn sec_12_only_public_changes_are_open_without_a_token_and_they_name_one_exact_address() {
    let mut open = 0;
    for operation in operations() {
        let exempt = csrf_exempt(&operation.method, &operation.path);
        let public_change = operation.changes() && operation.needs == Declared::Public;
        assert_eq!(
            exempt, public_change,
            "{} {}",
            operation.method, operation.path
        );
        if exempt {
            open += 1;
            assert_eq!(operation.method, Method::POST, "{}", operation.path);
            // The address is compared as it is written, so it has no part that varies.
            assert!(!operation.path.contains('{'), "{}", operation.path);
        }
    }
    assert!(open > 0, "no route is open without a token");
}
