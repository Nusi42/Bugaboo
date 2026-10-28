//! The tests of ARCH-AUTHZ-ONLY: a good and a bad example of each thing it reads.

use super::*;

fn source(path: &str, text: &str) -> Vec<Source> {
    vec![(path.to_owned(), text.to_owned())]
}

#[test]
fn good_example_a_use_case_asks_the_evaluation_and_the_policy_crate_builds_the_type() {
    let app = source(
        "crates/bb-app/src/roles/mod.rs",
        "fn view(access: &Evaluation) -> Authorized<RoleList> {\n    access.authorize(RoleList::default())\n}\n// Authorized { value } in a comment\nconst TEXT: &str = \"Authorized::new(\";\n",
    );
    assert!(evaluate(&app).is_empty());
    let policy = source(
        "crates/bb-policy/src/mask.rs",
        "#[derive(Clone, Debug)]\npub struct Authorized<T> {\n    value: T,\n    mask: FieldMask,\n}\nimpl<T> Authorized<T> {\n    pub const fn mask(&self) -> &FieldMask { &self.mask }\n}\nimpl Evaluation {\n    pub fn authorize<T>(&self, value: T) -> Authorized<T> {\n        Authorized { value, mask: self.mask() }\n    }\n}\n",
    );
    assert!(evaluate(&policy).is_empty());
    // A test may open one.
    let tests = source(
        "crates/bb-app/src/roles/mod.rs",
        "fn code() {}\n#[cfg(test)]\nmod tests {\n    fn t() { let x = answer.into_value(); }\n}\n",
    );
    assert!(evaluate(&tests).is_empty());
}

#[test]
fn bad_example_a_copy_a_literal_and_a_constructor_outside_the_policy_crate() {
    for text in [
        "pub struct Authorized<T> { pub value: T }\n",
        "fn f() -> Authorized<u8> { Authorized { value: 1 } }\n",
        "fn f() { let a = Authorized::new(1); }\n",
        "type Authorized<T> = Vec<T>;\n",
        "fn f() { let a = Authorized::<u8>::from(1); }\n",
    ] {
        let findings = evaluate(&source("crates/bb-api/src/api/x.rs", text));
        assert_eq!(findings.len(), 1, "{text}");
    }
}

#[test]
fn bad_example_the_engine_call_of_one_read_outside_the_policy_crate_and_the_access_module() {
    for text in [
        "fn f() { let e = evaluation_from_person_policy(&p, who, place, chain); }\n",
        "fn f() { let i = p.inputs_from_rows(who, &rows, place, chain); }\n",
        "fn f() { let e = p.evaluate_with(who, &rows, &tree, place); }\n",
        "fn f() { let e = p.evaluate_at(who, &tree, place); }\n",
        "fn f() { let r = p.rows_of(who); }\n",
        "fn f() { let r = Policy::rows_of(&p, who); }\n",
        "fn f() { let e = Policy::evaluate_with(&p, who, &rows, &tree, place); }\n",
        "fn f() { let e = <Policy>::evaluate_at(&p, who, &tree, place); }\n",
    ] {
        for folder in ["crates/bb-api/src/api/x.rs", "crates/bb-sample/src/x.rs"] {
            assert_eq!(evaluate(&source(folder, text)).len(), 1, "{folder}: {text}");
        }
    }
}

#[test]
fn bad_example_the_engine_methods_by_a_path_in_a_use_case() {
    for text in [
        "fn f() { let r = Policy::rows_of(&p, who); }\n",
        "fn f() { let e = Policy::evaluate_with(&p, who, &rows, &tree, place); }\n",
        "fn f() { let e = <Policy>::evaluate_at(&p, who, &tree, place); }\n",
        "fn f() { let e = p.evaluate_at(who, &tree, place); }\n",
        "fn f() { let v = Policy::visible(&p, who, &tree); }\n",
    ] {
        let findings = evaluate(&source("crates/bb-app/src/projects/x.rs", text));
        assert_eq!(findings.len(), 1, "{text}");
    }
}

#[test]
fn good_example_the_access_module_and_the_policy_crate_may_call_the_engine() {
    let text = "fn f() { let e = evaluation_from_person_policy(&p, who, place, chain); let i = p.inputs_from_rows(who, &rows, place, chain); let r = Policy::rows_of(&p, who); }\n";
    for folder in [
        "crates/bb-app/src/access/mod.rs",
        "crates/bb-policy/src/tree.rs",
    ] {
        assert!(evaluate(&source(folder, text)).is_empty(), "{folder}");
    }
    // A comment and a test do not count.
    let quiet = source(
        "crates/bb-api/src/api/x.rs",
        "// p.inputs_from_rows(who)\n#[cfg(test)]\nmod tests { fn t() { p.rows_of(who); } }\n",
    );
    assert!(evaluate(&quiet).is_empty());
}

#[test]
fn bad_example_code_after_a_test_module_is_still_read() {
    let findings = evaluate(&source(
        "crates/bb-api/src/api/x.rs",
        "#[cfg(test)]
mod tests {
fn t() { let a = Authorized::new(1); }
}
fn ships() { let b = Authorized::new(2); }
",
    ));
    assert_eq!(findings.len(), 1);
    assert!(
        findings[0]
            .location
            .as_deref()
            .is_some_and(|place| place.ends_with(":5")),
        "{}",
        findings[0].message
    );
}

#[test]
fn bad_example_a_character_literal_a_raw_string_or_a_block_comment_hides_nothing() {
    // A quote in a character literal does not start a string: the construction after it
    // is still read. The same text in a raw string or a block comment is not code.
    let findings = evaluate(&source(
        "crates/bb-api/src/api/x.rs",
        "fn f() { let quote = '\"'; let a = Authorized::new(1); }
",
    ));
    assert_eq!(findings.len(), 1);
    let hidden = evaluate(&source(
        "crates/bb-api/src/api/x.rs",
        "/* Authorized::new(1) */
const RAW: &str = r#\"Authorized::new(\"#;
const CHAR: char = '\"';
",
    ));
    assert!(hidden.is_empty());
}

#[test]
fn bad_example_the_engine_is_asked_or_its_input_built_outside_its_places() {
    for (path, text) in [
        (
            "crates/bb-api/src/api/x.rs",
            "fn f(i: &Inputs) { let e = evaluate(i); }
",
        ),
        (
            "crates/bb-api/src/api/x.rs",
            "fn f() { let e = bb_policy::evaluate(&input); }
",
        ),
        (
            "crates/bb-app/src/roles/x.rs",
            "fn f() { let i = Inputs { person, rows }; }
",
        ),
        (
            "crates/bb-server/src/x.rs",
            "fn f() { let i = Inputs {
person,
}; }
",
        ),
    ] {
        assert!(!evaluate(&source(path, text)).is_empty(), "{text}");
    }
}

#[test]
fn bad_example_an_import_or_an_alias_of_the_engine_or_its_input() {
    for text in [
        "use bb_policy::evaluate as judge;\n",
        "fn f() { let ask = bb_policy::evaluate; }\n",
        "use bb_policy::Inputs as In;\n",
        "use bb_policy::{Inputs, evaluate};\n",
        "fn f(make: fn(&Inputs) -> u8) {}\n",
    ] {
        let findings = evaluate(&source("crates/bb-api/src/api/x.rs", text));
        assert!(!findings.is_empty(), "{text}");
    }
}

#[test]
fn good_example_a_method_a_definition_and_names_that_only_contain_the_word() {
    let text = "async fn f(ctx: &Ctx) { ctx.evaluate(a, b).await; }\nfn evaluate_all() {}\nstruct MyInputs { x: u8 }\nfn evaluate(x: u8) {}\n";
    assert!(evaluate(&source("crates/bb-api/src/api/x.rs", text)).is_empty());
}

#[test]
fn good_example_the_places_that_may_ask_and_build_and_the_methods_that_only_share_the_name() {
    let ok = |path: &str, text: &str| evaluate(&source(path, text)).is_empty();
    assert!(ok(
        "crates/bb-app/src/access/mod.rs",
        "fn f() { let e = evaluate(&inputs); }\npub(crate) async fn evaluate(&self) {}\nuse bb_policy::Inputs;\n",
    ));
    assert!(ok(
        "crates/bb-store/src/policy.rs",
        "fn f() { let i = Inputs { person, rows }; }\n",
    ));
    assert!(ok(
        "crates/bb-app/src/roles/x.rs",
        "async fn f(ctx: &Ctx) { ctx.evaluate(person, place).await; }\nfn g(x: &MyInputs) {}\n",
    ));
}

#[test]
fn bad_example_a_glob_import_of_the_policy_crate_brings_the_engine_in_unnamed() {
    for text in [
        "use bb_policy::*;
fn f(i: &Inputs) { let e = evaluate(i); }
",
        "pub use bb_policy::*;
",
        "use bb_policy::{self, *};
",
        "use ::bb_policy::*;
",
        "use bb_policy::{
Permission,
*,
};
",
        "pub(crate) use bb_policy::evaluate::*;
",
    ] {
        let all = evaluate(&source("crates/bb-api/src/api/x.rs", text));
        assert!(
            all.iter()
                .any(|finding| finding.message.contains("glob import")),
            "{text}"
        );
    }
}

#[test]
fn good_example_named_imports_other_globs_and_the_places_that_ask_the_engine() {
    for text in [
        "use bb_policy::{Authorized, Permission};
",
        "use super::*;
use std::collections::*;
",
        "use bb_policy_extra::*;
",
        "// use bb_policy::*;
const T: &str = \"use bb_policy::*;\";
",
        "fn f() { let a = 2 * 3; use_it(bb_policy::Place::Organization); }
",
    ] {
        let found = evaluate(&source("crates/bb-api/src/api/x.rs", text));
        assert!(found.is_empty(), "{text}");
    }
    // The places that may name the engine may also import everything of it.
    for path in [
        "crates/bb-app/src/access/mod.rs",
        "crates/bb-store/src/policy.rs",
        "crates/bb-policy/src/lib.rs",
    ] {
        assert!(
            evaluate(&source(
                path,
                "use bb_policy::*;
"
            ))
            .is_empty(),
            "{path}"
        );
    }
}

#[test]
fn bad_example_a_judgment_about_the_actor_that_skips_the_narrowing_of_an_access_key() {
    for text in [
        "async fn f(ctx: &Ctx, actor: &Actor) { ctx.evaluate(actor.person.id, place).await; }\n",
        "async fn f() {\n    let held = ctx\n        .evaluate(\n            actor.person.id,\n            place,\n        )\n        .await;\n}\n",
        "fn f() { let seen = policy.visible(actor.person.id, &tree); }\n",
        "fn f() { let held = policy.evaluate_at(person, &tree, place); }\n",
        "fn f() { let held = Policy::evaluate_at(&policy, person, &tree, place); }\n",
        "fn f() { let seen = Policy::visible(&policy, person, &tree); }\n",
        "fn f() { let held = policy.evaluate_as(person, None, &tree, place); }\n",
        "fn f() { let rows = policy.rows_of(person); }\n",
        "fn f() { let held = policy.evaluate_with(person, &rows, &tree, place); }\n",
        "fn f() { let seen = policy.visible_as(\n    person,\n    None,\n    &tree,\n); }\n",
    ] {
        let found = evaluate(&source("crates/bb-app/src/projects/x.rs", text));
        assert_eq!(found.len(), 1, "{text}");
        assert!(found[0].message.contains("narrow"), "{text}");
    }
    // The line of a split call is the line of the call.
    let split = evaluate(&source(
        "crates/bb-app/src/projects/x.rs",
        "fn f() {\n    ctx\n        .evaluate(\n            actor.person.id,\n        );\n}\n",
    ));
    assert!(
        split[0]
            .location
            .as_deref()
            .is_some_and(|place| place.ends_with(":3")),
        "{:?}",
        split[0].location
    );
}

#[test]
fn good_example_the_narrowed_judgments_other_people_and_the_access_module() {
    for text in [
        "async fn f() { ctx.evaluation_of(actor, place).await; ctx.require(actor, p, place).await; }\n",
        "fn f() { let seen = policy.visible_as(actor.person.id, actor.key_scope(), &tree); }\n",
        "fn f() { let held = policy.evaluate_as(person, key, &tree, place); }\n",
        // Another person than the actor is judged as they are: a key narrows its own owner.
        "async fn f() { ctx.evaluate(input.person, place).await; }\n",
        "async fn f() { ctx.evaluate(record.person.id, place).await; }\n",
        "fn f() { let x = 1; } // ctx.evaluate(actor.person.id, place)\n",
    ] {
        let found = evaluate(&source("crates/bb-app/src/projects/x.rs", text));
        assert!(found.is_empty(), "{text}");
    }
    // The access module is where the narrowing is made.
    let access = evaluate(&source(
        "crates/bb-app/src/access/mod.rs",
        "async fn f() { self.evaluate(actor.person.id, place).await; }\n",
    ));
    assert!(access.is_empty());
    let access_with = evaluate(&source(
        "crates/bb-app/src/access/mod.rs",
        "fn f() { policy.evaluate_with(person, &policy.rows_of(person), &tree, place); }
",
    ));
    assert!(access_with.is_empty());
    // Other crates are not use cases.
    let policy = evaluate(&source(
        "crates/bb-policy/src/tree.rs",
        "fn f() { self.evaluate_at(person, tree, place); self.visible(person, tree); }\n",
    ));
    assert!(policy.is_empty());
}

#[test]
fn bad_example_into_value_in_code_that_ships() {
    let findings = evaluate(&source(
        "crates/bb-api/src/api/x.rs",
        "fn f(answer: Authorized<u8>) -> u8 { answer.into_value() }\n",
    ));
    assert_eq!(findings.len(), 1);
}

#[test]
fn bad_example_the_definition_inside_the_policy_crate_is_open() {
    let open = source(
        "crates/bb-policy/src/mask.rs",
        "#[derive(Clone, Default)]\npub struct Authorized<T> {\n    pub value: T,\n    mask: FieldMask,\n}\nimpl<T> Authorized<T> {\n    pub fn new(value: T) -> Self { todo!() }\n}\n",
    );
    let findings = evaluate(&open);
    // The derive of `Default`, the public field and the public constructor.
    assert_eq!(
        findings.len(),
        3,
        "{:?}",
        findings.iter().map(|f| &f.message).collect::<Vec<_>>()
    );
}

#[test]
fn peek_is_switched_on_only_by_dev_dependencies() {
    let good = vec![
        (
            "crates/bb-app/Cargo.toml".to_owned(),
            "[features]\npeek = [\"bb-policy/peek\"]\n\n[dependencies]\nbb-policy = { path = \"../bb-policy\" }\n".to_owned(),
        ),
        (
            "crates/bb-server/Cargo.toml".to_owned(),
            "[dev-dependencies]\nbb-app = { path = \"../bb-app\", features = [\"peek\"] }\n".to_owned(),
        ),
    ];
    assert!(evaluate_manifests(&good).is_empty());
    let bad = vec![
        (
            "crates/bb-server/Cargo.toml".to_owned(),
            "[dependencies]\nbb-app = { path = \"../bb-app\", features = [\"peek\"] }\n".to_owned(),
        ),
        (
            "crates/bb-app/Cargo.toml".to_owned(),
            "[features]\ndefault = [\"peek\"]\npeek = []\n".to_owned(),
        ),
    ];
    assert_eq!(evaluate_manifests(&bad).len(), 2);
}

#[test]
fn good_example_another_method_that_starts_with_the_name_is_not_the_engine() {
    let text = "use super::map::rows_of;
fn f() { let r = p.rows_of_all(); let e = Policy::evaluate_at_home; }
";
    assert!(evaluate(&source("crates/bb-api/src/api/x.rs", text)).is_empty());
}

/// Uses of the engine methods that a check by text has to find, one finding each.
const ENGINE_USES: [&str; 6] = [
    "use bb_policy::Policy as P;\nfn f() { let r = P::rows_of(&p, who); }\n",
    "fn f() { let r = bb_policy::Policy::rows_of(&p, who); }\n",
    "use bb_policy::Policy as P;\nfn f() { let r = P::rows_of::<T>(&p, who); }\n",
    "fn f() { let e = people.map(Policy::evaluate_at); }\n",
    "fn f() { let e = <Policy>::evaluate_at(&p, who, &tree, place); }\n",
    "fn f() { let g = p.rows_of; }\n",
];

#[test]
fn bad_example_an_engine_method_by_an_alias_a_full_path_a_turbofish_or_as_a_value() {
    for text in ENGINE_USES {
        for folder in [
            "crates/bb-api/src/api/x.rs",
            "crates/bb-app/src/projects/x.rs",
        ] {
            assert_eq!(evaluate(&source(folder, text)).len(), 1, "{folder}: {text}");
        }
    }
}

#[test]
fn bad_example_the_actors_evaluation_whatever_follows_person() {
    let text = "fn f() { let e = ctx.evaluate(actor.person_id, place); }\n";
    let findings = evaluate(&source("crates/bb-app/src/projects/x.rs", text));
    assert_eq!(findings.len(), 1, "{text}");
}

#[test]
fn good_example_an_import_of_another_rows_of_and_a_longer_name() {
    let text = "use super::map::rows_of;\nfn f() { let r = p.rows_of_all(); let e = Policy::evaluate_at_home; }\n";
    for folder in [
        "crates/bb-api/src/api/x.rs",
        "crates/bb-app/src/projects/x.rs",
    ] {
        assert!(evaluate(&source(folder, text)).is_empty(), "{folder}");
    }
}
