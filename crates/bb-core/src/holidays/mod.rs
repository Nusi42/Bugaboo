//! Public holidays of Germany and its 16 federal states (PLAN-11, ADR-0030, amendment M8), worked
//! out here and nowhere else: the fixed dates and the days that follow Easter. Pure: no clock, no
//! download and no outside service, so no service learns which state a person works in.
//!
//! Only a holiday of a whole state counts. A holiday of one town or of part of a state (the
//! *Augsburger Friedensfest*, *Mariä Himmelfahrt* in the Bavarian towns that keep it) is entered
//! as an away time. The results are tested against the official lists of 2026 to 2030
//! (`plan_11_german_holidays_match_the_official_lists_2026_to_2030`).

mod easter;
mod rules;

use jiff::civil::Date;

use crate::settings::Layers;
use crate::settings::work_times::WORK_TIMES_HOLIDAYS;

words! {
    /// Which public holidays count for a person: none, the ones every state has, or those of one
    /// state by its ISO 3166-2 code (the setting *Holidays*, `work-times.holidays`).
    pub enum Region {
        /// No public holidays.
        None => "none",
        /// The days every state has.
        Germany => "de",
        /// Baden-Württemberg.
        BadenWuerttemberg => "de-bw",
        /// Bavaria.
        Bavaria => "de-by",
        /// Berlin.
        Berlin => "de-be",
        /// Brandenburg.
        Brandenburg => "de-bb",
        /// Bremen.
        Bremen => "de-hb",
        /// Hamburg.
        Hamburg => "de-hh",
        /// Hesse.
        Hesse => "de-he",
        /// Mecklenburg-Vorpommern.
        MecklenburgVorpommern => "de-mv",
        /// Lower Saxony.
        LowerSaxony => "de-ni",
        /// North Rhine-Westphalia.
        NorthRhineWestphalia => "de-nw",
        /// Rhineland-Palatinate.
        RhinelandPalatinate => "de-rp",
        /// Saarland.
        Saarland => "de-sl",
        /// Saxony.
        Saxony => "de-sn",
        /// Saxony-Anhalt.
        SaxonyAnhalt => "de-st",
        /// Schleswig-Holstein.
        SchleswigHolstein => "de-sh",
        /// Thuringia.
        Thuringia => "de-th",
    }
    messages name = "holidays.region.";
}

words! {
    /// One public holiday, by what it is called; the words of the days are the messages
    /// `holiday.<word>`.
    pub enum Holiday {
        /// 1 January.
        NewYear => "new-year",
        /// 6 January.
        Epiphany => "epiphany",
        /// 8 March.
        WomenDay => "women-day",
        /// The Friday before Easter.
        GoodFriday => "good-friday",
        /// Easter Sunday.
        EasterSunday => "easter-sunday",
        /// The Monday after Easter.
        EasterMonday => "easter-monday",
        /// 1 May.
        LabourDay => "labour-day",
        /// 8 May, in Berlin in 2020 and 2025 only.
        LiberationDay => "liberation-day",
        /// 39 days after Easter.
        AscensionDay => "ascension-day",
        /// Pentecost Sunday, 49 days after Easter.
        WhitSunday => "whit-sunday",
        /// Pentecost Monday, 50 days after Easter.
        WhitMonday => "whit-monday",
        /// 60 days after Easter.
        CorpusChristi => "corpus-christi",
        /// 15 August.
        AssumptionDay => "assumption-day",
        /// 20 September.
        WorldChildrensDay => "world-childrens-day",
        /// 3 October.
        UnityDay => "unity-day",
        /// 31 October.
        ReformationDay => "reformation-day",
        /// 1 November.
        AllSaints => "all-saints",
        /// The Wednesday before 23 November.
        RepentanceDay => "repentance-day",
        /// 25 December.
        ChristmasDay => "christmas-day",
        /// 26 December.
        SecondChristmasDay => "second-christmas-day",
    }
    messages name = "holiday.";
}

words_in_api!(
    Region as "HolidaysChoice",
    "Which public holidays count for a person: none, the days every state of Germany has (`de`), or those of one state by its ISO 3166-2 code, such as `de-by`."
);

words_in_api!(Holiday, "One public holiday, by what it is called.");

/// One public holiday on a date.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PublicHoliday {
    /// The day.
    pub date: Date,
    /// Which holiday it is.
    pub holiday: Holiday,
}

impl Region {
    /// The holidays a person's layers of settings choose: their own choice, or the organization's
    /// (*Germany* at the start).
    #[must_use]
    pub fn chosen(layers: &Layers) -> Self {
        Self::from_code(&layers.text(&WORK_TIMES_HOLIDAYS)).unwrap_or(Self::Germany)
    }

    /// The public holidays of a year in this region, by date. *None* has none.
    #[must_use]
    pub fn holidays_in(self, year: i16) -> Vec<PublicHoliday> {
        let mut days: Vec<PublicHoliday> = rules::holidays_of(self, year)
            .filter_map(|holiday| {
                rules::date_of(holiday, year).map(|date| PublicHoliday { date, holiday })
            })
            .collect();
        days.sort_by_key(|day| day.date);
        days.dedup_by_key(|day| day.date);
        days
    }

    /// The public holiday on a date in this region, if there is one.
    #[must_use]
    pub fn holiday_on(self, date: Date) -> Option<Holiday> {
        rules::holidays_of(self, date.year())
            .find(|holiday| rules::date_of(*holiday, date.year()) == Some(date))
    }

    /// The public holidays from `first` to `last` (both included), by date. Empty when `last` is
    /// before `first`.
    #[must_use]
    pub fn holidays_between(self, first: Date, last: Date) -> Vec<PublicHoliday> {
        (first.year()..=last.year())
            .flat_map(|year| self.holidays_in(year))
            .filter(|day| first <= day.date && day.date <= last)
            .collect()
    }

    /// The next `count` public holidays on or after `from`, by date: the ones a person checks their
    /// choice of *Holidays* against. Every region has a holiday within a year, so a year is looked at.
    #[must_use]
    pub fn next_holidays(self, from: Date, count: usize) -> Vec<PublicHoliday> {
        let last = from.saturating_add(jiff::Span::new().years(1));
        let mut days = self.holidays_between(from, last);
        days.truncate(count);
        days
    }
}

#[cfg(test)]
mod tests;
