//! Relative times in the zone of the person (QRY-04): *today*, *this week*, *in last 7 days* become
//! moments by the calendar of the zone the request carries, so a day begins where the person's day
//! begins. The tree keeps the words; only here do they become numbers.

use bb_searchlang::{Date, TimeWord, Unit};
use jiff::civil;
use jiff::tz::TimeZone;
use jiff::{Span, ToSpan as _};

use super::{CompileError, Compiler};

/// How a column holds a moment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Clock {
    /// Microseconds since 1970 (UTC): an issue's dates, a date and time.
    Micros,
    /// Days since 1970-01-01: the *Date* of an own field.
    Days,
}

/// The stretch of time a word stands for, as the values of a column: from this one on, until that
/// one (not including it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Range {
    pub(super) from: i64,
    pub(super) to: i64,
}

const NOT_A_DATE: CompileError = CompileError::Unsupported("a day outside the calendar");

impl Compiler<'_> {
    fn zone(&self) -> &TimeZone {
        &self.request.zone
    }

    fn today(&self) -> civil::Date {
        self.request.now.to_zoned(self.zone().clone()).date()
    }

    /// The value of the start of `day` in the column.
    fn at(&self, day: civil::Date, clock: Clock) -> Result<i64, CompileError> {
        match clock {
            Clock::Micros => day
                .to_zoned(self.zone().clone())
                .map(|start| start.timestamp().as_microsecond())
                .map_err(|_| NOT_A_DATE),
            Clock::Days => day
                .to_zoned(TimeZone::UTC)
                .map(|start| start.timestamp().as_second().div_euclid(86_400))
                .map_err(|_| NOT_A_DATE),
        }
    }

    fn days(
        &self,
        from: civil::Date,
        to: civil::Date,
        clock: Clock,
    ) -> Result<Range, CompileError> {
        Ok(Range {
            from: self.at(from, clock)?,
            to: self.at(to, clock)?,
        })
    }

    /// The day written in a search.
    pub(super) fn day(&self, date: Date, clock: Clock) -> Result<Range, CompileError> {
        let start = i16::try_from(date.year)
            .ok()
            .and_then(|year| {
                let month = i8::try_from(date.month).ok()?;
                let day = i8::try_from(date.day).ok()?;
                civil::Date::new(year, month, day).ok()
            })
            .ok_or(NOT_A_DATE)?;
        let next = start.checked_add(1.day()).map_err(|_| NOT_A_DATE)?;
        self.days(start, next, clock)
    }

    /// The stretch a time word stands for.
    pub(super) fn word(&self, word: TimeWord, clock: Clock) -> Result<Range, CompileError> {
        let today = self.today();
        let monday = today
            .checked_sub(Span::new().days(i64::from(today.weekday().to_monday_zero_offset())))
            .map_err(|_| NOT_A_DATE)?;
        let step = |day: civil::Date, by: Span| day.checked_add(by).map_err(|_| NOT_A_DATE);
        let (from, to) = match word {
            TimeWord::Today => (today, step(today, 1.day())?),
            TimeWord::Yesterday => (step(today, -1.day())?, today),
            TimeWord::ThisWeek => (monday, step(monday, 7.days())?),
            TimeWord::LastWeek => (step(monday, -7.days())?, monday),
            TimeWord::ThisMonth => {
                let first = today.first_of_month();
                (first, step(first, 1.month())?)
            }
        };
        self.days(from, to, clock)
    }

    /// The stretch from `count` units ago until now (the whole of today for a column of days).
    pub(super) fn last(&self, count: u32, unit: Unit, clock: Clock) -> Result<Range, CompileError> {
        let span = match unit {
            Unit::Day => Span::new().days(i64::from(count)),
            Unit::Week => Span::new().weeks(i64::from(count)),
            Unit::Month => Span::new().months(i64::from(count)),
        };
        let now = self.request.now.to_zoned(self.zone().clone());
        let since = now.checked_sub(span).map_err(|_| NOT_A_DATE)?;
        match clock {
            Clock::Micros => Ok(Range {
                from: since.timestamp().as_microsecond(),
                to: now.timestamp().as_microsecond().saturating_add(1),
            }),
            Clock::Days => self.days(since.date(), step_one(now.date())?, clock),
        }
    }
}

fn step_one(day: civil::Date) -> Result<civil::Date, CompileError> {
    day.checked_add(1.day()).map_err(|_| NOT_A_DATE)
}
