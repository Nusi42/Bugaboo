//! *E-mail me this search* (QRY-07, ADR-0033, amendment 2026-11-24): a saved search that its
//! person gets by e-mail on the weekdays and at the whole hour they chose, in their own time zone.
//!
//! What is kept is the choice and the time of the last e-mail, never the issues. When the next
//! e-mail is due, and how far into its hour it goes, is worked out here and nowhere else, so the
//! window, the job that finds the due ones and the tests agree.

use jiff::civil::{Date, Weekday};
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};

use crate::id::{SavedSearchId, SearchMailId, UserId};
use crate::personal::Entity;

/// The most e-mailed searches one person has: the 21st is refused, counted in the change that
/// writes it.
pub const SEARCH_MAILS_MAX: usize = 20;

/// The most issues one e-mail lists; the rest are counted ("and 120 more").
pub const SEARCH_MAIL_ISSUES_MAX: usize = 50;

/// The last hour of a day a person may choose.
pub const LAST_HOUR: i8 = 23;

/// How long the e-mails of one hour are spread over, so many people at 8:00 do not run at once:
/// each e-mailed search goes at its own fixed point of the hour.
pub const SPREAD: SignedDuration = SignedDuration::from_hours(1);

/// How many days ahead the next weekday that is chosen is looked for: a whole week and the day
/// of the same weekday after it.
const DAYS_LOOKED_AHEAD: i8 = 8;

/// The weekdays an e-mailed search goes on, at least one: Monday is the lowest bit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MailDays(u8);

/// The bits of the seven weekdays.
const ALL_DAYS: u8 = 0b111_1111;

impl MailDays {
    /// The days ticked in `days`, Monday first; `None` when none is ticked.
    #[must_use]
    pub fn new(days: [bool; 7]) -> Option<Self> {
        let bits = days
            .iter()
            .enumerate()
            .filter(|(_, ticked)| **ticked)
            .fold(0u8, |bits, (at, _)| bits | (1 << at));
        Self::from_bits(bits)
    }

    /// The days as stored; `None` for no day or a bit that is no weekday.
    #[must_use]
    pub const fn from_bits(bits: u8) -> Option<Self> {
        if bits == 0 || bits & !ALL_DAYS != 0 {
            None
        } else {
            Some(Self(bits))
        }
    }

    /// The days as stored.
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// The seven days, Monday first, each ticked or not.
    #[must_use]
    pub fn days(self) -> [bool; 7] {
        std::array::from_fn(|at| self.0 & (1 << at) != 0)
    }

    /// Whether the e-mail goes on `weekday`.
    #[must_use]
    pub fn has(self, weekday: Weekday) -> bool {
        let at = weekday.to_monday_zero_offset().unsigned_abs();
        self.0 & (1 << at) != 0
    }
}

/// The whole hour of the day an e-mailed search goes at, 0 to [`LAST_HOUR`], in the person's time
/// zone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MailHour(i8);

impl MailHour {
    /// The hour; `None` outside 0 to [`LAST_HOUR`].
    #[must_use]
    pub fn new(hour: i64) -> Option<Self> {
        i8::try_from(hour)
            .ok()
            .filter(|hour| (0..=LAST_HOUR).contains(hour))
            .map(Self)
    }

    /// The hour as a number.
    #[must_use]
    pub const fn get(self) -> i8 {
        self.0
    }
}

words! {
    /// Why an e-mailed search sends nothing until its person starts it again (ADR-0033, amendment
    /// 2026-11-24, decision 5). The pauses that end by themselves (e-mail or `search-mail.allowed`
    /// off) are not stored: they are read from the settings when the e-mail is due.
    pub enum MailPause {
        /// The person was switched off.
        SwitchedOff => "switched-off",
        /// The person can no longer open the saved search: it was deleted, is no longer shared
        /// with them, or no longer checks for them.
        NoLongerOpens => "no-longer-opens",
    }
}

words_in_api!(
    MailPause,
    "Why an e-mailed search sends nothing until its person starts it again: they were switched off, or they can no longer open the saved search."
);

/// An e-mailed search (QRY-07): one saved search that one person gets by e-mail.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct SearchMail {
    /// The ID of the e-mailed search.
    #[not_personal]
    pub id: SearchMailId,
    /// The person who gets it, the only one it is ever sent to.
    #[personal(category = "identity", basis = "contract")]
    #[person(holder)]
    pub holder: UserId,
    /// The saved search it sends; at most one e-mailed search of each saved search for a person.
    #[personal(category = "identity", basis = "contract")]
    #[names_no_person]
    pub search: SavedSearchId,
    /// The weekdays it goes on.
    #[personal(category = "activity", basis = "contract")]
    pub days: MailDays,
    /// The hour it goes at, in the person's time zone.
    #[personal(category = "activity", basis = "contract")]
    pub hour: MailHour,
    /// *Only when something is new or changed*: nothing goes when no issue it finds changed since
    /// its last e-mail.
    #[personal(category = "activity", basis = "contract")]
    pub only_when_changed: bool,
    /// Why it sends nothing until the person starts it again, while it does.
    #[personal(category = "activity", basis = "contract")]
    pub paused: Option<MailPause>,
    /// When its next e-mail is due: the hour it was chosen for, before its place in the hour.
    #[personal(category = "activity", basis = "contract")]
    pub next_at: Timestamp,
    /// The due time whose e-mail was last claimed by a job: each due time goes at most once, whatever
    /// jobs were queued for it.
    #[personal(category = "activity", basis = "contract")]
    pub last_due: Option<Timestamp>,
    /// When its last e-mail went; the issues of it are never kept.
    #[personal(category = "activity", basis = "contract")]
    pub last_sent_at: Option<Timestamp>,
    /// When it was made.
    #[not_personal]
    pub created_at: Timestamp,
    /// When its person last changed it.
    #[not_personal]
    pub updated_at: Timestamp,
}

/// The first moment after `after` that is on one of `days` at `hour` in `zone`. An hour that a
/// change to summer time skips is the first moment after the gap; an hour that the change back has
/// twice is the first of them (`jiff`'s compatible choice). `None` only past the last day `jiff`
/// knows.
#[must_use]
pub fn next_due(
    days: MailDays,
    hour: MailHour,
    zone: &TimeZone,
    after: Timestamp,
) -> Option<Timestamp> {
    let today: Date = after.to_zoned(zone.clone()).date();
    (0..=DAYS_LOOKED_AHEAD)
        .filter_map(|ahead| today.checked_add(jiff::Span::new().days(ahead)).ok())
        .filter(|day| days.has(day.weekday()))
        .filter_map(|day| day.at(hour.get(), 0, 0, 0).to_zoned(zone.clone()).ok())
        .map(|zoned| zoned.timestamp())
        .find(|moment| *moment > after)
}

/// The fixed point of its hour an e-mailed search goes at: from the random end of its ID, so the
/// e-mails of one hour are spread evenly over it and each always goes at the same minute.
#[must_use]
pub fn place_in_hour(id: SearchMailId) -> SignedDuration {
    let bytes = id.to_bytes();
    let random = u64::from(u16::from_be_bytes([bytes[14], bytes[15]]));
    let seconds = SPREAD.as_secs().unsigned_abs();
    SignedDuration::from_secs(i64::try_from(random % seconds).unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> Timestamp {
        text.parse().unwrap()
    }

    fn monday_and_thursday() -> MailDays {
        MailDays::new([true, false, false, true, false, false, false]).unwrap()
    }

    #[test]
    fn qry_07_the_days_need_one_weekday_and_keep_to_the_seven() {
        assert_eq!(MailDays::new([false; 7]), None);
        assert_eq!(MailDays::from_bits(0b1000_0000), None);
        let days = monday_and_thursday();
        assert_eq!(MailDays::from_bits(days.bits()), Some(days));
        assert_eq!(days.days(), [true, false, false, true, false, false, false]);
        assert!(days.has(Weekday::Thursday) && !days.has(Weekday::Sunday));
        assert_eq!(MailHour::new(24), None);
        assert_eq!(MailHour::new(-1), None);
        assert_eq!(MailHour::new(23).map(MailHour::get), Some(23));
    }

    #[test]
    fn qry_07_the_next_e_mail_is_at_the_hour_of_the_zone_across_summer_time() {
        let berlin = TimeZone::get("Europe/Berlin").unwrap();
        let eight = MailHour::new(8).unwrap();
        // Friday 2027-03-26, winter (UTC+1): the next Monday at 8:00 is in summer time (UTC+2).
        let friday = at("2027-03-26T12:00:00Z");
        let monday = next_due(monday_and_thursday(), eight, &berlin, friday).unwrap();
        assert_eq!(monday, at("2027-03-29T06:00:00Z"));
        // The Thursday before, still in winter time.
        let before = next_due(
            monday_and_thursday(),
            eight,
            &berlin,
            at("2027-03-23T12:00:00Z"),
        );
        assert_eq!(before, Some(at("2027-03-25T07:00:00Z")));
        // Exactly at the moment: the next one.
        let next = next_due(monday_and_thursday(), eight, &berlin, monday).unwrap();
        assert_eq!(next, at("2027-04-01T06:00:00Z"));
        // 2:00 on the Sunday of the change does not exist: it goes at 3:00 summer time.
        let sunday = MailDays::new([false, false, false, false, false, false, true]).unwrap();
        let two = MailHour::new(2).unwrap();
        assert_eq!(
            next_due(sunday, two, &berlin, at("2027-03-27T12:00:00Z")),
            Some(at("2027-03-28T01:00:00Z"))
        );
    }

    #[test]
    fn qry_07_each_e_mailed_search_has_a_fixed_place_inside_its_hour() {
        for last in [0u8, 7, 255] {
            let mut random = [9u8; 10];
            random[9] = last;
            let id = SearchMailId::from_parts(at("2027-01-01T00:00:00Z"), random).unwrap();
            let place = place_in_hour(id);
            assert!(place >= SignedDuration::ZERO && place < SPREAD, "{place:?}");
            assert_eq!(place, place_in_hour(id));
        }
    }
}
