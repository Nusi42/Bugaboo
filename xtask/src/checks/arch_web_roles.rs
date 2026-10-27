//! ARCH-WEB-ROLES: the ARIA roles of the issue list's head, rows and cells are written in
//! `web/packages/ui/src/ListRoles.tsx` and nowhere else (coding-standards §4, ADR-0026).
//!
//! A virtualized grid of divs cannot use `table`, `tr`, `th` or `hr`, so these roles are given by
//! attribute, once, in the small components of that file; every use goes through them. In the
//! browser app and its packages (tests excluded) a finding is
//!
//! * a string literal of a grid role word (`row`, `cell`, `table`, `grid`, `gridcell`, `rowgroup`,
//!   `columnheader`, `rowheader`, `separator`) outside `ListRoles.tsx`, however it is used (an
//!   attribute, a constant, an object): the word is written once. A CSS selector string, one that
//!   starts with `[`, only looks for an element and is not a role given to one;
//! * `role=` followed by an expression on a plain element (`<div role={x}>`) in a file that is not
//!   one of the few homes of role constants: `ListRoles.tsx` and the files listed in `ROLE_HOMES`,
//!   each with the reason.

use super::web_source::code_of;
use super::{Check, Context, Finding};
use crate::source::{Kind, Source};

/// The one file that writes the roles of the list.
const ROLES_HOME: &str = "web/packages/ui/src/ListRoles.tsx";
const SOURCE: [&str; 2] = ["web/app/src/", "web/packages/ui/src/"];

/// Words that are roles of a grid and nothing else: a literal of one is a finding wherever it stands.
const GRID_ONLY: [&str; 5] = [
    "gridcell",
    "rowgroup",
    "columnheader",
    "rowheader",
    "separator",
];

/// Words that are roles of a grid and also ordinary words (a variant `grid`, a key `row`): a literal
/// of one is a finding where it is given as a role (a line with `role` before an `=` or `:`) or
/// stands in an object whose name has `role` in it.
const GRID_AND_PLAIN: [&str; 4] = ["row", "cell", "table", "grid"];

/// The files that hold the role constants of other parts, with the reason: each gives a role to
/// one element by a named constant of its own (an alert, a status line, a dialog, a list box).
const ROLE_HOMES: [(&str, &str); 5] = [
    (
        "web/packages/ui/src/Alert.tsx",
        "the role of a message: alert or status",
    ),
    (
        "web/app/src/features/shell/UndoHost.tsx",
        "the polite status line of the undo offer",
    ),
    (
        "web/packages/ui/src/SaveStatus.tsx",
        "the polite status of a save",
    ),
    (
        "web/packages/ui/src/ConfirmDialog.tsx",
        "the role of a question: alertdialog",
    ),
    (
        "web/app/src/features/search/OfferList.tsx",
        "the list box of the offers and its options",
    ),
];

pub(crate) struct ArchWebRoles;

impl Check for ArchWebRoles {
    fn id(&self) -> &'static str {
        "ARCH-WEB-ROLES"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        Ok(evaluate(&context.sources(self.id(), Kind::Web)?))
    }
}

fn message() -> &'static str {
    "A role of the list's head, rows or cells is written here. They live in web/packages/ui/src/ListRoles.tsx only: use its components or add one there."
}

/// The string literals of a line, read as a language does: a text runs from a quote to the same
/// quote, so another quote inside it is part of it. A CSS selector (a text that starts with `[`)
/// only looks for an element, so it is left out with everything inside it, and the rest of the line
/// is still read.
fn literals(line: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut chars = line.char_indices().peekable();
    while let Some((start, quote)) = chars.next() {
        if !matches!(quote, '"' | '\'' | '`') {
            continue;
        }
        let from = start + quote.len_utf8();
        let mut escaped = false;
        for (at, each) in chars.by_ref() {
            if escaped {
                escaped = false;
            } else if each == '\\' {
                escaped = true;
            } else if each == quote {
                let text = &line[from..at];
                if !text.starts_with('[') {
                    found.push(text);
                }
                break;
            }
        }
    }
    found
}

/// Whether the line names a role and then gives it something: `role=…`, `role: …`, `tableRole = …`.
fn names_a_role(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower
        .find("role")
        .is_some_and(|at| lower[at..].contains(['=', ':']))
}

/// Whether a plain element is given a role by an expression: `<div … role={…}`.
fn gives_a_role_by_expression(text: &str) -> Vec<usize> {
    let mut lines = Vec::new();
    let mut from = 0;
    while let Some(found) = text[from..].find("role=") {
        let at = from + found;
        from = at + 5;
        let after = text[from..].trim_start();
        if !after.starts_with('{') {
            continue;
        }
        // The element it belongs to: the last tag opened before it. A lower-case name is an element of
        // the page, a capital one is a component with a prop that is called `role`.
        let tag = text[..at].rfind('<').map(|open| &text[open + 1..]);
        let plain = tag.is_some_and(|name| {
            name.chars()
                .next()
                .is_some_and(|first| first.is_ascii_lowercase())
        });
        if plain {
            lines.push(text[..at].matches('\n').count() + 1);
        }
    }
    lines
}

fn evaluate(sources: &[Source]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (path, text) in sources {
        let in_source = SOURCE.iter().any(|root| path.starts_with(root));
        if !in_source || path == ROLES_HOME || path.contains(".test.") {
            continue;
        }
        let mut flagged: Vec<usize> = Vec::new();
        let mut in_role_object = false;
        for (index, line) in text.lines().enumerate() {
            let code = code_of(line);
            let words = literals(code);
            let named = names_a_role(code);
            let mut bad = words.iter().any(|word| GRID_ONLY.contains(word))
                || ((named || in_role_object)
                    && words.iter().any(|word| GRID_AND_PLAIN.contains(word)));
            if in_role_object && code.trim_start().starts_with('}') {
                in_role_object = false;
            } else if named
                && (code.trim_end().ends_with("= {") || code.trim_end().ends_with(": {"))
            {
                in_role_object = true;
            }
            bad = bad && !code.trim_start().starts_with("//");
            if bad {
                flagged.push(index + 1);
            }
        }
        if ROLE_HOMES.iter().all(|(home, _)| home != path) {
            flagged.extend(gives_a_role_by_expression(text));
        }
        flagged.sort_unstable();
        flagged.dedup();
        findings.extend(
            flagged
                .into_iter()
                .map(|line| Finding::at(format!("{path}:{line}"), message())),
        );
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(path: &str, text: &str) -> usize {
        evaluate(&[(path.to_owned(), text.to_owned())]).len()
    }

    #[test]
    fn good_example_the_roles_in_their_home_the_selectors_and_the_tests() {
        assert_eq!(found(ROLES_HOME, "<div role=\"row\" />"), 0);
        assert_eq!(
            found(
                "web/packages/ui/src/parts-lists.test.tsx",
                "expect(head).toContain('role=\"columnheader\"');"
            ),
            0
        );
        // The ordinary words are fine where they are no role; a role constant in a listed home, a
        // selector, and a component prop called role.
        assert_eq!(
            found(
                "web/app/src/features/issues/ListRows.tsx",
                "variants: [phone ? \"phone\" : \"grid\"], row: \"row\""
            ),
            0
        );
        assert_eq!(
            found("web/packages/ui/src/Alert.tsx", "<div role={roles.alert}>")
                + found(
                    "web/app/src/features/issues/use-row-keys.ts",
                    "const rows = '[data-rac][role=\"row\"]';"
                )
                + found(
                    "web/app/src/features/roles/RolePage.tsx",
                    "<RoleSections role={page.role} />"
                )
                + found(
                    "web/app/src/features/roles/RolePage.tsx",
                    "function RolePage({ role }) {
  const kind = \"row\";
}"
                ),
            0
        );
    }

    #[test]
    fn bad_example_a_role_written_elsewhere_in_any_shape() {
        let in_ui = "web/packages/ui/src/Y.tsx";
        assert_eq!(found(in_ui, "<p role=\"row\">"), 1);
        assert_eq!(found(in_ui, "<p role={\"row\"}>"), 1);
        assert_eq!(found(in_ui, "const roles = { head: \"columnheader\" };"), 1);
        assert_eq!(
            found(
                in_ui,
                "const role = {
  table: \"table\",
  row: \"row\",
} as const;"
            ),
            2
        );
        assert_eq!(found(in_ui, "const tableRole = \"table\";"), 1);
        // A selector on the line does not hide a role beside it.
        assert_eq!(
            found(in_ui, "const a = '[role=\"row\"]', roleRow = \"row\";"),
            1
        );
        assert_eq!(found(in_ui, "const w = 'gridcell';"), 1);
        assert_eq!(
            found(in_ui, "<div\n  className=\"a\"\n  role={headRole}\n>"),
            1
        );
        assert_eq!(
            found(
                "web/app/src/features/x/X.tsx",
                "<div {...x} role={rowRole}>"
            ),
            1
        );
    }
}
