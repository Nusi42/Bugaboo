use super::*;
use crate::repo::Repo;

fn text() -> String {
    crate::source::read_text(&Repo::discover().unwrap(), THEME_PATH).unwrap()
}

/// The theme file with one value of a color token replaced: `key` is `dusk` or `dusk-more`.
fn with_value(token: &str, key: &str, value: &str) -> ThemeFile {
    let text = text();
    let start = text
        .find(&format!("color \"{token}\""))
        .expect("the token is in the file");
    let line_end = text[start..].find('\n').map_or(text.len(), |at| start + at);
    let line = &text[start..line_end];
    let marker = format!(" {key}=\"");
    let value_at = line.find(&marker).expect("the token has this mode") + marker.len();
    let value_end = line[value_at..].find('"').expect("the value ends") + value_at;
    let changed = format!(
        "{}{}{}{}",
        &text[..start + value_at],
        value,
        &line[value_end..],
        &text[line_end..]
    );
    ThemeFile::parse(&changed).unwrap()
}

fn messages(findings: &[Finding]) -> Vec<String> {
    findings
        .iter()
        .map(|finding| finding.message.clone())
        .collect()
}

#[test]
fn known_ratios() {
    assert!((contrast_ratio("#000000", "#FFFFFF") - 21.0).abs() < 0.01);
    assert!((contrast_ratio("#777777", "#FFFFFF") - 4.48).abs() < 0.01);
}

#[test]
fn good_example() {
    let file = ThemeFile::load(&Repo::discover().unwrap()).unwrap();
    let findings = evaluate(&file);
    assert!(findings.is_empty(), "{:#?}", messages(&findings));
}

#[test]
fn every_text_color_is_checked_on_every_neutral_surface() {
    let file = ThemeFile::load(&Repo::discover().unwrap()).unwrap();
    let pairs = file.contrast.pairs("daylight");
    for text in ["fg", "fg-muted", "fg-subtle", "warning", "accent-fg"] {
        for surface in ["bg-canvas", "bg-hover", "bg-selected", "bg-sunken"] {
            assert!(pairs.contains(&(text, surface)), "{text} on {surface}");
        }
    }
}

#[test]
fn cf_04_every_look_is_readable_in_all_four_themes() {
    // Every choice color the server can store is a pair of tokens in the theme, and the
    // text of a badge, a pill and the muted text on a plain surface of that color all pass,
    // in the four themes and with more contrast.
    let file = ThemeFile::load(&Repo::discover().unwrap()).unwrap();
    for theme in file.themes.iter().map(|theme| theme.name.as_str()) {
        let pairs = file.contrast.pairs(theme);
        for color in bb_core::own_fields::ChoiceColor::ALL {
            let (text, surface) = (
                format!("{}-fg", color.code()),
                format!("{}-bg", color.code()),
            );
            assert!(
                pairs.contains(&(text.as_str(), surface.as_str())),
                "{text} on {surface} is not checked in {theme}"
            );
            assert!(
                pairs.contains(&("fg-muted", surface.as_str())),
                "fg-muted on {surface} is not checked in {theme}"
            );
        }
    }
    let findings = evaluate(&file);
    assert!(findings.is_empty(), "{:#?}", messages(&findings));
}

#[test]
fn cf_04_the_progress_bar_and_the_avatar_are_seen_in_all_four_themes() {
    // The fill of a progress bar is a shape, not text: it is seen against every plain surface
    // at 3:1 (WCAG 1.4.11). The initials of an avatar are text on the soft accent.
    let file = ThemeFile::load(&Repo::discover().unwrap()).unwrap();
    for theme in file.themes.iter().map(|theme| theme.name.as_str()) {
        let value = |name: &str| color(&file, name, theme).expect("the token has a color");
        for surface in ["bg-canvas", "bg-surface", "bg-raised"] {
            let ratio = contrast_ratio(value("accent"), value(surface));
            assert!(ratio >= 3.0, "{theme}: accent on {surface} is {ratio:.2}:1");
        }
        let ratio = contrast_ratio(value("accent-fg"), value("accent-soft"));
        assert!(ratio >= 4.5, "{theme}: the initials are {ratio:.2}:1");
    }
}

#[test]
fn a_choice_color_that_is_hard_to_read_is_rejected() {
    // Category 3 text on a background that is nearly the same color in Dusk.
    let findings = evaluate(&with_value("cat3-bg", "dusk", "#6a8f78"));
    assert!(
        findings.iter().any(|f| f.message.contains("`cat3-fg`")),
        "{:?}",
        messages(&findings)
    );
}

#[test]
fn a_color_without_a_classification_is_rejected() {
    let text = text();
    let start = text.find("    non-text").expect("the rule is in the file");
    let end = text[start..]
        .find("reason=")
        .map(|at| start + at)
        .expect("it has a reason");
    let bad = format!("{}    non-text \"border\" {}", &text[..start], &text[end..]);
    let error = ThemeFile::parse(&bad).err().unwrap();
    assert!(format!("{error:#}").contains("border-subtle"), "{error:#}");
}

#[test]
fn the_edge_of_a_disabled_button_is_checked_as_it_is_seen() {
    // The edge is the color of quiet words faded by the opacity of a disabled control: this
    // color reaches 2:1 on the raised surface of Dusk as it is (2.56:1) and no longer once it is
    // faded (1.76:1), so only the fade makes the check find it.
    let file = ThemeFile::load(&Repo::discover().unwrap()).unwrap();
    let raised = color(&file, "bg-raised", "dusk").expect("the token has a color");
    assert!(contrast_ratio("#686f80", raised) >= 2.0);
    let findings = evaluate(&with_value("fg-muted", "dusk", "#686f80"));
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("the line `fg-muted`")),
        "{:?}",
        messages(&findings)
    );
}

#[test]
fn a_line_faded_by_something_that_is_no_opacity_is_a_finding_not_a_skip() {
    let mut file = ThemeFile::load(&Repo::discover().unwrap()).unwrap();
    let lines = file
        .contrast
        .plain_lines
        .iter_mut()
        .find(|lines| lines.faded_by.is_some())
        .expect("a group of lines is faded");
    lines.faded_by = Some("no-such-opacity".to_owned());
    let findings = evaluate(&file);
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("no-such-opacity")),
        "{:?}",
        messages(&findings)
    );
}

#[test]
fn a_color_fades_toward_the_surface_behind_it() {
    let near = |seen: [f64; 3], wanted: [f64; 3]| {
        seen.iter().zip(wanted).all(|(a, b)| (a - b).abs() < 1e-9)
    };
    assert!(near(fade("#ffffff", "#000000", 0.5), [127.5; 3]));
    assert!(near(fade("#123456", "#000000", 1.0), [18.0, 52.0, 86.0]));
}

#[test]
fn every_status_group_has_a_tint_with_readable_text_in_every_theme() {
    // The three groups, Not started included, each tint a row (UX §4.2): every text color of a
    // row is checked on each tint, in every theme (a row keeps its tint when hovered or selected).
    let file = ThemeFile::load(&Repo::discover().unwrap()).unwrap();
    for theme in file.themes.iter().map(|theme| theme.name.as_str()) {
        let pairs = file.contrast.pairs(theme);
        for group in ["ns", "ip", "done"] {
            let tint = format!("tint-{group}");
            for text in ["fg", "fg-muted", "fg-subtle", "accent-fg", "danger"] {
                assert!(
                    pairs.contains(&(text, tint.as_str())),
                    "{text} on {tint} is not checked in {theme}"
                );
            }
        }
    }
}

#[test]
fn the_ring_and_the_hover_line_are_seen_on_every_tint() {
    // A selected row has a ring of the accent color and a hovered row a line of the control edge
    // color, on the tint of its group: both are lines to see (3:1), as they are and with more contrast.
    let file = ThemeFile::load(&Repo::discover().unwrap()).unwrap();
    let rule = file
        .contrast
        .plain_lines
        .iter()
        .find(|lines| lines.colors.iter().any(|name| name == "accent"))
        .expect("the ring has a rule");
    for tint in ["tint-ns", "tint-ip", "tint-done"] {
        assert!(rule.on.iter().any(|name| name == tint), "{tint}");
    }
    assert!(rule.colors.iter().any(|name| name == "border-control"));
    assert!(rule.min >= 3.0);
    let findings = evaluate(&with_value("accent", "dusk", "#5a5a8a"));
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("the line `accent`") && f.message.contains("tint-")),
        "{:?}",
        messages(&findings)
    );
}

#[test]
fn a_line_that_vanishes_into_a_tint_only_with_more_contrast_is_rejected() {
    // The control edge color of Dusk with more contrast, one step from the tint of Not started.
    let findings = evaluate(&with_value("border-control", "dusk-more", "#343c4f"));
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("with more contrast")
                && f.message.contains("`border-control`")
                && f.message.contains("tint-ns")),
        "{:?}",
        messages(&findings)
    );
}

#[test]
fn a_tint_that_is_too_strong_for_the_text_on_it_is_rejected() {
    let findings = evaluate(&with_value("tint-ns", "daylight", "#c9d0dc"));
    assert!(
        findings.iter().any(|f| f.message.contains("`tint-ns`")),
        "{:?}",
        messages(&findings)
    );
}

#[test]
fn a_hover_color_that_vanishes_into_a_surface_is_rejected() {
    // Dusk hover one step away from the raised surface, as in the rejected retune.
    let findings = evaluate(&with_value("bg-hover", "dusk", "#2d323d"));
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("bg-hover") && f.message.contains("bg-raised")),
        "{:?}",
        messages(&findings)
    );
}

#[test]
fn hover_and_selected_that_look_the_same_are_rejected() {
    let file = ThemeFile::load(&Repo::discover().unwrap()).unwrap();
    let hover = file
        .token("bg-hover")
        .unwrap()
        .in_theme("daylight")
        .unwrap()
        .to_owned();
    let findings = evaluate(&with_value("bg-selected", "daylight", &hover));
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("bg-selected") && f.message.contains("bg-hover"))
    );
}

#[test]
fn a_faint_edge_of_a_control_is_rejected_in_the_plain_mode() {
    // The edge of a box to tick put back to the faint divider of Midnight.
    let findings = evaluate(&with_value("border-control", "midnight", "#272a31"));
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("`border-control`") && f.message.contains("midnight")),
        "{:?}",
        messages(&findings)
    );
}

#[test]
fn weak_text_with_more_contrast_is_rejected() {
    // The stronger muted text of Daylight put back to a color that only reaches AA.
    let findings = evaluate(&with_value("fg-muted", "daylight-more", "#5a6170"));
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("with more contrast") && f.message.contains("`fg-muted`")),
        "{:?}",
        messages(&findings)
    );
}

#[test]
fn a_weak_accent_fill_with_more_contrast_is_rejected() {
    // The accent fill of Daylight put back to the usual one: white text on it reaches only AA.
    let findings = evaluate(&with_value("accent", "daylight-more", "#5b3fd9"));
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("`on-accent`") && f.message.contains("`accent`")),
        "{:?}",
        messages(&findings)
    );
}

#[test]
fn faint_lines_with_more_contrast_are_rejected() {
    let findings = evaluate(&with_value("border", "dusk-more", "#3b414d"));
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("line `border`") && f.message.contains("Theme dusk")),
        "{:?}",
        messages(&findings)
    );
}

#[test]
fn bad_example() {
    // Light grey text on the white page in Daylight.
    let findings = evaluate(&with_value("fg", "daylight", "#cccccc"));
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("Theme daylight"))
    );
}
