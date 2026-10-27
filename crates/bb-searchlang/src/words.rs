//! The words of the search language, spelled once. The parser reads them, the printer writes them
//! and the suggestions offer them from this one table (ADR-0005, amendment M5): a word that the
//! language gains is added here and nowhere else.

use bb_core::time::MINUTES_PER_HOUR;

use crate::syntax::{Direction, TimeWord, Unit};

/// Makes the enum of the words, with the spelling of each (lower case, as the printer writes it).
macro_rules! keywords {
    ($($variant:ident => $text:literal),+ $(,)?) => {
        /// A word that has a meaning in the language. A text that is only one of these words must
        /// be quoted to be a value or the name of a field.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub(crate) enum Keyword {
            $(#[doc = $text] $variant),+
        }

        impl Keyword {
            /// Every keyword.
            pub(crate) const ALL: &'static [Self] = &[$(Self::$variant),+];

            /// The canonical spelling.
            #[must_use]
            pub(crate) const fn text(self) -> &'static str {
                match self {
                    $(Self::$variant => $text),+
                }
            }
        }
    };
}

keywords! {
    And => "and",
    Or => "or",
    Not => "not",
    Is => "is",
    In => "in",
    Contains => "contains",
    Empty => "empty",
    Member => "member",
    Of => "of",
    Within => "within",
    Last => "last",
    This => "this",
    Today => "today",
    Yesterday => "yesterday",
    Me => "me",
    Sort => "sort",
    By => "by",
    Under => "under",
    Blocks => "blocks",
    Blocked => "blocked",
    Connected => "connected",
    To => "to",
    Step => "step",
    Steps => "steps",
    Day => "day",
    Days => "days",
    Week => "week",
    Weeks => "weeks",
    Month => "month",
    Months => "months",
    Minute => "minute",
    Minutes => "minutes",
    Hour => "hour",
    Hours => "hours",
    First => "first",
    Highest => "highest",
    Lowest => "lowest",
    Newest => "newest",
    Oldest => "oldest",
}

impl Keyword {
    /// Whether a bare word that spells this keyword cannot be a value: it joins, negates, or starts
    /// a time word, so it has to be quoted to be a value. The other keywords (`highest`, `blocked`,
    /// `first`) mean something only at their own place and stay free as values, because a status
    /// *Blocked* and a priority *highest* are what people search for.
    #[must_use]
    pub(crate) const fn reserved_as_value(self) -> bool {
        matches!(
            self,
            Self::And
                | Self::Or
                | Self::Not
                | Self::Is
                | Self::In
                | Self::Contains
                | Self::Empty
                | Self::Member
                | Self::Within
                | Self::Last
                | Self::This
                | Self::Today
                | Self::Yesterday
                | Self::Me
                | Self::Sort
        )
    }

    /// The keyword that `word` spells, in any case.
    #[must_use]
    pub(crate) fn of(word: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|keyword| keyword.text().eq_ignore_ascii_case(word))
    }
}

/// One word of a phrase of the language.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Part {
    /// A keyword.
    Key(Keyword),
    /// A single letter that is no keyword (`A to Z`).
    Letter(char),
}

impl Part {
    /// How the printer writes the part.
    #[must_use]
    pub(crate) fn text(self) -> String {
        match self {
            Self::Key(keyword) => keyword.text().to_owned(),
            Self::Letter(letter) => letter.to_string(),
        }
    }

    /// Whether `word` is this part, in any case.
    #[must_use]
    pub(crate) fn is(self, word: &str) -> bool {
        self.text().eq_ignore_ascii_case(word)
    }
}

/// The words of the time words: `today`, `this week`.
#[must_use]
pub(crate) fn time_phrase(word: TimeWord) -> &'static [Part] {
    use Keyword::{Last, Month, This, Today, Week, Yesterday};
    match word {
        TimeWord::Today => &[Part::Key(Today)],
        TimeWord::Yesterday => &[Part::Key(Yesterday)],
        TimeWord::ThisWeek => &[Part::Key(This), Part::Key(Week)],
        TimeWord::LastWeek => &[Part::Key(Last), Part::Key(Week)],
        TimeWord::ThisMonth => &[Part::Key(This), Part::Key(Month)],
    }
}

/// The words of a direction of `sort by`: `highest first`, `A to Z`.
#[must_use]
pub(crate) fn direction_phrase(direction: Direction) -> &'static [Part] {
    use Keyword::{First, Highest, Lowest, Newest, Oldest, To};
    match direction {
        Direction::HighestFirst => &[Part::Key(Highest), Part::Key(First)],
        Direction::LowestFirst => &[Part::Key(Lowest), Part::Key(First)],
        Direction::NewestFirst => &[Part::Key(Newest), Part::Key(First)],
        Direction::OldestFirst => &[Part::Key(Oldest), Part::Key(First)],
        Direction::AToZ => &[Part::Letter('A'), Part::Key(To), Part::Letter('Z')],
        Direction::ZToA => &[Part::Letter('Z'), Part::Key(To), Part::Letter('A')],
    }
}

/// The keyword for a unit, in the form that fits the count (`1 day`, `2 days`).
#[must_use]
pub(crate) const fn unit_word(unit: Unit, count: u32) -> Keyword {
    match (unit, count == 1) {
        (Unit::Day, true) => Keyword::Day,
        (Unit::Day, false) => Keyword::Days,
        (Unit::Week, true) => Keyword::Week,
        (Unit::Week, false) => Keyword::Weeks,
        (Unit::Month, true) => Keyword::Month,
        (Unit::Month, false) => Keyword::Months,
    }
}

/// The unit that a keyword names, singular or plural.
#[must_use]
pub(crate) const fn unit_of(keyword: Keyword) -> Option<Unit> {
    match keyword {
        Keyword::Day | Keyword::Days => Some(Unit::Day),
        Keyword::Week | Keyword::Weeks => Some(Unit::Week),
        Keyword::Month | Keyword::Months => Some(Unit::Month),
        _ => None,
    }
}

/// The keyword for the word `steps` after a count.
#[must_use]
pub(crate) const fn steps_word(count: u32) -> Keyword {
    if count == 1 {
        Keyword::Step
    } else {
        Keyword::Steps
    }
}

impl TimeWord {
    /// How the printer writes it.
    #[must_use]
    pub(crate) fn text(self) -> String {
        phrase_text(time_phrase(self))
    }
}

impl Direction {
    /// How the printer writes it.
    #[must_use]
    pub(crate) fn text(self) -> String {
        phrase_text(direction_phrase(self))
    }
}

/// The words of a phrase, one space apart.
#[must_use]
pub(crate) fn phrase_text(phrase: &[Part]) -> String {
    phrase
        .iter()
        .map(|part| part.text())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether `words` are the start of `phrase`, in any case.
#[must_use]
pub(crate) fn starts_phrase(phrase: &[Part], words: &[&str]) -> bool {
    words.len() <= phrase.len() && phrase.iter().zip(words).all(|(part, word)| part.is(word))
}

/// A unit of a length of time (`1 hour 30 minutes`), carrying the minutes it stands for, so no
/// other place writes the number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TimeUnit {
    /// A minute.
    Minute,
    /// An hour.
    Hour,
}

impl TimeUnit {
    /// The units, the longest first (the order a length of time is written in).
    pub(crate) const LONGEST_FIRST: [Self; 2] = [Self::Hour, Self::Minute];

    /// How many minutes one of the unit is.
    #[must_use]
    pub(crate) const fn minutes(self) -> u32 {
        match self {
            Self::Minute => 1,
            Self::Hour => MINUTES_PER_HOUR,
        }
    }

    /// The keyword for the unit, in the form that fits the count (`1 hour`, `2 hours`).
    #[must_use]
    pub(crate) const fn word(self, count: u32) -> Keyword {
        match (self, count == 1) {
            (Self::Hour, true) => Keyword::Hour,
            (Self::Hour, false) => Keyword::Hours,
            (Self::Minute, true) => Keyword::Minute,
            (Self::Minute, false) => Keyword::Minutes,
        }
    }

    /// The unit that a keyword names, singular or plural.
    #[must_use]
    pub(crate) const fn of(keyword: Keyword) -> Option<Self> {
        match keyword {
            Keyword::Minute | Keyword::Minutes => Some(Self::Minute),
            Keyword::Hour | Keyword::Hours => Some(Self::Hour),
            _ => None,
        }
    }
}

/// The words of the units of a length of time for the placeholders of a message about it.
#[must_use]
pub(crate) const fn duration_words() -> [(&'static str, &'static str); 3] {
    [
        ("hour", Keyword::Hour.text()),
        ("hours", Keyword::Hours.text()),
        ("minutes", Keyword::Minutes.text()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_keyword_is_found_in_any_case_and_a_plain_word_is_not() {
        assert_eq!(Keyword::of("AND"), Some(Keyword::And));
        assert_eq!(Keyword::of("Blocked"), Some(Keyword::Blocked));
        assert_eq!(Keyword::of("priority"), None);
    }

    #[test]
    fn phrases_are_written_with_one_space_and_the_letters_of_the_directions() {
        assert_eq!(TimeWord::ThisWeek.text(), "this week");
        assert_eq!(Direction::AToZ.text(), "A to Z");
        assert_eq!(Direction::HighestFirst.text(), "highest first");
        assert!(starts_phrase(
            direction_phrase(Direction::ZToA),
            &["z", "TO"]
        ));
        assert!(!starts_phrase(direction_phrase(Direction::ZToA), &["a"]));
    }

    #[test]
    fn units_follow_the_count() {
        assert_eq!(unit_word(Unit::Day, 1), Keyword::Day);
        assert_eq!(unit_word(Unit::Month, 7), Keyword::Months);
        assert_eq!(unit_of(Keyword::Weeks), Some(Unit::Week));
        assert_eq!(unit_of(Keyword::And), None);
        assert_eq!(steps_word(1), Keyword::Step);
    }
}
