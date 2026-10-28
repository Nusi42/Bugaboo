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
use crate::generate::theme_file::{
    Declaration, Item, Rule, RuleKind, THEME_PATH, ThemeFile, TokenKind,
};

pub(crate) struct StyBold;

impl Check for StyBold {
    fn id(&self) -> &'static str {
        "STY-BOLD"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        Ok(evaluate(&ThemeFile::load(context.repo)?))
    }
}

/// The elements that the browser draws in bold; the theme file must make each of them regular, so
/// bold only comes from a rule the `bold` list names.
const BOLD_BY_BROWSER: [&str; 10] = [
    "h1", "h2", "h3", "h4", "h5", "h6", "th", "strong", "b", "optgroup",
];

/// Whether a word is a bold weight: a keyword, a number from 600, or a weight token with such a number.
fn word_is_bold(file: &ThemeFile, word: &str) -> bool {
    if matches!(word, "bold" | "bolder" | "semibold") {
        return true;
    }
    let number = match word.strip_prefix('$') {
        Some(name) => file
            .token(name)
            .filter(|token| token.kind == TokenKind::Weight)
            .and_then(|token| token.same.as_deref()),
        None => Some(word),
    };
    number.is_some_and(|number| number.parse::<u32>().is_ok_and(|weight| weight >= 600))
}

/// Whether a line sets a bold weight: `font-weight`, or a bold word anywhere in a `font` shorthand.
fn is_bold(file: &ThemeFile, declaration: &Declaration) -> bool {
    match declaration.property.as_str() {
        "font-weight" => word_is_bold(file, declaration.value.trim()),
        "font" => declaration
            .value
            .split_whitespace()
            .any(|word| word_is_bold(file, word)),
        _ => false,
    }
}

/// The line of the first bold weight in these lines, also inside `forced`, `media` and `container`.
fn bold_line(file: &ThemeFile, items: &[Item]) -> Option<usize> {
    items.iter().find_map(|item| match item {
        Item::Declaration(declaration) if is_bold(file, declaration) => Some(declaration.line),
        Item::Media { items, .. } => bold_line(file, items),
        _ => None,
    })
}

/// The elements of the browser's bold that no base rule makes regular.
fn not_reset(file: &ThemeFile) -> Vec<&'static str> {
    let reset: Vec<&str> = file
        .rules
        .iter()
        .filter(|rule| rule.kind == RuleKind::Base)
        .filter(|rule| {
            rule.items.iter().any(|item| {
                matches!(item, Item::Declaration(d) if d.property == "font-weight" && !word_is_bold(file, d.value.trim()))
            })
        })
        .flat_map(|rule| rule.selectors.iter().flat_map(|s| s.split(',')))
        .map(str::trim)
        .collect();
    BOLD_BY_BROWSER
        .into_iter()
        .filter(|element| !reset.contains(element))
        .collect()
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
        if let Some(line) = bold_line(file, &mixin.items) {
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
        let Some(line) = bold_line(file, &rule.items) else {
            continue;
        };
        findings.extend(unlisted(rule, line, &listed));
        made_bold.extend(rule.selectors.iter().map(String::as_str));
    }
    for element in not_reset(file) {
        findings.push(Finding::at(
            THEME_PATH,
            format!(
                "The browser draws `{element}` in bold, and no base rule of the theme file makes it regular. Add it to the base rule that sets the regular weight, so bold only comes from the `bold` list."
            ),
        ));
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
    fn bad_example_a_weight_token_of_600_is_bold() {
        let text = text()
            .replacen(
                "    weight \"weight-bold\" 700",
                "    weight \"weight-bold\" 700
    weight \"weight-semi\" 600",
                1,
            )
            .replacen(
                LABEL_REGULAR,
                &LABEL_REGULAR.replace("$weight-regular", "$weight-semi"),
                1,
            );
        let findings = evaluate(&ThemeFile::parse(&text).unwrap());
        assert!(
            findings.iter().any(|f| f.message.contains("`field.label`")),
            "{:?}",
            messages(&findings)
        );
    }

    #[test]
    fn bad_example_the_weight_in_a_font_shorthand_is_found() {
        let file = changed(
            LABEL_REGULAR,
            "part \"field.label\" {
    font-size \"$fs-md\"
    font \"bold 14px sans-serif\"",
        );
        let findings = evaluate(&file);
        assert!(
            findings.iter().any(|f| f.message.contains("`field.label`")),
            "{:?}",
            messages(&findings)
        );
    }

    #[test]
    fn bad_example_an_element_that_is_bold_in_the_browser_and_not_reset() {
        let file = changed(" \"strong\" \"b\" \"optgroup\" {", " \"b\" \"optgroup\" {");
        let findings = evaluate(&file);
        assert!(
            findings.iter().any(|f| f.message.contains("`strong`")),
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
