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
//! - inside `bb-policy`: a `pub` field in the definition of `Authorized`, a `pub` constructor
//!   (`new`, `from`), or a derive that would make one out of nothing (`Default`, `Deserialize`).
//!
//! Comments and string literals do not count, and neither does code behind `#[cfg(test)]`.

use super::{Check, Context, Finding};
use crate::rust_source::{Source, before_tests, read_product_sources};

/// The only crate that makes `Authorized<T>`.
const POLICY_FOLDER: &str = "crates/bb-policy/";
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

/// The code with comments and the insides of string literals replaced by spaces.
fn code_of(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_string = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if in_string {
            if c == '\\' {
                chars.next();
                out.push_str("  ");
            } else if c == '"' {
                in_string = false;
                out.push(' ');
            } else {
                out.push(if c == '\n' { '\n' } else { ' ' });
            }
        } else if c == '"' {
            in_string = true;
            out.push(' ');
        } else if c == '/' && chars.peek() == Some(&'/') {
            for rest in chars.by_ref() {
                if rest == '\n' {
                    out.push('\n');
                    break;
                }
                out.push(' ');
            }
        } else {
            out.push(c);
        }
    }
    out
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
        let code = code_of(before_tests(text));
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
