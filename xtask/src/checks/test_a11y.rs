//! TEST-A11Y: every address of the browser app is visited by the accessibility suite (UX-04).
//!
//! The suite is the Playwright tests `web/app/e2e/a11y*.spec.ts`: each opens addresses in the
//! four themes, on a computer and on a phone, and runs axe on them. The addresses are the entries of
//! `paths` in `web/app/src/shared/paths.ts`. This check fails when an address has no entry in the
//! suite, so a new page cannot be added without being checked; the suite itself runs in
//! `cargo xtask ci` (the browser end-to-end tests).
//!
//! Every drag of the timeline has a key (UX §4.7, ADR-0043): each kind of thing the timeline
//! drags is named in `drag-kinds.ts` together with the message of the typed way that does the same
//! (`typed:`), a drag in the timeline takes its kind from there and never writes one of its own, and
//! the timeline that drags a bar with the pointer also moves it with keys (`keyMoveOf`).

use super::web_source::code_of;
use super::{Check, Context, Finding};
use crate::docs::source::sorted_files;
use crate::docs::text::has_extension;
use crate::source::read_text;

const PATHS_FILE: &str = "web/app/src/shared/paths.ts";
const SUITE_FOLDER: &str = "web/app/e2e";
const SUITE_PREFIX: &str = "a11y";
/// The feature of the timeline, whose drags each have a key.
const TIMELINE_FOLDER: &str = "web/app/src/features/timeline";
/// The one place that names what the timeline drags, each with its typed way.
const DRAG_KINDS_FILE: &str = "web/app/src/features/timeline/drag-kinds.ts";
/// The catalog the typed ways are messages of.
const MESSAGES_FILE: &str = "messages/en.json";

pub(crate) struct TestA11y;

impl Check for TestA11y {
    fn id(&self) -> &'static str {
        "TEST-A11Y"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        let repo = context.repo;
        let paths = read_text(repo, PATHS_FILE)?;
        let mut suite = String::new();
        for name in sorted_files(repo, SUITE_FOLDER, |name| has_extension(name, "ts"))? {
            if name.starts_with(SUITE_PREFIX) && name.ends_with(".spec.ts") {
                suite.push_str(&read_text(repo, &format!("{SUITE_FOLDER}/{name}"))?);
            }
        }
        let mut findings = evaluate(&paths, &suite);
        let timeline = timeline_files(repo)?;
        let messages = read_text(repo, MESSAGES_FILE)?;
        findings.extend(drags_have_keys(&timeline, &messages));
        Ok(findings)
    }
}

/// The code of the timeline, by path: its TypeScript files, tests left out.
fn timeline_files(repo: &crate::repo::Repo) -> anyhow::Result<Vec<(String, String)>> {
    let mut files = Vec::new();
    for name in sorted_files(repo, TIMELINE_FOLDER, |name| {
        (has_extension(name, "ts") || has_extension(name, "tsx")) && !name.contains(".test.")
    })? {
        let path = format!("{TIMELINE_FOLDER}/{name}");
        let text = read_text(repo, &path)?;
        files.push((path, text));
    }
    Ok(files)
}

/// Every drag of the timeline has a key: the kinds of `drag-kinds.ts` each name their typed way, no
/// file of the timeline writes a kind of its own, and a drag of a bar comes with its keys.
fn drags_have_keys(files: &[(String, String)], messages: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    match files.iter().find(|(path, _)| path == DRAG_KINDS_FILE) {
        None => findings.push(Finding::at(
            DRAG_KINDS_FILE,
            "The timeline has no `drag-kinds.ts`: name each kind it drags there, with its typed way.",
        )),
        Some((_, text)) => findings.extend(kinds_have_typed_ways(&code(text), messages)),
    }
    let mut keys_called = false;
    for (path, text) in files.iter().filter(|(path, _)| path != DRAG_KINDS_FILE) {
        let text = code(text);
        // A call, not the definition of the function.
        keys_called |=
            text.matches("keyMoveOf(").count() > text.matches("function keyMoveOf(").count();
        if writes_a_kind(&text) {
            findings.push(Finding::at(
                path,
                "A drag of the timeline writes its own kind. Take it from `dragKinds`, where its typed way is named.",
            ));
        }
    }
    for (path, text) in files {
        if code(text).contains("useBarDrag(") && !keys_called {
            findings.push(Finding::at(
                path,
                "A bar is dragged with the pointer but no key moves it: call `keyMoveOf` where the keys of a bar are read.",
            ));
        }
    }
    findings
}

/// The kinds of `drag-kinds.ts` each name a typed way (`typed: "<message key>"`) that is a message
/// of the catalog.
fn kinds_have_typed_ways(text: &str, messages: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    for line in text
        .lines()
        .map(str::trim)
        .filter(|line| line.contains("kind:"))
    {
        match quoted_after(line, "typed:") {
            None => findings.push(Finding::at(
                DRAG_KINDS_FILE,
                format!(
                    "`{line}` names a drag without its typed way. Add `typed: \"<message key>\"`, the button or window that does the same with the keyboard."
                ),
            )),
            Some(key) if !messages.contains(&format!("\"{key}\":")) => findings.push(Finding::at(
                DRAG_KINDS_FILE,
                format!("The typed way `{key}` of `{line}` is no message of messages/en.json."),
            )),
            Some(_) => {}
        }
    }
    findings
}

/// The text in quotes after `marker` on a line.
fn quoted_after<'a>(line: &'a str, marker: &str) -> Option<&'a str> {
    let rest = line.split_once(marker)?.1.trim_start();
    let rest = rest.strip_prefix('"')?;
    rest.split_once('"').map(|(inside, _)| inside)
}

/// The places that take the kind of a drag: an element that drags or one that is dropped on (the
/// hook that drops is looked for apart).
const DRAG_PLACES: [&str; 3] = ["<DragHandle", "<DraggableHead", "<DropSlot"];

/// An element from its `<` to the `>` that ends its tag: a `>` inside braces (`() => end()`) or
/// inside quotes is part of an attribute, not the end.
fn element_at(text: &str) -> &str {
    let mut depth = 0_usize;
    let mut quoted = false;
    for (at, sign) in text.char_indices() {
        match sign {
            '"' if depth == 0 => quoted = !quoted,
            '{' if !quoted => depth += 1,
            '}' if !quoted => depth = depth.saturating_sub(1),
            '>' if depth == 0 && !quoted => return &text[..at],
            _ => {}
        }
    }
    text
}

/// Whether code gives a drag a kind of its own, where a drag takes one: a text as the `kind` of
/// `DragHandle`, `DraggableHead` or `DropSlot`, or as the first argument of `useDropOf`.
fn writes_a_kind(text: &str) -> bool {
    let compact: String = text.split_whitespace().collect();
    let in_an_element = DRAG_PLACES.iter().any(|place| {
        compact.match_indices(place).any(|(at, _)| {
            let element = element_at(&compact[at..]);
            element.contains("kind=\"") || element.contains("kind={\"")
        })
    });
    in_an_element || compact.contains("useDropOf(\"")
}

/// The entries of `paths`: the name and the address of each.
fn entries(paths_text: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut inside = false;
    for line in paths_text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("export const paths") {
            inside = true;
        } else if inside && trimmed.starts_with('}') {
            break;
        } else if inside && let Some((name, rest)) = trimmed.split_once(':') {
            let address = rest.trim().trim_end_matches(',').trim_matches('"');
            if address.starts_with('/') {
                found.push((name.trim().to_owned(), address.to_owned()));
            }
        }
    }
    found
}

/// The code of the suite: a comment that names an address does not visit it.
fn code(suite: &str) -> String {
    suite.lines().map(code_of).collect::<Vec<_>>().join(
        "
",
    )
}

/// Whether the suite mentions `paths.<name>` as a whole word.
fn mentions(suite: &str, name: &str) -> bool {
    let needle = format!("paths.{name}");
    suite.match_indices(&needle).any(|(at, _)| {
        suite[at + needle.len()..]
            .chars()
            .next()
            .is_none_or(|next| !(next.is_alphanumeric() || next == '_'))
    })
}

fn evaluate(paths_text: &str, suite: &str) -> Vec<Finding> {
    let suite = code(suite);
    let suite = suite.as_str();
    let all = entries(paths_text);
    if all.is_empty() {
        return vec![Finding::at(
            PATHS_FILE,
            "No addresses found in `paths`; the check needs `export const paths = { name: \"/address\", … }`.",
        )];
    }
    all.iter()
        .filter(|(name, _)| !mentions(suite, name))
        .map(|(name, address)| {
            Finding::at(
                PATHS_FILE,
                format!(
                    "The address `paths.{name}` ({address}) has no entry in the accessibility suite. Add it to a test in {SUITE_FOLDER}/{SUITE_PREFIX}*.spec.ts, which visits it in the four themes on a computer and a phone."
                ),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATHS: &str = "export const paths = {\n  home: \"/\",\n  signIn: \"/sign-in\",\n  longer: \"/longer\",\n} as const;\n";

    #[test]
    fn bad_example() {
        // `longer` is only a longer name that starts like `paths.long`: the entry for it is missing.
        let suite = "visit(paths.home); visit(paths.signIn); visit(paths.long);";
        let findings = evaluate(PATHS, suite);
        assert_eq!(
            findings.len(),
            1,
            "{:?}",
            findings.iter().map(|f| &f.message).collect::<Vec<_>>()
        );
        assert!(findings[0].message.contains("paths.longer"));
    }

    #[test]
    fn a_mention_in_a_comment_is_no_entry() {
        // The names are only in comments, one of them behind code on the same line.
        let suite = "// visit(paths.home);
/* paths.signIn */
visit(paths.longer); // paths.home and paths.signIn
 * paths.home
";
        let findings = evaluate(PATHS, suite);
        let missing: Vec<&str> = findings.iter().map(|f| f.message.as_str()).collect();
        assert_eq!(missing.len(), 2, "{missing:?}");
        assert!(missing[0].contains("paths.home") && missing[1].contains("paths.signIn"));
    }

    #[test]
    fn good_example() {
        let suite = "visit(paths.home);\nvisit(paths.signIn);\nvisit(paths.longer);";
        assert!(evaluate(PATHS, suite).is_empty());
    }

    #[test]
    fn a_paths_file_without_addresses_is_a_finding() {
        assert_eq!(evaluate("export const other = 1;", "").len(), 1);
    }

    /// The catalog of the tests: the typed way of the dot.
    const MESSAGES: &str = "{\"timeline.add-dependency\": \"Add dependency…\"}";

    /// The files of a timeline whose drags have their keys.
    fn timeline() -> Vec<(String, String)> {
        vec![
            (
                DRAG_KINDS_FILE.to_owned(),
                "export const dragKinds = {\n  dot: { kind: \"a-dot\", typed: \"timeline.add-dependency\" },\n};\n"
                    .to_owned(),
            ),
            (
                format!("{TIMELINE_FOLDER}/bar-keys.ts"),
                "export function keyMoveOf(event) {}".to_owned(),
            ),
            (
                format!("{TIMELINE_FOLDER}/Bar.tsx"),
                "useBarDrag(1); keyMoveOf(event); <DragHandle kind={dragKinds.dot.kind} />; useDropOf(dragKinds.dot.kind, ref)".to_owned(),
            ),
        ]
    }

    /// The messages of the findings for the files.
    fn found(files: &[(String, String)]) -> Vec<String> {
        drags_have_keys(files, MESSAGES)
            .into_iter()
            .map(|finding| finding.message)
            .collect()
    }

    #[test]
    fn every_drag_of_the_timeline_has_a_key() {
        assert!(found(&timeline()).is_empty());
    }

    #[test]
    fn a_kind_without_its_typed_way_is_a_finding() {
        let mut files = timeline();
        files[0].1 = "export const dragKinds = {\n  dot: { kind: \"a-dot\" },\n};\n".to_owned();
        let messages = found(&files);
        assert_eq!(messages.len(), 1, "{messages:?}");
        assert!(messages[0].contains("typed way"));
    }

    #[test]
    fn a_typed_way_that_is_no_message_is_a_finding() {
        let mut files = timeline();
        files[0].1 = "export const dragKinds = {\n  dot: { kind: \"a-dot\", typed: \"timeline.nothing\" },\n};\n".to_owned();
        let messages = found(&files);
        assert_eq!(messages.len(), 1, "{messages:?}");
        assert!(messages[0].contains("timeline.nothing"));
    }

    #[test]
    fn a_bar_dragged_without_a_call_of_its_keys_is_a_finding() {
        let mut files = timeline();
        files[2].1 = "useBarDrag(1); <DragHandle kind={dragKinds.dot.kind} />".to_owned();
        let messages = found(&files);
        assert_eq!(messages.len(), 1, "{messages:?}");
        assert!(messages[0].contains("keyMoveOf"));
    }

    #[test]
    fn a_kind_written_in_place_is_a_finding_wherever_it_is_given() {
        for written in [
            "<DragHandle kind=\"a-dot\" />",
            "<DraggableHead kind={\"a-dot\"} />",
            "useDropOf( \"a-dot\", ref)",
        ] {
            let mut files = timeline();
            files[2].1 = format!("useBarDrag(1); keyMoveOf(event); {written}");
            let messages = found(&files);
            assert_eq!(messages.len(), 1, "{written}: {messages:?}");
            assert!(messages[0].contains("own kind"), "{written}");
        }
    }

    #[test]
    fn a_kind_after_an_arrow_in_the_element_is_a_finding() {
        let mut files = timeline();
        files[2].1 =
            "useBarDrag(1); keyMoveOf(event); <DragHandle onEnd={() => end()} kind=\"a-dot\" />"
                .to_owned();
        let messages = found(&files);
        assert_eq!(messages.len(), 1, "{messages:?}");
        assert!(messages[0].contains("own kind"));
    }

    #[test]
    fn a_drop_slot_with_a_kind_written_in_place_is_a_finding() {
        let mut files = timeline();
        files[2].1 = "useBarDrag(1); keyMoveOf(event); <DropSlot kind=\"a-dot\" onDrop={place} />"
            .to_owned();
        let messages = found(&files);
        assert_eq!(messages.len(), 1, "{messages:?}");
        assert!(messages[0].contains("own kind"));
    }

    #[test]
    fn a_kind_that_is_no_drag_is_no_finding() {
        let mut files = timeline();
        files[2].1 =
            "useBarDrag(1); keyMoveOf(event); const line = { kind: \"group\" };".to_owned();
        assert!(found(&files).is_empty());
    }

    #[test]
    fn the_real_timeline_has_a_key_for_every_drag() {
        let repo = crate::repo::Repo::discover().unwrap();
        let files = timeline_files(&repo).unwrap();
        let messages = read_text(&repo, MESSAGES_FILE).unwrap();
        assert!(drags_have_keys(&files, &messages).is_empty());
    }

    #[test]
    fn the_real_paths_are_read() {
        let repo = crate::repo::Repo::discover().unwrap();
        let text = read_text(&repo, PATHS_FILE).unwrap();
        let names: Vec<String> = entries(&text).into_iter().map(|(name, _)| name).collect();
        assert!(names.contains(&"home".to_owned()));
        assert!(names.contains(&"appearance".to_owned()));
    }
}
