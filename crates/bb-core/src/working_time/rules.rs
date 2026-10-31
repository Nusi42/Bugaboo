//! The working-time rules as data (ADR-0035, decision 6): the organization's preset and its numbers,
//! read from the settings of the registry, where the defaults are the numbers of the preset
//! *Germany* (Working Hours Act §3 to §5). Nothing else holds a copy of a number.

use crate::settings::Layers;
use crate::settings::working_hours::{
    WORKING_HOURS_DAY_MINUTES_MAX, WORKING_HOURS_FIRST_BREAK_AFTER_MINUTES,
    WORKING_HOURS_FIRST_BREAK_MINUTES, WORKING_HOURS_IN_A_ROW_MINUTES_MAX,
    WORKING_HOURS_KEEP_YEARS, WORKING_HOURS_PRESET, WORKING_HOURS_REST_MINUTES_MIN,
    WORKING_HOURS_SECOND_BREAK_AFTER_MINUTES, WORKING_HOURS_SECOND_BREAK_MINUTES,
    WORKING_HOURS_SHORTEST_BREAK_MINUTES, WORKING_HOURS_TOTALS_ONLY,
};
use crate::settings::{Integer, Setting};
use crate::time::Minutes;

words! {
    /// The ready sets of working-time rules an organization starts from.
    pub enum Preset {
        /// The German Working Hours Act (the Working Hours Act): the numbers of the settings are checked.
        Germany => "germany",
        /// Nothing is checked.
        NoRules => "no-rules",
    }
    messages name = "working-time.preset."
}

/// The longest day when the organization checks no rules: what the long-timer warning uses then
/// (ADR-0035, decision 3).
pub const NO_RULES_LONGEST_DAY: Minutes = Minutes::new(600);

/// The fewest whole years a work day is kept with the preset *Germany* (Working Hours Act §16 (2)): the keep
/// period cannot be set below it then (ADR-0035, decision 7).
pub const GERMANY_KEEP_YEARS_LEAST: i64 = 2;

/// A break that is due after some time of work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BreakRule {
    /// The work after which it is due: it is due when the day has more.
    pub after: Minutes,
    /// How long the breaks of the day are together at least then.
    pub needed: Minutes,
}

/// The organization's working-time rules, each a number of the registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rules {
    /// The most work in a day.
    pub day_most: Minutes,
    /// The break due after the first stretch of a day.
    pub first_break: BreakRule,
    /// The longer break due after a longer day.
    pub second_break: BreakRule,
    /// The shortest break that counts: a shorter one is neither a break of the rules nor the end
    /// of work in a row.
    pub shortest_break: Minutes,
    /// The most work in a row without a break that counts.
    pub in_a_row_most: Minutes,
    /// The least rest between the end of a day and the start of the next.
    pub rest_least: Minutes,
}

/// The value of a number of the rules: the bounds of the registry keep it a whole number of
/// minutes; a stored value out of them counts as none.
fn minutes(layers: &Layers, setting: &Setting<Integer>) -> Minutes {
    Minutes::try_from(layers.number(setting)).unwrap_or_default()
}

/// The preset the organization chose; a stored word that is none counts as the default.
#[must_use]
pub fn preset_of(layers: &Layers) -> Preset {
    let chosen = |word: String| Preset::from_code(&word);
    chosen(layers.text(&WORKING_HOURS_PRESET))
        .or_else(|| WORKING_HOURS_PRESET.default_value().ok().and_then(chosen))
        // The default of the registry is one of the presets: this is never reached.
        .unwrap_or(Preset::Germany)
}

impl Rules {
    /// The rules in force for the organization whose settings `layers` are; `None` with the
    /// preset *No rules*, which checks nothing.
    #[must_use]
    pub fn of(layers: &Layers) -> Option<Self> {
        match preset_of(layers) {
            Preset::NoRules => None,
            Preset::Germany => Some(Self {
                day_most: minutes(layers, &WORKING_HOURS_DAY_MINUTES_MAX),
                first_break: BreakRule {
                    after: minutes(layers, &WORKING_HOURS_FIRST_BREAK_AFTER_MINUTES),
                    needed: minutes(layers, &WORKING_HOURS_FIRST_BREAK_MINUTES),
                },
                second_break: BreakRule {
                    after: minutes(layers, &WORKING_HOURS_SECOND_BREAK_AFTER_MINUTES),
                    needed: minutes(layers, &WORKING_HOURS_SECOND_BREAK_MINUTES),
                },
                shortest_break: minutes(layers, &WORKING_HOURS_SHORTEST_BREAK_MINUTES),
                in_a_row_most: minutes(layers, &WORKING_HOURS_IN_A_ROW_MINUTES_MAX),
                rest_least: minutes(layers, &WORKING_HOURS_REST_MINUTES_MIN),
            }),
        }
    }

    /// The break due after `worked`: the longer one past its time, else the first one past its
    /// time, else none.
    #[must_use]
    pub fn break_due(&self, worked: Minutes) -> Option<BreakRule> {
        [self.second_break, self.first_break]
            .into_iter()
            .find(|rule| worked > rule.after && rule.needed > Minutes::default())
    }
}

/// The longest day of the organization's rules: what a timer may run before it warns (ADR-0035,
/// decision 3); [`NO_RULES_LONGEST_DAY`] with *No rules*.
#[must_use]
pub fn longest_day(layers: &Layers) -> Minutes {
    Rules::of(layers).map_or(NO_RULES_LONGEST_DAY, |rules| rules.day_most)
}

/// Whether a keep period of `years` may be set while the preset is `preset`: never below
/// [`GERMANY_KEEP_YEARS_LEAST`] with *Germany*.
#[must_use]
pub fn keep_years_allowed(preset: Preset, years: i64) -> bool {
    preset != Preset::Germany || years >= GERMANY_KEEP_YEARS_LEAST
}

/// The keep period that counts, in whole years: the setting, and with the preset *Germany* never
/// less than the law asks, also when the number was set under *No rules* before.
#[must_use]
pub fn keep_years(layers: &Layers) -> i64 {
    let years = layers.number(&WORKING_HOURS_KEEP_YEARS);
    match preset_of(layers) {
        Preset::Germany => years.max(GERMANY_KEEP_YEARS_LEAST),
        Preset::NoRules => years,
    }
}

/// Whether *Totals only* is on (ADR-0035, decision 8).
#[must_use]
pub fn totals_only(layers: &Layers) -> bool {
    layers.yes(&WORKING_HOURS_TOTALS_ONLY)
}
