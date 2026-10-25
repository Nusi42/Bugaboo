//! The search language, tested through its public functions: the example of QRY-01 and every helper
//! of QRY-04 parse, check and print back to their canonical text (ADR-0005, M5 WP-02).

#![cfg(test)]

use bb_core as _;
use proptest as _;
use serde as _;
use serde_json as _;

use bb_searchlang::messages;
use bb_searchlang::vectors::{ACCEPTED, REFUSED, sample_fields};
use bb_searchlang::{
    Expr, Field, FieldKind, MAX_FILTERS, MAX_SORT_KEYS, MAX_STEPS, MAX_TEXT_BYTES, Mistake,
    Operand, ReadableFields, Suggestion, Test, check, parse, parse_with, print, suggest,
};

/// A text parsed and checked against the sample fields, printed.
fn checked(text: &str) -> Result<String, Vec<Mistake>> {
    let search = parse_with(text, &sample_fields()).map_err(|mistake| vec![mistake])?;
    check(search, &sample_fields()).map(|search| print(search.search()))
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
fn qry_01_every_accepted_text_prints_to_a_canonical_text_that_prints_to_itself() {
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
fn qry_01_every_refused_text_has_mistakes_with_a_key_and_a_place_inside_the_text() {
    for text in REFUSED {
        for mistake in mistakes_of(text) {
            assert!(mistake.key.starts_with("search.mistake."), "{text}");
            assert!(mistake.span.start <= mistake.span.end, "{text}");
            assert!(mistake.span.end <= text.len(), "{text}: {mistake:?}");
        }
    }
}

#[test]
fn qry_02_the_canonical_text_has_one_spelling_for_everything() {
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
fn qry_01_a_mistake_names_the_field_and_offers_what_may_be_meant() {
    let mistakes = mistakes_of("prio is high");
    assert_eq!(mistakes.len(), 1);
    assert_eq!(mistakes[0].key, messages::UNKNOWN_FIELD_DID_YOU_MEAN);
    assert_eq!(mistakes[0].args["field"], "prio");
    assert_eq!(mistakes[0].args["suggestion"], "priority");
    assert_eq!(mistakes[0].suggestions, ["priority"]);
    assert_eq!((mistakes[0].span.start, mistakes[0].span.end), (0, 4));
}

/// The fields of a person who may not read `reportedBy`. They are written out here, so a change of
/// the sample cannot make the test pass for another reason.
fn without_reported_by() -> ReadableFields {
    ReadableFields {
        fields: vec![
            Field::new("title", FieldKind::Text),
            Field::new("status", FieldKind::Choice).with_choices(&["Open", "Done"]),
            Field::new("assignedTo", FieldKind::Person),
            Field::new("reportedOn", FieldKind::Date),
        ],
    }
}

/// What a mistake says apart from the word it is about: its key, what it suggests, how long the
/// word is and every argument but the word itself.
fn shape(mistake: &Mistake) -> (String, Vec<String>, usize, Vec<(String, String)>) {
    let args = mistake
        .args
        .iter()
        .filter(|(name, _)| name.as_str() != "field")
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();
    (
        mistake.key.clone(),
        mistake.suggestions.clone(),
        mistake.span.end - mistake.span.start,
        args,
    )
}

#[test]
fn qry_05_a_hidden_field_and_an_unknown_field_give_the_same_mistake() {
    let readable = without_reported_by();
    // Both words are as close to the visible `reportedOn`, and as long, as each other.
    for (hidden, invented) in [
        ("reportedBy is me", "reportedXx is me"),
        (
            "title is x sort by reportedBy",
            "title is x sort by reportedXx",
        ),
        ("reportedBy is not empty", "reportedXx is not empty"),
    ] {
        let from_hidden = check(parse(hidden).unwrap(), &readable).unwrap_err();
        let from_invented = check(parse(invented).unwrap(), &readable).unwrap_err();
        assert_eq!(from_hidden.len(), from_invented.len(), "{hidden}");
        assert_eq!(
            shape(&from_hidden[0]),
            shape(&from_invented[0]),
            "{hidden} and {invented}"
        );
        assert_eq!(
            from_hidden[0].key,
            messages::UNKNOWN_FIELD_DID_YOU_MEAN,
            "{hidden}"
        );
        assert_eq!(from_hidden[0].args["field"], "reportedBy");
        assert_eq!(from_invented[0].args["field"], "reportedXx");
    }
    // A text where nothing visible is close: the same plain mistake for both.
    let nothing = ReadableFields::default();
    let from_hidden = check(parse("reportedBy is me").unwrap(), &nothing).unwrap_err();
    let from_invented = check(parse("qqqqqqqqqq is me").unwrap(), &nothing).unwrap_err();
    assert_eq!(shape(&from_hidden[0]), shape(&from_invented[0]));
    assert_eq!(from_hidden[0].key, messages::UNKNOWN_FIELD);
    // Neither the suggestions of a mistake nor the offers at the cursor give the field away.
    for text in ["reportedby is me", "reportedB is me", "reported is me"] {
        let mistakes = check(parse(text).unwrap(), &readable).unwrap_err();
        assert!(
            mistakes[0]
                .suggestions
                .iter()
                .all(|word| word != "reportedBy"),
            "{text}"
        );
    }
    let offered = suggest("reportedB", 9, &readable);
    assert!(offered.items.iter().all(|offer| match offer {
        Suggestion::Word { text, .. } => text != "reportedBy",
        Suggestion::Ask { .. } => true,
    }));
    // The person who may read it gets it offered, so the checks above can fail.
    let close = mistakes_of("reportedby is me");
    assert_eq!(close[0].suggestions, ["reportedBy"]);
}

#[test]
fn qry_01_the_limits_fifty_filters_pass_and_fifty_one_fail() {
    let many = |count: usize| vec!["status is Open"; count].join(" and ");
    assert!(checked(&many(MAX_FILTERS)).is_ok());
    let text = many(MAX_FILTERS + 1);
    let mistakes = mistakes_of(&text);
    assert_eq!(mistakes.len(), 1);
    assert_eq!(mistakes[0].key, messages::TOO_MANY_FILTERS);
    assert_eq!(mistakes[0].args["limit"], MAX_FILTERS.to_string());
    // The mistake stands on the first filter over the limit.
    assert_eq!(mistakes[0].span.end, text.len());
}

#[test]
fn qry_01_the_values_of_a_list_count_against_the_limit_of_filters() {
    let list = |count: usize| format!("status in ({})", vec!["Open"; count].join(", "));
    assert!(checked(&list(MAX_FILTERS)).is_ok());
    let mistakes = mistakes_of(&list(MAX_FILTERS + 1));
    assert_eq!(mistakes.len(), 1);
    assert_eq!(mistakes[0].key, messages::TOO_MANY_FILTERS);
    // A list and a filter share the limit.
    let both = format!("title is x and {}", list(MAX_FILTERS));
    assert_eq!(mistakes_of(&both)[0].key, messages::TOO_MANY_FILTERS);
}

#[test]
fn qry_01_nothing_is_looked_at_beyond_the_limit_of_filters() {
    // Each of these would need a search for close words; after the limit there is none.
    let text = vec!["statuz is Open"; MAX_FILTERS * 4].join(" and ");
    let mistakes = mistakes_of(&text);
    assert_eq!(mistakes.len(), MAX_FILTERS + 1);
    assert_eq!(
        mistakes
            .iter()
            .filter(|mistake| mistake.key == messages::TOO_MANY_FILTERS)
            .count(),
        1
    );
}

#[test]
fn qry_01_five_sort_keys_pass_and_six_fail() {
    let keys = |count: usize| format!("sort by {}", vec!["title"; count].join(", "));
    assert!(checked(&keys(MAX_SORT_KEYS)).is_ok());
    let mistakes = mistakes_of(&keys(MAX_SORT_KEYS + 1));
    assert_eq!(mistakes.len(), 1);
    assert_eq!(mistakes[0].key, messages::TOO_MANY_SORT_KEYS);
    assert_eq!(mistakes[0].args["limit"], MAX_SORT_KEYS.to_string());
}

#[test]
fn qry_01_a_text_over_the_limit_is_refused_before_anything_is_read_of_it() {
    // The most filters, each with a field, an operator and a usual value, fit with room to spare.
    let usual = vec!["assignedTo is \"Marie Curie-Sklodowska\""; MAX_FILTERS].join(" and ");
    assert!(usual.len() < MAX_TEXT_BYTES, "{}", usual.len());
    // A text of exactly the limit is read; one byte more is not.
    let padded = |bytes: usize| format!("title is x{}", " ".repeat(bytes - "title is x".len()));
    assert!(parse(&padded(MAX_TEXT_BYTES)).is_ok());
    let long = padded(MAX_TEXT_BYTES + 1);
    let mistake = parse(&long).unwrap_err();
    assert_eq!(mistake.key, messages::TOO_LONG);
    assert_eq!(mistake.span.start, MAX_TEXT_BYTES);
    assert_eq!(mistake.span.end, long.len());
    // The place is a place of characters, also when the limit falls inside one.
    let wide = "ü".repeat(MAX_TEXT_BYTES);
    let mistake = parse(&wide).unwrap_err();
    assert_eq!(mistake.key, messages::TOO_LONG);
    assert!(wide.is_char_boundary(mistake.span.start));
    // Nothing is offered for it either.
    assert!(suggest(&long, 3, &sample_fields()).items.is_empty());
}

#[test]
fn qry_01_a_long_unknown_word_costs_no_search_for_close_words() {
    let word = "x".repeat(MAX_TEXT_BYTES - " is 1".len());
    let mistakes = mistakes_of(&format!("{word} is 1"));
    assert_eq!(mistakes[0].key, messages::UNKNOWN_FIELD);
    assert!(mistakes[0].suggestions.is_empty());
}

#[test]
fn qry_01_the_limits_five_steps_pass_and_six_fail() {
    let within = |steps: u32| format!("connected to WEB-7 within {steps} steps");
    assert!(checked(&within(MAX_STEPS)).is_ok());
    let mistakes = mistakes_of(&within(MAX_STEPS + 1));
    assert_eq!(mistakes.len(), 1);
    assert_eq!(mistakes[0].key, messages::TOO_MANY_STEPS);
    assert_eq!(mistakes[0].args["limit"], MAX_STEPS.to_string());
}

#[test]
fn qry_01_a_text_nested_too_deep_is_a_mistake_and_never_a_crash() {
    let deep = format!("{}status is Open{}", "(".repeat(900), ")".repeat(900));
    assert_eq!(parse(&deep).unwrap_err().key, messages::TOO_DEEP);
    let nots = format!("{}status is Open", "not ".repeat(900));
    assert_eq!(parse(&nots).unwrap_err().key, messages::TOO_DEEP);
    // More nesting than the text may hold is a text that is too long, and is never read.
    let long = format!("{}status is Open", "not ".repeat(MAX_TEXT_BYTES));
    assert_eq!(parse(&long).unwrap_err().key, messages::TOO_LONG);
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

#[test]
fn qry_04_a_value_is_read_by_its_field() {
    let fields = sample_fields();
    let value_of = |text: &str, fields: Option<&ReadableFields>| {
        let search = match fields {
            Some(fields) => parse_with(text, fields),
            None => parse(text),
        }
        .unwrap();
        let Some(Expr::Field {
            test: Test::Is { value, .. },
            ..
        }) = search.filter
        else {
            panic!("{text}: an `is` test");
        };
        value
    };
    // Where the field holds issues, an ID is an ID, in any case, written in capitals.
    let Operand::Issue { id, .. } = value_of("parent is web-142", Some(&fields)) else {
        panic!("an issue");
    };
    assert_eq!(id.text, "WEB-142");
    assert_eq!(
        checked("parent is web-142").as_deref(),
        Ok("parent is WEB-142")
    );
    // Where it does not, a word that looks like an ID stays the text it is.
    let Operand::Text { text, .. } = value_of("tag is re-test", Some(&fields)) else {
        panic!("a text");
    };
    assert_eq!(text, "re-test");
    assert_eq!(
        checked("tag is re-test").as_deref(),
        Ok("tag is \"re-test\"")
    );
    assert_eq!(
        checked("tag in (re-test, WEB-1)").as_deref(),
        Ok("tag in (\"re-test\", \"WEB-1\")")
    );
    // Without the fields nothing says what the field holds: the word is read as an ID.
    assert!(matches!(
        value_of("tag is re-test", None),
        Operand::Issue { .. }
    ));
    // A field that is not known is read as an issue field would be; the checker names the field.
    assert!(matches!(
        value_of("nothing is re-test", Some(&fields)),
        Operand::Issue { .. }
    ));
    // The helpers expect an issue, in any case.
    assert_eq!(checked("under web-7").as_deref(), Ok("under WEB-7"));
    assert_eq!(
        checked("blocked by web-7").as_deref(),
        Ok("blocked by WEB-7")
    );
}

#[test]
fn qry_04_a_short_name_is_read_in_any_case_and_written_in_capitals() {
    assert_eq!(
        checked("project within web").as_deref(),
        Ok("project within WEB")
    );
    assert_eq!(
        checked("project within Web2").as_deref(),
        Ok("project within WEB2")
    );
    // A word of the language is a project only in capitals.
    assert!(checked("project within and").is_err());
    assert_eq!(
        checked("project within AND").as_deref(),
        Ok("project within AND")
    );
    // A name that is no short name is refused as before.
    assert!(checked("project within 2web").is_err());
    assert!(checked("project within w").is_err());
}

/// The words offered at the bar in `marked`, and the bytes they replace.
fn suggestions_at(marked: &str) -> (Vec<String>, (usize, usize)) {
    let at = marked.find('|').unwrap();
    let text = marked.replacen('|', "", 1);
    let found = suggest(&text, at, &sample_fields());
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
fn qry_03_suggestions_after_a_typed_start_offer_the_fields_that_begin_so() {
    let (words, replace) = suggestions_at("assi|");
    assert_eq!(words, ["assignedTo"]);
    assert_eq!(replace, (0, 4));
}

#[test]
fn qry_03_suggestions_after_a_field_offer_what_it_allows() {
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
fn qry_03_suggestions_after_is_offer_the_choices_of_the_field() {
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
fn qry_03_suggestions_inside_quotes_offer_the_values_and_replace_the_quoted_text() {
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
fn qry_03_suggestions_name_what_the_server_alone_knows_as_a_request() {
    let (words, _) = suggestions_at("project within |");
    assert_eq!(words, ["ask Projects "]);
    let (words, _) = suggestions_at("parent is |");
    assert!(words.contains(&"ask Issues ".to_owned()));
    let (words, _) = suggestions_at("under |");
    assert_eq!(words, ["ask Issues "]);
}

#[test]
fn qry_03_suggestions_after_a_whole_filter_offer_the_joining_words() {
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
fn qry_03_suggestions_for_a_text_that_is_wrong_before_the_cursor_are_empty() {
    let (words, _) = suggestions_at("nothing |");
    assert_eq!(words, Vec::<String>::new());
    let (words, _) = suggestions_at("status = Open and |");
    assert_eq!(words, Vec::<String>::new());
}

#[test]
fn qry_03_the_cursor_may_stand_anywhere_in_any_text_without_a_crash() {
    for text in ACCEPTED.iter().chain(REFUSED) {
        for cursor in 0..=text.len() + 2 {
            let _ = suggest(text, cursor, &sample_fields());
        }
    }
    let _ = suggest("ünï is é ö", 3, &sample_fields());
}
