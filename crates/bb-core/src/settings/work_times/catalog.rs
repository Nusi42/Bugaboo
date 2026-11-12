//! The settings of work times: *Holidays*, the public holidays that count for a person, and the
//! two switches of the notification *Your work times were changed*. *Holidays* is not part of
//! *Region*, which is about how dates and numbers are written. The holidays are worked out in
//! one place, `bb_core::holidays`.

use super::super::{
    Bool, ChangedBy, Choice, Format, Level, Setting, SettingDefinition, Text, settings,
};
use crate::holidays::Region;

/// The choices of *Holidays*: *None*, *Germany* or one of the 16 states. Written out like the other
/// lists of choices (the registry is read as text by the tools that make the constants of the
/// browser), and held to the words of `Region` by the assertions below.
const HOLIDAYS_CHOICES: &[Choice] = &[
    Choice::new("none", Region::None.name()),
    Choice::new("de", Region::Germany.name()),
    Choice::new("de-bw", Region::BadenWuerttemberg.name()),
    Choice::new("de-by", Region::Bavaria.name()),
    Choice::new("de-be", Region::Berlin.name()),
    Choice::new("de-bb", Region::Brandenburg.name()),
    Choice::new("de-hb", Region::Bremen.name()),
    Choice::new("de-hh", Region::Hamburg.name()),
    Choice::new("de-he", Region::Hesse.name()),
    Choice::new("de-mv", Region::MecklenburgVorpommern.name()),
    Choice::new("de-ni", Region::LowerSaxony.name()),
    Choice::new("de-nw", Region::NorthRhineWestphalia.name()),
    Choice::new("de-rp", Region::RhinelandPalatinate.name()),
    Choice::new("de-sl", Region::Saarland.name()),
    Choice::new("de-sn", Region::Saxony.name()),
    Choice::new("de-st", Region::SaxonyAnhalt.name()),
    Choice::new("de-sh", Region::SchleswigHolstein.name()),
    Choice::new("de-th", Region::Thuringia.name()),
];

const _: () = {
    assert!(HOLIDAYS_CHOICES.len() == Region::ALL.len());
    let mut at = 0;
    while at < HOLIDAYS_CHOICES.len() {
        assert!(crate::words::same_text(
            HOLIDAYS_CHOICES[at].value(),
            Region::ALL[at].code()
        ));
        at += 1;
    }
};

settings! {
    /// Which public holidays count for a person: *None*, *Germany* (the days every state has) or
    /// one of the 16 states. The organization's value is the start of everybody; each person
    /// chooses theirs in *My settings › Work times*.
    WORK_TIMES_HOLIDAYS: Text = {
        key: "work-times.holidays",
        name: crate::message::SETTING_WORK_TIMES_HOLIDAYS_NAME,
        default: "de",
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: crate::message::SETTING_WORK_TIMES_HOLIDAYS_HELP,
        lockable: false,
        format: Format::Choices(HOLIDAYS_CHOICES),
    }

    /// Whether the person is told in the app when somebody else enters or changes an away time,
    /// their usual work days or their work hours (ADR-0033, amendment M8).
    NOTIFICATIONS_WORK_TIMES_CHANGED_IN_APP: Bool = {
        key: "notifications.work-times-changed-in-app",
        name: crate::message::SETTING_NOTIFICATIONS_WORK_TIMES_CHANGED_IN_APP_NAME,
        default: true,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: crate::message::SETTING_NOTIFICATIONS_WORK_TIMES_CHANGED_IN_APP_HELP,
        lockable: true,
    }

    /// Whether the person is told by e-mail when somebody else changes their work times.
    NOTIFICATIONS_WORK_TIMES_CHANGED_BY_MAIL: Bool = {
        key: "notifications.work-times-changed-by-mail",
        name: crate::message::SETTING_NOTIFICATIONS_WORK_TIMES_CHANGED_BY_MAIL_NAME,
        default: false,
        levels: [Organization, Person],
        changed_by: EachPerson,
        help: crate::message::SETTING_NOTIFICATIONS_WORK_TIMES_CHANGED_BY_MAIL_HELP,
        lockable: true,
    }
}
