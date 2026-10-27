//! The one rule that makes a text comparable for the full-text search (ADR-0016, amendment M5):
//! lower case, Unicode NFC, split into words. No letter is changed into another: an accent stays
//! (*müller* finds *Müller*, not *Muller* or *Mueller*), `ß` stays `ß`.
//!
//! Two steps, each with one rule:
//!
//! - **What the checker decides** ([`checked_words`]): how many words a `contains` has, and
//!   whether a word typed with `*` has enough letters ([`letters`]). The text is lower-cased and
//!   split as typed, with no NFC, so the browser and the server give the same answer.
//! - **What is stored and matched** (`search_words`, `prepare`, `prepare_joined`): the whole text
//!   is lower-cased, put into NFC and then split, so a letter written as two characters is the
//!   same word as the letter written as one. They exist only with the feature `canonical-text`
//!   (see ADR-0016), so no code can store a word that is not NFC.
//!
//! Both go through the same function that splits, and the index and the words of a search both go
//! through the second step, so a word is found exactly when it was indexed.

#[cfg(any(test, feature = "canonical-text"))]
use unicode_normalization::UnicodeNormalization as _;

/// A word of a search. A word typed with a `*` at its end matches every word that starts with it.
/// Only [`checked_words`] and `search_words` make one, so a word holds no character that could be
/// the syntax of a database query: letters, digits and the marks of a letter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchWord {
    text: String,
    prefix: bool,
}

/// The fewest letters a word of a search needs before its `*` (`cra*` is fine, `c*` would match
/// nearly every word), counted by [`letters`]. The checker of the search language refuses a
/// shorter start, and the search looks it up as the whole word: both ask [`SearchWord::matches_by_start`].
pub const MIN_PREFIX_CHARS: usize = 2;

/// The most bytes a word may have to be stored in the index or to be looked up (ADR-0016). A real
/// word is never this long; a longer run of letters is a dump (a key, a hash, a line of text
/// without a space or a stop) that nobody searches for. Both databases stay below their own limits
/// by far (a lexeme of PostgreSQL may have 2046 bytes at most, and one over it fails the whole
/// statement). A longer run is not stored, so a text never fails to be indexed, and a longer word
/// is refused when it is typed in a search (the checker), so a search never fails either.
pub const MAX_WORD_BYTES: usize = 255;

/// How many letters a word has: its characters that are not combining marks, so a letter written
/// as two characters counts once. The one place that counts them; what asks whether a start has
/// enough letters calls it.
#[must_use]
pub fn letters(word: &str) -> usize {
    word.chars().filter(|c| !is_mark(*c)).count()
}

impl SearchWord {
    /// Whether the word is looked up by its start: it was typed with a `*` and has at least
    /// [`MIN_PREFIX_CHARS`] letters. The one place that decides it.
    #[must_use]
    pub fn matches_by_start(&self) -> bool {
        self.prefix && letters(&self.text) >= MIN_PREFIX_CHARS
    }

    /// The word (lower case; NFC where the feature is on).
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Whether the word is longer than the index keeps a word ([`MAX_WORD_BYTES`]): such a word is
    /// refused by the checker as typed, and is found nowhere in the index. The one place that
    /// compares a word with the bound.
    #[must_use]
    pub const fn is_too_long(&self) -> bool {
        self.text.len() > MAX_WORD_BYTES
    }

    /// Whether the word matches by its start (it was typed with a `*` at its end).
    #[must_use]
    pub const fn is_prefix(&self) -> bool {
        self.prefix
    }
}

/// The words of a search text as the checker counts them: lower case, as typed (no NFC), each with
/// whether it ended in `*`. A word that is looked up twice (the same text, both times whole or
/// both times by its start, as [`SearchWord::matches_by_start`] says) is returned once. The
/// answer is the same in the browser and on the server.
#[must_use]
pub fn checked_words(text: &str) -> Vec<SearchWord> {
    once_each(split(&lowered(text)))
}

/// The words of a search text as they are looked up in the index: lower case and NFC for the whole
/// text, then split; each with whether it ended in `*`. A word looked up twice is returned once, so
/// a repeated word costs the database nothing.
#[must_use]
#[cfg(any(test, feature = "canonical-text"))]
pub fn search_words(text: &str) -> Vec<SearchWord> {
    once_each(split(&canonical(text)))
}

/// The words of `text` as they are stored, in order. A word that comes twice comes twice: this is
/// what the index keeps. A word over [`MAX_WORD_BYTES`] is left out, so storing a text never
/// fails because of one.
#[must_use]
#[cfg(any(test, feature = "canonical-text"))]
pub fn prepare(text: &str) -> Vec<String> {
    split(&canonical(text))
        .into_iter()
        .filter(|word| !word.is_too_long())
        .map(|word| word.text)
        .collect()
}

/// The text of a words list as it is stored in the index: the words, one space between.
#[must_use]
#[cfg(any(test, feature = "canonical-text"))]
pub fn prepare_joined(text: &str) -> String {
    prepare(text).join(" ")
}

/// The text in lower case, character by character.
fn lowered(text: &str) -> String {
    text.chars().flat_map(char::to_lowercase).collect()
}

/// The text in lower case and NFC.
#[cfg(any(test, feature = "canonical-text"))]
fn canonical(text: &str) -> String {
    lowered(text).nfc().collect()
}

/// Each lookup once. The same lookup again keeps the first word, and its `*` if the later one had
/// it, so whether a short start is seen does not depend on the order of the words.
fn once_each(all: Vec<SearchWord>) -> Vec<SearchWord> {
    let mut words: Vec<SearchWord> = Vec::new();
    for word in all {
        let same = |other: &SearchWord| {
            other.text == word.text && other.matches_by_start() == word.matches_by_start()
        };
        match words.iter_mut().find(|other| same(other)) {
            Some(kept) => kept.prefix |= word.prefix,
            None => words.push(word),
        }
    }
    words
}

/// The text of a person who is still typing, ready to be searched: when the last word is not
/// finished and the checker would take it as a start (`logi` finds `login`), it gets a `*`. A
/// word that is finished (a space after it), a start that is too short, and a text that already
/// ends in `*` stay as they are. Whether the word may be a start is decided by
/// [`SearchWord::matches_by_start`], the one place that counts letters, so the star and the
/// checker of the search language never disagree.
#[must_use]
pub fn open_last_word(text: &str) -> String {
    if text.ends_with('*') {
        return text.to_owned();
    }
    let open = format!("{text}*");
    if split(&lowered(&open))
        .last()
        .is_some_and(SearchWord::matches_by_start)
    {
        open
    } else {
        text.to_owned()
    }
}

/// Every word of a lower-cased text in order, repeated ones too: a run of letters and digits with
/// the combining marks that belong to them. Everything else (spaces, hyphens, dots, `@`,
/// punctuation) separates words.
fn split(text: &str) -> Vec<SearchWord> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        // A combining mark belongs to the letter before it (a letter written as two characters);
        // one with no letter before it separates, like any other sign.
        if c.is_alphanumeric() || (!current.is_empty() && is_mark(c)) {
            current.push(c);
            if chars.peek().is_some() {
                continue;
            }
        }
        if !current.is_empty() {
            words.push(SearchWord {
                text: std::mem::take(&mut current),
                prefix: c == '*',
            });
        }
    }
    words
}

/// Whether the character is a combining mark: the blocks that hold the accents written after
/// their letter (U+0300 to U+036F and the extensions), and the marks that NFC leaves on a letter
/// of another script (the nukta of Devanagari, Bengali, Gurmukhi, Oriya and Kaithi, the sign of
/// Sinhala that two of its vowel signs end in, and the signs of Japanese). A table of the
/// general categories would add 14 KB to the search module of the browser (PERF-04).
fn is_mark(c: char) -> bool {
    matches!(
        c,
        '\u{300}'..='\u{36f}'
            | '\u{1ab0}'..='\u{1aff}'
            | '\u{1dc0}'..='\u{1dff}'
            | '\u{20d0}'..='\u{20ff}'
            | '\u{fe20}'..='\u{fe2f}'
            | '\u{93c}'
            | '\u{9bc}'
            | '\u{a3c}'
            | '\u{b3c}'
            | '\u{dca}'
            | '𑂺'
            | '\u{3099}'..='\u{309a}'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> Vec<String> {
        prepare(text)
    }

    #[test]
    fn a_last_word_of_one_letter_gets_no_star() {
        assert_eq!(open_last_word("login l"), "login l");
        assert_eq!(open_last_word("l"), "l");
    }

    #[test]
    fn a_last_word_of_two_letters_gets_a_star() {
        assert_eq!(open_last_word("lo"), "lo*");
        assert_eq!(open_last_word("fix logi"), "fix logi*");
    }

    #[test]
    fn a_finished_word_or_an_empty_text_gets_no_star() {
        assert_eq!(open_last_word("login "), "login ");
        assert_eq!(open_last_word("login*"), "login*");
        assert_eq!(open_last_word(""), "");
    }

    #[test]
    fn the_letters_are_counted_as_the_checker_counts_them() {
        // `ß` and `æ` are one letter each: letters are not changed into other letters.
        assert_eq!(open_last_word("ß"), "ß");
        assert_eq!(open_last_word("æ"), "æ");
        assert_eq!(open_last_word("ße"), "ße*");
        // An accent written as a character of its own belongs to the letter before it: `e` and
        // its accent are one letter, `e`, its accent and `t` are two, and a text that ends in the
        // accent of the second letter is a start as well.
        assert_eq!(open_last_word("e\u{301}"), "e\u{301}");
        assert_eq!(open_last_word("e\u{301}t"), "e\u{301}t*");
        assert_eq!(open_last_word("me\u{301}"), "me\u{301}*");
    }

    #[test]
    fn a_text_that_ends_in_a_star_keeps_one_star() {
        assert_eq!(open_last_word("login*"), "login*");
    }

    #[test]
    fn a_word_with_an_umlaut_counts_its_letters() {
        assert_eq!(open_last_word("ü"), "ü");
        assert_eq!(open_last_word("mü"), "mü*");
        assert_eq!(open_last_word("Müll"), "Müll*");
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

    /// What the checker decides is made on the text as typed, so it is the same with or without
    /// the NFC step (the browser has none).
    #[test]
    fn the_checker_counts_the_text_as_typed() {
        let starts = |text: &str| checked_words(text)[0].matches_by_start();
        assert!(starts("\u{1100}\u{1161}*"));
        assert!(!starts("\u{fb1d}*"));
        assert!(!starts("\u{958}*"));
        assert!(!starts("\u{304b}\u{3099}*"));
        let nine: Vec<String> = (1..=9).map(|n| format!("a{n}")).collect();
        let text = format!("{} m\u{fc} mu\u{308}", nine.join(" "));
        assert_eq!(checked_words(&text).len(), 11);
        // What is stored and looked up is NFC: the same two are one word.
        assert_eq!(search_words(&text).len(), 10);
        assert_eq!(letters("mu\u{308}ller"), 6);
    }

    /// NFC is applied to the whole text before it is split, so a letter that NFC joins or takes
    /// apart stays inside its word.
    #[test]
    fn the_whole_text_is_made_nfc_before_it_is_split() {
        assert_eq!(words("\u{304b}\u{3099}"), ["\u{304c}"]);
        assert_eq!(words("\u{304b}\u{3099}x"), ["\u{304c}x"]);
        // Devanagari: the letter with a dot is one letter written as one or as two characters.
        assert_eq!(words("\u{958}x"), ["\u{915}\u{93c}x"]);
        assert_eq!(words("\u{915}\u{93c}x"), ["\u{915}\u{93c}x"]);
        // A capital I with a dot above keeps its mark and stays in its word.
        assert_eq!(words("\u{130}stanbul"), ["i\u{307}stanbul"]);
    }

    /// A letter that NFC takes apart or puts together stays one word with the letter after it,
    /// whatever its script: the list is every letter there is, not a list written by hand. The
    /// checker, which does not put the text into NFC, counts the words of the same text the same.
    #[test]
    fn every_letter_stays_one_word_in_every_form() {
        let mut broken = Vec::new();
        let mut uncounted = Vec::new();
        for letter in ('\0'..=char::MAX).filter(|c| c.is_alphanumeric()) {
            let forms = [
                letter.to_string(),
                letter.to_string().nfd().collect::<String>(),
            ];
            for form in forms {
                let text = format!("{form}x");
                if words(&text).len() != 1 {
                    broken.push(format!("{:04X}", u32::from(letter)));
                    break;
                }
                if checked_words(&text).len() != 1 {
                    uncounted.push(format!("{:04X}", u32::from(letter)));
                    break;
                }
            }
        }
        assert!(broken.is_empty(), "these letters split a word: {broken:?}");
        assert!(
            uncounted.is_empty(),
            "the checker counts these letters as more than one word: {uncounted:?}"
        );
    }

    /// A word is stored whole or not at all: a run over the bound is left out, the words around it
    /// stay (a text never fails to be indexed because of one). The bound counts bytes, so a word of
    /// a script with three bytes for a letter is cut earlier than one of ASCII letters.
    #[test]
    fn a_word_over_the_bound_is_not_stored_and_the_others_are() {
        let ascii = "a".repeat(MAX_WORD_BYTES);
        assert_eq!(words(&format!("x {ascii} y")), ["x", ascii.as_str(), "y"]);
        let long_ascii = "a".repeat(MAX_WORD_BYTES + 1);
        assert_eq!(words(&format!("x {long_ascii} y")), ["x", "y"]);
        let long_cjk = "\u{4e2d}".repeat(MAX_WORD_BYTES / 3 + 1);
        assert!(long_cjk.len() > MAX_WORD_BYTES);
        assert_eq!(words(&format!("x {long_cjk} y")), ["x", "y"]);
        let cjk_fits = "\u{4e2d}".repeat(MAX_WORD_BYTES / 3);
        assert_eq!(words(&cjk_fits), [cjk_fits.as_str()]);
        assert_eq!(prepare_joined(&long_cjk), "");
    }

    /// The checker sees the long word (it refuses it); the lookup keeps it too, and it fits
    /// nowhere.
    #[test]
    fn a_word_over_the_bound_is_seen_by_the_checker_and_fits_nowhere() {
        let long = "A".repeat(MAX_WORD_BYTES + 1);
        let checked = checked_words(&format!("ok {long}"));
        assert_eq!(checked.len(), 2);
        assert!(!checked[0].is_too_long());
        assert!(checked[1].is_too_long());
        assert!(search_words(&long)[0].is_too_long());
    }

    /// The edge of the bound: the checker counts the word as typed, the lookup the word in NFC. A
    /// letter that NFC takes apart grows; this finds how much, and a word that passed the checker
    /// stays below the limit of PostgreSQL (2046 bytes) when it is looked up.
    #[test]
    fn nfc_cannot_grow_a_checked_word_beyond_what_a_database_takes() {
        const POSTGRES_LEXEME_BYTES: usize = 2046;
        let most = ('\0'..=char::MAX)
            .filter(|c| c.is_alphanumeric() || is_mark(*c))
            .map(|c| {
                let one = c.to_string();
                one.nfc().collect::<String>().len().div_ceil(one.len())
            })
            .max()
            .unwrap_or(1);
        assert!(
            MAX_WORD_BYTES * most <= POSTGRES_LEXEME_BYTES,
            "NFC grows a word {most} times"
        );
    }
}
