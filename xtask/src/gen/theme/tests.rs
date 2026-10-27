//! The tests of theme.

use super::*;

fn file() -> ThemeFile {
    ThemeFile::load(&Repo::discover().unwrap()).unwrap()
}

#[test]
fn generated_files_are_up_to_date() {
    let repo = Repo::discover().unwrap();
    for output in outputs(&repo).unwrap() {
        let on_disk = crate::source::read_text(&repo, output.path).unwrap();
        assert_eq!(
            on_disk, output.content,
            "{} is stale; run `cargo xtask gen theme`",
            output.path
        );
    }
}

#[test]
fn css_has_every_theme_and_the_device_fallback() {
    let css = css(&file()).unwrap();
    for theme in ["daylight", "paper", "dusk", "midnight"] {
        assert!(css.contains(&look_selector(&file(), theme)));
        assert!(css.contains(&preview_selector(&file(), theme)));
    }
    assert!(css.contains("prefers-color-scheme: dark"));
}

#[test]
fn every_line_of_the_compiled_rules_knows_its_line_in_the_theme_file() {
    let (css, from) = parts_css_with_lines(&file()).unwrap();
    assert_eq!(from.len(), css.lines().count());
    assert!(from.iter().all(|line| *line > 0));
}

#[test]
fn the_forced_colors_win_over_every_block_that_sets_a_theme() {
    let css = css(&file()).unwrap();
    let forced_at = css.find("@media (forced-colors: active)").unwrap();
    let forced = &css[forced_at..];
    // Every selector that sets the colors of a theme is in the forced block, which comes after
    // all of them: at the same weight the later rule wins.
    let file = file();
    let mut wanted = vec![not_light(&file), forced_dark(&file)];
    for theme in ["daylight", "paper", "dusk", "midnight"] {
        wanted.push(look_selector(&file, theme));
        wanted.push(preview_selector(&file, theme));
    }
    for selector in wanted {
        assert!(css[..forced_at].contains(&selector), "{selector}");
        assert!(forced.contains(&selector), "{selector} is not forced");
    }
    assert!(forced.contains("--accent: Highlight;"), "{forced}");
}

#[test]
fn css_has_the_choices_more_contrast_and_forced_colors() {
    let css = css(&file()).unwrap();
    for expected in [
        ":root[data-spacing=\"compact\"]",
        ":root[data-text=\"larger\"]",
        ":root[data-motion=\"less\"]",
        "@media (prefers-reduced-motion: reduce)",
        "@media (prefers-contrast: more)",
        "@media (forced-colors: active)",
        "--space-4: calc(16px * var(--density));",
    ] {
        assert!(css.contains(expected), "{expected}");
    }
}

#[test]
fn typescript_has_the_breakpoints_the_choices_and_the_parts() {
    let ts = typescript(&file()).unwrap();
    for expected in [
        "look: \"data-look\",",
        "previewLook: \"data-preview-look\",",
        "tablet: 768,",
        "computer: 1280,",
        "windowSide: 880,",
        "lightThemeIds = [\"daylight\", \"paper\"]",
        "darkThemeIds = [\"dusk\", \"midnight\"]",
        "\"larger-text\": { attribute: \"data-text\", value: \"larger\" }",
        "button: \"button\",",
    ] {
        assert!(ts.contains(expected), "{expected}");
    }
}

#[test]
fn the_design_pages_get_their_two_sheets_and_their_tokens() {
    let outputs = generate(&file()).unwrap();
    let sheet = |path: &str| {
        outputs
            .iter()
            .find(|output| output.path == path)
            .map(|output| output.content.clone())
            .unwrap()
    };
    let components = sheet("docs/design/components.css");
    // A plain selector stays as it is; a value is a token.
    assert!(components.contains(".btn.primary:hover {"), "{components}");
    assert!(components.contains("padding: var(--doc-space-7) var(--space-3);"));
    let docpage = sheet("docs/design/docpage.css");
    assert!(
        docpage.contains("@media (max-width: 1023px) {"),
        "{docpage}"
    );
    // The tokens of the pages are in the copy of the pages, and not in the app's sheet.
    let tokens = sheet("docs/design/theme.css");
    assert!(tokens.contains("--doc-space-7: 7px;"));
    assert!(!sheet(STYLE_SHEET).contains("--doc-"));
}

#[test]
fn the_design_pages_get_the_rules_of_the_parts_to_draw_their_pictures_with() {
    let outputs = generate(&file()).unwrap();
    let sheet = |path: &str| {
        outputs
            .iter()
            .find(|output| output.path == path)
            .map(|output| output.content.clone())
            .unwrap()
    };
    let parts = sheet("docs/design/parts.css");
    // The same rules as the app's sheet, so a picture and the app look the same ...
    assert!(parts.contains("[data-part=\"status-icon\"]"), "{parts}");
    assert!(parts.contains(&parts_css(&file()).unwrap()));
    // ... and none of the tokens, which the sheet of the pages already has.
    assert!(!parts.contains("--bg-canvas:"));
}

#[test]
fn rust_has_the_theme_choices_and_the_defaults() {
    let rust = rust(&file());
    for expected in [
        "Choice::new(\"paper\", message::THEME_PAPER),",
        "Choice::new(\"midnight\", message::THEME_MIDNIGHT),",
        "DEFAULT_DARK_THEME: &str = \"dusk\"",
    ] {
        assert!(rust.contains(expected), "{expected}");
    }
}

#[test]
fn a_part_rule_is_compiled_with_its_mixins_and_media() {
    let text = std::fs::read_to_string(Repo::discover().unwrap().path("design/theme.kdl")).unwrap();
    let extra = "\nmixin \"m\" {\n  color \"$fg\"\n}\npart \"zz[variant=a]:hover\" \"zz[state=busy]\" {\n  use \"m\"\n  margin 0\n  forced {\n    color \"CanvasText\"\n  }\n}\n";
    let file = ThemeFile::parse(&format!("{text}{extra}")).unwrap();
    let parts = parts_css(&file).unwrap();
    assert!(parts.contains("[data-part=\"zz\"][data-variant~=\"a\"]:hover,\n[data-part=\"zz\"][data-state~=\"busy\"] {\n  color: var(--fg);\n  margin: 0;\n}"), "{parts}");
    assert!(
        parts.contains("@media (forced-colors: active) {"),
        "{parts}"
    );
}

/// The theme file with a choice of the device and a rule under a container query added.
fn file_with_device_choice_and_container() -> ThemeFile {
    let text = std::fs::read_to_string(Repo::discover().unwrap().path("design/theme.kdl")).unwrap();
    let extra = "\nvariants {\n  variant \"zz-touch\" media=\"(pointer: coarse)\"\n}\ntokens {\n  size \"zz-height\" \"36px\" zz-touch=\"44px\"\n}\npart \"zz\" {\n  container \"(min-width: $bp-tablet)\" {\n    margin 0\n  }\n}\n";
    ThemeFile::parse(&format!("{text}{extra}")).unwrap()
}

#[test]
fn a_choice_of_the_device_alone_is_a_media_block_and_has_no_attribute() {
    let file = file_with_device_choice_and_container();
    let css = css(&file).unwrap();
    assert!(
        css.contains("@media (pointer: coarse) {\n  :root {\n    --zz-height: 44px;"),
        "{css}"
    );
    assert!(!css.contains("zz-touch\"]"), "{css}");
    let ts = typescript(&file).unwrap();
    assert!(!ts.contains("zz-touch"), "{ts}");
}

#[test]
fn the_media_query_of_a_choice_may_name_a_breakpoint_token() {
    let css = css(&file()).unwrap();
    assert!(
        css.contains(
            "@media (width < 768px) {\n  :root {\n    --fs-ui: calc(16px * var(--text-scale));"
        ),
        "{css}"
    );
}

#[test]
fn only_the_marked_sizes_are_variables_the_code_may_set() {
    let file = file();
    let ts = typescript(&file).unwrap();
    let marked: String = file
        .tokens
        .iter()
        .filter(|token| token.run_time)
        .map(|token| format!("  \"{}\",\n", token.name))
        .collect();
    assert!(!marked.is_empty());
    assert!(
        ts.contains(&format!("runTimeVariables = [\n{marked}] as const;")),
        "{ts}"
    );
    let text = std::fs::read_to_string(Repo::discover().unwrap().path("design/theme.kdl")).unwrap();
    let color = text.replacen(
        "color \"accent\" ",
        "color \"accent\" run-time=\"true\" ",
        1,
    );
    assert!(ThemeFile::parse(&color).is_err());
}

#[test]
fn a_container_block_is_written_under_container_with_the_token_resolved() {
    let css = css(&file_with_device_choice_and_container()).unwrap();
    assert!(
        css.contains("@container (min-width: 768px) {\n  [data-part=\"zz\"] {\n    margin: 0;"),
        "{css}"
    );
}

#[test]
fn a_choice_needs_an_attribute_and_a_value_or_only_a_device_setting() {
    let text = std::fs::read_to_string(Repo::discover().unwrap().path("design/theme.kdl")).unwrap();
    for extra in [
        "\nvariants {\n  variant \"zz\" attribute=\"data-zz\"\n}\n",
        "\nvariants {\n  variant \"zz\"\n}\n",
    ] {
        assert!(
            ThemeFile::parse(&format!("{text}{extra}")).is_err(),
            "{extra}"
        );
    }
}

#[test]
fn an_unknown_token_or_mixin_is_refused() {
    let text = std::fs::read_to_string(Repo::discover().unwrap().path("design/theme.kdl")).unwrap();
    for extra in [
        "\npart \"zz\" {\n  color \"$nope\"\n}\n",
        "\npart \"zz\" {\n  use \"nope\"\n}\n",
    ] {
        assert!(
            ThemeFile::parse(&format!("{text}{extra}")).is_err(),
            "{extra}"
        );
    }
}
