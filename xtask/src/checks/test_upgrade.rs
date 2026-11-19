//! TEST-UPGRADE: every release is tested by upgrading a database made by the program of the
//! previous milestone (ADR-0046, decision 5; architecture §11, "The upgrade test").
//!
//! The run itself is `cargo xtask upgrade-test` (`xtask/src/upgrade.rs`). This check keeps it from
//! going missing unseen:
//!
//! - the upgrade test is there;
//! - it asks Git for the tag of the previous milestone (`git describe` with the pattern of the
//!   milestone tags), and its code names no milestone tag by hand, which would leave the test on
//!   an old milestone when the next one is tagged;
//! - `cargo xtask ci` runs it as a step that nothing switches off: the step stands in the list
//!   alone or in `needs_browser_app`, which skips it only while `web/app/package.json` is missing,
//!   as TEST-CONCURRENT keeps its server test from being switched off.

use super::{Check, Context, Finding};
use crate::rust_source::{balanced, strip_comments, without_tests};
use crate::source::read_text;
use crate::upgrade::MILESTONE_TAGS;

/// The file of the upgrade test.
const TEST_FILE: &str = "xtask/src/upgrade.rs";
/// The file that lists the steps of `cargo xtask ci`.
const CI_FILE: &str = "xtask/src/ci.rs";
/// What the step of `cargo xtask ci` must call.
const COMMAND: &str = "\"upgrade-test\"";

pub(crate) struct TestUpgrade;

impl Check for TestUpgrade {
    fn id(&self) -> &'static str {
        "TEST-UPGRADE"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let repo = context.repo;
        let test = read_text(repo, TEST_FILE).ok();
        let ci = read_text(repo, CI_FILE)?;
        let mut findings = finds_the_tag(test.as_deref());
        findings.extend(ci_runs_it(&ci));
        Ok(findings)
    }
}

/// The upgrade test is there, asks Git for the tag, and names none by hand.
fn finds_the_tag(file: Option<&str>) -> Vec<Finding> {
    let Some(file) = file else {
        return vec![Finding::at(
            TEST_FILE,
            "The upgrade test is missing: `cargo xtask upgrade-test` upgrades a database made by the program of the previous milestone.",
        )];
    };
    let code = strip_comments(&without_tests(file));
    let mut findings = Vec::new();
    if !(code.contains("\"describe\"") && code.contains("MILESTONE_TAGS")) {
        findings.push(Finding::at(
            TEST_FILE,
            format!(
                "The upgrade test does not ask Git for the previous milestone (`git describe --match {MILESTONE_TAGS}`)."
            ),
        ));
    }
    if let Some(tag) = written_tag(&code) {
        findings.push(Finding::at(
            TEST_FILE,
            format!(
                "The upgrade test names the tag `{tag}` by hand; Git finds the previous milestone."
            ),
        ));
    }
    findings
}

/// The first milestone tag (`m` and digits, then `-done`) written in `code`.
fn written_tag(code: &str) -> Option<String> {
    code.match_indices("-done").find_map(|(at, _)| {
        let before = &code[..at];
        let digits = before.len() - before.trim_end_matches(|c: char| c.is_ascii_digit()).len();
        let start = at.checked_sub(digits + 1)?;
        (digits > 0 && code[start..].starts_with('m'))
            .then(|| code[start..at + "-done".len()].to_owned())
    })
}

/// What may stand around the step of the upgrade test in the list of `cargo xtask ci`, spaces
/// left out: nothing, or the wrapper that skips it only while `web/app/package.json` is missing
/// (the driver uses the helpers of the browser tests). Anything else (`linux_only(`, `if`, a
/// `skip`) switches it off somewhere.
const MAY_WRAP: [&str; 2] = ["", "needs_browser_app(root,"];

/// `cargo xtask ci` runs the upgrade test as a step, and nothing switches the step off.
fn ci_runs_it(ci: &str) -> Vec<Finding> {
    let code = strip_comments(&without_tests(ci));
    let Some(at) = code.find(COMMAND) else {
        return vec![Finding::at(
            CI_FILE,
            "`cargo xtask ci` does not run the upgrade test (`cargo xtask upgrade-test`, both databases).",
        )];
    };
    if step_around(&code, at).is_some_and(is_on) {
        Vec::new()
    } else {
        vec![Finding::at(
            CI_FILE,
            "The step of the upgrade test in `cargo xtask ci` is switched off: it must be a `Step::new(…)` of the list, alone or in `needs_browser_app(root, …)`, with no `skip`.",
        )]
    }
}

/// Whether the step stands in the list as it must (see [`MAY_WRAP`]): the call `Step::new(…)`,
/// alone or as the last argument of its wrapper, and the element ends right after it, so nothing
/// chained to it (`.only_on(…)`) can switch it off.
fn is_on(step: &str) -> bool {
    const CALL: &str = "Step::new";
    let Some(start) = step.find("Step::new(") else {
        return false;
    };
    let wrapper: String = step[..start]
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let open = start + CALL.len();
    let Some(arguments) = balanced(&step[open..], '(') else {
        return false;
    };
    let after: String = step[open + arguments.len()..]
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    // A wrapper closes right after the call, with the comma rustfmt puts after its last argument.
    let closing = if wrapper.is_empty() {
        after.is_empty()
    } else {
        after == ")" || after == ",)"
    };
    MAY_WRAP.contains(&wrapper.as_str()) && closing && !step.contains("skip")
}

/// The element of the list (`vec![…]`) around the byte `at` of `code` (comments blank, strings
/// kept), trimmed: from the comma before it to the comma or the bracket after it. `None` when
/// `at` is in no such list.
fn step_around(code: &str, at: usize) -> Option<&str> {
    let mut open = Vec::new();
    let mut list = None;
    let mut text = Text::default();
    for (index, character) in code.char_indices() {
        if index == at {
            list = open
                .iter()
                .rev()
                .find(|&&opener| code[..opener].ends_with("vec!"))
                .copied();
            break;
        }
        match text.read(character) {
            Some('(' | '[' | '{') => open.push(index),
            Some(')' | ']' | '}') => {
                open.pop();
            }
            _ => {}
        }
    }
    let body = list? + 1;
    let mut depth = 0_usize;
    let mut start = body;
    let mut text = Text::default();
    for (offset, character) in code[body..].char_indices() {
        let index = body + offset;
        let end = match text.read(character) {
            Some('(' | '[' | '{') => {
                depth += 1;
                None
            }
            Some(')' | ']' | '}') if depth == 0 => Some(true),
            Some(')' | ']' | '}') => {
                depth -= 1;
                None
            }
            Some(',') if depth == 0 => Some(false),
            _ => None,
        };
        match end {
            Some(_) if (start..index).contains(&at) => return Some(code[start..index].trim()),
            Some(true) => return None,
            Some(false) => start = index + 1,
            None => {}
        }
    }
    None
}

/// Reads code a character at a time and tells which characters are code, not inside a string.
#[derive(Default)]
struct Text {
    in_string: bool,
    escaped: bool,
}

impl Text {
    /// The character when it is code, `None` inside a string (its quotes included).
    fn read(&mut self, character: char) -> Option<char> {
        if self.in_string {
            match (self.escaped, character) {
                (true, _) => self.escaped = false,
                (false, '\\') => self.escaped = true,
                (false, '"') => self.in_string = false,
                _ => {}
            }
            None
        } else if character == '"' {
            self.in_string = true;
            None
        } else {
            Some(character)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURES: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tools/checks/fixtures/TEST-UPGRADE"
    );

    fn fixture(kind: &str, name: &str) -> String {
        std::fs::read_to_string(format!("{FIXTURES}/{kind}/{name}")).unwrap()
    }

    #[test]
    fn bad_example_a_ci_without_the_step() {
        let found = ci_runs_it(&fixture("bad", "ci.rs"));
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn good_example_the_wired_step() {
        assert!(ci_runs_it(&fixture("good", "ci.rs")).is_empty());
    }

    #[test]
    fn bad_example_a_step_that_is_switched_off() {
        let found = ci_runs_it(&fixture("bad", "ci-switched-off.rs"));
        assert_eq!(found.len(), 1);
        assert!(
            found[0].message.contains("switched off"),
            "{}",
            found[0].message
        );
        let step = r#"Step::new("upgrade", xtask, &["upgrade-test"])"#;
        for switched_off in [
            format!("linux_only({step})"),
            format!(r#"Step {{ skip: Some("later"), ..{step} }}"#),
            format!("if false {{ {step} }} else {{ {step} }}"),
        ] {
            let ci = format!(
                r#"fn plan(xtask: &str) -> Vec<Step> {{ vec![Step::new("a", xtask, &[]), {switched_off}] }}"#
            );
            assert_eq!(ci_runs_it(&ci).len(), 1, "{switched_off}");
        }
    }

    #[test]
    fn bad_example_a_step_switched_off_by_a_chained_call() {
        let found = ci_runs_it(&fixture("bad", "ci-chained.rs"));
        assert_eq!(found.len(), 1);
        let step = r#"Step::new("upgrade", xtask, &["upgrade-test"])"#;
        for chained in [
            format!("{step}.only_on(\"linux\")"),
            format!("needs_browser_app(root, {step}).only_on(\"linux\")"),
            format!("needs_browser_app(root, {step}.only_on(\"linux\"))"),
        ] {
            let ci = format!(
                r#"fn plan(xtask: &str) -> Vec<Step> {{ vec![Step::new("a", xtask, &[]), {chained}] }}"#
            );
            assert_eq!(ci_runs_it(&ci).len(), 1, "{chained}");
        }
    }

    #[test]
    fn good_example_the_step_skipped_only_without_the_browser_app() {
        let ci = r#"fn plan(root: &Path, xtask: &str) -> Vec<Step> {
            vec![
                Step::new("a (b, c)", xtask, &["x"]),
                needs_browser_app(root, Step::new("upgrade", xtask, &["upgrade-test"])),
            ]
        }"#;
        assert!(ci_runs_it(ci).is_empty());
    }

    #[test]
    fn bad_example_a_tag_written_by_hand() {
        let found = finds_the_tag(Some(&fixture("bad", "upgrade.rs")));
        assert_eq!(found.len(), 2);
        assert!(
            found[1].message.contains("`m8-done`"),
            "{}",
            found[1].message
        );
    }

    #[test]
    fn good_example_the_tag_found_by_git() {
        assert!(finds_the_tag(Some(&fixture("good", "upgrade.rs"))).is_empty());
        assert_eq!(finds_the_tag(None).len(), 1);
    }

    #[test]
    fn the_upgrade_test_of_the_repository_passes() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
        let read = |path: &str| std::fs::read_to_string(format!("{root}/{path}")).unwrap();
        assert!(finds_the_tag(Some(&read(TEST_FILE))).is_empty());
        assert!(ci_runs_it(&read(CI_FILE)).is_empty());
    }
}
