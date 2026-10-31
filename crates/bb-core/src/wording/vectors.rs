//! The test vectors that hold the two writings of a value together (ADR-0033, the amendment about
//! value wording): `tools/testdata/value-words.json`, written by `cargo xtask gen word-vectors`
//! from this code, so the expected words are always what the server writes.
//!
//! The browser test reads the file and checks that the screen (`Intl`, `web/app/src/shared`) gives
//! the same words for the same value and the same choices under *Region*. DOC-GENERATED fails when
//! the file does not match this code, and the browser test fails when the screen differs.

use serde_json::{Value, json};

use jiff::Timestamp;

use super::region::Choices;
use super::{
    Change, Region, Side, change_sentence, length_said, length_sentence, list_said, typed_said,
};
use crate::own_fields::FieldType;
use crate::time::length::{self, DecimalMark, LengthWords, Minutes};

/// The choices under *Region* that a server knows, by the name the cases use: the date format, the
/// time of day, the number format and the zone. The date formats are fixed ones (a server does not
/// know the device's); the time of day *device* follows them, which a server knows. Each date order
/// is here with both clocks.
const REGIONS: [(&str, &str, &str, &str, &str); 7] = [
    ("usual", "year-month-day", "device", "point", "UTC"),
    (
        "german",
        "day-month-year",
        "device",
        "comma",
        "Europe/Berlin",
    ),
    (
        "american",
        "month-day-year",
        "12-hour",
        "point",
        "America/Los_Angeles",
    ),
    ("french", "year-month-day", "device", "space", "UTC"),
    (
        "usual on 12 hours",
        "year-month-day",
        "12-hour",
        "point",
        "UTC",
    ),
    (
        "german on 12 hours",
        "day-month-year",
        "12-hour",
        "comma",
        "Europe/Berlin",
    ),
    (
        "american on 24 hours",
        "month-day-year",
        "24-hour",
        "point",
        "America/Los_Angeles",
    ),
];

/// A case: a name, the kind of the field, the value as the API gives it, and the region.
type Case = (&'static str, &'static str, fn() -> Value, &'static str);

const MOMENT: &str = "2026-10-05T07:30:00Z";

/// The cases of yes and no, lengths of time and numbers. The values are functions so the table
/// can hold any JSON value.
fn numbers_and_words() -> Vec<Case> {
    vec![
        ("yes", "yes-no", || json!(true), "usual"),
        ("no", "yes-no", || json!(false), "german"),
        ("yes-no given text", "yes-no", || json!("maybe"), "usual"),
        ("hours and minutes", "duration", || json!(7500), "usual"),
        ("minutes only", "duration", || json!(2700), "usual"),
        ("whole hours", "duration", || json!(7200), "usual"),
        ("under a minute", "duration", || json!(30), "usual"),
        ("duration given yes", "duration", || json!(true), "usual"),
        (
            "negative length of time",
            "duration",
            || json!(-90),
            "usual",
        ),
        ("fractional seconds", "duration", || json!(90.5), "usual"),
        (
            "seconds just under a minute",
            "duration",
            || json!(59.9),
            "usual",
        ),
        (
            "whole number with a point",
            "whole-number",
            || json!(1_234_567),
            "usual",
        ),
        (
            "whole number with a comma",
            "whole-number",
            || json!(1_234_567),
            "german",
        ),
        (
            "whole number with a space",
            "whole-number",
            || json!(1_234_567),
            "french",
        ),
        ("small whole number", "whole-number", || json!(12), "usual"),
        (
            "negative whole number",
            "whole-number",
            || json!(-1234),
            "usual",
        ),
        ("decimal with a point", "decimal", || json!(1234.5), "usual"),
        (
            "decimal with a comma",
            "decimal",
            || json!(1234.5),
            "german",
        ),
        (
            "decimal with a space",
            "decimal",
            || json!(1234.5),
            "french",
        ),
        (
            "decimal rounded to three places",
            "decimal",
            || json!(1234.5678),
            "usual",
        ),
        ("decimal below one", "decimal", || json!(0.25), "german"),
        (
            "decimal tie rounds half up",
            "decimal",
            || json!(0.0625),
            "usual",
        ),
        (
            "decimal tie of shortest digits",
            "decimal",
            || json!(1.0005),
            "usual",
        ),
        (
            "decimal rounds up through nines",
            "decimal",
            || json!(0.9996),
            "usual",
        ),
        (
            "negative decimal tie",
            "decimal",
            || json!(-0.0625),
            "usual",
        ),
    ]
}

/// The cases of days and moments.
fn days_and_moments() -> Vec<Case> {
    vec![
        ("day, year first", "date", || json!("2026-10-05"), "usual"),
        ("day, day first", "date", || json!("2026-10-05"), "german"),
        (
            "day, month first, west of Greenwich",
            "date",
            || json!("2026-10-05"),
            "american",
        ),
        ("not a day", "date", || json!("someday"), "usual"),
        (
            "a day that does not exist",
            "date",
            || json!("2026-02-30"),
            "usual",
        ),
        (
            "a day in a leap year",
            "date",
            || json!("2028-02-29"),
            "german",
        ),
        ("moment in UTC", "date-time", || json!(MOMENT), "usual"),
        (
            "moment in Berlin in summer",
            "date-time",
            || json!(MOMENT),
            "german",
        ),
        (
            "moment in Berlin in winter",
            "date-time",
            || json!("2026-12-05T07:30:00Z"),
            "german",
        ),
        (
            "moment that is the next day in Berlin",
            "date-time",
            || json!("2026-10-05T23:30:00Z"),
            "german",
        ),
        (
            "moment at noon",
            "date-time",
            || json!("2026-10-05T12:05:00Z"),
            "usual",
        ),
        (
            "moment at midnight",
            "date-time",
            || json!("2026-10-05T00:00:00Z"),
            "usual",
        ),
        (
            "moment in the morning on a 12-hour clock",
            "date-time",
            || json!("2026-10-05T09:07:00Z"),
            "usual",
        ),
        (
            "moment that is the day before in Los Angeles",
            "date-time",
            || json!("2026-10-05T03:15:00Z"),
            "american",
        ),
        (
            "moment on 12 hours, year first",
            "date-time",
            || json!("2026-10-05T13:05:00Z"),
            "usual on 12 hours",
        ),
        (
            "moment on 12 hours, day first",
            "date-time",
            || json!("2026-10-05T13:05:00Z"),
            "german on 12 hours",
        ),
        (
            "moment on 24 hours, month first",
            "date-time",
            || json!("2026-10-05T23:30:00Z"),
            "american on 24 hours",
        ),
        ("not a moment", "date-time", || json!("later"), "usual"),
        ("short text", "text", || json!("hello"), "usual"),
        (
            "web address",
            "web-address",
            || json!("https://example.com"),
            "usual",
        ),
    ]
}

/// Every case.
fn cases() -> Vec<Case> {
    let mut all = numbers_and_words();
    all.extend(days_and_moments());
    all
}

/// The lists of names the vectors check.
const LISTS: [&[&str]; 5] = [
    &[],
    &["Ana"],
    &["Ana", "Ben"],
    &["Ana", "Ben", "Cem"],
    &["Ana", "Ben", "Cem", "Dee"],
];

fn region_named(name: &str) -> Option<Region> {
    REGIONS
        .iter()
        .find(|(each, ..)| *each == name)
        .map(|(_, date, time, number, zone)| {
            Region::from_choices(&Choices {
                date_format: Some(date),
                time_of_day: Some(time),
                number_format: Some(number),
                time_zone: Some(zone),
                first_day: None,
            })
        })
}

/// The moments whose time of day the vectors check in every region: in the morning, exactly at
/// noon, after noon, a minute before midnight, and exactly at midnight.
const TIMES: [&str; 5] = [
    MOMENT,
    "2026-10-05T12:00:00Z",
    "2026-10-05T13:05:00Z",
    "2026-10-05T23:59:00Z",
    "2026-10-05T00:00:00Z",
];

/// The times of day of [`TIMES`] in every region, as the screen shows them.
fn times() -> Result<Vec<Value>, String> {
    let mut all = Vec::new();
    for (name, ..) in REGIONS {
        let region = region_named(name).ok_or_else(|| format!("no region {name:?}"))?;
        for moment in TIMES {
            let at: Timestamp = moment
                .parse()
                .map_err(|_| format!("{moment:?} is no moment"))?;
            all.push(json!({
                "moment": moment,
                "region": name,
                "expected": region.time_of_day(at),
            }));
        }
    }
    Ok(all)
}

/// The lengths of time a person types that the vectors check, with the decimal mark of their
/// number format: the forms of one length, the marks, the limit of an entry and mistakes.
const LENGTHS: [(&str, DecimalMark); 19] = [
    ("40h", DecimalMark::Point),
    ("0", DecimalMark::Point),
    ("1İ", DecimalMark::Point),
    ("1:30", DecimalMark::Point),
    ("1.5", DecimalMark::Point),
    ("1,5", DecimalMark::Comma),
    ("90m", DecimalMark::Point),
    ("1h 30m", DecimalMark::Point),
    ("2h", DecimalMark::Point),
    ("2", DecimalMark::Comma),
    ("1H 30M", DecimalMark::Point),
    ("0.33", DecimalMark::Point),
    ("0:05", DecimalMark::Point),
    ("24:00", DecimalMark::Point),
    ("24:01", DecimalMark::Point),
    ("1,5", DecimalMark::Point),
    ("30m 1h", DecimalMark::Point),
    ("soon", DecimalMark::Comma),
    ("", DecimalMark::Point),
];

/// The lengths that the vectors write on the clock, in minutes.
const ON_CLOCK: [u32; 4] = [0, 5, 90, 1440];

/// The changes whose sentence the vectors check: values of digits that are texts, not counts.
fn changes() -> Vec<Value> {
    let text = |text: &str| Side::Text {
        text: text.to_owned(),
    };
    [
        (Some(text("007")), Some(text("0815"))),
        (None, Some(text("+5"))),
    ]
    .into_iter()
    .map(|(before, after)| {
        let change = Change {
            who: "Ben".to_owned(),
            field: "code".to_owned(),
            field_name: "Code".to_owned(),
            before,
            after,
        };
        json!({"change": change, "expected": change_sentence(&change).render()})
    })
    .collect()
}

/// The lengths that the vectors write in a sentence, in minutes: none, one of each unit, many.
const IN_SENTENCES: [u32; 7] = [0, 1, 42, 60, 61, 600, 2368];

/// What a reader made of a typed length: the minutes and the two short forms the writer gives
/// back (`1h 30m`, `1:30`), or the sentence of the mistake.
fn read_as(read: Result<Minutes, length::LengthMistake>, mark: DecimalMark) -> Value {
    match read {
        Ok(minutes) => json!({
            "minutes": minutes,
            "short": length_said(minutes).render(),
            "grid": minutes.on_clock(),
            "mistake": null,
        }),
        Err(mistake) => json!({
            "minutes": null,
            "short": null,
            "grid": null,
            "mistake": mistake.said(mark).render(),
        }),
    }
}

/// What the reader makes of each of [`LENGTHS`], without a bound and as one time entry.
fn lengths() -> Vec<Value> {
    LENGTHS
        .iter()
        .map(|(text, mark)| {
            let words = LengthWords::of_catalog(*mark);
            json!({
                "text": text,
                "mark": mark,
                "read": read_as(length::read(text, &words), *mark),
                "entry": read_as(length::read_time_entry(text, &words), *mark),
            })
        })
        .collect()
}

/// The words of a value of a kind for a region, as the screen shows it: no name of a time zone.
fn words(kind: FieldType, value: &Value, region: &Region) -> Option<String> {
    let worded = region.worded(kind, value);
    typed_said(kind, value, worded.as_deref()).map(|said| said.render())
}

/// The file `tools/testdata/value-words.json`, as it should be.
///
/// # Errors
/// A case names a kind or a region that does not exist, or the file cannot be written as JSON: the
/// table is wrong, and a file without the case would hide it.
pub fn value_words_file() -> Result<String, String> {
    let regions: serde_json::Map<String, Value> = REGIONS
        .iter()
        .map(|(name, date, time, number, zone)| {
            (
                (*name).to_owned(),
                json!({
                    "dateFormat": date,
                    "timeOfDay": time,
                    "numberFormat": number,
                    "timeZone": zone,
                }),
            )
        })
        .collect();
    let cases = cases()
        .into_iter()
        .map(|(name, kind, value, region)| {
            let value = value();
            let kind = serde_json::from_value::<FieldType>(json!(kind))
                .map_err(|_| format!("the case {name:?} names the kind {kind:?}, which is none"))?;
            let known = region_named(region)
                .ok_or_else(|| format!("the case {name:?} names the region {region:?}"))?;
            let expected = words(kind, &value, &known);
            Ok(json!({
                "name": name,
                "kind": kind,
                "value": value,
                "region": region,
                "expected": expected,
            }))
        })
        .collect::<Result<Vec<Value>, String>>()?;
    let lists: Vec<Value> = LISTS
        .iter()
        .map(|names| {
            let names: Vec<String> = names.iter().map(|name| (*name).to_owned()).collect();
            let expected = list_said(&names).map_or_else(String::new, |said| said.render());
            json!({"names": names, "expected": expected})
        })
        .collect();
    let clocks: Vec<Value> = ON_CLOCK
        .iter()
        .map(|minutes| json!({"minutes": minutes, "expected": Minutes::new(*minutes).on_clock()}))
        .collect();
    let sentences: Vec<Value> = IN_SENTENCES
        .iter()
        .map(|minutes| {
            json!({
                "minutes": minutes,
                "expected": length_sentence(Minutes::new(*minutes)).render(),
            })
        })
        .collect();
    let file = json!({
        "about": "How the value of an own field, a time of day and a length of time are worded for a person, and how a typed length of time is read. This file is written by `cargo xtask gen word-vectors` from crates/bb-core/src/wording/vectors.rs: the expected words are what the server writes. The browser test (web/app/src/shared/value-words.test.ts) reads it and checks that the screen gives the same words for the same value and the same choices under Region, and that the reader of lengths in the module of the browser reads the same (ADR-0033, the amendment about value wording). A null expected means the value has no words for its kind. Only choices the server can know are here: no date format of the device (a time of day of the device follows the fixed date format).",
        "regions": regions,
        "cases": cases,
        "times": times()?,
        "lists": lists,
        "changes": changes(),
        "lengths": lengths(),
        "clocks": clocks,
        "sentences": sentences,
    });
    let pretty = serde_json::to_string_pretty(&file).map_err(|error| error.to_string())?;
    let mut text = collapse_names(&pretty);
    text.push('\n');
    Ok(text)
}

/// Writes each list of names on one line, the way the formatter of JSON files does (FMT-TS), so
/// the generated file needs no second formatting step.
fn collapse_names(pretty: &str) -> String {
    let mut lines = Vec::new();
    let mut names: Option<Vec<&str>> = None;
    for line in pretty.lines() {
        let trimmed = line.trim();
        match (&mut names, trimmed) {
            (None, "\"names\": [") => names = Some(Vec::new()),
            (None, _) => lines.push(line.to_owned()),
            (Some(items), "]" | "],") => {
                let indent = " ".repeat(line.len() - trimmed.len());
                let comma = if trimmed.ends_with(',') { "," } else { "" };
                lines.push(format!("{indent}\"names\": [{}]{comma}", items.join(", ")));
                names = None;
            }
            (Some(items), item) => items.push(item.trim_end_matches(',')),
        }
    }
    lines.join("\n")
}
