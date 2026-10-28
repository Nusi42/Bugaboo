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
//!   an access key, `ctx.evaluate(actor.person.id, …)` or the policy's `visible(` and the engine
//!   methods in the same forms (use `ctx.evaluation_of(actor, …)`, `ctx.require(…)`,
//!   `visible_as` and `evaluate_as` with the actor's key scope), so a key can never see more than
//!   it was given (M2-F27);
//! - the engine calls that make an evaluation of anybody from rows
//!   (`evaluation_from_person_policy`, `inputs_from_rows`, and outside `bb-app` also
//!   `evaluate_with`, `evaluate_at`, `rows_of`, in the forms of [`CALL_FORMS`] and
//!   [`VALUE_FORMS`]) outside `bb-policy` and the access module;
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
/// The methods of the policy that make an evaluation, or the rows it is made from, of anybody:
/// none narrows to an access key. Written once; they are found in the [`CALL_FORMS`] and the
/// [`VALUE_FORMS`].
const ENGINE_METHODS: [&str; 3] = ["evaluate_with", "evaluate_at", "rows_of"];
/// The functions that make an evaluation from the rows a person holds: only `bb-policy` and the
/// access module (which narrows it to an access key) may call them, in any crate.
const ROW_ENGINE_FUNCTIONS: [&str; 2] = ["evaluation_from_person_policy", "inputs_from_rows"];
/// What to say of the engine methods in a use case.
const ENGINE_CALL_NOT_NARROWED: &str = "`Policy::evaluate_with`, `Policy::evaluate_at` and \
     `Policy::rows_of` do not narrow to an access key. Judge the actor with \
     `evaluate_as(person, actor.key_scope(), tree, place)`; about another person, ask in the \
     access module.";
/// What to say of the engine calls outside `bb-policy` and the access module.
const ENGINE_CALL_ONLY_IN_ACCESS: &str = "Only `bb-policy` and `bb-app/src/access` make an \
     evaluation of a person from their rows (`evaluation_from_person_policy`, \
     `inputs_from_rows`, `evaluate_with`, `evaluate_at`, `rows_of`): elsewhere it would skip \
     the narrowing of an access key. Ask `ctx.evaluation_of(actor, place)` or `ctx.require(…)`.";
/// What a use case must not write about the actor besides the engine methods: each skips the
/// narrowing of an access key (M2-F27), with what to write instead.
const UNNARROWED: [(&str, &str); 2] = [
    (
        "evaluate(actor.person",
        "`ctx.evaluate(actor.person.id, …)` skips the narrowing of an access key. Judge the actor \
         with `ctx.evaluation_of(actor, place)` or `ctx.require(actor, permission, place)`.",
    ),
    (
        "visible(",
        "`Policy::visible` does not narrow to an access key. Use `visible_as(person, \
         actor.key_scope(), tree)` for the actor.",
    ),
];

/// How a method is called by a path: `Policy::name(`, `<Policy>::name(`, `P::name(` after
/// `use … Policy as P`, `bb_policy::Policy::name(`, also with a turbofish (`P::name::<T>(`). Any
/// path counts, so `map::rows_of(` of another module is a finding too: the check reads text and
/// does not know which `rows_of` is meant, and the helper of another module is renamed.
const CALL_FORMS: [&str; 1] = ["::"];
/// How a method is used after a value (`p.name(`, `p.name`) or taken as a value from the type
/// (`.map(Policy::name)`, `<Policy>::name`). Only a path of the type is read here, so
/// `use super::map::name;` is another function. An alias taken as a value (`let f = P::name;`) is
/// a known limit of a check by text (M5-F118): the compile-time guard, `Authorized<T>` built only
/// in `bb-policy`, is the primary one.
const VALUE_FORMS: [&str; 3] = [".", "Policy::", ">::"];

/// Where `text` uses the method `token`, in the [`CALL_FORMS`] or the [`VALUE_FORMS`]. A bare name
/// is not followed by a name character, so `rows_of_all` is another method; a token with a
/// parenthesis is read as it is written.
fn uses_of(text: &str, token: &str) -> Vec<usize> {
    let bare = !token.contains('(');
    let mut found: Vec<(usize, usize)> = Vec::new();
    for (forms, call_forms) in [
        (CALL_FORMS.as_slice(), true),
        (VALUE_FORMS.as_slice(), false),
    ] {
        for form in forms {
            let pattern = format!("{form}{token}");
            for (at, _) in text.match_indices(&pattern) {
                let rest = &text[at + pattern.len()..];
                let found_use = !bare
                    || match rest.chars().next() {
                        Some(c) if c.is_alphanumeric() || c == '_' => false,
                        _ if call_forms => rest.starts_with('(') || rest.starts_with("::<"),
                        _ => true,
                    };
                if found_use {
                    found.push((at + pattern.len(), at));
                }
            }
        }
    }
    // One use is found once, by the call forms and by the value forms alike.
    found.sort_unstable();
    found.dedup_by_key(|(end, _)| *end);
    found.into_iter().map(|(_, at)| at).collect()
}

/// Whether `text` uses the method `name` (see [`uses_of`]).
fn calls_method(text: &str, name: &str) -> bool {
    !uses_of(text, name).is_empty()
}

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
            if !in_policy && !in_access {
                let row_function = ROW_ENGINE_FUNCTIONS
                    .iter()
                    .any(|function| names_word(line, function) || calls_method(line, function));
                // In `bb-app` the use cases are held to the engine methods with advice, below.
                let method = !path.starts_with(USE_CASES_FOLDER)
                    && ENGINE_METHODS.iter().any(|name| calls_method(line, name));
                if row_function || method {
                    findings.push(Finding::at(place(), ENGINE_CALL_ONLY_IN_ACCESS));
                }
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
    let patterns = UNNARROWED
        .iter()
        .map(|(name, advice)| (*name, *advice))
        .chain(
            ENGINE_METHODS
                .iter()
                .map(|name| (*name, ENGINE_CALL_NOT_NARROWED)),
        );
    for (token, advice) in patterns {
        for at in uses_of(&squeezed, token) {
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
                    "`None` in place of the key scope skips the narrowing: the actor is judged as \
                     if they came without their access key. Pass `actor.key_scope()`.",
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
mod tests;
