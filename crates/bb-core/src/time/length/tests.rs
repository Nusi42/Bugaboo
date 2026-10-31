use proptest::prelude::*;

use super::*;

fn minutes(text: &str, mark: DecimalMark) -> Result<u32, LengthMistake> {
    read(text, &LengthWords::of_catalog(mark)).map(Minutes::get)
}

fn read_with(text: &str, mark: DecimalMark) -> Result<Minutes, LengthMistake> {
    read(text, &LengthWords::of_catalog(mark))
}

#[test]
fn time_04_a_typed_length_reads_the_same_in_every_form_and_respects_the_decimal_mark() {
    let point = DecimalMark::Point;
    let germany = DecimalMark::Comma;
    for (text, mark) in [
        ("1:30", point),
        ("1.5", point),
        ("1,5", germany),
        ("90m", point),
        ("1h 30m", point),
        ("1h30m", germany),
        ("1.5h", point),
        (" 90 m ", point),
        ("1H 30M", point),
    ] {
        assert_eq!(minutes(text, mark), Ok(90), "{text}");
    }
    assert_eq!(minutes("2h", point), Ok(120));
    assert_eq!(minutes("2", point), Ok(120));
    assert_eq!(minutes("0:05", point), Ok(5));
    assert_eq!(minutes(",25", germany), Ok(15));
    // The mark of another number format is no decimal mark here.
    assert_eq!(minutes("1,5", point), Err(LengthMistake::NotALength));
    assert_eq!(minutes("1.5", germany), Err(LengthMistake::NotALength));
}

#[test]
fn time_04_decimals_of_an_hour_round_to_the_nearest_minute() {
    assert_eq!(minutes("0.33", DecimalMark::Point), Ok(20));
    assert_eq!(minutes("0.01", DecimalMark::Point), Ok(1));
    assert_eq!(minutes("0.0083", DecimalMark::Point), Ok(0));
    assert_eq!(minutes("0.125", DecimalMark::Point), Ok(8));
}

fn entry(text: &str) -> Result<u32, LengthMistake> {
    read_time_entry(text, &LengthWords::of_catalog(DecimalMark::Point)).map(Minutes::get)
}

#[test]
fn time_04_one_time_entry_is_at_least_a_minute_and_at_most_a_day() {
    assert_eq!(entry("24:00"), Ok(1440));
    assert_eq!(entry("24h"), Ok(1440));
    assert_eq!(entry("0:01"), Ok(1));
    for text in ["24:01", "25", "1441m", "24h 1m", "999999h"] {
        assert_eq!(entry(text), Err(LengthMistake::TooLong), "{text}");
    }
    for text in ["0", "0:00", "0m", "0.001"] {
        assert_eq!(entry(text), Err(LengthMistake::Zero), "{text}");
    }
    assert_eq!(
        LengthMistake::Zero.said(DecimalMark::Point).render(),
        "A time entry is at least 1 minute."
    );
}

#[test]
fn time_04_the_reader_has_no_bound_of_its_own() {
    // A length of an own field may be longer than a day, and zero is a length.
    assert_eq!(minutes("40h", DecimalMark::Point), Ok(2400));
    assert_eq!(minutes("999999h", DecimalMark::Point), Ok(59_999_940));
    assert_eq!(minutes("0", DecimalMark::Point), Ok(0));
    // Only a length too long to count is no length.
    assert_eq!(
        minutes("999999999h", DecimalMark::Point),
        Err(LengthMistake::NotALength)
    );
}

#[test]
fn time_04_anything_else_is_a_plain_mistake() {
    for text in [
        "", "soon", "1:3", "1:60", "1:300", ":30", "1.5m", "30m 1h", "1h 2h", "1h 30", "-1", "1.",
        "h", "1 h m", "1:30h", "1e3", "١", "1İ",
    ] {
        assert_eq!(
            minutes(text, DecimalMark::Point),
            Err(LengthMistake::NotALength),
            "{text:?}"
        );
    }
}

#[test]
fn time_04_a_mistake_shows_how_to_write_it_with_the_persons_mark() {
    assert_eq!(
        LengthMistake::NotALength.said(DecimalMark::Comma).render(),
        "Type a length of time like 1:30, 1,5 or 1h 30m."
    );
    assert_eq!(
        LengthMistake::TooLong.said(DecimalMark::Point).render(),
        "One entry is at most 24 hours."
    );
}

#[test]
fn time_04_a_length_is_written_in_three_forms_by_one_writer() {
    use crate::wording::{length_said, length_sentence};
    let said = |minutes| {
        let length = Minutes::new(minutes);
        [
            length.on_clock(),
            length_said(length).render(),
            length_sentence(length).render(),
        ]
    };
    assert_eq!(said(90), ["1:30", "1h 30m", "1 hour 30 minutes"]);
    assert_eq!(said(42), ["0:42", "42m", "42 minutes"]);
    assert_eq!(said(600), ["10:00", "10h", "10 hours"]);
    assert_eq!(said(2368), ["39:28", "39h 28m", "39 hours 28 minutes"]);
    assert_eq!(said(61), ["1:01", "1h 1m", "1 hour 1 minute"]);
    assert_eq!(said(0), ["0:00", "0m", "0 minutes"]);
}

#[test]
fn time_04_a_length_on_the_clock_has_two_digits_of_minutes() {
    assert_eq!(Minutes::new(90).on_clock(), "1:30");
    assert_eq!(Minutes::new(5).on_clock(), "0:05");
    assert_eq!(TIME_ENTRY_MINUTES_MAX.on_clock(), "24:00");
}

proptest! {
    #[test]
    fn time_04_a_length_on_the_clock_reads_back_as_the_same_minutes(count in 0u32..=1440) {
        let written = Minutes::new(count).on_clock();
        prop_assert_eq!(read_with(&written, DecimalMark::Point), Ok(Minutes::new(count)));
        prop_assert_eq!(read_with(&written, DecimalMark::Comma), Ok(Minutes::new(count)));
    }

    #[test]
    fn time_04_hours_and_minutes_read_the_same_with_units_and_on_the_clock(
        hours in 0u32..24,
        rest in 0u32..60,
    ) {
        let on_clock = read_with(&format!("{hours}:{rest:02}"), DecimalMark::Point);
        prop_assert_eq!(read_with(&format!("{hours}h {rest}m"), DecimalMark::Point), on_clock);
        prop_assert_eq!(read_with(&format!("{}m", hours * 60 + rest), DecimalMark::Point), on_clock);
    }
}

#[test]
fn time_04_minutes_count_and_convert_without_overflowing() {
    let (hour, half) = (Minutes::new(60), Minutes::new(30));
    assert_eq!(hour.checked_add(half), Some(Minutes::new(90)));
    assert_eq!(half.checked_sub(hour), None);
    assert_eq!(Minutes::new(u32::MAX).checked_add(half), None);
    assert_eq!(Minutes::from_seconds(5_459), Some(Minutes::new(90)));
    assert_eq!(Minutes::from_seconds(-60), None);
    assert_eq!(Minutes::new(90).as_seconds(), 5_400);
    assert_eq!(Minutes::try_from(90_i64).ok(), Some(Minutes::new(90)));
    assert!(Minutes::try_from(-1_i64).is_err());
    assert_eq!(i64::from(Minutes::new(90)), 90);
}
