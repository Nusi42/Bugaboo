//! The one rule that makes a text comparable for the full-text search (ADR-0016, amendment M5):
//! lower case, Unicode NFC, split into words. No letter is changed into another: an accent stays
//! (*müller* finds *Müller*, not *Muller* or *Mueller*), `ß` stays `ß`. The index and the words of
//! a search both go through `prepare` / [`search_words`], so a word is found exactly when it
//! was indexed.
//!
//! Only the words that go into the index are made NFC, and only with the feature `canonical-text`
//! (the NFC tables are too large for the search module of the browser, which checks a search and
//! never writes an index). Without it `prepare` and `prepare_joined` do not exist, so no code
//! can store a word that is not NFC; [`search_words`] still splits and lowers a text the same way,
//! and everything the checker asks of a word (how many letters, how many words) is counted so that
//! the NFC step cannot change it, except that a letter typed twice, once as one character and once
//! as two, is two words to the browser and one to the server, which checks again.

/// A word of a search. A word typed with a `*` at its end matches every word that starts with it.
/// Only [`search_words`] makes one, so every word is letters and digits of the one rule and
/// nothing else: it cannot carry the syntax of a database query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchWord {
    text: String,
    prefix: bool,
}

/// The fewest letters a word of a search needs before its `*` (`cra*` is fine, `c*` would match
/// nearly every word), counted on the prepared word. The checker of the search language refuses
/// a shorter start, and the search looks it up as the whole word: both ask [`SearchWord::matches_by_start`].
pub const MIN_PREFIX_CHARS: usize = 2;

impl SearchWord {
    /// Whether the word is looked up by its start: it was typed with a `*` and has at least
    /// [`MIN_PREFIX_CHARS`] letters. The one place that decides it.
    #[must_use]
    pub fn matches_by_start(&self) -> bool {
        self.prefix && self.letters() >= MIN_PREFIX_CHARS
    }

    /// How many letters the word has: its characters that are not combining marks, so a letter
    /// written as two characters counts once, with or without the NFC step.
    fn letters(&self) -> usize {
        self.text.chars().filter(|c| !is_mark(*c)).count()
    }

    /// The prepared word (lower case, NFC).
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Whether the word matches by its start (it was typed with a `*` at its end).
    #[must_use]
    pub const fn is_prefix(&self) -> bool {
        self.prefix
    }
}

/// The words of `text`, in order: lower case, NFC (a letter written as two characters is the same
/// word as the letter written as one), a word is a run of letters and digits with the combining
/// marks that belong to them, everything else (spaces, hyphens, dots, `@`, punctuation) separates words. A word that
/// comes twice comes twice: this is what the index keeps.
#[must_use]
#[cfg(any(test, feature = "canonical-text"))]
pub fn prepare(text: &str) -> Vec<String> {
    words_of(text).into_iter().map(|word| word.text).collect()
}

/// The words of a search text, as [`prepare`] makes them, each with whether it ended in `*`. A
/// word that is looked up twice (the same text, both times whole or both times by its start, as
/// [`SearchWord::matches_by_start`] says) is returned once, so a repeated word costs the database
/// nothing.
#[must_use]
pub fn search_words(text: &str) -> Vec<SearchWord> {
    let mut words: Vec<SearchWord> = Vec::new();
    for word in words_of(text) {
        let same = |other: &SearchWord| {
            other.text == word.text && other.matches_by_start() == word.matches_by_start()
        };
        // The same lookup again keeps the first word, and its `*` if the later one had it, so
        // whether a short start is seen does not depend on the order of the words.
        match words.iter_mut().find(|other| same(other)) {
            Some(kept) => kept.prefix |= word.prefix,
            None => words.push(word),
        }
    }
    words
}

/// Every word of the text in order, repeated ones too.
fn words_of(text: &str) -> Vec<SearchWord> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        // A combining mark belongs to the letter before it (a letter written as two characters);
        // one with no letter before it separates, like any other sign.
        if c.is_alphanumeric() || (!current.is_empty() && is_mark(c)) {
            current.extend(c.to_lowercase());
            if chars.peek().is_some() {
                continue;
            }
        }
        if !current.is_empty() {
            words.push(SearchWord {
                text: canonical(std::mem::take(&mut current)),
                prefix: c == '*',
            });
        }
    }
    words
}

/// The word in NFC where the feature is on.
fn canonical(word: String) -> String {
    #[cfg(any(test, feature = "canonical-text"))]
    {
        use unicode_normalization::UnicodeNormalization as _;
        word.nfc().collect()
    }
    #[cfg(not(any(test, feature = "canonical-text")))]
    word
}

/// Whether the character is a combining mark of the blocks that hold the accents written after
/// their letter (U+0300 to U+036F and the extensions). The marks of other scripts (vowel signs of
/// Indic scripts, Arabic marks) are alphabetic for Rust's `is_alphanumeric` and so belong to their
/// word already. Ranges, not the table of general categories: that table would add 14 KB to the
/// search module of the browser (PERF-04).
fn is_mark(c: char) -> bool {
    matches!(
        c,
        '\u{300}'..='\u{36f}'
            | '\u{1ab0}'..='\u{1aff}'
            | '\u{1dc0}'..='\u{1dff}'
            | '\u{20d0}'..='\u{20ff}'
            | '\u{fe20}'..='\u{fe2f}'
    )
}

/// The text of a words list as it is stored in the index: the words, one space between.
#[cfg(any(test, feature = "canonical-text"))]
#[must_use]
pub fn prepare_joined(text: &str) -> String {
    prepare(text).join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> Vec<String> {
        prepare(text)
    }

    #[test]
    fn case_does_not_matter() {
        assert_eq!(words("Crash On SAVE"), ["crash", "on", "save"]);
    }

    #[test]
    fn letters_are_not_changed_into_other_letters() {
        assert_eq!(
            words("Größe Äpfel über Straße"),
            ["größe", "äpfel", "über", "straße"]
        );
        assert_eq!(words("Æther Œuvre Ørsted"), ["æther", "œuvre", "ørsted"]);
        assert_eq!(words("Café naïve résumé"), ["café", "naïve", "résumé"]);
        assert_ne!(words("Müller"), words("Muller"));
        assert_ne!(words("Müller"), words("Mueller"));
        assert_ne!(words("Straße"), words("Strasse"));
        assert_eq!(words("MÜLLER"), words("müller"));
    }

    #[test]
    fn a_letter_written_as_two_characters_is_the_letter_written_as_one() {
        assert_eq!(words("Mu\u{308}ller"), words("M\u{fc}ller"));
        assert_eq!(words("Mu\u{308}ller"), ["m\u{fc}ller"]);
        assert_eq!(words("A\u{308}pfel"), ["\u{e4}pfel"]);
        assert_eq!(words("Cafe\u{301}"), ["café"]);
        // A mark with no letter before it separates words.
        assert_eq!(words("a \u{301}b"), ["a", "b"]);
    }

    #[test]
    fn hyphens_ids_and_mail_like_words_split_into_words() {
        assert_eq!(words("BB-123 is e-mail"), ["bb", "123", "is", "e", "mail"]);
        assert_eq!(
            words("anna.meier@example.de"),
            ["anna", "meier", "example", "de"]
        );
        assert_eq!(words("  ...  "), Vec::<String>::new());
    }

    #[test]
    fn a_star_at_the_end_of_a_word_makes_it_a_prefix() {
        let found = search_words("crash* on stor*e Fail*");
        assert_eq!(
            found,
            [
                SearchWord {
                    text: "crash".into(),
                    prefix: true
                },
                SearchWord {
                    text: "on".into(),
                    prefix: false
                },
                SearchWord {
                    text: "stor".into(),
                    prefix: true
                },
                SearchWord {
                    text: "e".into(),
                    prefix: false
                },
                SearchWord {
                    text: "fail".into(),
                    prefix: true
                },
            ]
        );
    }

    #[test]
    fn a_word_typed_twice_is_one_word_of_a_search_and_two_words_of_a_text() {
        let found: Vec<(String, bool)> = search_words("Crash crash CRASH* crash* save")
            .iter()
            .map(|word| (word.text().to_owned(), word.is_prefix()))
            .collect();
        let wanted = [("crash", false), ("crash", true), ("save", false)];
        assert_eq!(found.len(), wanted.len());
        for (found, wanted) in found.iter().zip(wanted) {
            assert_eq!((found.0.as_str(), found.1), wanted);
        }
        assert_eq!(words("crash crash"), ["crash", "crash"]);
    }

    #[test]
    fn the_same_rule_prepares_what_is_stored_and_what_is_searched() {
        assert_eq!(prepare_joined("Größe, BB-1"), "größe bb 1");
        let searched: Vec<String> = search_words("GRÖ\u{1e9e}E")
            .into_iter()
            .map(|w| w.text)
            .collect();
        assert_eq!(searched, words("Größe"));
        let decomposed: Vec<String> = search_words("Gro\u{308}\u{df}e")
            .into_iter()
            .map(|w| w.text)
            .collect();
        assert_eq!(decomposed, words("Größe"));
    }

    #[test]
    fn a_start_counts_letters_not_characters() {
        let by_start = |text: &str| search_words(text)[0].matches_by_start();
        assert!(!by_start("\u{e4}*"));
        assert!(!by_start("a\u{308}*"));
        assert!(by_start("\u{e4}b*"));
        assert!(by_start("a\u{308}b*"));
    }
}
