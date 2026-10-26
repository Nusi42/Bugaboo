//! The one rule that makes a text comparable for the full-text search (ADR-0016, amendment M5):
//! lower case, accents removed, split into words. The index and the words of a search both go
//! through [`prepare`] / [`search_words`], so a word is found exactly when it was indexed.

/// A word of a search. A word typed with a `*` at its end matches every word that starts with it.
/// Only [`search_words`] makes one, so every word is letters and digits of the one rule and
/// nothing else: it cannot carry the syntax of a database query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchWord {
    text: String,
    prefix: bool,
}

impl SearchWord {
    /// The prepared word (lower case, no accents).
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

/// The words of `text`, in order: lower case, accents removed, a word is a run of letters and
/// digits, everything else (spaces, hyphens, dots, `@`, punctuation) separates words. A word that
/// comes twice comes twice: this is what the index keeps.
#[must_use]
pub fn prepare(text: &str) -> Vec<String> {
    words_of(text).into_iter().map(|word| word.text).collect()
}

/// The words of a search text, as [`prepare`] makes them, each with whether it ended in `*`. A
/// word that is typed twice (the same word, both times whole or both times by its start) is
/// returned once, so a repeated word costs the database nothing.
#[must_use]
pub fn search_words(text: &str) -> Vec<SearchWord> {
    let mut words: Vec<SearchWord> = Vec::new();
    for word in words_of(text) {
        if !words.contains(&word) {
            words.push(word);
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
        if c.is_alphanumeric() {
            for lower in c.to_lowercase() {
                fold(lower, &mut current);
            }
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

/// The text of a words list as it is stored in the index: the words, one space between.
#[must_use]
pub fn prepare_joined(text: &str) -> String {
    prepare(text).join(" ")
}

/// Appends `c` (already lower case) without its accent. `ß` is `ss`, `æ` is `ae`, `œ` is `oe`.
fn fold(c: char, out: &mut String) {
    let plain = match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => "a",
        'ç' | 'ć' | 'ĉ' | 'ċ' | 'č' => "c",
        'ď' | 'đ' => "d",
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' => "e",
        'ĝ' | 'ğ' | 'ġ' | 'ģ' => "g",
        'ĥ' | 'ħ' => "h",
        'ì' | 'í' | 'î' | 'ï' | 'ĩ' | 'ī' | 'ĭ' | 'į' | 'ı' => "i",
        'ĵ' => "j",
        'ķ' => "k",
        'ĺ' | 'ļ' | 'ľ' | 'ŀ' | 'ł' => "l",
        'ñ' | 'ń' | 'ņ' | 'ň' => "n",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ŏ' | 'ő' => "o",
        'ŕ' | 'ŗ' | 'ř' => "r",
        'ś' | 'ŝ' | 'ş' | 'š' => "s",
        'ţ' | 'ť' | 'ŧ' => "t",
        'ù' | 'ú' | 'û' | 'ü' | 'ũ' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' => "u",
        'ŵ' => "w",
        'ý' | 'ÿ' | 'ŷ' => "y",
        'ź' | 'ż' | 'ž' => "z",
        'ß' => "ss",
        'æ' => "ae",
        'œ' => "oe",
        // A combining accent after a letter written as two characters.
        '\u{0300}'..='\u{036f}' => "",
        other => {
            out.push(other);
            return;
        }
    };
    out.push_str(plain);
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
    fn german_umlauts_and_sharp_s_lose_their_accent() {
        assert_eq!(
            words("Größe Äpfel über Straße"),
            ["grosse", "apfel", "uber", "strasse"]
        );
        assert_eq!(words("GROSSE"), words("Größe"));
    }

    #[test]
    fn english_and_french_accents_are_removed() {
        assert_eq!(words("Café naïve résumé"), ["cafe", "naive", "resume"]);
        assert_eq!(words("Cafe\u{0301}"), ["cafe"]);
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
        assert_eq!(prepare_joined("Größe, BB-1"), "grosse bb 1");
        let searched: Vec<String> = search_words("GRÖSSE").into_iter().map(|w| w.text).collect();
        assert_eq!(searched, words("Größe"));
    }
}
