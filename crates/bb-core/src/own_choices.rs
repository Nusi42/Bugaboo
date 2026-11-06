//! The number of a choice of an own field and the one rule that orders the choices (TIME-13,
//! CF-08): a choice is written "4100 Product development", and the choices of a field with numbers
//! are listed by the number.

use crate::names::name_key;
use crate::own_fields::{CHOICE_NUMBER_DIGITS_MAX, FieldChoice};

/// Whether a text is a number a choice may have: digits only, one to [`CHOICE_NUMBER_DIGITS_MAX`] of
/// them. The digits are kept as typed, so `0410` stays `0410`.
#[must_use]
pub fn is_choice_number(text: &str) -> bool {
    (1..=CHOICE_NUMBER_DIGITS_MAX).contains(&text.len())
        && text.bytes().all(|byte| byte.is_ascii_digit())
}

/// Whether another choice of the field is found by a search key (`name_key`) that `candidate` is
/// found by: by its name, its number or the two written together. A search words a choice by any
/// of them, so no two choices of a field may share one, whichever way they are written. The one
/// rule for adding, changing and making choices.
#[must_use]
pub fn keys_clash(choices: &[FieldChoice], candidate: &FieldChoice) -> bool {
    let keys = candidate.search_keys();
    choices.iter().any(|found| {
        found.id != candidate.id && found.search_keys().iter().any(|key| keys.contains(key))
    })
}

/// How a choice is written wherever it is shown as plain text: its number, a space and its name
/// (`4100 Product development`), or the name alone. The one place that joins them.
#[must_use]
pub fn choice_shown_as(number: Option<&str>, name: &str) -> String {
    number.map_or_else(|| name.to_owned(), |number| format!("{number} {name}"))
}

impl FieldChoice {
    /// The choice as plain text (see [`choice_shown_as`]).
    #[must_use]
    pub fn shown_as(&self) -> String {
        choice_shown_as(self.number.as_deref(), &self.name)
    }

    /// The number read as a whole number (`0410` is 410): what the order of the choices and the
    /// comparison of a search go by. None for a choice without a number.
    #[must_use]
    pub fn number_value(&self) -> Option<u64> {
        self.number
            .as_deref()
            .map(|number| number.parse::<u64>().unwrap_or(u64::MAX))
    }

    /// The words the search language reads for the choice: its number, its name, and the two
    /// written together as a screen shows them (`4100`, `Product development`,
    /// `4100 Product development`). A choice without a number has its name only.
    #[must_use]
    pub fn search_words(&self) -> Vec<String> {
        match &self.number {
            Some(number) => vec![number.clone(), self.name.clone(), self.shown_as()],
            None => vec![self.name.clone()],
        }
    }

    /// The keys (`name_key`) by which a search finds the choice: those of its search words.
    #[must_use]
    pub fn search_keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = Vec::new();
        for word in self.search_words() {
            let key = name_key(&word);
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
        keys
    }
}

/// Puts the choices of a field in their order, the one rule for every place that lists them: the
/// choices with a number first, by the digits read as a whole number (and, for two numbers that read
/// the same, `0410` before `410`, by their text); then the others in the order the administrator
/// set. A field whose choices have no numbers keeps that order.
pub fn order_choices(choices: &mut [FieldChoice]) {
    choices.sort_by_key(|choice| {
        (
            choice.number.is_none(),
            choice.number_value(),
            choice.number.clone(),
            choice.position,
            choice.id,
        )
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::{ChoiceId, FieldId};
    use crate::own_fields::ChoiceColor;

    fn choice(
        source: &mut crate::id::SequentialIds,
        name: &str,
        number: Option<&str>,
        position: i64,
    ) -> FieldChoice {
        FieldChoice {
            id: ChoiceId::generate(source).unwrap(),
            field: FieldId::generate(source).unwrap(),
            name: name.to_owned(),
            number: number.map(str::to_owned),
            color: ChoiceColor::Cat1,
            icon: None,
            position,
            retired_at: None,
        }
    }

    #[test]
    fn time_13_a_choice_is_written_with_its_number_in_front_of_its_name() {
        let mut source = crate::id::SequentialIds::default();
        let numbered = choice(&mut source, "Product development", Some("4100"), 0);
        assert_eq!(numbered.shown_as(), "4100 Product development");
        assert_eq!(choice_shown_as(None, "Overhead"), "Overhead");
        // A search finds it by its number, its name and the way it is written.
        assert_eq!(
            numbered.search_words(),
            ["4100", "Product development", "4100 Product development"]
        );
        assert_eq!(
            numbered.search_keys(),
            ["4100", "product development", "4100 product development"]
        );
    }

    #[test]
    fn time_13_choices_are_ordered_by_number_then_by_the_order_the_administrator_set() {
        let mut source = crate::id::SequentialIds::default();
        let mut choices = vec![
            choice(&mut source, "Overhead", None, 0),
            choice(&mut source, "Sales", Some("5200"), 1),
            choice(&mut source, "Other", None, 2),
            choice(&mut source, "Product development", Some("4100"), 3),
        ];
        order_choices(&mut choices);
        let names: Vec<&str> = choices.iter().map(|each| each.name.as_str()).collect();
        assert_eq!(names, ["Product development", "Sales", "Overhead", "Other"]);
        // Without any number the administrator's order stays.
        for each in &mut choices {
            each.number = None;
        }
        order_choices(&mut choices);
        let names: Vec<&str> = choices.iter().map(|each| each.name.as_str()).collect();
        assert_eq!(names, ["Overhead", "Sales", "Other", "Product development"]);
    }

    #[test]
    fn time_13_two_choices_found_by_the_same_key_clash_whichever_way_they_are_written() {
        let mut source = crate::id::SequentialIds::default();
        let sales = choice(&mut source, "Sales", Some("5200"), 0);
        let stored = vec![sales.clone()];
        let clashes = |source: &mut crate::id::SequentialIds, name: &str, number| {
            keys_clash(&stored, &choice(source, name, number, 9))
        };
        // A name that is another choice's number, or the two written together, clashes ...
        assert!(clashes(&mut source, "5200", None));
        assert!(clashes(&mut source, "5200 sales", None));
        assert!(clashes(&mut source, "  SALES ", None));
        // ... and so does a number that is another choice's name.
        let named = vec![choice(&mut source, "4100", None, 1)];
        assert!(keys_clash(
            &named,
            &choice(&mut source, "Product", Some("4100"), 2)
        ));
        // The same in the other direction: the name that is written like the two together.
        let together = vec![choice(&mut source, "4100 Product development", None, 3)];
        let numbered = choice(&mut source, "Product development", Some("4100"), 4);
        assert!(keys_clash(&together, &numbered));
        assert!(keys_clash(
            &[numbered],
            &choice(&mut source, "4100 Product development", None, 5)
        ));
        // A choice does not clash with itself, nor with others that share nothing.
        assert!(!keys_clash(&stored, &sales));
        assert!(!clashes(&mut source, "Marketing", Some("5300")));
    }

    #[test]
    fn time_13_a_number_is_digits_kept_as_typed_up_to_nine() {
        for good in ["4100", "0410", "0", "999999999"] {
            assert!(is_choice_number(good), "{good}");
        }
        for bad in [
            "",
            " 41",
            "41 ",
            "-1",
            "4.1",
            "K-100",
            "1234567890",
            "\u{663}",
        ] {
            assert!(!is_choice_number(bad), "{bad}");
        }
        let mut source = crate::id::SequentialIds::default();
        let kept = choice(&mut source, "Overhead", Some("0410"), 0);
        assert_eq!(kept.shown_as(), "0410 Overhead");
    }

    #[test]
    fn time_13_numbers_sort_by_their_value_and_then_by_their_text() {
        let mut source = crate::id::SequentialIds::default();
        let mut choices = vec![
            choice(&mut source, "Ten", Some("10"), 0),
            choice(&mut source, "Nine", Some("9"), 1),
            choice(&mut source, "Plain", Some("410"), 2),
            choice(&mut source, "Padded", Some("0410"), 3),
            choice(&mut source, "None", None, 4),
        ];
        order_choices(&mut choices);
        let names: Vec<&str> = choices.iter().map(|each| each.name.as_str()).collect();
        // 9 before 10 (not "10" before "9"); 0410 before 410 once and for all.
        assert_eq!(names, ["Nine", "Ten", "Padded", "Plain", "None"]);
    }

    #[test]
    fn time_13_the_check_of_a_number_and_the_pattern_of_the_api_agree() {
        use crate::own_fields::{CHOICE_NUMBER_DIGITS_MAX, CHOICE_NUMBER_PATTERN};
        // The pattern is the one shape `^[0-9]{1,n}$` with the limit the check uses.
        assert_eq!(
            CHOICE_NUMBER_PATTERN,
            format!("^[0-9]{{1,{CHOICE_NUMBER_DIGITS_MAX}}}$")
        );
        let longest = "9".repeat(CHOICE_NUMBER_DIGITS_MAX);
        let too_long = "9".repeat(CHOICE_NUMBER_DIGITS_MAX + 1);
        for (text, fits) in [
            ("0", true),
            ("0410", true),
            (longest.as_str(), true),
            (too_long.as_str(), false),
            ("", false),
            ("41 0", false),
            ("-1", false),
            ("4a", false),
        ] {
            assert_eq!(is_choice_number(text), fits, "{text:?}");
        }
    }
}
