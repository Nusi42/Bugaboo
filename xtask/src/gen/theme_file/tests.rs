//! The tests of `theme_file`.

use super::*;

#[test]
fn dollar_names_become_variables_and_back() {
    assert_eq!(
        substitute("calc(4px * $density)"),
        "calc(4px * var(--density))"
    );
    assert_eq!(
        references("$ring-width solid $accent"),
        ["ring-width", "accent"]
    );
}

#[test]
fn the_theme_file_of_the_repository_is_valid() {
    let repo = Repo::discover().unwrap();
    let file = ThemeFile::load(&repo).unwrap();
    assert_eq!(file.themes.len(), 4);
    assert_eq!(file.default_light, "daylight");
    assert!(file.token("accent").is_some());
}

/// The theme file of the repository with `extra` at the end: the error, or nothing.
fn problem_with(extra: &str) -> Option<String> {
    let repo = Repo::discover().unwrap();
    let text = crate::source::read_text(&repo, THEME_PATH).unwrap();
    ThemeFile::parse(&format!("{text}\n{extra}\n"))
        .err()
        .map(|error| format!("{error:#}"))
}

/// The pixels of a size token such as `36px`.
fn pixels(value: &str) -> f64 {
    value.trim_end_matches("px").parse().unwrap()
}

#[test]
fn ux_04_no_control_is_below_24_and_no_touch_control_is_below_44() {
    let file = ThemeFile::load(&Repo::discover().unwrap()).unwrap();
    let least = pixels(
        file.token("size-target-least")
            .unwrap()
            .same
            .as_ref()
            .unwrap(),
    );
    assert!(
        (least - 24.0).abs() < f64::EPSILON,
        "WCAG 2.2 AA asks for 24"
    );
    let controls: Vec<&Token> = file
        .tokens
        .iter()
        .filter(|token| token.name.starts_with("size-control") || token.name == "size-row")
        .collect();
    assert!(controls.len() >= 3, "the control and row sizes are tokens");
    for control in controls {
        let usual = pixels(control.same.as_deref().unwrap());
        assert!(usual >= least, "{} is {usual}, below {least}", control.name);
        let touch = control.variants.iter().find(|(name, _)| name == "touch");
        let touch = pixels(
            &touch
                .unwrap_or_else(|| panic!("{} has no touch value", control.name))
                .1,
        );
        assert!(
            touch >= 44.0,
            "{} is {touch} for a finger, below 44",
            control.name
        );
    }
    // The old single size of 44 for every target is gone, not kept beside the new ones.
    assert!(file.token("size-target").is_none());
}

#[test]
fn bad_example_a_token_of_the_design_pages_must_be_named_doc() {
    let repo = Repo::discover().unwrap();
    let text = crate::source::read_text(&repo, THEME_PATH).unwrap();
    let renamed = text.replacen("\"doc-lh-page\"", "\"lh-page\"", 1);
    let error = ThemeFile::parse(&renamed)
        .err()
        .map(|error| format!("{error:#}"));
    assert!(error.is_some_and(|text| text.contains("doc-")));
}

#[test]
fn bad_example_a_sheet_of_the_design_pages_has_a_known_name() {
    let problem = problem_with("docs \"other\" description=\"x\" {\n}");
    assert!(problem.is_some_and(|text| text.contains("not a sheet")));
    let twice = problem_with("docs \"components\" description=\"x\" {\n}");
    assert!(twice.is_some_and(|text| text.contains("twice")));
}

#[test]
fn bad_example_a_value_that_could_break_out_of_its_rule() {
    for bad in [
        "part \"zz\" {\n  color \"red; } * { display: none\"\n}",
        "part \"zz\" {\n  color \"red}\"\n}",
        "part \"zz\" {\n  color \"red /* x\"\n}",
        "part \"zz\" {\n  background \"url(<x>)\"\n}",
        "part \"zz\" {\n  media \"(min-width: 1px) { a { b: c } }\" {\n    color red\n  }\n}",
        "keyframes \"zz\" {\n  from {\n    opacity \"0; }\"\n  }\n}",
    ] {
        let error = problem_with(bad).unwrap_or_default();
        assert!(error.contains("could end the rule"), "{bad}: {error}");
    }
}

#[test]
fn bad_example_duplicates_and_forced_values_that_are_no_system_color() {
    let mixin = "mixin \"m\" {\n  color red\n}\n";
    let error = problem_with(&format!("{mixin}{mixin}")).unwrap_or_default();
    assert!(error.contains("defined twice"), "{error}");
    let frames = "keyframes \"k\" {\n  from {\n    opacity 0\n  }\n}\n";
    let error = problem_with(&format!("{frames}{frames}")).unwrap_or_default();
    assert!(error.contains("defined twice"), "{error}");
    let text = crate::source::read_text(&Repo::discover().unwrap(), THEME_PATH).unwrap();
    let bad = text.replace("forced=\"Highlight\"", "forced=\"#ff0000\"");
    let error = ThemeFile::parse(&bad)
        .err()
        .map(|error| format!("{error:#}"))
        .unwrap_or_default();
    assert!(error.contains("must be a CSS system color"), "{error}");
}

#[test]
fn a_bad_file_names_the_line() {
    let error = ThemeFile::parse("defaults light=\"a\" dark=\"b\"\nthemes {\n  theme \"a\" kind=\"light\"\n}\ntokens {\n  color \"x\" a=\"red\"\n}\n")
            .err()
            .map(|error| format!("{error:#}"))
            .unwrap_or_default();
    assert!(error.contains("line 6"), "{error}");
}

/// The theme file of the repository with one piece of text replaced: the error, or nothing.
fn problem_replacing(from: &str, to: &str) -> Option<String> {
    let repo = Repo::discover().unwrap();
    // The file has no faded lines of its own; the example group stands in for them.
    let text = crate::generate::contrast_spec::with_faded_lines(
        &crate::source::read_text(&repo, THEME_PATH).unwrap(),
    );
    assert!(text.contains(from), "{from} is not in the theme file");
    ThemeFile::parse(&text.replacen(from, to, 1))
        .err()
        .map(|error| format!("{error:#}"))
}

#[test]
fn an_opacity_is_read_once_and_only_from_an_opacity_token() {
    let file = ThemeFile::load(&Repo::discover().unwrap()).unwrap();
    assert_eq!(file.opacity("opacity-disabled"), Some(0.6));
    assert_eq!(file.opacity("fg-muted"), None);
    assert_eq!(file.opacity("no-such-token"), None);
}

#[test]
fn a_line_faded_by_a_color_or_an_unknown_token_is_rejected() {
    for name in ["fg-muted", "no-such-token"] {
        let problem = problem_replacing(
            "faded-by=\"opacity-disabled\"",
            &format!("faded-by=\"{name}\""),
        );
        assert!(
            problem.is_some_and(|text| text.contains("not an opacity token")),
            "{name}"
        );
    }
}

#[test]
fn an_opacity_outside_zero_to_one_is_rejected() {
    for value in ["1.5", "-0.2"] {
        let problem = problem_replacing(
            "opacity \"opacity-disabled\" 0.6",
            &format!("opacity \"opacity-disabled\" {value}"),
        );
        assert!(
            problem.is_some_and(|text| text.contains("not an opacity token")),
            "{value}"
        );
    }
}

/// The pixels of a size token at the density of 1: `560px`, or `calc(16px * $density)`.
fn token_pixels(file: &ThemeFile, name: &str) -> f64 {
    let value = file
        .token(name)
        .and_then(|token| token.same.as_deref())
        .unwrap_or_else(|| panic!("{name} is a size of the theme file"));
    pixels(
        value
            .trim_start_matches("calc(")
            .trim_end_matches(')')
            .trim_end_matches(" * $density"),
    )
}

/// Whether the breakpoint under which Customize stands under the list is the room the two need side by side: the least
/// width of the list, the gap of `list-body` and the panel of Customize. A container query cannot read the three
/// tokens, so the breakpoint is written as their sum and held to it here.
fn list_beside_is_the_sum(file: &ThemeFile) -> bool {
    let sum = token_pixels(file, "size-list-main")
        + token_pixels(file, "space-4")
        + token_pixels(file, "size-list");
    (token_pixels(file, "bp-list-beside") - sum).abs() < 0.5
}

#[test]
fn the_breakpoint_of_customize_beside_the_list_is_the_room_the_two_need_at_density_1() {
    let repo = Repo::discover().unwrap();
    let text = crate::source::read_text(&repo, THEME_PATH).unwrap();
    assert!(list_beside_is_the_sum(&ThemeFile::parse(&text).unwrap()));
    // A list made wider without the breakpoint is refused.
    let wider = text.replacen(
        "size \"size-list-main\" \"560px\"",
        "size \"size-list-main\" \"600px\"",
        1,
    );
    assert_ne!(wider, text, "the least width of the list is in the file");
    assert!(!list_beside_is_the_sum(&ThemeFile::parse(&wider).unwrap()));
}
