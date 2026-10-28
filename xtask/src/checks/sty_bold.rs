//! STY-BOLD: bold is only for what UX §7.3 names (the rule of `design/theme.kdl`, the `bold` list).
//!
//! UX §7.3 allows bold for the title of an issue, the title of a page or a window, the heading of
//! a section and the word of a Critical or High priority. The theme file names the selectors that
//! may set the bold weight in one list (`bold { … }`), each with its role. A rule of the app, or a
//! mixin, that sets the bold weight anywhere else fails, so bold cannot creep back; a selector in
//! the list that no rule makes bold fails too, so the list stays true. The pages of `docs/` are
//! documents with their own reading type and are not part of the rule.

use std::collections::BTreeSet;

use super::{Check, Context, Finding};
use crate::generate::theme_file::{Item, Rule, RuleKind, THEME_PATH, ThemeFile};

pub(crate) struct StyBold;

impl Check for StyBold {
    fn id(&self) -> &'static str {
        "STY-BOLD"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        Ok(evaluate(&ThemeFile::load(context.repo)?))
    }
}

/// Whether a value of `font-weight` is a bold one: the token, a keyword or a number from 600.
fn is_bold(value: &str) -> bool {
    let value = value.trim();
    value == "$weight-bold"
        || matches!(value, "bold" | "bolder" | "semibold")
        || value.parse::<u32>().is_ok_and(|weight| weight >= 600)
}

/// The line of the first bold weight in these lines, also inside `forced`, `media` and `container`.
fn bold_line(items: &[Item]) -> Option<usize> {
    items.iter().find_map(|item| match item {
        Item::Declaration(declaration)
            if declaration.property == "font-weight" && is_bold(&declaration.value) =>
        {
            Some(declaration.line)
        }
        Item::Media { items, .. } => bold_line(items),
        _ => None,
    })
}

fn evaluate(file: &ThemeFile) -> Vec<Finding> {
    let mut findings = Vec::new();
    let listed: BTreeSet<&str> = file
        .bold
        .selectors
        .iter()
        .map(|entry| entry.selector.as_str())
        .collect();
    let mut made_bold: BTreeSet<&str> = BTreeSet::new();
    for mixin in &file.mixins {
        if let Some(line) = bold_line(&mixin.items) {
            findings.push(Finding::at(
                format!("{THEME_PATH}:{line}"),
                format!(
                    "The mixin `{}` sets the bold weight, and any rule can use a mixin. Set bold in a rule that the `bold` list names (UX §7.3).",
                    mixin.name
                ),
            ));
        }
    }
    for rule in file.rules.iter().filter(|rule| rule.kind != RuleKind::Doc) {
        let Some(line) = bold_line(&rule.items) else {
            continue;
        };
        findings.extend(unlisted(rule, line, &listed));
        made_bold.extend(rule.selectors.iter().map(String::as_str));
    }
    for entry in &file.bold.selectors {
        if !made_bold.contains(entry.selector.as_str()) {
            findings.push(Finding::at(
                format!("{THEME_PATH}:{}", entry.line),
                format!(
                    "`{}` is in the `bold` list (role `{}`) but no rule makes it bold. Take it out of the list, or give the rule its bold weight.",
                    entry.selector, entry.role
                ),
            ));
        }
    }
    findings
}

/// A finding for each selector of a bold rule that the list does not name.
fn unlisted<'a>(
    rule: &'a Rule,
    line: usize,
    listed: &'a BTreeSet<&str>,
) -> impl Iterator<Item = Finding> + 'a {
    rule.selectors
        .iter()
        .filter(|selector| !listed.contains(selector.as_str()))
        .map(move |selector| {
            Finding::at(
                format!("{THEME_PATH}:{line}"),
                format!(
                    "`{selector}` is bold, but bold is only for what UX §7.3 names (the title of an issue, a page or a window, the heading of a section, the word of a Critical or High priority). Use the regular weight, or add the selector with its role to the `bold` list of the theme file."
                ),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::Repo;

    const LABEL_REGULAR: &str =
        "part \"field.label\" {\n    font-size \"$fs-md\"\n    font-weight \"$weight-regular\"";

    fn text() -> String {
        crate::source::read_text(&Repo::discover().unwrap(), THEME_PATH).unwrap()
    }

    fn messages(findings: &[Finding]) -> Vec<String> {
        findings
            .iter()
            .map(|finding| finding.message.clone())
            .collect()
    }

    /// The theme file with `from` written as `to`, once.
    fn changed(from: &str, to: &str) -> ThemeFile {
        let text = text();
        assert!(text.contains(from), "{from} is in the file");
        ThemeFile::parse(&text.replacen(from, to, 1)).unwrap()
    }

    #[test]
    fn good_example() {
        let findings = evaluate(&ThemeFile::load(&Repo::discover().unwrap()).unwrap());
        assert!(findings.is_empty(), "{:#?}", messages(&findings));
    }

    #[test]
    fn bad_example_a_field_label_in_bold() {
        let file = changed(
            LABEL_REGULAR,
            "part \"field.label\" {\n    font-size \"$fs-md\"\n    font-weight \"$weight-bold\"",
        );
        let findings = evaluate(&file);
        assert!(
            findings.iter().any(|f| f.message.contains("`field.label`")),
            "{:?}",
            messages(&findings)
        );
    }

    #[test]
    fn bad_example_a_bold_number_is_bold_too() {
        let file = changed(
            LABEL_REGULAR,
            "part \"field.label\" {\n    font-size \"$fs-md\"\n    font-weight 700",
        );
        assert!(!evaluate(&file).is_empty());
    }

    #[test]
    fn bad_example_a_mixin_in_bold() {
        let file = changed(
            "mixin \"quiet-text\" {\n    color \"$fg-muted\"",
            "mixin \"quiet-text\" {\n    font-weight \"$weight-bold\"\n    color \"$fg-muted\"",
        );
        let findings = evaluate(&file);
        assert!(
            findings.iter().any(|f| f.message.contains("`quiet-text`")),
            "{:?}",
            messages(&findings)
        );
    }

    #[test]
    fn bad_example_a_listed_selector_that_is_not_bold() {
        let file = changed(
            "selector \"priority[variant=high]\" role=\"priority\"",
            "selector \"priority[variant=high]\" role=\"priority\"\n    selector \"field.label\" role=\"section\"",
        );
        let findings = evaluate(&file);
        assert!(
            findings
                .iter()
                .any(|f| f.message.contains("no rule makes it bold")),
            "{:?}",
            messages(&findings)
        );
    }

    #[test]
    fn a_role_that_is_not_in_the_list_of_roles_is_refused() {
        let text = text().replacen("role=\"priority\"", "role=\"shouting\"", 1);
        assert!(ThemeFile::parse(&text).is_err());
    }
}
