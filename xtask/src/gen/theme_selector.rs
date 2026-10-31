//! The selectors of the rules in `design/theme.kdl` (ADR-0026): how `part "button[variant=primary]:hover"`
//! becomes `[data-part="button"][data-variant~="primary"]:hover`.
//!
//! A part rule names parts, never classes or tags. A part is `name` or `name.sub-part`. After it
//! come, in any order:
//!
//! - `[variant=x]` and `[state=x]`: one word of the space-separated list in `data-variant` or
//!   `data-state` (`[data-variant~="x"]`);
//! - `[key]` and `[key=value]`: the attribute `data-key`, which is how React Aria Components mark
//!   `hovered`, `pressed`, `selected`, `focus-visible`, `entering` and the like;
//! - `:hover`, `:focus-visible` and the other pseudo-classes of [`PSEUDO_CLASSES`];
//! - any other `:word` is refused, so that a typo of a pseudo-class is an error and not a new state
//!   that nothing ever sets: a state is written `[state=word]`;
//! - `:has(…)` and `:not(…)`, which hold a selector in this same way;
//! - `::before`, `::after`, `::placeholder`: the pseudo-elements of [`PSEUDO_ELEMENTS`].
//!
//! Parts are joined by a space (inside), `>` (child), `+` and `~` (sibling).

use std::fmt::Write as _;

use anyhow::{Context as _, bail};

/// The pseudo-classes of CSS that a rule may name.
pub(crate) const PSEUDO_CLASSES: [&str; 17] = [
    "visited",
    "invalid",
    "read-only",
    "hover",
    "focus",
    "focus-visible",
    "focus-within",
    "active",
    "disabled",
    "checked",
    "empty",
    "first-child",
    "last-child",
    "placeholder-shown",
    "has",
    "not",
    "root",
];

/// The pseudo-classes that hold a selector.
const WITH_SELECTOR: [&str; 2] = ["has", "not"];

/// The pseudo-elements a rule may name.
pub(crate) const PSEUDO_ELEMENTS: [&str; 4] = [
    "before",
    "after",
    "placeholder",
    "-webkit-search-cancel-button",
];

/// The attributes that hold a list of words, matched by one word.
const WORD_LISTS: [&str; 2] = ["variant", "state"];

enum Modifier {
    Attribute { key: String, value: Option<String> },
    Pseudo(String),
    PseudoWithSelector(String, Vec<Piece>),
    Element(String),
}

struct Compound {
    part: String,
    modifiers: Vec<Modifier>,
}

enum Piece {
    Compound(Compound),
    Combinator(char),
}

/// The CSS selector of a part rule.
pub(crate) fn compile(selector: &str) -> anyhow::Result<String> {
    let pieces = parse(selector).with_context(|| format!("selector \"{selector}\""))?;
    Ok(render(&pieces))
}

/// What a selector asks of a part: the words of its `data-variant` and `data-state`.
pub(crate) struct Words {
    pub(crate) part: String,
    pub(crate) variants: Vec<String>,
    pub(crate) states: Vec<String>,
}

/// The words each part of a selector must have, for the TypeScript that names them.
pub(crate) fn part_words(selector: &str) -> Vec<Words> {
    let mut found = Vec::new();
    if let Ok(pieces) = parse(selector) {
        gather_words(&pieces, &mut found);
    }
    found
}

fn gather_words(pieces: &[Piece], found: &mut Vec<Words>) {
    for piece in pieces {
        let Piece::Compound(compound) = piece else {
            continue;
        };
        let mut words = Words {
            part: compound.part.clone(),
            variants: Vec::new(),
            states: Vec::new(),
        };
        for modifier in &compound.modifiers {
            match modifier {
                Modifier::Attribute {
                    key,
                    value: Some(value),
                } if key == "variant" => words.variants.push(value.clone()),
                Modifier::Attribute {
                    key,
                    value: Some(value),
                } if key == "state" => words.states.push(value.clone()),
                Modifier::PseudoWithSelector(_, inner) => gather_words(inner, found),
                _ => {}
            }
        }
        found.push(words);
    }
}

/// The names of the parts a selector styles.
pub(crate) fn part_names(selector: &str) -> Vec<String> {
    let mut names = Vec::new();
    if let Ok(pieces) = parse(selector) {
        collect(&pieces, &mut names);
    }
    names
}

fn collect(pieces: &[Piece], names: &mut Vec<String>) {
    for piece in pieces {
        if let Piece::Compound(compound) = piece {
            names.push(compound.part.clone());
            for modifier in &compound.modifiers {
                if let Modifier::PseudoWithSelector(_, inner) = modifier {
                    collect(inner, names);
                }
            }
        }
    }
}

fn parse(selector: &str) -> anyhow::Result<Vec<Piece>> {
    let mut pieces = Vec::new();
    let mut previous_was_compound = false;
    for word in split_words(selector)? {
        if let [combinator @ ('>' | '+' | '~')] = word.chars().collect::<Vec<_>>()[..] {
            if !previous_was_compound {
                bail!("`{combinator}` needs a part before it");
            }
            pieces.push(Piece::Combinator(combinator));
            previous_was_compound = false;
        } else {
            pieces.push(Piece::Compound(compound(&word)?));
            previous_was_compound = true;
        }
    }
    if !previous_was_compound {
        bail!("a selector must end with a part");
    }
    Ok(pieces)
}

/// The words of a selector, split at spaces that are not inside `()` or `[]`.
fn split_words(selector: &str) -> anyhow::Result<Vec<String>> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut closers: Vec<char> = Vec::new();
    for c in selector.chars() {
        match c {
            '(' => closers.push(')'),
            '[' => closers.push(']'),
            ')' | ']' if closers.pop() != Some(c) => {
                bail!("a bracket `{c}` does not close the bracket that is open");
            }
            _ => {}
        }
        if c.is_whitespace() && closers.is_empty() {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
        } else {
            current.push(c);
        }
    }
    if !closers.is_empty() {
        bail!("a bracket is never closed");
    }
    if !current.is_empty() {
        words.push(current);
    }
    Ok(words)
}

/// A name of the theme: a part, a word of a part, a token or a theme. It starts with a small letter
/// and holds small letters, digits and `-`.
pub(crate) fn is_word(text: &str) -> bool {
    text.chars().next().is_some_and(|c| c.is_ascii_lowercase()) && text.chars().all(is_name_char)
}

/// A letter that may stand in a name of the theme (and in a CSS property after its first).
pub(crate) fn is_name_char(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'
}

fn is_part_name(text: &str) -> bool {
    text.split('.').all(is_word)
}

fn compound(word: &str) -> anyhow::Result<Compound> {
    let start = word.find(['[', ':']).unwrap_or(word.len());
    let part = &word[..start];
    if !is_part_name(part) {
        bail!(
            "`{part}` is not the name of a part (lowercase words with `-`, and `.` for a sub-part)"
        );
    }
    let mut modifiers = Vec::new();
    let mut rest = &word[start..];
    while !rest.is_empty() {
        let (modifier, tail) = modifier(rest)?;
        modifiers.push(modifier);
        rest = tail;
    }
    Ok(Compound {
        part: part.to_owned(),
        modifiers,
    })
}

/// The matching closing bracket of the one that opens `text`; returns what is inside and the rest.
fn bracketed(text: &str) -> anyhow::Result<(&str, &str)> {
    let mut depth = 0_i32;
    for (at, c) in text.char_indices() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => {
                depth -= 1;
                if depth == 0 {
                    return Ok((&text[1..at], &text[at + 1..]));
                }
            }
            _ => {}
        }
    }
    bail!("a bracket is never closed")
}

fn modifier(text: &str) -> anyhow::Result<(Modifier, &str)> {
    if text.starts_with('[') {
        let (inside, rest) = bracketed(text)?;
        let (key, value) = match inside.split_once('=') {
            Some((key, value)) => (key, Some(value)),
            None => (inside, None),
        };
        if !is_word(key) || value.is_some_and(|value| !is_word(value)) {
            bail!("`[{inside}]` must be `[key]` or `[key=value]` with lowercase words");
        }
        return Ok((
            Modifier::Attribute {
                key: key.to_owned(),
                value: value.map(str::to_owned),
            },
            rest,
        ));
    }
    if let Some(after) = text.strip_prefix("::") {
        let end = after.find([':', '[']).unwrap_or(after.len());
        let name = &after[..end];
        if !PSEUDO_ELEMENTS.contains(&name) {
            bail!("`::{name}` is not one of {PSEUDO_ELEMENTS:?}");
        }
        return Ok((Modifier::Element(name.to_owned()), &after[end..]));
    }
    let after = text.strip_prefix(':').context("expected `:` or `[`")?;
    let end = after.find([':', '[', '(']).unwrap_or(after.len());
    let name = &after[..end];
    if !is_word(name) {
        bail!("`:{name}` is not a word");
    }
    let tail = &after[end..];
    if tail.starts_with('(') {
        let (inside, rest) = bracketed(tail)?;
        if !WITH_SELECTOR.contains(&name) {
            bail!("`:{name}(…)` is not allowed; only {WITH_SELECTOR:?} hold a selector");
        }
        return Ok((
            Modifier::PseudoWithSelector(name.to_owned(), parse(inside)?),
            rest,
        ));
    }
    if !PSEUDO_CLASSES.contains(&name) {
        bail!(
            "`:{name}` is not a pseudo-class of CSS that rules may use; a state is `[state={name}]`"
        );
    }
    if WITH_SELECTOR.contains(&name) {
        bail!("`:{name}` needs a selector in brackets");
    }
    Ok((Modifier::Pseudo(name.to_owned()), tail))
}

fn render(pieces: &[Piece]) -> String {
    let mut out = String::new();
    let mut previous_was_compound = false;
    for piece in pieces {
        match piece {
            Piece::Compound(compound) => {
                if previous_was_compound {
                    out.push(' ');
                }
                let _ = write!(out, "[data-part=\"{}\"]", compound.part);
                for modifier in &compound.modifiers {
                    out.push_str(&render_modifier(modifier));
                }
                previous_was_compound = true;
            }
            Piece::Combinator(c) => {
                let _ = write!(out, " {c} ");
                previous_was_compound = false;
            }
        }
    }
    out
}

fn render_modifier(modifier: &Modifier) -> String {
    match modifier {
        Modifier::Attribute { key, value } => match value {
            Some(value) if WORD_LISTS.contains(&key.as_str()) => {
                format!("[data-{key}~=\"{value}\"]")
            }
            Some(value) => format!("[data-{key}=\"{value}\"]"),
            None => format!("[data-{key}]"),
        },
        Modifier::Pseudo(name) => format!(":{name}"),
        Modifier::PseudoWithSelector(name, inner) => format!(":{name}({})", render(inner)),
        Modifier::Element(name) => format!("::{name}"),
    }
}

/// A selector of a `base` rule: elements of the page, written as CSS. Only the characters of
/// element names, ids, classes of the page frame and simple combinators are allowed.
pub(crate) fn check_base(selector: &str) -> anyhow::Result<()> {
    let allowed = |c: char| {
        c.is_ascii_alphanumeric()
            || matches!(
                c,
                ' ' | ','
                    | '#'
                    | '.'
                    | '>'
                    | '+'
                    | '~'
                    | ':'
                    | '['
                    | ']'
                    | '='
                    | '"'
                    | '-'
                    | '_'
                    | '*'
                    | '('
                    | ')'
            )
    };
    if selector.trim().is_empty() || !selector.chars().all(allowed) {
        bail!("base selector \"{selector}\" holds a character that is not allowed");
    }
    Ok(())
}

/// A selector of a `rule` of the design pages: any CSS selector, except what could end the rule
/// and start another.
pub(crate) fn check_doc(selector: &str) -> anyhow::Result<()> {
    if selector.trim().is_empty()
        || ["{", "}", ";", "/*", "*/", "<"]
            .iter()
            .any(|bad| selector.contains(bad))
    {
        bail!("selector \"{selector}\" is empty or holds a character that is not allowed");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_part_is_a_data_part_attribute() {
        assert_eq!(compile("button").unwrap(), "[data-part=\"button\"]");
        assert_eq!(
            compile("field.label").unwrap(),
            "[data-part=\"field.label\"]"
        );
    }

    #[test]
    fn variants_states_and_react_aria_attributes_are_data_attributes() {
        assert_eq!(
            compile("button[variant=primary][hovered]").unwrap(),
            "[data-part=\"button\"][data-variant~=\"primary\"][data-hovered]"
        );
        assert_eq!(
            compile("button[state=busy]").unwrap(),
            "[data-part=\"button\"][data-state~=\"busy\"]"
        );
        assert_eq!(
            compile("modal[placement=top]").unwrap(),
            "[data-part=\"modal\"][data-placement=\"top\"]"
        );
    }

    #[test]
    fn pseudo_classes_pseudo_elements_and_combinators() {
        assert_eq!(
            compile("switch[selected] switch.track::after").unwrap(),
            "[data-part=\"switch\"][data-selected] [data-part=\"switch.track\"]::after"
        );
        assert_eq!(
            compile("choice.input:checked + choice.label").unwrap(),
            "[data-part=\"choice.input\"]:checked + [data-part=\"choice.label\"]"
        );
        assert_eq!(
            compile("picker.option:has(picker.input:focus-visible)").unwrap(),
            "[data-part=\"picker.option\"]:has([data-part=\"picker.input\"]:focus-visible)"
        );
    }

    #[test]
    fn the_parts_of_a_selector_are_listed_once_each_where_they_stand() {
        assert_eq!(
            part_names("picker.option:has(picker.input:focus) + picker.row"),
            ["picker.option", "picker.input", "picker.row"]
        );
    }

    #[test]
    fn a_selector_that_is_not_a_part_is_refused() {
        for bad in [
            "Button",
            ".button",
            "button[Variant=x]",
            "button::marker",
            "button:has",
            "button:busy",
            "button:hovr",
            "button[hovered)",
            "button(:hover]",
            "button:has(a]",
            "button:nth-child(2)",
            "button >",
            "> button",
            "button[hovered",
            "",
        ] {
            assert!(compile(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn base_selectors_allow_elements_and_ids() {
        assert!(check_base("html, body, #root").is_ok());
        assert!(check_base("a { color: red }").is_err());
    }
}
