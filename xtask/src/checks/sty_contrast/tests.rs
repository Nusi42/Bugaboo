use super::*;
use crate::repo::Repo;

fn text() -> String {
    crate::source::read_text(&Repo::discover().unwrap(), THEME_PATH).unwrap()
}

/// The theme file with one value of a color token replaced: `key` is `dusk` or `dusk-more`.
fn with_value(token: &str, key: &str, value: &str) -> ThemeFile {
    ThemeFile::parse(&text_with_value(&text(), token, key, value)).unwrap()
}

/// `text` with the value of a color token in one theme (or mode) written as `value`.
fn text_with_value(text: &str, token: &str, key: &str, value: &str) -> String {
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
    changed
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
fn a_faded_line_is_checked_as_it_is_seen() {
    // A line in the color of quiet words faded by the opacity of a disabled control (the example
    // group): this color reaches 2:1 on the raised surface of Dusk as it is (2.56:1) and no longer
    // once it is faded (1.76:1), so only the fade makes the check find it.
    let file = ThemeFile::load(&Repo::discover().unwrap()).unwrap();
    let raised = color(&file, "bg-raised", "dusk").expect("the token has a color");
    assert!(contrast_ratio("#686f80", raised) >= 2.0);
    let faded = crate::generate::contrast_spec::with_faded_lines(&text_with_value(
        &text(),
        "fg-muted",
        "dusk",
        "#686f80",
    ));
    let findings = evaluate(&ThemeFile::parse(&faded).unwrap());
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
    let mut file =
        ThemeFile::parse(&crate::generate::contrast_spec::with_faded_lines(&text())).unwrap();
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
fn the_main_button_keeps_its_contrast_while_it_is_hovered() {
    // Hovering brightens the fill and its words (`hover-brightness`), which takes contrast away
    // from white on a light violet: with more contrast the fill of Daylight reached 7.09:1 as it
    // is and only 6.02:1 hovered (M5 exit, A5). The old fill is rejected by the hovered rule only.
    let findings = evaluate(&with_value("accent", "daylight-more", "#573cd0"));
    assert!(
        findings.iter().any(|f| f.message.contains("hovered")
            && f.message.contains("more contrast")
            && f.message.contains("daylight")
            && f.message.contains("`on-accent`")),
        "{:?}",
        messages(&findings)
    );
    // Every theme is held to 4.5:1 hovered and 7:1 hovered with more contrast.
    let file = ThemeFile::load(&Repo::discover().unwrap()).unwrap();
    let rule = file
        .contrast
        .hovered
        .iter()
        .find(|rule| rule.text == "on-accent" && rule.on == "accent")
        .expect("the main button has a hovered rule");
    assert!(rule.min >= 4.5 && rule.min_more >= 7.0);
}

#[test]
fn the_three_status_tints_are_told_apart_from_each_other_and_from_the_card() {
    // M5 exit A7: in Daylight the tints of Not started and In progress were 7.7 apart (delta E);
    // A8: in Dusk Not started was 6.9 from the raised card. Both are rejected now.
    let findings = evaluate(&with_value("tint-ip", "daylight", "#e0ebfa"));
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("`tint-ns`") && f.message.contains("`tint-ip`")),
        "{:?}",
        messages(&findings)
    );
    let findings = evaluate(&with_value("tint-ns", "dusk", "#343c4e"));
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("`tint-ns`") && f.message.contains("`bg-raised`")),
        "{:?}",
        messages(&findings)
    );
}

#[test]
fn the_twelve_choice_fills_are_told_apart_from_each_other_and_from_the_card() {
    // M5 exit: in Daylight the fill of Coral (then Red) was about 10 from Pink (delta E). The bad examples stand just
    // under each limit, so a weaker rule would let them pass: a Coral 13.5 from the Pink of now (#ffc8da) against the 15
    // between fills, and a Dusk Gray 6.75 from the raised card (#2d323c) against the 7.5 from the surfaces.
    let findings = evaluate(&with_value("cat7-bg", "daylight", "#d8a3b5"));
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("`cat1-bg`") && f.message.contains("`cat7-bg`")),
        "{:?}",
        messages(&findings)
    );
    let findings = evaluate(&with_value("cat11-bg", "dusk", "#3c414b"));
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("`cat11-bg`") && f.message.contains("`bg-raised`")),
        "{:?}",
        messages(&findings)
    );
}

#[test]
fn the_shadow_at_the_end_of_a_scrolling_box_is_far_from_the_surfaces() {
    // M5 exit, round 8: in Dusk and Midnight the shadow was 5 to 7 levels from the box and was not seen.
    // A shade of Dusk that is close to its surfaces is rejected.
    let findings = evaluate(&with_value("cue", "dusk", "#3a3f4a"));
    assert!(
        findings.iter().any(|f| f.message.contains("`cue`")),
        "{:?}",
        messages(&findings)
    );
}

#[test]
fn the_quiet_words_of_a_tinted_row_have_room_to_spare() {
    // M5 exit L23: the ID and the hint words of a row were 4.5:1 and a little over on the tints,
    // which a small thin text shows as hard to read. The old muted colors of Paper are rejected.
    let findings = evaluate(&with_value("fg-subtle", "paper", "#6b635a"));
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("`fg-subtle`") && f.message.contains("a row")),
        "{:?}",
        messages(&findings)
    );
    let findings = evaluate(&with_value("fg-muted", "paper", "#6a6157"));
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("`fg-muted`") && f.message.contains("a row")),
        "{:?}",
        messages(&findings)
    );
}

#[test]
fn the_edge_of_high_is_told_from_the_edge_of_critical_and_is_seen() {
    // M5 exit L24: the brown of High was 28 from the red of Critical in Daylight. The edge is a
    // line (3:1 on every tint and surface) and a clear orange.
    let findings = evaluate(&with_value("edge-high", "daylight", "#aa4e09"));
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("`edge-high`") && f.message.contains("`danger`")),
        "{:?}",
        messages(&findings)
    );
    let findings = evaluate(&with_value("edge-high", "paper", "#e8c9a0"));
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("the line `edge-high`")),
        "{:?}",
        messages(&findings)
    );
}

#[test]
fn the_line_of_a_hovered_row_is_seen_on_every_tint_in_every_theme() {
    let findings = evaluate(&with_value("hover-line", "dusk", "#3b414d"));
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("the line `hover-line`")),
        "{:?}",
        messages(&findings)
    );
}

#[test]
fn the_page_behind_a_sheet_is_dimmed_so_the_sheet_stands_out_and_the_page_text_is_dim() {
    // M5 exit R4: behind Quick search and the dialogs of Daylight and Paper the page was a light
    // wash (the sunken surface at 80%), and its text stayed easy to read right under the panel. The
    // wash as the scrim is rejected twice: the sheet does not stand out, and the text is not dim.
    let findings = evaluate(&with_value("scrim", "daylight", "#eef0f3"));
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("Theme daylight") && f.message.contains("to stand out")),
        "{:?}",
        messages(&findings)
    );
    // A dark scrim that is only a little opaque leaves the page text readable under the panel.
    let faint = text().replacen(
        "opacity \"opacity-scrim\" 0.9",
        "opacity \"opacity-scrim\" 0.4",
        1,
    );
    let findings = evaluate(&ThemeFile::parse(&faint).unwrap());
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("Theme daylight") && f.message.contains("still")),
        "{:?}",
        messages(&findings)
    );
}

#[test]
fn a_distinct_rule_that_names_no_minimum_is_rejected() {
    // It would check nothing and say nothing.
    let bad = text().replacen(
        "distinct min-delta-e-between-states=31 min-delta-e-against-surfaces=30 reason=",
        "distinct reason=",
        1,
    );
    let error = ThemeFile::parse(&bad).err().expect("it is rejected");
    assert!(
        format!("{error:#}").contains("at least one minimum"),
        "{error:#}"
    );
}

#[test]
fn the_knob_of_a_switch_is_seen_on_its_track_in_every_theme() {
    // M5 exit: a dark knob on a dark track in Dusk and Midnight. The knob has the color of quiet words.
    let findings = evaluate(&with_value("fg-muted", "midnight", "#2a2d33"));
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("the line `fg-muted`") && f.message.contains("bg-sunken")),
        "{:?}",
        messages(&findings)
    );
}

#[test]
fn a_main_button_that_cannot_be_pressed_is_told_from_the_surface_and_stays_readable() {
    // M5 exit: in Dusk and Midnight it read like an enabled outlined button. Its fill put back next to the raised
    // surface is rejected, and so are words that sink into the fill, plain and with more contrast.
    let findings = evaluate(&with_value("disabled-fill", "dusk", "#30363f"));
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("`disabled-fill`") && f.message.contains("`bg-raised`")),
        "{:?}",
        messages(&findings)
    );
    let findings = evaluate(&with_value("on-disabled", "dusk", "#6c727e"));
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("`on-disabled` on `disabled-fill`")),
        "{:?}",
        messages(&findings)
    );
    // With more contrast the words need 7:1 on the fill (the divider colors gave 1.7:1, M5 exit B1).
    let findings = evaluate(&with_value("on-disabled", "daylight-more", "#565d6b"));
    assert!(
        findings.iter().any(
            |f| f.message.contains("with more contrast") && f.message.contains("`on-disabled`")
        ),
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

#[test]
fn the_tint_of_not_started_is_a_neutral_grey_in_every_theme() {
    // M5 exit F-A4: in Dusk the tint of Not started was a blue grey (47, 60, 80), close to In progress.
    let findings = evaluate(&with_value("tint-ns", "dusk", "#2f3c50"));
    assert!(
        findings.iter().any(|f| f.message.contains("Theme dusk")
            && f.message.contains("`tint-ns`")
            && f.message.contains("grey")),
        "{:?}",
        messages(&findings)
    );
}
