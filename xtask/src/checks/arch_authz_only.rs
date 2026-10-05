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
//! - a call of `evaluate(` outside `bb-policy` and `bb-app/src/access/` (the one place that loads
//!   what a person holds and asks the engine), and a literal of `Inputs { … }` outside `bb-policy`
//!   and `bb-store` (the one place that reads the rows): whoever can build the input can make an
//!   evaluation for anybody and so an `Authorized<T>`;
//! - inside `bb-policy`: a `pub` field in the definition of `Authorized`, a `pub` constructor
//!   (`new`, `from`), or a derive that would make one out of nothing (`Default`, `Deserialize`).
//!
//! Comments, string and character literals do not count, and neither does code behind
//! `#[cfg(test)]` (but code after a test module does).

use super::{Check, Context, Finding};
use crate::rust_source::{Source, read_product_sources, strip, without_tests};

/// The only crate that makes `Authorized<T>`.
const POLICY_FOLDER: &str = "crates/bb-policy/";
/// The one place outside `bb-policy` that asks the engine.
const ENGINE_CALLER_FOLDER: &str = "crates/bb-app/src/access/";
/// The one place outside `bb-policy` that builds the input of the engine, from the stored rows.
const INPUT_BUILDER_FOLDER: &str = "crates/bb-store/src/";
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
        let sources = read_product_sources(context.repo)?;
        let mut manifests = Vec::new();
        for file in context.repo.all_files()? {
            if file == "Cargo.toml" || file.ends_with("/Cargo.toml") {
                let text = std::fs::read_to_string(context.repo.path(&file))?;
                manifests.push((file, text.replace("\r\n", "\n")));
            }
        }
        let mut findings = evaluate(&sources);
        findings.extend(evaluate_manifests(&manifests));
        Ok(findings)
    }
}

/// Whether `line` names the type as a whole word.
fn names_the_type(line: &str) -> bool {
    line.match_indices(TYPE).any(|(at, _)| {
        let before = line[..at].chars().next_back();
        let after = line[at + TYPE.len()..].chars().next();
        !before.is_some_and(|c| c.is_alphanumeric() || c == '_')
            && !after.is_some_and(|c| c.is_alphanumeric() || c == '_')
    })
}

/// Whether the line calls the function `evaluate` (a path or a bare call, not a method of
/// another type and not a definition).
fn calls_evaluate(line: &str) -> bool {
    line.match_indices("evaluate(").any(|(at, _)| {
        let before = &line[..at];
        !before
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.')
            && !before.trim_end().ends_with("fn")
    })
}

/// Whether the line builds the input of the engine with a struct literal.
fn builds_inputs(line: &str) -> bool {
    line.match_indices("Inputs").any(|(at, word)| {
        let before = line[..at].chars().next_back();
        let after = line[at + word.len()..].trim_start();
        !before.is_some_and(|c| c.is_alphanumeric() || c == '_') && after.starts_with('{')
    })
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
            if !in_policy && !path.starts_with(ENGINE_CALLER_FOLDER) && calls_evaluate(line) {
                findings.push(Finding::at(
                    place(),
                    "Only `bb-app/src/access` asks the policy engine (`evaluate`): it loads what \n                     the person holds from the store. Ask `ctx.evaluate(person, place)` or \n                     `ctx.require(…)` instead.",
                ));
            }
            if !in_policy && !path.starts_with(INPUT_BUILDER_FOLDER) && builds_inputs(line) {
                findings.push(Finding::at(
                    place(),
                    "Only `bb-store` builds the input of the policy engine (`Inputs`), from the \n                     stored rows. Built anywhere else, it could say that anybody holds anything.",
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
        if path.starts_with(POLICY_FOLDER) {
            findings.extend(definition_findings(path, &code));
        }
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
            assert_eq!(evaluate(&source(path, text)).len(), 1, "{text}");
        }
    }

    #[test]
    fn good_example_the_places_that_may_ask_and_build_and_the_methods_that_only_share_the_name() {
        assert!(
            evaluate(&source(
                "crates/bb-app/src/access/mod.rs",
                "fn f() { let e = evaluate(&inputs); }
pub(crate) async fn evaluate(&self) {}
"
            ))
            .is_empty()
        );
        assert!(
            evaluate(&source(
                "crates/bb-store/src/policy.rs",
                "fn f() { let i = Inputs { person, rows }; }
"
            ))
            .is_empty()
        );
        assert!(
            evaluate(&source(
                "crates/bb-app/src/roles/x.rs",
                "async fn f(ctx: &Ctx) { ctx.evaluate(person, place).await; }
fn g(x: &Inputs) {}
"
            ))
            .is_empty()
        );
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
