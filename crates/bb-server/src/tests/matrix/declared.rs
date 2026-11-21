//! The matrix walks only the routes that have a row in the table of what routes need, so it must
//! refuse to run while a route has none (SEC-MATRIX, ARCH-PERMS).

use bb_testing::Failure;

/// Fails when a route of the API description has no row in the table of what routes need: the
/// matrix would pass such a route unrun.
pub(super) fn every_route_declares(undeclared: &[String]) -> Result<(), Failure> {
    if undeclared.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "routes that declare no permission, so the matrix would not run them: {}",
            undeclared.join(", ")
        )
        .into())
    }
}

#[test]
fn sec_01_the_matrix_fails_for_a_route_that_declares_no_permission() {
    assert!(every_route_declares(&[]).is_ok());
    let refused = every_route_declares(&["GET /api/v1/undeclared".to_owned()])
        .expect_err("a route without a declaration passes the matrix");
    assert!(
        refused.to_string().contains("GET /api/v1/undeclared"),
        "{refused}"
    );
}
