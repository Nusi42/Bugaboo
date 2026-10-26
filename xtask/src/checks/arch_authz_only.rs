//! ARCH-AUTHZ-ONLY: `Authorized<T>` is made only inside `bb-policy` (ADR-0004, ADR-0025 decision 10).
//!
//! The type system does most of it: the fields of `Authorized<T>` and of `Evaluation` are private
//! to `bb-policy`, and there is no constructor but `Evaluation::authorize`, which needs a real
//! evaluation (a compile-fail test in `bb-policy` builds the type from outside and must fail).
//! This check finds the ways around the compiler that a text can show:
//!
//! - a definition called `Authorized` outside `bb-policy` (a copy of the type), and a struct
//!   literal or a call of a constructor of it (`Authorized { … }`, `Authorized::new(…)`) outside
//!   `bb-policy`;
//! - `into_value(` in code that ships (the feature `peek` makes it for tests), and the feature
//!   `peek` switched on anywhere but in a `[dev-dependencies]` table, or in a default feature;
//! - the word `evaluate` (a call, an import, an alias) outside `bb-policy` and
//!   `bb-app/src/access/` (the one place that loads what a person holds and asks the engine), and
//!   the word `Inputs` outside those and `bb-store` (which reads the rows): whoever can build the
//!   input can make an evaluation for anybody and so an `Authorized<T>`. A method of a value
//!   (`ctx.evaluate(…)`) and a definition of another function of that name are not the engine;
//! - a glob import of the policy crate (`use bb_policy::*`, `use bb_policy::{self, *}`, also across
//!   lines) outside the same places: it would bring `evaluate` and `Inputs` in without naming them
//!   (M2-F24);
//! - in `bb-app` outside its access module: a judgment about the actor that skips the narrowing of
//!   an access key, `ctx.evaluate(actor.person.id, …)` or the policy's `visible(`, `evaluate_at(`,
//!   `evaluate_with(` and `rows_of(` (use `ctx.evaluation_of(actor, …)`, `ctx.require(…)`,
//!   `visible_as` and `evaluate_as` with the actor's key scope), so a key can never see more than
//!   it was given (M2-F27);
//! - inside `bb-policy`: a `pub` field in the definition of `Authorized`, a `pub` constructor
//!   (`new`, `from`), or a derive that would make one out of nothing (`Default`, `Deserialize`).
//!
//! Comments, string and character literals do not count, and neither does code behind
//! `#[cfg(test)]` (but code after a test module does).

use super::{Check, Context, Finding};
use crate::rust_source::{squeezed, strip, without_tests};
use crate::source::{Kind, Source, contains_word, name_uses, read_text};

/// The only crate that makes `Authorized<T>`.
const POLICY_FOLDER: &str = "crates/bb-policy/";
/// The one place outside `bb-policy` that asks the engine.
const ENGINE_CALLER_FOLDER: &str = "crates/bb-app/src/access/";
/// The one place outside `bb-policy` and the access module that builds the input of the engine,
/// from the stored rows.
const INPUT_BUILDER_FOLDER: &str = "crates/bb-store/src/";
/// What to say of the engine call that `Policy::evaluate_at` is made of.
const ENGINE_CALL_NOT_NARROWED: &str = "`Policy::evaluate_with` and `Policy::rows_of` are the      engine call of `evaluate_at`, which does not narrow to an access key. Judge the actor with      `evaluate_as(person, actor.key_scope(), tree, place)`; about another person, ask in the      access module.";
/// What a use case must not write about the actor: each skips the narrowing of an access key
/// (M2-F27), with what to write instead.
const UNNARROWED: [(&str, &str); 7] = [
    (
        ".evaluate(actor.person",
        "`ctx.evaluate(actor.person.id, …)` skips the narrowing of an access key. Judge the actor \
         with `ctx.evaluation_of(actor, place)` or `ctx.require(actor, permission, place)`.",
    ),
    (
        ".visible(",
        "`Policy::visible` does not narrow to an access key. Use `visible_as(person, \
         actor.key_scope(), tree)` for the actor.",
    ),
    (
        "Policy::visible(",
        "`Policy::visible` does not narrow to an access key. Use `visible_as(person, \
         actor.key_scope(), tree)` for the actor.",
    ),
    (
        "Policy::evaluate_at(",
        "`Policy::evaluate_at` does not narrow to an access key. Use `evaluate_as(person, \
         actor.key_scope(), tree, place)` for the actor.",
    ),
    (
        ".evaluate_at(",
        "`Policy::evaluate_at` does not narrow to an access key. Use `evaluate_as(person, \
         actor.key_scope(), tree, place)` for the actor.",
    ),
    (".evaluate_with(", ENGINE_CALL_NOT_NARROWED),
    (".rows_of(", ENGINE_CALL_NOT_NARROWED),
];
/// The narrowed forms, which narrow to nothing when they are given no key: the second argument
/// of each is the key scope.
const NARROWED: [&str; 2] = ["evaluate_as(", "visible_as("];
/// The crate whose names a glob import must not pull in.
const POLICY_CRATE: &str = "bb_policy";
/// The type.
const TYPE: &str = "Authorized";
/// The method that opens an `Authorized<T>` for tests, and the feature that switches it on.
const PEEK_METHOD: &str = "into_value(";
const PEEK_FEATURE: &str = "peek";
/// Derives that make a value out of nothing.
const BAD_DERIVES: [&str; 2] = ["Default", "Deserialize"];

pub(crate) struct ArchAuthzOnly;

impl Check for ArchAuthzOnly {
    fn id(&self) -> &'static str {
        "ARCH-AUTHZ-ONLY"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let sources = context.sources(self.id(), Kind::Rust)?;
        let mut manifests = Vec::new();
        for file in context.repo.all_files()? {
            if file == "Cargo.toml" || file.ends_with("/Cargo.toml") {
                let text = read_text(context.repo, &file)?;
                manifests.push((file, text));
            }
        }
        let mut findings = evaluate(&sources);
        findings.extend(evaluate_manifests(&manifests));
        Ok(findings)
    }
}

/// Whether `line` names the type as a whole word.
fn names_the_type(line: &str) -> bool {
    contains_word(line, TYPE)
}

/// Whether `word` stands in the line as a whole identifier, and is not a method of a value (after
/// a dot) or the name of a function that is defined there (after `fn`).
fn names_word(line: &str, word: &str) -> bool {
    name_uses(line, word).next().is_some()
}

/// What makes a line a way to build the type: a definition, a struct literal, a constructor call.
fn builds_the_type(line: &str) -> bool {
    if !names_the_type(line) {
        return false;
    }
    let trimmed = line.trim_start();
    let defines = ["struct ", "enum ", "union ", "type ", "trait "]
        .iter()
        .any(|keyword| {
            trimmed
                .strip_prefix(keyword)
                .or_else(|| {
                    trimmed
                        .strip_prefix("pub ")
                        .and_then(|rest| rest.strip_prefix(keyword))
                })
                .is_some_and(|rest| rest.trim_start().starts_with(TYPE))
        });
    let literal_or_call = [
        "Authorized {",
        "Authorized::new(",
        "Authorized::from(",
        "Authorized::default(",
        "Authorized::<",
        "Authorized(",
    ]
    .iter()
    .any(|pattern| line.contains(pattern));
    defines || literal_or_call
}

fn evaluate(sources: &[Source]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (path, text) in sources {
        let code = strip(&without_tests(text));
        let outside = !path.starts_with(POLICY_FOLDER);
        for (index, line) in code.lines().enumerate() {
            let place = || format!("{path}:{}", index + 1);
            if outside && builds_the_type(line) {
                findings.push(Finding::at(
                    place(),
                    "`Authorized<T>` is made only inside `bb-policy`, from an evaluation \
                     (`evaluation.authorize(value)`). Do not define it again or build it here.",
                ));
            }
            let in_policy = !outside;
            let in_access = path.starts_with(ENGINE_CALLER_FOLDER);
            if !in_policy && !in_access && names_word(line, "evaluate") {
                findings.push(Finding::at(
                    place(),
                    "Only `bb-app/src/access` asks the policy engine (`evaluate`): it loads what \
                     the person holds from the store. Ask `ctx.evaluate(person, place)` or \
                     `ctx.require(…)` instead.",
                ));
            }
            if !in_policy
                && !in_access
                && !path.starts_with(INPUT_BUILDER_FOLDER)
                && names_word(line, "Inputs")
            {
                findings.push(Finding::at(
                    place(),
                    "Only `bb-store` builds the input of the policy engine (`Inputs`), from the \
                     stored rows. Built or named anywhere else, it could say that anybody holds \
                     anything.",
                ));
            }
            if line.contains(PEEK_METHOD) && line.contains('.') {
                findings.push(Finding::at(
                    place(),
                    "`into_value` opens an `Authorized<T>` and is for tests only (the feature \
                     `peek`). Code that ships answers with the `Authorized<T>` itself.",
                ));
            }
        }
        if path.starts_with(USE_CASES_FOLDER) && !path.starts_with(ENGINE_CALLER_FOLDER) {
            findings.extend(unnarrowed_findings(path, &code));
        }
        let may_name_engine = path.starts_with(POLICY_FOLDER)
            || path.starts_with(ENGINE_CALLER_FOLDER)
            || path.starts_with(INPUT_BUILDER_FOLDER);
        if !may_name_engine {
            findings.extend(glob_import_findings(path, &code));
        }
        if path.starts_with(POLICY_FOLDER) {
            findings.extend(definition_findings(path, &code));
        }
    }
    findings
}

/// Where the use cases are.
const USE_CASES_FOLDER: &str = "crates/bb-app/src/";

/// The judgments about the actor that skip an access key's narrowing. The text is read with its
/// white space gone, so a call that rustfmt split over lines is found too.
fn unnarrowed_findings(path: &str, code: &str) -> Vec<Finding> {
    let (squeezed, lines) = squeezed(code);
    let mut findings = Vec::new();
    for (pattern, advice) in UNNARROWED {
        for (at, _) in squeezed.match_indices(pattern) {
            let line = lines.get(at).copied().unwrap_or(1);
            findings.push(Finding::at(format!("{path}:{line}"), advice));
        }
    }
    // `evaluate_as(person, None, …)` and `visible_as(person, None, …)` judge the actor without
    // their key: a literal `None` in place of the key scope is no scope.
    for name in NARROWED {
        for (at, _) in squeezed.match_indices(name) {
            let arguments = &squeezed[at + name.len()..];
            if arguments.split(',').nth(1) == Some("None") {
                let line = lines.get(at).copied().unwrap_or(1);
                findings.push(Finding::at(
                    format!("{path}:{line}"),
                    "`None` in place of the key scope skips the narrowing: the actor is judged as if they came without \
                     their access key. Pass `actor.key_scope()`.",
                ));
            }
        }
    }
    findings
}

/// Every `use` of the policy crate with a `*` in it, read as a whole statement so a list over
/// several lines is found too. `code` has its comments and texts blanked already.
fn glob_import_findings(path: &str, code: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let lines: Vec<&str> = code.lines().collect();
    let mut at = 0;
    while at < lines.len() {
        let trimmed = lines[at].trim_start();
        let rest = ["pub(crate) ", "pub(super) ", "pub "]
            .iter()
            .find_map(|visibility| trimmed.strip_prefix(visibility))
            .unwrap_or(trimmed);
        if !rest.starts_with("use ") {
            at += 1;
            continue;
        }
        // The statement runs to its `;`.
        let mut statement = String::new();
        let mut end = at;
        while end < lines.len() {
            statement.push_str(lines[end]);
            statement.push(' ');
            if lines[end].contains(';') {
                break;
            }
            end += 1;
        }
        let names_policy = statement
            .split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .any(|word| word == POLICY_CRATE);
        if names_policy && statement.contains('*') {
            findings.push(Finding::at(
                format!("{path}:{}", at + 1),
                "A glob import of `bb_policy` would bring the engine (`evaluate`, `Inputs`) in \
                 without naming it. Import the names you use; only `bb-app/src/access` asks the \
                 engine and only `bb-store` builds its input.",
            ));
        }
        at = end + 1;
    }
    findings
}

/// What the definition of `Authorized` inside `bb-policy` must not have.
fn definition_findings(path: &str, code: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let lines: Vec<&str> = code.lines().collect();
    for (at, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("pub struct Authorized") || trimmed.contains(';') {
            continue;
        }
        // The derives written just above.
        let mut above = at;
        while above > 0 && lines[above - 1].trim_start().starts_with('#') {
            above -= 1;
            for derive in BAD_DERIVES {
                if lines[above].contains(derive) {
                    findings.push(Finding::at(
                        format!("{path}:{}", above + 1),
                        format!(
                            "`Authorized<T>` must not derive `{derive}`: that would make one out of \
                             nothing."
                        ),
                    ));
                }
            }
        }
        // The fields, up to the closing brace.
        for (offset, field) in lines[at + 1..]
            .iter()
            .take_while(|field| !field.starts_with('}'))
            .enumerate()
        {
            if field.trim_start().starts_with("pub ") {
                findings.push(Finding::at(
                    format!("{path}:{}", at + offset + 2),
                    "A field of `Authorized<T>` is private to `bb-policy`: a public field lets \
                     anybody build one.",
                ));
            }
        }
    }
    // A public constructor of the type.
    let mut inside = false;
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("impl") {
            inside = trimmed.contains(TYPE) && !trimmed.contains(" for ");
        }
        let makes_one = [
            "pub fn new",
            "pub const fn new",
            "pub fn from",
            "pub fn make",
        ]
        .iter()
        .any(|constructor| trimmed.starts_with(constructor));
        if inside && makes_one {
            findings.push(Finding::at(
                format!("{path}:{}", index + 1),
                "`Authorized<T>` has no public constructor: the only way to make one is \
                 `Evaluation::authorize`, from a real evaluation.",
            ));
        }
    }
    findings
}

/// The feature `peek` is only switched on in a `[dev-dependencies]` table, and is no default.
fn evaluate_manifests(manifests: &[Source]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (path, text) in manifests {
        let mut section = String::new();
        for (index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                trimmed
                    .trim_matches(|c| c == '[' || c == ']')
                    .clone_into(&mut section);
                continue;
            }
            let mentions = trimmed.contains(&format!("\"{PEEK_FEATURE}\""))
                || trimmed.starts_with(&format!("{PEEK_FEATURE} ="));
            if !mentions || trimmed.starts_with('#') {
                continue;
            }
            let dev = section.ends_with("dev-dependencies");
            let defines =
                section == "features" && trimmed.starts_with(&format!("{PEEK_FEATURE} ="));
            let default = section == "features" && trimmed.starts_with("default");
            if !dev && (!defines || default) {
                findings.push(Finding::at(
                    format!("{path}:{}", index + 1),
                    format!(
                        "The feature `{PEEK_FEATURE}` opens an `Authorized<T>` for tests. Only \
                         `[dev-dependencies]` may switch it on, and it is never a default."
                    ),
                ));
            }
        }
    }
    findings
}

#[cfg(test)]
mod tests {
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
                "[dependencies]\nbb-app = { path = \"../bb-app\", features = [\"peek\"] }\n"
                    .to_owned(),
            ),
            (
                "crates/bb-app/Cargo.toml".to_owned(),
                "[features]\ndefault = [\"peek\"]\npeek = []\n".to_owned(),
            ),
        ];
        assert_eq!(evaluate_manifests(&bad).len(), 2);
    }
}
