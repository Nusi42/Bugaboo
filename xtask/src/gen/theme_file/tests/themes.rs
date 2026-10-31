//! The theme files of `design/themes/`: what the reader takes and what it refuses (THM-01).

use super::*;
use crate::generate::theme_file::themes::THEME_FILE_ENDING;

/// The base of the repository.
fn base() -> String {
    crate::source::read_text(&Repo::discover().unwrap(), THEME_PATH).unwrap()
}

/// A theme file `design/themes/<id>.kdl` with these token lines.
fn theme_file(id: &str, kind: &str, lines: &str) -> ThemeSource {
    ThemeSource {
        path: format!("{THEMES_FOLDER}/{id}{THEME_FILE_ENDING}"),
        text: format!(
            "// A theme of the tests.\ntheme \"{id}\" extends=\"default\" kind=\"{kind}\" {{\n    tokens {{\n{lines}\n    }}\n}}\n"
        ),
    }
}

/// The base and the theme files of the repository, with one more file.
fn with_theme(extra: ThemeSource) -> anyhow::Result<ThemeFile> {
    let mut sources = theme_sources(&Repo::discover().unwrap()).unwrap();
    sources.push(extra);
    ThemeFile::parse(&base(), &sources)
}

/// The problem with one more theme file, as it is reported.
fn problem_with_theme(extra: ThemeSource) -> String {
    with_theme(extra)
        .err()
        .map(|error| format!("{error:#}"))
        .unwrap_or_default()
}

#[test]
fn thm_01_the_four_built_in_themes_are_the_files_of_the_theme_folder() {
    let repo = Repo::discover().unwrap();
    let file = ThemeFile::load(&repo).unwrap();
    let names: Vec<&str> = file
        .themes
        .iter()
        .map(|theme| theme.name.as_str())
        .collect();
    // Light ones first, each kind in name order.
    assert_eq!(names, ["daylight", "paper", "dusk", "midnight"]);
    let mut files: Vec<String> = theme_sources(&repo)
        .unwrap()
        .into_iter()
        .map(|source| source.path)
        .collect();
    files.sort();
    let mut paths: Vec<String> = file.themes.iter().map(|theme| theme.path.clone()).collect();
    paths.sort();
    assert_eq!(files, paths, "every file is a theme and every theme a file");
    for theme in &file.themes {
        assert_eq!(
            theme.path,
            format!("{THEMES_FOLDER}/{}{THEME_FILE_ENDING}", theme.name)
        );
    }
    // The base lists no theme of its own.
    assert!(!base().lines().any(|line| line.starts_with("themes")));
}

#[test]
fn thm_01_the_base_has_the_colors_of_the_light_theme_that_starts_and_no_value_is_written_twice() {
    let repo = Repo::discover().unwrap();
    let file = ThemeFile::load(&repo).unwrap();
    for color in file.colors() {
        let daylight = color
            .themes
            .iter()
            .find(|value| value.theme == file.default_light)
            .unwrap();
        assert_eq!(
            daylight.path, THEME_PATH,
            "{} comes from the base",
            color.name
        );
    }
    // A theme file sets a color only where it differs from the base.
    for source in theme_sources(&repo).unwrap() {
        let id = source
            .path
            .trim_start_matches(THEMES_FOLDER)
            .trim_start_matches('/')
            .trim_end_matches(THEME_FILE_ENDING);
        for color in file.colors() {
            let own = color.themes.iter().find(|value| value.theme == id).unwrap();
            if own.path != source.path {
                continue;
            }
            let daylight = color
                .themes
                .iter()
                .find(|value| value.theme == file.default_light)
                .unwrap();
            assert!(
                (own.value.as_str(), own.more.as_deref())
                    != (daylight.value.as_str(), daylight.more.as_deref()),
                "{}: `{}` repeats the base",
                source.path,
                color.name
            );
        }
    }
}

#[test]
fn thm_01_a_token_that_names_another_takes_its_value_in_every_theme_and_mode() {
    let file = ThemeFile::load(&Repo::discover().unwrap()).unwrap();
    // The base names the source of the two (M5-F121); a theme that does not set them follows it.
    for (token, source) in [("hover-line", "border-control"), ("st-ns", "fg-muted")] {
        let base = file.token(token).unwrap();
        assert_eq!(
            base.in_theme("daylight"),
            Some(format!("${source}").as_str())
        );
        assert_eq!(
            file.color(token, "daylight"),
            file.color(source, "daylight")
        );
        assert_eq!(
            file.color_with_more_contrast(token, "daylight"),
            file.color_with_more_contrast(source, "daylight")
        );
    }
    // Midnight names the source too, with a color of its own for more contrast.
    assert_eq!(
        file.color("st-ns", "midnight"),
        file.color("fg-muted", "midnight")
    );
    assert_eq!(
        file.color_with_more_contrast("st-ns", "midnight"),
        Some("#abb1bb")
    );
    // A link in a theme of its own, in both modes, and in the sheet as a variable.
    let linked = with_theme(theme_file(
        "harbor",
        "light",
        "        color \"fg-subtle\" \"$fg\" more=\"#000000\"",
    ))
    .unwrap();
    assert_eq!(
        linked.color("fg-subtle", "harbor"),
        linked.color("fg", "harbor")
    );
    assert_eq!(
        linked.color_with_more_contrast("fg-subtle", "harbor"),
        Some("#000000")
    );
    let css = crate::generate::theme::css(&linked).unwrap();
    assert!(
        css.contains("--fg-subtle: var(--fg);"),
        "the sheet keeps the link"
    );
}

#[test]
fn bad_example_a_loop_of_two_tokens_is_refused() {
    let error = problem_with_theme(theme_file(
        "harbor",
        "light",
        "        color \"fg-muted\" \"$st-ns\"",
    ));
    assert!(error.contains("loop"), "{error}");
    assert!(
        error.contains("design/themes/harbor.kdl: line 4"),
        "{error}"
    );
}

#[test]
fn bad_example_a_link_to_a_name_that_is_no_color_is_refused() {
    for (target, problem) in [
        ("no-such-color", "is not a token"),
        ("space-1", "is no color token"),
    ] {
        let error = problem_with_theme(theme_file(
            "harbor",
            "light",
            &format!("        color \"fg-muted\" \"${target}\""),
        ));
        assert!(error.contains(problem), "{error}");
        assert!(error.contains("design/themes/harbor.kdl"), "{error}");
    }
}

#[test]
fn bad_example_a_theme_that_sets_a_size_is_refused() {
    let error = problem_with_theme(theme_file(
        "harbor",
        "light",
        "        size \"space-1\" \"5px\"",
    ));
    assert!(error.contains("design/themes/harbor.kdl"), "{error}");
    assert!(error.contains("line 4"), "{error}");
    assert!(error.contains("colors (and shadows) only"), "{error}");
}

#[test]
fn bad_example_a_theme_that_names_an_unknown_token_is_refused() {
    let error = problem_with_theme(theme_file(
        "harbor",
        "light",
        "        color \"bg-nowhere\" \"#ffffff\"",
    ));
    assert!(error.contains("design/themes/harbor.kdl"), "{error}");
    assert!(error.contains("line 4"), "{error}");
    assert!(error.contains("no color token `bg-nowhere`"), "{error}");
}

#[test]
fn bad_example_a_theme_file_without_extends_is_refused() {
    let mut source = theme_file("harbor", "light", "");
    source.text = source.text.replace(" extends=\"default\"", "");
    let error = problem_with_theme(source);
    assert!(error.contains("design/themes/harbor.kdl"), "{error}");
    assert!(error.contains("line 2"), "{error}");
    assert!(error.contains("extends=\"default\""), "{error}");
    let mut other = theme_file("harbor", "light", "");
    other.text = other
        .text
        .replace("extends=\"default\"", "extends=\"dusk\"");
    assert!(problem_with_theme(other).contains("extends=\"default\""));
}

#[test]
fn bad_example_a_theme_file_that_does_not_keep_the_shape_is_refused() {
    // The name in the file is not the name of the file.
    let mut renamed = theme_file("harbor", "light", "");
    renamed.path = format!("{THEMES_FOLDER}/haven{THEME_FILE_ENDING}");
    assert!(problem_with_theme(renamed).contains("the file name is the theme's ID"));
    // A file of the folder that is no theme file.
    let mut other = theme_file("harbor", "light", "");
    other.path = format!("{THEMES_FOLDER}/Harbor.txt");
    assert!(problem_with_theme(other).contains("one file per theme"));
    // A theme that is neither light nor dark, a color set twice, more contrast for a shadow, and a
    // value that is no color.
    assert!(problem_with_theme(theme_file("harbor", "grey", "")).contains("`kind` is"));
    let twice = "        color \"fg\" \"#000000\"\n        color \"fg\" \"#111111\"";
    assert!(problem_with_theme(theme_file("harbor", "light", twice)).contains("set twice"));
    let shadow = "        shadow \"shadow\" \"none\" more=\"none\"";
    assert!(problem_with_theme(theme_file("harbor", "light", shadow)).contains("not a mode"));
    let red = "        color \"fg\" \"red\"";
    assert!(problem_with_theme(theme_file("harbor", "light", red)).contains("#rrggbb"));
    // The base keeps no list of themes and no value per theme.
    let listed = base().replacen(
        "\nvariants {",
        "\nthemes {\n    theme \"daylight\" kind=\"light\"\n}\nvariants {",
        1,
    );
    let error = ThemeFile::parse_base(&listed)
        .err()
        .map(|error| format!("{error:#}"))
        .unwrap_or_default();
    assert!(error.contains("the files of design/themes/"), "{error}");
    let per_theme = base().replacen(
        "color \"fg\" \"#16181d\"",
        "color \"fg\" \"#16181d\" dusk=\"#e5e8ed\"",
        1,
    );
    let error = ThemeFile::parse_base(&per_theme)
        .err()
        .map(|error| format!("{error:#}"))
        .unwrap_or_default();
    assert!(error.contains("its own file"), "{error}");
}

#[test]
fn good_example_a_fifth_theme_file_is_a_fifth_theme_in_the_names_and_the_settings() {
    let repo = Repo::discover().unwrap();
    let four = ThemeFile::load(&repo).unwrap();
    let five = with_theme(theme_file(
        "harbor",
        "light",
        "        color \"bg-canvas\" \"#f7fbff\"",
    ))
    .unwrap();
    let names: Vec<&str> = five
        .themes
        .iter()
        .map(|theme| theme.name.as_str())
        .collect();
    assert_eq!(names, ["daylight", "harbor", "paper", "dusk", "midnight"]);
    // A theme that sets one color has the base's value of every other.
    assert_eq!(five.color("bg-canvas", "harbor"), Some("#f7fbff"));
    assert_eq!(five.color("fg", "harbor"), four.color("fg", "daylight"));
    let output = |file: &ThemeFile, path: &str| {
        crate::generate::theme::generate(file)
            .unwrap()
            .into_iter()
            .find(|generated| generated.path == path)
            .unwrap()
            .content
    };
    let changed = |path: &str| -> Vec<String> {
        let (before, after) = (output(&four, path), output(&five, path));
        let before: Vec<&str> = before.lines().collect();
        after
            .lines()
            .filter(|line| !before.contains(line))
            .map(str::to_owned)
            .collect()
    };
    // The TypeScript names: only the two lists of IDs change.
    assert_eq!(
        changed("web/packages/ui/src/tokens.ts"),
        [
            "export const themeIds = [\"daylight\", \"harbor\", \"paper\", \"dusk\", \"midnight\"] as const;",
            "export const lightThemeIds = [\"daylight\", \"harbor\", \"paper\"] as const;",
        ]
    );
    // The choices of the settings: one more.
    assert_eq!(
        changed("crates/bb-core/src/settings/themes.rs"),
        ["    Choice::new(\"harbor\", message::THEME_HARBOR),"]
    );
}

#[test]
fn bad_example_a_value_for_more_contrast_that_is_the_value_itself_is_refused() {
    // It says nothing: without `more=` the color is the same with more contrast.
    let same = "        color \"fg\" \"#000000\" more=\"#000000\"";
    let error = problem_with_theme(theme_file("harbor", "light", same));
    assert!(error.contains("design/themes/harbor.kdl"), "{error}");
    assert!(error.contains("leave it out"), "{error}");
    let base = base().replacen(
        "color \"fg-muted\" \"#565d6b\" more=\"#464c57\"",
        "color \"fg-muted\" \"#565d6b\" more=\"#565d6b\"",
        1,
    );
    let error = ThemeFile::parse_base(&base)
        .err()
        .map(|error| format!("{error:#}"))
        .unwrap_or_default();
    assert!(error.contains("leave it out"), "{error}");
}
