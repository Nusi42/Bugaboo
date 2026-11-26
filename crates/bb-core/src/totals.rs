//! The words of the sums a total adds up (QRY-08, ADR-0050, decision 7), each written once: the
//! use case reads a request by them, and the browser asks for the sums of the columns of a list by
//! the same words (generated into the browser client).

/// How many issues: every row of a total has it.
pub const COUNT: &str = "count";

/// *Estimated time*.
pub const ESTIMATED: &str = "estimated";

/// *Time left*.
pub const TIME_LEFT: &str = "time_left";

/// *Time spent*.
pub const TIME_SPENT: &str = "time_spent";

/// The word of a sum of an own field of numbers, before the word of the field (`number:budget`).
pub const NUMBER_SUM: &str = "number:";

/// The kinds of own field a total adds up: whole numbers and decimals.
pub const SUMMED_KINDS: [crate::own_fields::FieldType; 2] = [
    crate::own_fields::FieldType::WholeNumber,
    crate::own_fields::FieldType::Decimal,
];
