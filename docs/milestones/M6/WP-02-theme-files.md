# WP-02 — Theme files like the sister project: a base file and one file of colors per built-in theme

**Wave 1, the first code package of M6 · Folder / branch:** `bugaboo.win2` / `WIN2`
**Features:** THM-01 (the four built-in themes), THM-02, THM-06 (unchanged, proven again); the ground for THM-03 and THM-04 (M7). ADR-0026 with its amendment M6; follow-ups M5-F167, M5-F121.

## Goal

Bugaboo handles its themes like the sister project (a decision of the maintainer): a base theme file holds everything about how the app looks, and each built-in theme is a small file that only sets its colors and says `extends="default"`. A new theme, and later a custom one, is one short file of colors, not four values added to every color line of a 14,000-line file. Nothing on screen changes.

## Owns

`design/theme.kdl` (the base), `design/themes/**` (new), `xtask/src/gen/theme*` (the reader and the writer), the generated files (`web/packages/ui/src/theme.css`, `tokens.ts`, `crates/bb-core/src/settings/themes.rs`, `docs/design/*.css`, `docs/design/frame.js`), the parts of THEME-ONLY, STY-TOKENS, STY-CONTRAST, STY-BOLD and LINT-CSS that read the theme file, ADR-0026's amendment M6 (the details), coding-standards §4 and the line of `CLAUDE.md` that names the theme file.

It touches no feature of `web/app/src/features/` and no Rust use case.

## What is decided (M6-Q1)

- `design/theme.kdl` is the base theme `default`: `defaults`, `frame`, `variants`, every token with its base value and its values for `forced=`, `more=` and the choices, the `contrast`, `distinct` and `bold` rules, mixins, keyframes, part rules and the `docs` blocks.
- Today a color line has no value of its own, only the four theme values (`color "bg-canvas" daylight="…" paper="…" dusk="…" midnight="…"`). In the base, each color gets **the value of Daylight** (the default light theme of `defaults`) and its `more=` value; `forced=` and the choices stay on the line.
- `design/themes/<id>.kdl`, one per built-in theme: `theme "<id>" extends="default" kind="light|dark" { tokens { color "<token>" "<value>" more="<value>" … } }`. A theme file sets **colors only, and only the colors it changes**, so no value is written twice (Daylight's file is short or empty; Dusk and Midnight set nearly every color). A shadow is a shape in the base and a color token in each theme. A theme file that sets another kind of token, names a token the base does not have, or extends anything but `default` is an error of `gen theme` with the file and the line.
- **The file name is the theme's ID**, and the list of themes is the list of files: there is no `themes { … }` block left to keep in step. `tokens.ts` (`themeIds`, `lightThemeIds`, `darkThemeIds`) and `themes.rs` are generated from the files as before.
- **A token may name another token** (M5-F121): `color "hover-line" "$border-control"`. `gen theme` writes `var(--border-control)` where the base keeps the link, and STY-CONTRAST reads the value the link leads to in each theme and mode. A loop or a name that does not exist is an error. `hover-line` and `st-ns` use it.
- The content security policy stays `style-src 'self'`: the files are compiled at build time into the one static sheet, as ADR-0026 decision 5 says. Nothing parses KDL at run time.

## Tasks

1. **Before anything moves:** keep a copy of every generated file in `.work/theme-before/` and take the pictures of the showcase in four themes on computer and phone into `.work/screenshots/before/`.
2. **The reader.** `gen theme` reads the base and every file of `design/themes/` in name order; it merges each theme over the base; it refuses the errors above with the file and the line. Write the bad and good examples first (META-FIXTURES): a theme that sets a size, a theme that names an unknown token, a loop of two tokens, a theme file without `extends`.
3. **The move.** A small program in `.work/` (not committed) writes Daylight's value of each color as the base value (`daylight-more=` as `more=`), and each other theme's value into its theme file only where it differs from the base. Every theme is merged over the base, so a theme that sets no color looks like Daylight.
4. **The proof.** Every generated file is the same byte for byte as the copy of task 1, except the lines of `hover-line` and `st-ns`, which now name their source. The pictures of task 1 taken again are the same (a pixel compare in `.work/`, not committed); the report names how many pictures were compared and how many differed.
5. **The checks.** STY-CONTRAST runs per theme file and mode; STY-TOKENS, LINT-CSS and THEME-ONLY read the theme folder; DOC-GENERATED compares the output of both. A check message names the theme file, not only the token.
6. **The docs.** ADR-0026 amendment M6 (the syntax, `extends`, the link between tokens, why the file name is the ID); coding-standards §4 and `CLAUDE.md` ("all theming in the theme files of `design/`: the base `theme.kdl` and one color file per theme"); the user docs of themes do not change.

## Acceptance tests (write first)

- `gen theme` refuses each bad example with its file and line; the good example (a fifth theme file in the fixtures) yields a fifth theme in `tokens.ts` and `themes.rs` without any other change.
- `thm_01_the_four_built_in_themes_are_the_files_of_the_theme_folder`.
- `thm_01_a_token_that_names_another_takes_its_value_in_every_theme_and_mode`, and a loop is refused.
- DOC-GENERATED green; the sheet the same as before apart from the two tokens of M5-F121 (the report shows the diff).
- STY-CONTRAST green for every theme file and mode.
- The browser tests of the themes (`Match my device`, the theme picker) pass unchanged.

## Checks

STY-CONTRAST, STY-TOKENS, LINT-CSS, THEME-ONLY and DOC-GENERATED grow to the theme folder. No new check: META-COVERAGE already covers `.kdl`.

## Review

The SSOT/DRY/ODR gate of coding-standards §1: the list of themes written once (the files), no theme value left in the base, no second reader of KDL. A UX reviewer compares the pictures before and after in four themes on computer and phone.

## Depends on

`main` at the M6 plan. It is started first and merged first in wave 1; WP-08, WP-09, WP-10 and WP-11 build on it.
