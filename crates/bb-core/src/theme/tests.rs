use super::*;
use crate::contrast::{Rating, ThemeColors as _, base_rules};

fn rules() -> &'static crate::contrast::Rules {
    base_rules()
}

fn colors(pairs: &[(&str, &str)]) -> OwnValues {
    OwnValues {
        colors: pairs
            .iter()
            .map(|(name, value)| {
                (
                    (*name).to_owned(),
                    ColorChoice {
                        value: (*value).to_owned(),
                        more: None,
                    },
                )
            })
            .collect(),
        ..OwnValues::default()
    }
}

#[test]
fn thm_03_the_contrast_of_bb_core_gives_the_built_in_files_the_results_of_the_check() {
    // The built-in themes are the files the check reads: no pair of any of them is hard to read.
    for theme in built_in_themes() {
        let found = rate_own(theme.id, &OwnValues::default(), rules()).expect("a built-in theme");
        assert!(
            found.problems.is_empty(),
            "{}: {:?}",
            theme.id,
            found.problems
        );
        assert!(!found.results.is_empty());
        let hard: Vec<_> = found
            .hard()
            .map(|pair| (&pair.first, &pair.second))
            .collect();
        assert!(hard.is_empty(), "{}: {hard:?}", theme.id);
        assert!(found.results.iter().any(|pair| pair.rating == Rating::Easy));
    }
}

#[test]
fn thm_03_a_pair_that_is_hard_to_read_is_found_and_named() {
    let values = colors(&[("fg", "#fafafa")]);
    let found = rate_own("daylight", &values, rules()).expect("a built-in theme");
    assert!(
        found
            .hard()
            .any(|pair| pair.first == "fg" && pair.second == "bg-canvas"),
        "{:?}",
        found.hard().collect::<Vec<_>>()
    );
}

#[test]
fn thm_04_an_own_theme_follows_the_values_of_its_built_in_theme_for_what_it_does_not_change() {
    // Changing one color changes the colors that link to it, and nothing else.
    let daylight = built_in_theme("daylight").expect("daylight");
    let own = OwnColors::new(daylight, &colors(&[("border-control", "#112233")]));
    let plain = OwnColors::new(daylight, &OwnValues::default());
    assert_eq!(own.color("border-control"), Some("#112233"));
    assert_eq!(own.color("hover-line"), Some("#112233"));
    assert_eq!(own.color("fg"), plain.color("fg"));
}

#[test]
fn thm_04_a_value_outside_its_kind_or_an_unknown_token_is_refused() {
    let layer = r##"{"x":0,"y":1,"blur":2,"spread":0,"color":"#000000"}"##;
    let three = format!("{{\"shadow\":[{layer},{layer},{layer}]}}");
    for (text, token) in [
        (
            r##"{"colors":{"no-such-token":{"value":"#112233"}}}"##.to_owned(),
            "no-such-token",
        ),
        (r#"{"colors":{"fg":{"value":"red"}}}"#.to_owned(), "fg"),
        (r##"{"colors":{"fg":{"value":"#fff"}}}"##.to_owned(), "fg"),
        (
            r##"{"colors":{"fg":{"value":"#112233","more":"#112233"}}}"##.to_owned(),
            "fg",
        ),
        (
            r##"{"colors":{"fg":{"value":"#112233"}},"spacing":{"x":1}}"##.to_owned(),
            "spacing",
        ),
        (r#"{"radii":{"r-ctl":400}}"#.to_owned(), "r-ctl"),
        (r#"{"radii":{"r-pill":4}}"#.to_owned(), "r-pill"),
        (
            r#"{"fonts":{"font-ui":"Comic Sans"}}"#.to_owned(),
            "font-ui",
        ),
        (
            r#"{"fonts":{"font-mono":"font-ui"}}"#.to_owned(),
            "font-mono",
        ),
        (
            r#"{"shadow":[{"x":0,"y":1,"blur":2,"spread":0,"color":"red"}]}"#.to_owned(),
            "shadow",
        ),
        (three, "shadow"),
        (r##"{"colors":{"fg":"#112233"}}"##.to_owned(), "fg"),
    ] {
        let json: serde_json::Value = serde_json::from_str(&text).expect("JSON");
        let problems = OwnValues::from_json(&json).expect_err(&text);
        assert!(
            problems.iter().any(|problem| problem.token == token),
            "{text}: {problems:?}"
        );
    }
}

#[test]
fn thm_04_the_style_sheet_holds_custom_properties_only() {
    // Values that would close a rule, start a comment, import or call a URL are refused and never written.
    for evil in [
        "#112233; } body { display: none",
        "#112233 /*",
        "@import x",
        "url(https://example.invalid/x)",
        "#112233}",
        "expression(1)",
    ] {
        let values = colors(&[("fg", evil)]);
        assert!(!values.problems().is_empty(), "{evil}");
        assert_eq!(style_sheet("abc-1", &values), None, "{evil}");
    }
    let mut values = colors(&[("fg", "#ABCDEF")]);
    values.colors.get_mut("fg").expect("fg").more = Some("#010203".to_owned());
    values.radii.insert("r-ctl".to_owned(), 8);
    values
        .fonts
        .insert("font-ui".to_owned(), "font-reader".to_owned());
    values.shadow = Some(vec![ShadowLayer {
        x: 0,
        y: 1,
        blur: 2,
        spread: 0,
        color: "#00000040".to_owned(),
    }]);
    let sheet = style_sheet("0190-abc", &values).expect("a sheet");
    assert!(
        sheet.contains(":root[data-own-theme=\"0190-abc\"] {"),
        "{sheet}"
    );
    assert!(sheet.contains("--fg: #abcdef;"));
    assert!(sheet.contains("@media (prefers-contrast: more)"));
    assert!(sheet.contains("--r-ctl: 8px;"));
    assert!(sheet.contains("--shadow: 0px 1px 2px 0px #00000040;"));
    // Every line between the braces is `--name: value;`: variables and nothing else.
    for line in sheet
        .lines()
        .filter(|line| line.starts_with("  ") && !line.contains('{') && line.trim() != "}")
    {
        assert!(
            line.trim_start().starts_with("--") && line.ends_with(';'),
            "{line}"
        );
    }
    assert!(!sheet.contains("url(") && !sheet.contains("@import"));
    // An ID that could close the selector is no ID.
    assert_eq!(style_sheet("a\"] { x", &OwnValues::default()), None);
}

#[test]
fn thm_04_an_own_theme_stores_only_the_values_it_changes() {
    let values = colors(&[("accent", "#ABCDEF")]).normalized();
    let json = serde_json::to_string(&values).expect("JSON");
    assert_eq!(json, r##"{"colors":{"accent":{"value":"#abcdef"}}}"##);
    assert!(OwnValues::default().is_empty());
}
