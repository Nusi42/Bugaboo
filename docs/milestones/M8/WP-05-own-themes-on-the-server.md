# WP-05 — Own themes on the server: stored themes, their style sheet, contrast in one place

**Wave 1 · Folder / branch:** `bugaboo.win5` / `WIN5`
**Features:** THM-04 (on the server), THM-03 (the contrast). [ADR-0026](../../adr/0026-one-theme-file.md), update M8 (M8-Q8 to Q11); ADR-0010, ADR-0025 (amendments M8); follow-ups M0-F03, M6-F10, M6-F13, M6-F74.

## Goal

An organization and each person can keep own themes on the server, as token values, and every page that uses one gets its style sheet from Bugaboo without loosening the content security policy. Readability is worked out once, in `bb-core`, for the check of the built-in files, for the server and later for the editor (WP-08).

## Owns

The contrast maths and the reading of the `contrast` and `distinct` rules, moved from `xtask/src/checks/sty_contrast.rs` into `crates/bb-core/src/contrast/`; the list of the tokens an own theme may set and the rules of the base, written by `cargo xtask gen theme` (`xtask/src/gen/theme*.rs`) into `crates/bb-core/src/theme/` (generated, DOC-GENERATED); the frame attribute `own` in `design/theme.kdl`; the use cases in `crates/bb-app/src/themes/**`, the store in `crates/bb-store/src/themes*`, migration 51 (`crates/bb-store/src/migrations/m0051_own_themes.rs`), the routes `crates/bb-api/src/api/themes*.rs` and the style sheet `/themes/{id}.css` (`crates/bb-api/src/web.rs` and its router), the theme choices in `crates/bb-core/src/settings/themes.rs` and the settings catalogue, the theme files of `design/themes/` only for the follow-ups below, `web/packages/ui/src/theme.ts` (linking the sheet of an own theme).

## What is decided

- **Stored as token values only** (ADR-0026, update M8): a theme has a name (at most `THEME_NAME_MOST` characters), *for everyone* or *personal* with its person, the built-in theme it extends, a version, and the values it changes. Allowed: every `color` token (with its `more=`), the `shadow` token as parts (at most two layers of offsets, blur, spread and a color), `r-ctl` and `r-card` within the base file's bounds, and for `font-ui`, `font-display` and `font-reader` the name of one of those three stacks. Anything else is refused with its name. Limits as named constants: themes for everyone (`THEMES_FOR_EVERYONE_MOST`, 50), personal themes per person (`PERSONAL_THEMES_MOST`, 10), the size of an upload (`THEME_UPLOAD_MOST`, 64 KiB).
- **The style sheet** (M8-Q8): one writer in Rust turns the stored values and the extended built-in theme into custom properties under `:root[data-own-theme="<id>"]`, with a block for more contrast; nothing but variables. It is served at `/themes/{id}.css?v=<version>` with the cache rules of a hashed asset, under `style-src 'self'` (SEC-HEADERS unchanged). A theme for everyone is served to every signed-in person of the organization; a personal theme only to its maker; any other request answers as a theme that does not exist.
- **A draft for the preview** (for WP-08): the unsaved values of the editor are kept on the server as the theme's draft, for its editor only, and served at `/themes/{id}/draft.css?v=<n>` by the same writer; a new theme is a draft until its first save and is listed nowhere before. *Save* makes the draft the theme. The page never builds a sheet itself, because colors are never set from code (coding-standards §4).
- **Contrast in one place** (M8-Q9): `bb-core::contrast` answers, for each pair of each rule, the ratio or the delta E and the rating *Easy*, *OK* or *Hard to read*. STY-CONTRAST calls it for the built-in files with the same results as before (its tests unchanged), the server calls it on every save and upload, and `bb-wasm` exports it for the editor (WP-08). The size of `bb-wasm` stays within its budget (PERF-BUNDLE).
- **Saving** (M8-Q11): a theme for everyone with a pair that is *Hard to read* is refused, naming the pairs (`theme.hard-to-read`); a personal theme is saved and the answer names the pairs. An upload is a save of a JSON file in the shape of the export (name, the built-in theme, the values); an unknown token or a bad value is refused with its name.
- **Who:** themes for everyone need *Change organization settings*; personal themes need a signed-in person. Each save and delete of a theme for everyone is an event (ARCH-EVENTS). Deleting a theme that people chose leaves their choice to count as the built-in theme it extended.
- **Theme choices:** the settings *Light theme* and *Dark theme* accept the built-in IDs (generated, as now) and the IDs of own themes the person may use, of the right kind; the API lists them for the person. A choice of an own theme that is gone reads as its built-in theme.
- **An edit lock** for the editor (ADR-0019): the theme is a lockable record, as WP-08 needs, with the existing lock rows.
- **Follow-ups here:** M0-F03 (faint and muted text too close in Paper and Daylight), M6-F10 (whether the themes share one shadow shape: decide it, now that a theme may set its shadow), M6-F13 (`Theme.kind` as a type), M6-F74 (label squares written twice in the dark themes). Each row is closed or moved with a reason.
- **Personal data** as architecture §12, *Personal data in M8*.

## Tasks

1. Write the acceptance tests below.
2. Move the contrast maths into `bb-core`; STY-CONTRAST calls it; `gen theme` writes the rules and the list of allowed tokens into `bb-core` (DOC-GENERATED).
3. The record, migration 51, the marks, the store on both databases, the routes (list, read, create, change, delete, download, upload) and the events.
4. The writer of the style sheet and its route; `theme.ts` links the sheet when the person's choice is an own theme and sets the frame attribute.
5. The theme choices in the settings registry and the API.
6. `bb-wasm` exports the contrast; its size is measured.
7. The four follow-ups.

## Acceptance tests (write first)

- `thm_03_the_contrast_of_bb_core_gives_the_built_in_files_the_results_of_the_check` (the old cases of STY-CONTRAST, unchanged).
- `thm_03_a_theme_for_everyone_with_a_pair_hard_to_read_is_refused`.
- `thm_03_a_personal_theme_with_a_pair_hard_to_read_is_saved_with_the_pairs_named`.
- `thm_04_an_own_theme_stores_only_the_values_it_changes_and_follows_its_built_in_theme`.
- `thm_04_a_value_outside_its_kind_or_an_unknown_token_is_refused` (also in an upload).
- `thm_04_the_style_sheet_holds_custom_properties_only` (values that try to close a rule, start a comment, import or call a URL are refused, never written).
- `thm_04_a_personal_theme_is_served_to_its_maker_only`.
- `thm_04_a_draft_is_served_to_its_editor_only_and_listed_nowhere`.
- `thm_04_a_theme_for_everyone_needs_change_organization_settings`.
- `thm_04_a_choice_of_a_deleted_theme_counts_as_its_built_in_theme`.
- `thm_04_the_page_uses_an_own_theme_without_an_inline_style` (browser; the CSP report finds nothing).
- SEC-MATRIX: the routes of themes and their style sheets, for every starting role, both databases.

## Checks

STY-CONTRAST (now through `bb-core`), DOC-GENERATED, SEC-HEADERS, THEME-ONLY, ARCH-PURE, ARCH-WASM, PERF-BUNDLE, DATA-MIGRATIONS, DATA-MARKS, ARCH-DB-PARITY, ARCH-EVENTS, ARCH-SETTINGS, SEC-MATRIX, TXT-TERMS.

## Review

Code review with the SSOT/DRY/ODR gate: one contrast maths for the check, the server and the browser (no second copy in TypeScript); one list of allowed tokens, generated from the base file; one writer of the sheet. A **security review**: an own theme reaches every page of its users, so the reviewer tries values that would break out of a custom property, the cache and the reach of a personal sheet, and the policy that stays `style-src 'self'`.

## Depends on

WP-01 and WP-02 merged. Runs beside WP-03 and WP-04.
