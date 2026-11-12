//! Which holidays a region has and on which date each falls: the one table of the rules.

use jiff::ToSpan as _;
use jiff::civil::{Date, Weekday};

use super::easter::sunday_of;
use super::{Holiday, Region};

/// The days every state has: what *Germany* means.
const EVERY_STATE: [Holiday; 9] = [
    Holiday::NewYear,
    Holiday::GoodFriday,
    Holiday::EasterMonday,
    Holiday::LabourDay,
    Holiday::AscensionDay,
    Holiday::WhitMonday,
    Holiday::UnityDay,
    Holiday::ChristmasDay,
    Holiday::SecondChristmasDay,
];

/// The year before any year: a day that was always a holiday.
const ALWAYS: i16 = i16::MIN;

/// What a state has besides [`EVERY_STATE`], by the holiday laws of the states, with the first year
/// the day is a holiday. Only holidays of the whole state: a holiday of some towns or of part of a
/// state is not listed.
const fn extras(region: Region) -> &'static [(Holiday, i16)] {
    use Holiday::{
        AllSaints, AssumptionDay, CorpusChristi, EasterSunday, Epiphany, ReformationDay,
        RepentanceDay, WhitSunday, WomenDay, WorldChildrensDay,
    };
    match region {
        Region::None | Region::Germany => &[],
        Region::BadenWuerttemberg | Region::Bavaria => &[
            (Epiphany, ALWAYS),
            (CorpusChristi, ALWAYS),
            (AllSaints, ALWAYS),
        ],
        Region::Berlin => &[(WomenDay, 2019)],
        Region::Brandenburg => &[
            (EasterSunday, ALWAYS),
            (WhitSunday, ALWAYS),
            (ReformationDay, ALWAYS),
        ],
        Region::Bremen | Region::Hamburg | Region::LowerSaxony | Region::SchleswigHolstein => {
            &[(ReformationDay, 2018)]
        }
        Region::Hesse => &[
            (CorpusChristi, ALWAYS),
            (EasterSunday, ALWAYS),
            (WhitSunday, ALWAYS),
        ],
        Region::MecklenburgVorpommern => &[(WomenDay, 2023), (ReformationDay, ALWAYS)],
        Region::NorthRhineWestphalia | Region::RhinelandPalatinate => {
            &[(CorpusChristi, ALWAYS), (AllSaints, ALWAYS)]
        }
        Region::Saarland => &[
            (CorpusChristi, ALWAYS),
            (AssumptionDay, ALWAYS),
            (AllSaints, ALWAYS),
        ],
        Region::Saxony => &[(ReformationDay, ALWAYS), (RepentanceDay, ALWAYS)],
        Region::SaxonyAnhalt => &[(Epiphany, ALWAYS), (ReformationDay, ALWAYS)],
        Region::Thuringia => &[(WorldChildrensDay, 2019), (ReformationDay, ALWAYS)],
    }
}

/// The days that were a holiday in one year only: the 500th anniversary of the Reformation,
/// 31 October 2017, everywhere, and the 75th and 80th anniversary of the end of the war,
/// 8 May 2020 and 2025, in Berlin.
fn one_offs(region: Region, year: i16) -> &'static [Holiday] {
    match (region, year) {
        (Region::None, _) => &[],
        (_, 2017) => &[Holiday::ReformationDay],
        (Region::Berlin, 2020 | 2025) => &[Holiday::LiberationDay],
        _ => &[],
    }
}

/// The holidays a region has in a year, in no order and perhaps one twice (a state that has the
/// Reformation Day anyway has it in 2017 too). *None* has none.
pub(super) fn holidays_of(region: Region, year: i16) -> impl Iterator<Item = Holiday> {
    let common: &[Holiday] = if region == Region::None {
        &[]
    } else {
        &EVERY_STATE
    };
    let extra = extras(region)
        .iter()
        .filter(move |(_, from)| *from <= year)
        .map(|(holiday, _)| *holiday);
    common
        .iter()
        .copied()
        .chain(extra)
        .chain(one_offs(region, year).iter().copied())
}

/// How a holiday finds its date in a year.
enum Rule {
    /// The same day every year, by month and day.
    Fixed(i8, i8),
    /// A number of days after Easter Sunday (before it when negative).
    AfterEaster(i32),
    /// The Wednesday before 23 November.
    WednesdayBeforeNovember23,
}

const fn rule_of(holiday: Holiday) -> Rule {
    match holiday {
        Holiday::NewYear => Rule::Fixed(1, 1),
        Holiday::Epiphany => Rule::Fixed(1, 6),
        Holiday::WomenDay => Rule::Fixed(3, 8),
        Holiday::GoodFriday => Rule::AfterEaster(-2),
        Holiday::EasterSunday => Rule::AfterEaster(0),
        Holiday::EasterMonday => Rule::AfterEaster(1),
        Holiday::LabourDay => Rule::Fixed(5, 1),
        Holiday::LiberationDay => Rule::Fixed(5, 8),
        Holiday::AscensionDay => Rule::AfterEaster(39),
        Holiday::WhitSunday => Rule::AfterEaster(49),
        Holiday::WhitMonday => Rule::AfterEaster(50),
        Holiday::CorpusChristi => Rule::AfterEaster(60),
        Holiday::AssumptionDay => Rule::Fixed(8, 15),
        Holiday::WorldChildrensDay => Rule::Fixed(9, 20),
        Holiday::UnityDay => Rule::Fixed(10, 3),
        Holiday::ReformationDay => Rule::Fixed(10, 31),
        Holiday::AllSaints => Rule::Fixed(11, 1),
        Holiday::RepentanceDay => Rule::WednesdayBeforeNovember23,
        Holiday::ChristmasDay => Rule::Fixed(12, 25),
        Holiday::SecondChristmasDay => Rule::Fixed(12, 26),
    }
}

/// The date of a holiday in a year.
pub(super) fn date_of(holiday: Holiday, year: i16) -> Option<Date> {
    match rule_of(holiday) {
        Rule::Fixed(month, day) => Date::new(year, month, day).ok(),
        Rule::AfterEaster(days) => sunday_of(year)?.checked_add(days.days()).ok(),
        Rule::WednesdayBeforeNovember23 => Date::new(year, 11, 23)
            .ok()?
            .nth_weekday(-1, Weekday::Wednesday)
            .ok(),
    }
}
