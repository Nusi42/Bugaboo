//! The search language, tested through its public functions: the example of QRY-01 and every helper
//! of QRY-04 parse, check and print back to their canonical text (ADR-0005, M5 WP-02).

#![cfg(test)]

use proptest as _;
use serde as _;
use serde_json as _;

use bb_searchlang::vectors::{ACCEPTED, REFUSED, sample_catalogue};
use bb_searchlang::{
    Expr, FieldCatalogue, MAX_CONDITIONS, MAX_STEPS, Mistake, Suggestion, Test, check, parse,
    print, suggest,
};

/// A text parsed and checked against the sample catalogue, printed.
fn checked(text: &str) -> Result<String, Vec<Mistake>> {
    let search = parse(text).map_err(|mistake| vec![mistake])?;
    check(search, &sample_catalogue()).map(|search| print(search.search()))
}

fn mistakes_of(text: &str) -> Vec<Mistake> {
    checked(text).expect_err(text)
}

#[test]
fn qry_01_the_example_parses_checks_and_prints_back() {
    let text = "project within WEB and statusGroup is not Done and assignedTo is me and changed in last 7 days sort by priority, highest first";
    assert_eq!(checked(text).as_deref(), Ok(text));
}

#[test]
fn qry_04_every_helper_parses_checks_and_prints_back() {
    for text in [
        "assignedTo is me",
        "created is today",
        "created is yesterday",
        "created is this week",
        "created is last week",
        "created is this month",
        "changed in last 7 days",
        "changed in last 2 weeks",
        "changed in last 3 months",
        "assignedTo member of \"Backend\"",
        "project within WEB",
        "under WEB-7",
        "blocks WEB-1",
        "blocked by WEB-1",
        "connected to WEB-7",
        "connected to WEB-7 within 3 steps",
        "title contains \"crash on save\"",
        "parent is WEB-K7Q2X",
    ] {
        assert_eq!(checked(text).as_deref(), Ok(text), "{text}");
    }
}

#[test]
fn every_accepted_text_prints_to_a_canonical_text_that_prints_to_itself() {
    for text in ACCEPTED {
        let canonical = checked(text).unwrap_or_else(|mistakes| panic!("{text}: {mistakes:?}"));
        assert_eq!(
            checked(&canonical).as_deref(),
            Ok(canonical.as_str()),
            "{text}"
        );
    }
}

#[test]
fn every_refused_text_has_mistakes_with_a_key_and_a_place_inside_the_text() {
    for text in REFUSED {
        for mistake in mistakes_of(text) {
            assert!(mistake.key.starts_with("search.mistake."), "{text}");
            assert!(mistake.span.start <= mistake.span.end, "{text}");
            assert!(mistake.span.end <= text.len(), "{text}: {mistake:?}");
        }
    }
}

#[test]
fn the_canonical_text_has_one_spelling_for_everything() {
    for (written, canonical) in [
        ("STATUS  IS   NOT  Open", "STATUS is not Open"),
        (
            "status IS Open AND priority IS high",
            "status is Open and priority is high",
        ),
        ("status is \"Open\"", "status is Open"),
        ("status is \"In progress\"", "status is \"In progress\""),
        ("estimate is 1.50", "estimate is 1.5"),
        ("status in ( Open ,Done )", "status in (Open, Done)"),
        ("(status is Open)", "status is Open"),
        (
            "(status is Open and priority is high) and due is today",
            "status is Open and priority is high and due is today",
        ),
        (
            "status is Open and (priority is high or due is today)",
            "status is Open and (priority is high or due is today)",
        ),
        ("not (status is Open)", "not status is Open"),
        (
            "not (status is Open and priority is high)",
            "not (status is Open and priority is high)",
        ),
        ("changed IN LAST 1 days", "changed in last 1 day"),
        (
            "sort by due oldest first , title",
            "sort by due, oldest first, title",
        ),
        ("status is \"and\"", "status is \"and\""),
        ("status is \"highest\"", "status is highest"),
    ] {
        let search = parse(written).unwrap_or_else(|mistake| panic!("{written}: {mistake:?}"));
        assert_eq!(print(&search), canonical, "{written}");
    }
}

#[test]
fn a_mistake_names_the_field_and_offers_what_may_be_meant() {
    let mistakes = mistakes_of("prio is high");
    assert_eq!(mistakes.len(), 1);
    assert_eq!(mistakes[0].key, "search.mistake.unknown-field-did-you-mean");
    assert_eq!(mistakes[0].args["field"], "prio");
    assert_eq!(mistakes[0].args["suggestion"], "priority");
    assert_eq!(mistakes[0].suggestions, ["priority"]);
    assert_eq!((mistakes[0].span.start, mistakes[0].span.end), (0, 4));
}

#[test]
fn qry_05_a_hidden_field_and_an_unknown_field_give_the_same_mistake() {
    // "reportedBy" exists, but this person may not read it, so the caller leaves it out of the
    // catalogue. A second catalogue never had it. The checker gets nothing else to go by.
    let without = |words: &[&str]| FieldCatalogue {
        fields: sample_catalogue()
            .fields
            .into_iter()
            .filter(|field| !words.contains(&field.word.as_str()))
            .collect(),
    };
    let hidden = without(&["reportedBy"]);
    let never_existed = without(&["reportedBy"]);
    for text in ["reportedBy is me", "reportedBy is not empty sort by title"] {
        let search = parse(text).unwrap();
        let from_hidden = check(search.clone(), &hidden).unwrap_err();
        let from_unknown = check(search, &never_existed).unwrap_err();
        assert_eq!(from_hidden, from_unknown, "{text}");
        assert_eq!(from_hidden[0].key, "search.mistake.unknown-field");
        assert_eq!(from_hidden[0].args["field"], "reportedBy");
        assert!(from_hidden[0].suggestions.is_empty());
    }
    // Neither the suggestions nor the offers give the hidden field away.
    let offered = suggest("repo", 4, &hidden);
    assert!(offered.items.is_empty());
    let close = mistakes_of("reportedby is me");
    assert_eq!(close[0].suggestions, ["reportedBy"]);
    let search = parse("reportedby is me").unwrap();
    let hidden_close = check(search, &hidden).unwrap_err();
    assert!(hidden_close[0].suggestions.is_empty());
}

#[test]
fn the_limits_fifty_filters_pass_and_fifty_one_fail() {
    let many = |count: usize| vec!["status is Open"; count].join(" and ");
    assert!(checked(&many(MAX_CONDITIONS)).is_ok());
    let text = many(MAX_CONDITIONS + 1);
    let mistakes = mistakes_of(&text);
    assert_eq!(mistakes.len(), 1);
    assert_eq!(mistakes[0].key, "search.mistake.too-many-conditions");
    assert_eq!(mistakes[0].args["limit"], MAX_CONDITIONS.to_string());
    // The mistake stands on the first filter over the limit.
    assert_eq!(mistakes[0].span.end, text.len());
}

#[test]
fn the_limits_five_steps_pass_and_six_fail() {
    let within = |steps: u32| format!("connected to WEB-7 within {steps} steps");
    assert!(checked(&within(MAX_STEPS)).is_ok());
    let mistakes = mistakes_of(&within(MAX_STEPS + 1));
    assert_eq!(mistakes.len(), 1);
    assert_eq!(mistakes[0].key, "search.mistake.too-many-steps");
    assert_eq!(mistakes[0].args["limit"], MAX_STEPS.to_string());
}

#[test]
fn a_text_nested_too_deep_is_a_mistake_and_never_a_crash() {
    let deep = format!("{}status is Open{}", "(".repeat(5000), ")".repeat(5000));
    assert_eq!(parse(&deep).unwrap_err().key, "search.mistake.too-deep");
    let nots = format!("{}status is Open", "not ".repeat(5000));
    assert_eq!(parse(&nots).unwrap_err().key, "search.mistake.too-deep");
}

#[test]
fn qry_03_the_tree_keeps_where_each_word_stands() {
    let text = "status is Open and title contains \"x y\"";
    let Expr::And { items, .. } = parse(text).unwrap().filter.unwrap() else {
        panic!("an and");
    };
    let Expr::Field { field, test, span } = &items[1] else {
        panic!("a field");
    };
    assert_eq!(&text[field.span.start..field.span.end], "title");
    assert_eq!(&text[span.start..span.end], "title contains \"x y\"");
    let Test::Contains { text: found, .. } = test else {
        panic!("contains");
    };
    assert_eq!(&text[found.span.start..found.span.end], "\"x y\"");
}

/// The words offered at the bar in `marked`, and the bytes they replace.
fn suggestions_at(marked: &str) -> (Vec<String>, (usize, usize)) {
    let at = marked.find('|').unwrap();
    let text = marked.replacen('|', "", 1);
    let found = suggest(&text, at, &sample_catalogue());
    let words = found
        .items
        .iter()
        .map(|item| match item {
            Suggestion::Word { text, .. } => text.clone(),
            Suggestion::Ask {
                what,
                starting_with,
            } => format!("ask {what:?} {starting_with}"),
        })
        .collect();
    (words, (found.replace.start, found.replace.end))
}

#[test]
fn suggestions_after_a_typed_start_offer_the_fields_that_begin_so() {
    let (words, replace) = suggestions_at("assi|");
    assert_eq!(words, ["assignedTo"]);
    assert_eq!(replace, (0, 4));
}

#[test]
fn suggestions_after_a_field_offer_what_it_allows() {
    let (words, replace) = suggestions_at("assignedTo |");
    assert_eq!(
        words,
        [
            "is",
            "is not",
            "in",
            "member of",
            "is empty",
            "is not empty"
        ]
    );
    assert_eq!(replace, (11, 11));
    let (words, _) = suggestions_at("title |");
    assert!(words.contains(&"contains".to_owned()));
    assert!(!words.contains(&"member of".to_owned()));
}

#[test]
fn suggestions_after_is_offer_the_choices_of_the_field() {
    let (words, _) = suggestions_at("priority is |");
    assert_eq!(
        words,
        ["not", "empty", "lowest", "low", "normal", "high", "highest"]
    );
    let (words, _) = suggestions_at("priority is hi|");
    assert_eq!(words, ["high", "highest"]);
    let (words, _) = suggestions_at("assignedTo is |");
    assert_eq!(words, ["not", "empty", "me", "ask People "]);
}

#[test]
fn suggestions_inside_quotes_offer_the_values_and_replace_the_quoted_text() {
    let (words, replace) = suggestions_at("status is \"In pr|\" and");
    assert_eq!(words, ["\"In progress\""]);
    assert_eq!(replace, (10, 17));
    let (words, replace) = suggestions_at("status is \"In|");
    assert_eq!(words, ["\"In progress\""]);
    assert_eq!(replace, (10, 13));
    let (words, _) = suggestions_at("assignedTo is \"An|");
    assert_eq!(words, ["ask People An"]);
    let (words, _) = suggestions_at("assignedTo member of \"Bac|");
    assert_eq!(words, ["ask Groups Bac"]);
}

#[test]
fn suggestions_name_what_the_server_alone_knows_as_a_request() {
    let (words, _) = suggestions_at("project within |");
    assert_eq!(words, ["ask Projects "]);
    let (words, _) = suggestions_at("parent is |");
    assert!(words.contains(&"ask Issues ".to_owned()));
    let (words, _) = suggestions_at("under |");
    assert_eq!(words, ["ask Issues "]);
}

#[test]
fn suggestions_after_a_whole_filter_offer_the_joining_words() {
    let (words, _) = suggestions_at("status is Open |");
    assert_eq!(words, ["and", "or", "sort by"]);
    let (words, _) = suggestions_at("(status is Open |");
    assert_eq!(words, ["and", "or", ")"]);
    let (words, _) = suggestions_at("status is Open sort by priority |");
    assert!(words.contains(&"highest first".to_owned()));
    assert!(words.contains(&",".to_owned()));
    let (words, _) = suggestions_at("status is Open sort by priority, highest |");
    assert_eq!(words, ["first"]);
}

#[test]
fn suggestions_for_a_text_that_is_wrong_before_the_cursor_are_empty() {
    let (words, _) = suggestions_at("nothing |");
    assert_eq!(words, Vec::<String>::new());
    let (words, _) = suggestions_at("status = Open and |");
    assert_eq!(words, Vec::<String>::new());
}

#[test]
fn the_cursor_may_stand_anywhere_in_any_text_without_a_crash() {
    for text in ACCEPTED.iter().chain(REFUSED) {
        for cursor in 0..=text.len() + 2 {
            let _ = suggest(text, cursor, &sample_catalogue());
        }
    }
    let _ = suggest("ünï is é ö", 3, &sample_catalogue());
}
