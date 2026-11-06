//! A number written for a choice in a search (TIME-13): the digits stay as typed, so `0410` finds the
//! choice numbered `0410` and not the one numbered `410`.

#![cfg(test)]

use bb_core as _;
use proptest as _;
use serde as _;
use serde_json as _;

use bb_searchlang::{Field, FieldKind, ReadableFields, check, parse_with, print};

#[test]
fn time_13_a_number_written_for_a_choice_keeps_its_digits_and_prints_back_quoted() {
    let fields = ReadableFields {
        fields: vec![
            Field::new("budget", FieldKind::Choice).with_choices(&[
                "0410",
                "Padded",
                "0410 Padded",
                "410",
                "Plain",
            ]),
            Field::new("estimate", FieldKind::Number),
        ],
    };
    let answer = |text: &str| {
        let search = parse_with(text, &fields).map_err(|mistake| vec![mistake])?;
        check(search, &fields).map(|search| print(search.search()))
    };
    // The digits are the choice as typed: 0410 is not 410.
    assert_eq!(
        answer("budget is 0410").as_deref(),
        Ok("budget is \"0410\"")
    );
    assert_eq!(answer("budget is 410").as_deref(), Ok("budget is \"410\""));
    assert_eq!(
        answer("budget is \"0410 Padded\"").as_deref(),
        Ok("budget is \"0410 Padded\"")
    );
    // What it prints is read again as the same search.
    let printed = answer("budget is 0410").unwrap_or_default();
    assert_eq!(answer(&printed), Ok(printed));
    // A field of numbers still reads a number as a number.
    assert!(answer("estimate is 0410").is_ok());
}
