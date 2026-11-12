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

/// What a state has besides [`EVERY_STATE`], by the holiday laws of the states. Only holidays of
/// the whole state: a holiday of some towns or of part of a state is not listed.
const fn extras(region: Region) -> &'static [Holiday] {
    use Holiday::{
        AllSaints, AssumptionDay, CorpusChristi, EasterSunday, Epiphany, ReformationDay,
        RepentanceDay, WhitSunday, WomenDay, WorldChildrensDay,
    };
    match region {
        Region::None | Region::Germany => &[],
        Region::BadenWuerttemberg | Region::Bavaria => &[Epiphany, CorpusChristi, AllSaints],
        Region::Berlin => &[WomenDay],
        Region::Brandenburg => &[EasterSunday, WhitSunday, ReformationDay],
        Region::Bremen | Region::Hamburg | Region::LowerSaxony | Region::SchleswigHolstein => {
            &[ReformationDay]
        }
        Region::Hesse => &[CorpusChristi],
        Region::MecklenburgVorpommern => &[WomenDay, ReformationDay],
        Region::NorthRhineWestphalia | Region::RhinelandPalatinate => &[CorpusChristi, AllSaints],
        Region::Saarland => &[CorpusChristi, AssumptionDay, AllSaints],
        Region::Saxony => &[ReformationDay, RepentanceDay],
        Region::SaxonyAnhalt => &[Epiphany, ReformationDay],
        Region::Thuringia => &[WorldChildrensDay, ReformationDay],
    }
}

/// The holidays a region has, in no order. *None* has none.
pub(super) fn holidays_of(region: Region) -> impl Iterator<Item = Holiday> {
    let common: &[Holiday] = if region == Region::None {
        &[]
    } else {
        &EVERY_STATE
    };
    common.iter().chain(extras(region)).copied()
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
