//! TEST-SEARCH-ROUNDTRIP: Simple search and Advanced search never disagree, because both are one
//! tree that prints to one canonical text (QRY-02, ADR-0005). The tests here are property tests:
//! over generated trees, `parse(print(tree)) == tree`; over generated texts, a parse never panics,
//! and the canonical text of an accepted text is stable.

#![cfg(test)]

use proptest::prelude::*;
use serde as _;
use serde_json as _;

use bb_searchlang::{
    Comparison, Date, Direction, Expr, FieldRef, Number, ReadableId, Search, ShortName, SortKey,
    Span, Steps, Test, Text, TimeWord, Unit, Value, parse, print,
};

fn span() -> Span {
    Span::default()
}

/// A word of a field: plain, with spaces, a word of the language, with quotes, with letters of
/// other languages.
fn field_word() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("status".to_owned()),
        Just("assignedTo".to_owned()),
        Just("Story points".to_owned()),
        Just("week".to_owned()),
        Just("ünï".to_owned()),
        "[a-zA-Z][a-zA-Z0-9_]{0,8}",
        ".{1,6}",
    ]
}

fn field() -> impl Strategy<Value = FieldRef> {
    field_word().prop_map(|word| FieldRef { word, span: span() })
}

fn readable_id() -> impl Strategy<Value = ReadableId> {
    "[A-Z][A-Z0-9]{0,9}-[A-Z0-9]{1,8}".prop_map(|text| ReadableId { text, span: span() })
}

fn number() -> impl Strategy<Value = Number> {
    (-999_999_999_i64..999_999_999, 0_u8..5).prop_map(|(mut value, mut scale)| {
        while scale > 0 && value % 10 == 0 {
            value /= 10;
            scale -= 1;
        }
        Number { value, scale }
    })
}

fn date() -> impl Strategy<Value = Date> {
    (1_u16..=9999, 1_u8..=12, 1_u8..=28).prop_map(|(year, month, day)| Date { year, month, day })
}

fn time_word() -> impl Strategy<Value = TimeWord> {
    prop_oneof![
        Just(TimeWord::Today),
        Just(TimeWord::Yesterday),
        Just(TimeWord::ThisWeek),
        Just(TimeWord::LastWeek),
        Just(TimeWord::ThisMonth),
    ]
}

/// A text: plain words, words of the language, numbers, days, IDs, anything.
fn words() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("Done".to_owned()),
        Just("In progress".to_owned()),
        Just("me".to_owned()),
        Just("this week".to_owned()),
        Just("and".to_owned()),
        Just("404".to_owned()),
        Just("2026-10-25".to_owned()),
        Just("WEB-1".to_owned()),
        Just(String::new()),
        "[a-zA-Z][a-zA-Z0-9_]{0,8}",
        ".{0,8}",
    ]
}

fn value() -> impl Strategy<Value = Value> {
    prop_oneof![
        words().prop_map(|text| Value::Text { text, span: span() }),
        number().prop_map(|number| Value::Number {
            number,
            span: span()
        }),
        date().prop_map(|date| Value::Date { date, span: span() }),
        readable_id().prop_map(|id| Value::Issue { id, span: span() }),
        Just(Value::Me { span: span() }),
        time_word().prop_map(|word| Value::Time { word, span: span() }),
    ]
}

fn text() -> impl Strategy<Value = Text> {
    words().prop_map(|text| Text { text, span: span() })
}

fn comparison() -> impl Strategy<Value = Comparison> {
    prop_oneof![
        Just(Comparison::Less),
        Just(Comparison::Greater),
        Just(Comparison::LessOrEqual),
        Just(Comparison::GreaterOrEqual),
    ]
}

fn unit() -> impl Strategy<Value = Unit> {
    prop_oneof![Just(Unit::Day), Just(Unit::Week), Just(Unit::Month)]
}

fn test() -> impl Strategy<Value = Test> {
    prop_oneof![
        value().prop_map(|value| Test::Is {
            value,
            span: span()
        }),
        value().prop_map(|value| Test::IsNot {
            value,
            span: span()
        }),
        (comparison(), value()).prop_map(|(operator, value)| Test::Compare {
            operator,
            value,
            span: span()
        }),
        prop::collection::vec(value(), 1..4).prop_map(|values| Test::In {
            values,
            span: span()
        }),
        text().prop_map(|text| Test::Contains { text, span: span() }),
        Just(Test::IsEmpty { span: span() }),
        Just(Test::IsNotEmpty { span: span() }),
        (1_u32..100_000, unit()).prop_map(|(count, unit)| Test::InLast {
            count,
            unit,
            span: span()
        }),
        text().prop_map(|group| Test::MemberOf {
            group,
            span: span()
        }),
        "[A-Z][A-Z0-9]{0,9}".prop_map(|text| Test::Within {
            project: ShortName { text, span: span() },
            span: span()
        }),
    ]
}

fn condition() -> impl Strategy<Value = Expr> {
    prop_oneof![
        (field(), test()).prop_map(|(field, test)| Expr::Field {
            field,
            test,
            span: span()
        }),
        readable_id().prop_map(|id| Expr::Under { id, span: span() }),
        readable_id().prop_map(|id| Expr::Blocks { id, span: span() }),
        readable_id().prop_map(|id| Expr::BlockedBy { id, span: span() }),
        (readable_id(), prop::option::of(1_u32..1000)).prop_map(|(id, steps)| Expr::ConnectedTo {
            id,
            steps: steps.map(|count| Steps {
                count,
                span: span()
            }),
            span: span()
        }),
    ]
}

/// Trees, built the way the parser and the editor of Simple search build them: no `and` directly
/// in an `and`, no `or` directly in an `or`, at least two items.
fn expression() -> impl Strategy<Value = Expr> {
    condition().prop_recursive(4, 24, 4, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 2..4).prop_map(|items| {
                let mut items = items.into_iter();
                let first = items.next().unwrap_or_else(placeholder);
                Expr::any(first, items.collect())
            }),
            prop::collection::vec(inner.clone(), 2..4).prop_map(|items| {
                let mut items = items.into_iter();
                let first = items.next().unwrap_or_else(placeholder);
                Expr::all(first, items.collect())
            }),
            inner.prop_map(|item| Expr::Not {
                item: Box::new(item),
                span: span()
            }),
        ]
    })
}

fn placeholder() -> Expr {
    Expr::Under {
        id: ReadableId {
            text: "A-1".to_owned(),
            span: span(),
        },
        span: span(),
    }
}

fn direction() -> impl Strategy<Value = Direction> {
    prop_oneof![
        Just(Direction::HighestFirst),
        Just(Direction::LowestFirst),
        Just(Direction::NewestFirst),
        Just(Direction::OldestFirst),
        Just(Direction::AToZ),
        Just(Direction::ZToA),
    ]
}

fn search() -> impl Strategy<Value = Search> {
    (
        prop::option::of(expression()),
        prop::collection::vec(
            (field(), prop::option::of(direction())).prop_map(|(field, direction)| SortKey {
                field,
                direction,
                span: span(),
            }),
            0..4,
        ),
    )
        .prop_map(|(filter, sort)| Search {
            filter,
            sort,
            span: span(),
        })
}

/// Pieces of a search, glued at random: most texts are nearly right, which reaches deeper than
/// random characters do.
fn nearly_a_search() -> impl Strategy<Value = String> {
    let pieces = prop::sample::select(vec![
        "(",
        ")",
        ",",
        "<",
        ">=",
        "and",
        "or",
        "not",
        "is",
        "in",
        "last",
        "contains",
        "member",
        "of",
        "within",
        "under",
        "blocks",
        "blocked",
        "by",
        "connected",
        "to",
        "sort",
        "highest",
        "first",
        "A",
        "Z",
        "status",
        "me",
        "today",
        "this",
        "week",
        "7",
        "2.5",
        "-3",
        "WEB",
        "WEB-1",
        "2026-10-25",
        "\"text\"",
        "\"open",
        "empty",
        "steps",
        "days",
        "ü",
        "=",
        "\\",
    ]);
    prop::collection::vec(pieces, 0..14).prop_map(|pieces| pieces.join(" "))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn search_roundtrip_parse_of_print_is_the_tree(tree in search()) {
        let text = print(&tree);
        let parsed = parse(&text);
        prop_assert!(parsed.is_ok(), "{text}: {:?}", parsed.err());
        prop_assert_eq!(parsed.unwrap(), tree, "{}", text);
    }

    #[test]
    fn search_roundtrip_a_parse_never_panics_on_any_text(any in ".{0,120}", glued in nearly_a_search()) {
        let _ = parse(&any);
        let _ = parse(&glued);
    }

    #[test]
    fn search_roundtrip_the_canonical_text_of_an_accepted_text_is_stable(glued in nearly_a_search(), tree in search()) {
        for text in [glued, print(&tree)] {
            if let Ok(first) = parse(&text) {
                let canonical = print(&first);
                let second = parse(&canonical);
                prop_assert!(second.is_ok(), "{text} -> {canonical}");
                let second = second.unwrap();
                prop_assert_eq!(&second, &first, "{} -> {}", text, canonical);
                prop_assert_eq!(print(&second), canonical);
            }
        }
    }
}
