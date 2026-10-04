//! DATA-MARKS (ADR-0015): a stored field without a personal-data mark does not compile, and the
//! marks that are accepted end up in the constant the personal-data map reads.

// Dependencies of the library that this test does not use itself (`unused_crate_dependencies`).
use bb_macros as _;
use jiff as _;
use serde as _;
use serde_json as _;
#[cfg(feature = "openapi")]
use utoipa as _;
use uuid as _;

use bb_core::personal::{Basis, Category, Entity, FieldMark};

#[derive(Entity)]
struct Marked {
    #[not_personal]
    id: u32,
    #[personal(category = "employee_time", basis = "legal_obligation")]
    worked_minutes: u32,
}

#[test]
fn priv_02_the_derive_writes_every_mark_into_the_constant() {
    let marked = Marked {
        id: 1,
        worked_minutes: 90,
    };
    assert_eq!(marked.id + marked.worked_minutes, 91);
    let marks = <Marked as Entity>::MARKS;
    assert_eq!(marks.name(), "Marked");
    assert_eq!(
        marks.fields(),
        [
            FieldMark::not_personal("id"),
            FieldMark::personal(
                "worked_minutes",
                Category::EmployeeTime,
                Basis::LegalObligation
            ),
        ]
    );
}

#[test]
fn priv_02_unmarked_or_wrongly_marked_fields_do_not_compile() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/marked.rs");
    cases.compile_fail("tests/ui/unmarked_field.rs");
    cases.compile_fail("tests/ui/two_marks.rs");
    cases.compile_fail("tests/ui/missing_basis.rs");
    cases.compile_fail("tests/ui/unknown_category.rs");
    cases.compile_fail("tests/ui/not_a_struct.rs");
    cases.compile_fail("tests/ui/tuple_struct.rs");
    cases.compile_fail("tests/ui/generic_struct.rs");
    cases.compile_fail("tests/ui/not_personal_person.rs");
    cases.compile_fail("tests/ui/not_personal_text.rs");
}
