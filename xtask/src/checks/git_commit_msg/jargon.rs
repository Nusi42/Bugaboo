//! The words a commit message must not use: the project's own working vocabulary (which folder,
//! which branch, which round of work) says nothing to a reader of the history. The message says
//! what changed and why, in plain English.

/// Single words, compared in any case: the word and why it is refused.
const WORDS: [(&str, &str); 5] = [
    ("master", "work-folder logistics"),
    ("lead", "work-folder logistics"),
    ("wave", "process jargon"),
    ("owner", "talk about a person's role"),
    ("wip", "a mark of unfinished work"),
];

/// Words that are refused in capital letters only, because in lower case they are ordinary words
/// (`dry run`).
const ACRONYMS: [&str; 3] = ["SSOT", "ODR", "DRY"];

/// Phrases of several words, compared in any case: the words (a word ending in `*` matches any
/// word that starts with it) and why the phrase is refused.
const PHRASES: [(&[&str], &str); 4] = [
    (&["work", "folder*"], "work-folder logistics"),
    (&["work", "package*"], "process jargon"),
    (&["work", "in", "progress"], "a mark of unfinished work"),
    (&["fast", "forward*"], "branch logistics"),
];

/// Phrases of the product that contain a refused word: a role and a kind of key. Their words are
/// not jargon there; `lead` and `master` on their own still are.
const PRODUCT_PHRASES: [[&str; 2]; 2] = [["project", "lead"], ["master", "key"]];

/// Splits a text into words: runs of letters and digits.
fn words_of(text: &str) -> Vec<&str> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect()
}

fn all_digits(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_ascii_digit())
}

/// `win1`, `WINn`, `win12`: the number or the letter `n` after `win`.
fn is_folder_name(lower: &str) -> bool {
    lower
        .strip_prefix("win")
        .is_some_and(|rest| rest == "n" || all_digits(rest))
}

/// `M5`, `M2C`, `M0`: the code of a milestone (a capital `M`, digits, perhaps one capital letter).
fn is_milestone_code(word: &str) -> bool {
    let Some(rest) = word.strip_prefix('M') else {
        return false;
    };
    let digits = rest.trim_end_matches(|c: char| c.is_ascii_uppercase());
    all_digits(digits) && rest.len() - digits.len() <= 1
}

/// `WP`, a number or `nn` (`WP-03`, `WPnn`, `wp12`).
fn is_package_code(lower: &str, next: Option<&str>) -> bool {
    match lower.strip_prefix("wp") {
        Some("") => next.is_some_and(|next| all_digits(next) || next.eq_ignore_ascii_case("nn")),
        Some(rest) => all_digits(rest) || rest == "nn",
        None => false,
    }
}

fn phrase_matches(phrase: &[&str], lowered: &[String]) -> bool {
    lowered.windows(phrase.len()).any(|window| {
        window.iter().zip(phrase).all(|(word, wanted)| {
            wanted
                .strip_suffix('*')
                .map_or(word == wanted, |start| word.starts_with(start))
        })
    })
}

/// What in the text is jargon: one sentence for each term found, naming the term and why.
pub(super) fn problems(text: &str) -> Vec<String> {
    let found = words_of(text);
    let lowered: Vec<String> = found.iter().map(|word| word.to_lowercase()).collect();
    let mut product = vec![false; found.len()];
    for (at, pair) in lowered.windows(2).enumerate() {
        if PRODUCT_PHRASES.iter().any(|phrase| pair == phrase) {
            product[at] = true;
            product[at + 1] = true;
        }
    }
    let mut terms: Vec<(String, &str)> = Vec::new();
    for (at, (word, lower)) in found.iter().zip(&lowered).enumerate() {
        if product[at] {
            continue;
        }
        let next = found.get(at + 1).copied();
        if is_folder_name(lower) {
            terms.push(((*word).to_owned(), "work-folder or branch logistics"));
        } else if is_milestone_code(word) {
            terms.push(((*word).to_owned(), "an internal milestone code"));
        } else if is_package_code(lower, next) {
            terms.push(((*word).to_owned(), "an internal work package code"));
        } else if ACRONYMS.contains(word) {
            terms.push(((*word).to_owned(), "internal jargon"));
        } else if let Some((term, why)) = WORDS.iter().find(|(term, _)| lower == term) {
            terms.push(((*term).to_owned(), why));
        }
    }
    for (phrase, why) in PHRASES {
        if phrase_matches(phrase, &lowered) {
            terms.push((phrase.join(" ").replace('*', ""), why));
        }
    }
    terms.sort();
    terms.dedup();
    terms
        .into_iter()
        .map(|(term, why)| format!("`{term}` is {why}. Say what changed in plain words instead."))
        .collect()
}
