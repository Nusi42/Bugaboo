//! ARCH-WEB-ROLES: the ARIA roles of the issue list's head, rows and cells are written in
//! `web/packages/ui/src/ListRoles.tsx` and nowhere else (coding-standards §4, ADR-0026).
//!
//! A virtualized grid of divs cannot use `table`, `tr`, `th` or `hr`, so these roles are given by
//! attribute, once, in the small components of that file; every use goes through them. The words
//! `columnheader`, `rowheader` and `separator`, and the roles `row`, `cell` and `table` given to an
//! element, are a finding anywhere else in the browser app and its packages (tests excluded), so the
//! next place that wants one asks for the component and does not write a second definition.

use super::web_source::code_of;
use super::{Check, Context, Finding};
use crate::source::{Kind, Source};

/// The one file that writes these roles.
const ROLES_HOME: &str = "web/packages/ui/src/ListRoles.tsx";
const SOURCE: [&str; 2] = ["web/app/src/", "web/packages/ui/src/"];

/// Words that are roles of a grid and nothing else.
const ROLE_WORDS: [&str; 3] = ["\"columnheader\"", "\"rowheader\"", "\"separator\""];

/// Roles that are given to an element with `role=` or `role:`.
const GIVEN: [&str; 6] = [
    "role=\"row\"",
    "role=\"cell\"",
    "role=\"table\"",
    "role:\"row\"",
    "role:\"cell\"",
    "role:\"table\"",
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

fn evaluate(sources: &[Source]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (path, text) in sources {
        let in_source = SOURCE.iter().any(|root| path.starts_with(root));
        if !in_source || path == ROLES_HOME || path.contains(".test.") {
            continue;
        }
        for (index, line) in text.lines().enumerate() {
            let code: String = code_of(line)
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect();
            if ROLE_WORDS.iter().any(|word| code.contains(word))
                || GIVEN.iter().any(|given| code.contains(given))
            {
                findings.push(Finding::at(
                    format!("{path}:{}", index + 1),
                    "A role of the list's head, rows or cells is written here. They live in web/packages/ui/src/ListRoles.tsx only: use its components or add one there.",
                ));
            }
        }
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
    fn good_example_the_roles_in_their_home_and_in_tests() {
        assert_eq!(found(ROLES_HOME, "<div role=\"row\" />"), 0);
        assert_eq!(
            found(
                "web/packages/ui/src/parts-lists.test.tsx",
                "expect(head).toContain('role=\"columnheader\"');"
            ),
            0
        );
        assert_eq!(
            found(
                "web/app/src/features/search/OfferList.tsx",
                "<ul role={ariaWords.listbox}>"
            ),
            0
        );
    }

    #[test]
    fn bad_example_a_role_of_the_grid_written_elsewhere() {
        assert_eq!(
            found(
                "web/app/src/features/issues/X.tsx",
                "<div role=\"columnheader\">"
            ),
            1
        );
        assert_eq!(
            found(
                "web/packages/ui/src/Y.tsx",
                "const r = { row: \"row\" }; <p role=\"row\">"
            ),
            1
        );
        assert_eq!(
            found("web/packages/ui/src/Z.tsx", "const w = \"separator\";"),
            1
        );
    }
}
