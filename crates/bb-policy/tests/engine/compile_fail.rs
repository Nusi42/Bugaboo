//! ARCH-AUTHZ-ONLY (ADR-0004): code outside `bb-policy` cannot make an `Authorized<T>`, and not an
//! evaluation to make one from. Each example below must fail to compile; the source check of
//! `cargo xtask check only ARCH-AUTHZ-ONLY` finds the ways around the compiler (a macro that
//! writes the type, a copy of it).

#[test]
fn arch_authz_only_authorized_cannot_be_made_outside_the_policy_crate() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/authorized_by_struct_literal.rs");
    cases.compile_fail("tests/ui/authorized_by_constructor.rs");
    cases.compile_fail("tests/ui/evaluation_by_hand.rs");
}

#[test]
fn sec_04_a_view_cannot_hold_a_member_that_does_not_say_how_it_is_masked() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/masked_member_not_declared.rs");
}

#[test]
fn sec_04_an_open_member_cannot_be_a_view_with_fields_to_mask() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/masked_open_view.rs");
}
