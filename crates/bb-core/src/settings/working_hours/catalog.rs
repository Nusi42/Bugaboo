//! The settings of working hours: one number for each working-time rule, whose defaults are the
//! numbers of the preset *Germany* (Working Hours Act §3 to §5), the preset itself, the keep period and *Totals
//! only*. Administrators change them on *Organization settings › Work › Working hours*. The rules
//! are read from these values in one place, `bb_core::working_time`.

use super::super::{
    Bool, ChangedBy, Choice, Format, Integer, Level, Setting, SettingDefinition, Text, settings,
};
use crate::working_time::Preset;

/// The choices of the preset: *Germany* or *No rules*. Written out like the modes of the issue
/// IDs (the registry is read as text by the tools that make the constants of the browser), and held
/// to the words of `Preset` by the assertions below.
const PRESET_CHOICES: &[Choice] = &[
    Choice::new("germany", Preset::Germany.name()),
    Choice::new("no-rules", Preset::NoRules.name()),
];

const _: () = {
    assert!(crate::words::same_text(Preset::Germany.code(), "germany"));
    assert!(crate::words::same_text(Preset::NoRules.code(), "no-rules"));
};

settings! {
    /// The preset the working-time rules are made from: *Germany* checks the numbers below,
    /// *No rules* checks nothing.
    WORKING_HOURS_PRESET: Text = {
        key: "working-hours.preset",
        name: crate::message::SETTING_WORKING_HOURS_PRESET_NAME,
        default: "germany",
        levels: [Organization],
        changed_by: Administrators,
        help: crate::message::SETTING_WORKING_HOURS_PRESET_HELP,
        lockable: false,
        format: Format::Choices(PRESET_CHOICES),
    }

    /// The most minutes of work in a day (Working Hours Act §3: 10 hours).
    WORKING_HOURS_DAY_MINUTES_MAX: Integer = {
        key: "working-hours.day-minutes-max",
        name: crate::message::SETTING_WORKING_HOURS_DAY_MINUTES_MAX_NAME,
        default: 600,
        levels: [Organization],
        changed_by: Administrators,
        help: crate::message::SETTING_WORKING_HOURS_DAY_MINUTES_MAX_HELP,
        lockable: false,
        bounds: (60, 1440),
        unit: Minutes,
    }

    /// After how many minutes of work the first break is due (Working Hours Act §4: more than 6 hours).
    WORKING_HOURS_FIRST_BREAK_AFTER_MINUTES: Integer = {
        key: "working-hours.first-break-after-minutes",
        name: crate::message::SETTING_WORKING_HOURS_FIRST_BREAK_AFTER_MINUTES_NAME,
        default: 360,
        levels: [Organization],
        changed_by: Administrators,
        help: crate::message::SETTING_WORKING_HOURS_FIRST_BREAK_AFTER_MINUTES_HELP,
        lockable: false,
        bounds: (60, 1440),
        unit: Minutes,
    }

    /// How many minutes of break are due after that (Working Hours Act §4: 30 minutes).
    WORKING_HOURS_FIRST_BREAK_MINUTES: Integer = {
        key: "working-hours.first-break-minutes",
        name: crate::message::SETTING_WORKING_HOURS_FIRST_BREAK_MINUTES_NAME,
        default: 30,
        levels: [Organization],
        changed_by: Administrators,
        help: crate::message::SETTING_WORKING_HOURS_FIRST_BREAK_MINUTES_HELP,
        lockable: false,
        bounds: (0, 240),
        unit: Minutes,
    }

    /// After how many minutes of work the longer break is due (Working Hours Act §4: more than 9 hours).
    WORKING_HOURS_SECOND_BREAK_AFTER_MINUTES: Integer = {
        key: "working-hours.second-break-after-minutes",
        name: crate::message::SETTING_WORKING_HOURS_SECOND_BREAK_AFTER_MINUTES_NAME,
        default: 540,
        levels: [Organization],
        changed_by: Administrators,
        help: crate::message::SETTING_WORKING_HOURS_SECOND_BREAK_AFTER_MINUTES_HELP,
        lockable: false,
        bounds: (60, 1440),
        unit: Minutes,
    }

    /// How many minutes of break are due after that (Working Hours Act §4: 45 minutes).
    WORKING_HOURS_SECOND_BREAK_MINUTES: Integer = {
        key: "working-hours.second-break-minutes",
        name: crate::message::SETTING_WORKING_HOURS_SECOND_BREAK_MINUTES_NAME,
        default: 45,
        levels: [Organization],
        changed_by: Administrators,
        help: crate::message::SETTING_WORKING_HOURS_SECOND_BREAK_MINUTES_HELP,
        lockable: false,
        bounds: (0, 240),
        unit: Minutes,
    }

    /// The shortest break that counts, in minutes: a shorter one counts neither as a break nor
    /// as an end of work in a row (Working Hours Act §4: 15 minutes).
    WORKING_HOURS_SHORTEST_BREAK_MINUTES: Integer = {
        key: "working-hours.shortest-break-minutes",
        name: crate::message::SETTING_WORKING_HOURS_SHORTEST_BREAK_MINUTES_NAME,
        default: 15,
        levels: [Organization],
        changed_by: Administrators,
        help: crate::message::SETTING_WORKING_HOURS_SHORTEST_BREAK_MINUTES_HELP,
        lockable: false,
        bounds: (1, 120),
        unit: Minutes,
    }

    /// The most minutes of work in a row without a break that counts (Working Hours Act §4: 6 hours).
    WORKING_HOURS_IN_A_ROW_MINUTES_MAX: Integer = {
        key: "working-hours.in-a-row-minutes-max",
        name: crate::message::SETTING_WORKING_HOURS_IN_A_ROW_MINUTES_MAX_NAME,
        default: 360,
        levels: [Organization],
        changed_by: Administrators,
        help: crate::message::SETTING_WORKING_HOURS_IN_A_ROW_MINUTES_MAX_HELP,
        lockable: false,
        bounds: (60, 1440),
        unit: Minutes,
    }

    /// The fewest minutes of rest between the end of one work day and the start of the next
    /// (Working Hours Act §5: 11 hours).
    WORKING_HOURS_REST_MINUTES_MIN: Integer = {
        key: "working-hours.rest-minutes-min",
        name: crate::message::SETTING_WORKING_HOURS_REST_MINUTES_MIN_NAME,
        default: 660,
        levels: [Organization],
        changed_by: Administrators,
        help: crate::message::SETTING_WORKING_HOURS_REST_MINUTES_MIN_HELP,
        lockable: false,
        bounds: (0, 1440),
        unit: Minutes,
    }

    /// How many whole years from its date a work day is kept at least (the keep period, ADR-0035,
    /// decision 7). With the preset *Germany* it is never less than two years (Working Hours Act §16 (2)).
    WORKING_HOURS_KEEP_YEARS: Integer = {
        key: "working-hours.keep-years",
        name: crate::message::SETTING_WORKING_HOURS_KEEP_YEARS_NAME,
        default: 2,
        levels: [Organization],
        changed_by: Administrators,
        help: crate::message::SETTING_WORKING_HOURS_KEEP_YEARS_HELP,
        lockable: false,
        bounds: (1, 30),
        unit: Years,
    }

    /// *Totals only*: *Working hours of others* shows each person's week total and nothing else,
    /// and other people's days cannot be changed while it is on (ADR-0035, decision 8).
    WORKING_HOURS_TOTALS_ONLY: Bool = {
        key: "working-hours.totals-only",
        name: crate::message::SETTING_WORKING_HOURS_TOTALS_ONLY_NAME,
        default: false,
        levels: [Organization],
        changed_by: Administrators,
        help: crate::message::SETTING_WORKING_HOURS_TOTALS_ONLY_HELP,
        lockable: false,
    }

    /// Whether a time entry offers *Billable*: off at the start; switching it off hides the box and
    /// keeps the values (ADR-0035, decision 1). It stands with the working-time settings because they
    /// are one page, *Working hours*, in this order (UX §5.1).
    TIME_BILLABLE: Bool = {
        key: "time.billable",
        name: crate::message::SETTING_TIME_BILLABLE_NAME,
        default: false,
        levels: [Organization],
        changed_by: Administrators,
        help: crate::message::SETTING_TIME_BILLABLE_HELP,
        lockable: false,
    }

    /// Whether the person is told in the app when somebody else changes one of their work days
    /// (ADR-0035, decision 7).
    NOTIFICATIONS_WORK_DAY_CHANGED_IN_APP: Bool = {
        key: "notifications.work-day-changed-in-app",
        name: crate::message::SETTING_NOTIFICATIONS_WORK_DAY_CHANGED_IN_APP_NAME,
        default: true,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: crate::message::SETTING_NOTIFICATIONS_WORK_DAY_CHANGED_IN_APP_HELP,
        lockable: true,
    }

    /// Whether the person is told by e-mail when somebody else changes one of their work days.
    NOTIFICATIONS_WORK_DAY_CHANGED_BY_MAIL: Bool = {
        key: "notifications.work-day-changed-by-mail",
        name: crate::message::SETTING_NOTIFICATIONS_WORK_DAY_CHANGED_BY_MAIL_NAME,
        default: true,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: crate::message::SETTING_NOTIFICATIONS_WORK_DAY_CHANGED_BY_MAIL_HELP,
        lockable: true,
    }
}
