//! TEST-SEARCH-ROUNDTRIP: Simple search and Advanced search never disagree, because both are one
//! tree that prints to one canonical text (QRY-02, ADR-0005). The tests here are property tests:
//! over generated trees, `parse(print(tree))` has the words of the tree; over generated texts, a
//! parse never panics, and the canonical text of an accepted text is stable. A tree that the
//! browser gives back as JSON is read, and held to the rules of a tree.

#![cfg(test)]

use bb_core as _;
use proptest::prelude::*;
use serde as _;

use bb_searchlang::{
    Comparison, Date, Decimal, Direction, Expr, FieldRef, IssueRef, MAX_DEPTH, MAX_TEXT_BYTES,
    Operand, ProjectRef, Search, SortKey, Span, Steps, Test, Text, TimeWord, Unit, parse, print,
    print_from_json, search_from_json,
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
        // Names of several words, some with a word that is a word of the language (D3).
        Just("assigned to".to_owned()),
        Just("found in release".to_owned()),
        Just("x in last".to_owned()),
        Just("a is b".to_owned()),
        Just("planned for a to z".to_owned()),
        Just("title oldest first".to_owned()),
        Just("two  spaces".to_owned()),
        "[a-z]{1,4}( (is|in|to|by|and|a|z|not|last|sort|[a-z]{1,3})){1,3}",
        Just("week".to_owned()),
        Just("ünï".to_owned()),
        "[a-zA-Z][a-zA-Z0-9_]{0,8}",
        ".{1,6}",
    ]
}

fn field() -> impl Strategy<Value = FieldRef> {
    field_word().prop_map(|word| FieldRef { word, span: span() })
}

/// An Issue ID by the rules of `bb-core`: a short name of 2 to 10 symbols, and a counting number or
/// a random code of 4 to 8 symbols without I, L, O and U, with a letter.
fn readable_id() -> impl Strategy<Value = IssueRef> {
    prop_oneof![
        "[A-Z][A-Z0-9]{1,9}-[1-9][0-9]{0,8}",
        "[A-Z][A-Z0-9]{1,9}-[A-HJKMNP-TV-Z][0-9A-HJKMNP-TV-Z]{3,7}",
    ]
    .prop_map(|text| IssueRef { text, span: span() })
}

fn number() -> impl Strategy<Value = Decimal> {
    (-999_999_999_i64..999_999_999, 0_u8..5).prop_map(|(mut value, mut scale)| {
        while scale > 0 && value % 10 == 0 {
            value /= 10;
            scale -= 1;
        }
        Decimal { value, scale }
    })
}

fn date() -> impl Strategy<Value = Date> {
    (1_u16..=9999, 1_u8..=12, 1_u8..=28).prop_map(|(year, month, day)| Date { year, month, day })
}

fn time_word() -> impl Strategy<Value = TimeWord> {
    prop::sample::select(TimeWord::ALL.to_vec())
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
        Just("web-1".to_owned()),
        Just(String::new()),
        "[a-zA-Z][a-zA-Z0-9_]{0,8}",
        ".{0,8}",
    ]
}

fn value() -> impl Strategy<Value = Operand> {
    prop_oneof![
        words().prop_map(|text| Operand::Text { text, span: span() }),
        number().prop_map(|number| Operand::Number {
            number,
            span: span()
        }),
        (0..=u32::MAX).prop_map(|minutes| Operand::Duration {
            minutes,
            span: span()
        }),
        date().prop_map(|date| Operand::Date { date, span: span() }),
        readable_id().prop_map(|id| Operand::Issue { id, span: span() }),
        Just(Operand::Me { span: span() }),
        time_word().prop_map(|word| Operand::Time { word, span: span() }),
    ]
}

fn text() -> impl Strategy<Value = Text> {
    words().prop_map(|text| Text { text, span: span() })
}

fn comparison() -> impl Strategy<Value = Comparison> {
    prop::sample::select(Comparison::ALL.to_vec())
}

fn unit() -> impl Strategy<Value = Unit> {
    prop::sample::select(Unit::ALL.to_vec())
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
        "[A-Z][A-Z0-9]{1,9}".prop_map(|text| Test::Within {
            project: ProjectRef { text, span: span() },
            span: span()
        }),
    ]
}

fn filter() -> impl Strategy<Value = Expr> {
    prop_oneof![
        (field(), test()).prop_map(|(field, test)| Expr::Field {
            field,
            test,
            span: span()
        }),
        text().prop_map(|text| Expr::Contains { text, span: span() }),
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
    filter().prop_recursive(4, 24, 4, |inner| {
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
        id: IssueRef {
            text: "AB-1".to_owned(),
            span: span(),
        },
        span: span(),
    }
}

fn direction() -> impl Strategy<Value = Direction> {
    prop::sample::select(Direction::ALL.to_vec())
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

/// The tree, the way the parser makes it, with the places of a real text.
fn parsed(tree: &Search) -> Search {
    parse(&print(tree)).unwrap()
}

/// A search with only this filter.
fn around(filter: Expr) -> Search {
    Search {
        filter: Some(filter),
        sort: Vec::new(),
        span: span(),
    }
}

/// `under` the issue with this ID, written as it is (a defect in it is the point).
fn under(text: &str) -> Search {
    around(Expr::Under {
        id: IssueRef {
            text: text.to_owned(),
            span: span(),
        },
        span: span(),
    })
}

/// A filter of the field `due` with this test.
fn due(test: Test) -> Search {
    around(Expr::Field {
        field: FieldRef {
            word: "due".to_owned(),
            span: span(),
        },
        test,
        span: span(),
    })
}

fn is(value: Operand) -> Search {
    due(Test::Is {
        value,
        span: span(),
    })
}

fn all_of(items: Vec<Expr>) -> Expr {
    Expr::And {
        items,
        span: span(),
    }
}

fn any_of(items: Vec<Expr>) -> Expr {
    Expr::Or {
        items,
        span: span(),
    }
}

/// Trees that break a rule of how `and`, `or` and `not` are built, around a filter of a good tree.
fn broken_combinations(good: &Expr) -> Vec<(&'static str, Search)> {
    vec![
        ("an and of one item", around(all_of(vec![good.clone()]))),
        ("an or of no item", around(any_of(Vec::new()))),
        (
            "an and in an and",
            around(all_of(vec![
                all_of(vec![good.clone(), placeholder()]),
                placeholder(),
            ])),
        ),
        (
            "an or in an or",
            around(any_of(vec![
                placeholder(),
                any_of(vec![good.clone(), placeholder()]),
            ])),
        ),
    ]
}

/// Trees with an ID, a short name or a count that the rules of `bb-core` and of the language refuse.
fn broken_names() -> Vec<(&'static str, Search)> {
    let steps = |count| {
        around(Expr::ConnectedTo {
            id: IssueRef {
                text: "AB-1".to_owned(),
                span: span(),
            },
            steps: Some(Steps {
                count,
                span: span(),
            }),
            span: span(),
        })
    };
    vec![
        ("an ID in lower case", under("ab-1")),
        ("an ID without a code", under("AB-")),
        ("a code with a leading zero", under("AB-01")),
        ("a code with an I", under("AB-KILO1")),
        ("a short name of one letter", under("A-1")),
        ("a short name of eleven letters", under("ABCDEFGHIJK-1")),
        ("a chain of no step", steps(0)),
        (
            "a project in lower case",
            due(Test::Within {
                project: ProjectRef {
                    text: "web".to_owned(),
                    span: span(),
                },
                span: span(),
            }),
        ),
    ]
}

/// Trees with a value that is not in its one form, or a test that holds nothing.
fn broken_values() -> Vec<(&'static str, Search)> {
    let number = |value, scale| {
        is(Operand::Number {
            number: Decimal { value, scale },
            span: span(),
        })
    };
    vec![
        (
            "a day that is not in the calendar",
            is(Operand::Date {
                date: Date {
                    year: 2026,
                    month: 2,
                    day: 30,
                },
                span: span(),
            }),
        ),
        ("a number with a zero at the end", number(10, 1)),
        (
            "a number with too many digits",
            number(1_000_000_000_000_000, 0),
        ),
        (
            "a list of no value",
            due(Test::In {
                values: Vec::new(),
                span: span(),
            }),
        ),
        (
            "a count of none",
            due(Test::InLast {
                count: 0,
                unit: Unit::Day,
                span: span(),
            }),
        ),
    ]
}

/// A tree that breaks one rule of a tree, for each of the rules.
fn broken(good: &Expr) -> Vec<(&'static str, Search)> {
    let mut all = broken_combinations(good);
    all.extend(broken_names());
    all.extend(broken_values());
    all
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn qry_03_the_json_of_a_tree_reads_back_as_the_tree(tree in search()) {
        // The browser gives the tree back as JSON; the module reads it with the reader that is
        // written from the types (`cargo xtask gen search-reader`). The places are real ones.
        let tree = parsed(&tree);
        let json = serde_json::to_string(&tree).unwrap();
        prop_assert_eq!(search_from_json(&json), Ok(tree.clone()));
        prop_assert_eq!(print_from_json(&json), Ok(print(&tree)));
    }

    #[test]
    fn qry_03_a_tree_in_json_with_other_places_has_the_same_words(tree in search()) {
        // Simple search builds trees without a text; the places in them mean nothing.
        let json = serde_json::to_string(&tree).unwrap();
        let read = search_from_json(&json).unwrap();
        prop_assert_eq!(&read, &tree);
        prop_assert!(read.same_words(&parsed(&tree)));
    }

    #[test]
    fn qry_02_a_tree_that_breaks_a_rule_is_refused_when_read(tree in search()) {
        let good = tree.filter.clone().unwrap_or_else(placeholder);
        for (rule, search) in broken(&good) {
            let json = serde_json::to_string(&search).unwrap();
            prop_assert!(search_from_json(&json).is_err(), "{rule}: {json}");
            prop_assert!(print_from_json(&json).is_err(), "{rule}");
        }
    }

    #[test]
    fn qry_02_parse_of_print_is_the_tree(tree in search()) {
        let text = print(&tree);
        let parsed = parse(&text);
        prop_assert!(parsed.is_ok(), "{text}: {:?}", parsed.err());
        let parsed = parsed.unwrap();
        prop_assert!(parsed.same_words(&tree), "{}", text);
        // A word that is not the same is seen: the places do not hide it.
        let other = Search { filter: None, ..tree.clone() };
        prop_assert_eq!(parsed.same_words(&other), tree.filter.is_none());
    }

    #[test]
    fn qry_01_a_parse_never_panics_on_any_text(any in ".{0,120}", glued in nearly_a_search()) {
        let _ = parse(&any);
        let _ = parse(&glued);
    }

    #[test]
    fn qry_01_the_canonical_text_of_an_accepted_text_is_stable(glued in nearly_a_search(), tree in search()) {
        for text in [glued, print(&tree)] {
            if let Ok(first) = parse(&text) {
                let canonical = print(&first);
                let second = parse(&canonical);
                prop_assert!(second.is_ok(), "{text} -> {canonical}");
                let second = second.unwrap();
                prop_assert!(second.same_words(&first), "{} -> {}", text, canonical);
                prop_assert_eq!(print(&second), canonical);
            }
        }
    }
}

/// A text of `levels` brackets, one inside the other, with `or` and `and` taking turns.
fn mixed_brackets(levels: usize) -> String {
    (0..levels).rev().fold("z is 0".to_owned(), |inner, level| {
        let joiner = if level % 2 == 0 { "or" } else { "and" };
        format!("(f{level} is {level} {joiner} {inner})")
    })
}

#[test]
fn qry_03_a_text_with_many_brackets_goes_through_the_module() {
    // The depth of a text is the parser's rule; the module reads and prints every tree it makes.
    for levels in [1, 16, 20, MAX_DEPTH] {
        let text = mixed_brackets(levels);
        let tree = parse(&text).unwrap();
        let json = serde_json::to_string(&tree).unwrap();
        assert_eq!(search_from_json(&json), Ok(tree.clone()), "{levels}");
        assert_eq!(print_from_json(&json), Ok(print(&tree)), "{levels}");
    }
}

#[test]
fn qry_03_a_tree_nested_deeper_than_the_parser_reads_has_no_text() {
    let mut item = placeholder();
    for _ in 0..=MAX_DEPTH {
        item = Expr::Not {
            item: Box::new(item),
            span: span(),
        };
    }
    let json = serde_json::to_string(&Search {
        filter: Some(item),
        sort: Vec::new(),
        span: span(),
    })
    .unwrap();
    // It breaks no rule of a tree, but the language does not read its text back.
    assert!(search_from_json(&json).is_ok());
    assert!(print_from_json(&json).is_err());
}

#[test]
fn qry_03_a_tree_whose_text_is_too_long_has_no_text() {
    let filter = || Expr::Under {
        id: IssueRef {
            text: "AB-1".to_owned(),
            span: span(),
        },
        span: span(),
    };
    let filters = (0..MAX_TEXT_BYTES / 4).map(|_| filter()).collect();
    let json = serde_json::to_string(&around(Expr::And {
        items: filters,
        span: span(),
    }))
    .unwrap();
    assert!(search_from_json(&json).is_ok());
    assert!(print(&search_from_json(&json).unwrap()).len() > MAX_TEXT_BYTES);
    assert!(print_from_json(&json).is_err());
}

/// A search with only these keys of `sort by`.
fn sorted_by(keys: &[(&str, Option<Direction>)]) -> Search {
    Search {
        filter: None,
        sort: keys
            .iter()
            .map(|(word, direction)| SortKey {
                field: FieldRef {
                    word: (*word).to_owned(),
                    span: span(),
                },
                direction: *direction,
                span: span(),
            })
            .collect(),
        span: span(),
    }
}

#[test]
fn qry_02_a_key_whose_name_begins_like_a_direction_reads_back_as_a_key() {
    // Found by the property test: `sort by status, z to` read `z to` as the start of `Z to A`.
    // Since D3 a name may have spaces, so a name that begins with the words of a direction is
    // written in quotes as a key of `sort by` (ADR-0005 amendment M5, decision 11).
    let names = [
        "z to",
        "Z to A",
        "a to",
        "a to z",
        "highest",
        "Highest first",
        "lowest",
        "newest",
        "oldest first",
        "me",
        "today",
    ];
    let mut wrong = Vec::new();
    for name in names {
        for before in [None, Some(Direction::AToZ)] {
            for keys in [
                vec![("status", before), (name, None)],
                vec![("status", before), (name, Some(Direction::ZToA))],
                vec![(name, None), ("status", before)],
                vec![(name, None)],
            ] {
                let tree = sorted_by(&keys);
                let text = print(&tree);
                if !parse(&text).is_ok_and(|back| back.same_words(&tree)) {
                    wrong.push(text);
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}
