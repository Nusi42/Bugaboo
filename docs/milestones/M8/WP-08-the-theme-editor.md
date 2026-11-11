# WP-08 — The theme editor

**Wave 2 · Folder / branch:** `bugaboo.win5` / `WIN5`
**Features:** THM-03, THM-04 (on screen). [ADR-0026](../../adr/0026-one-theme-file.md), update M8; ADR-0019 (the edit lock); UX design §5.6 and §4.13 (*Appearance*).

## Goal

An administrator makes themes for everyone in *Look › Themes*, and any person makes personal themes in *My settings › Appearance*, in one editor: color groups with plain names, a live preview of real screens, and the readability of each pair, worked out by the same code as the server's. A theme for everyone with a pair that is *Hard to read* cannot be saved.

## Owns

`web/app/src/features/theme-editor/**` (new: the editor, its groups, the preview, the readability line, *Create a theme*, *Save a copy*, *Download*, *Upload*), the lists of own themes on `web/app/src/features/settings/ThemesPage.tsx` and in `web/app/src/features/my-settings/` (`AppearancePage.tsx`, `ThemePicker.tsx`, `ThemePreview.tsx`), the use of the contrast of `bb-wasm` in `web/packages/wasm/`, the preview in `web/packages/ui/src/theme.ts` (only the preview attribute of the page frame), the messages, the parts the editor needs in `design/theme.kdl`, the user docs `userDocs/basics/own-themes.md` (new) and `userDocs/basics/appearance.md`.

## What is decided

- **The screens are UX design §5.6**: the groups *Backgrounds*, *Text*, *Lines*, *Highlight color*, *Status and tag colors*, *Shadow, corners and fonts*; each shows only the values the theme changes from its built-in theme, with *Use Dusk's color* to go back; the faces for *Text*, *Headings* and *Long text* among the built-in ones; the corners of controls and of cards.
- **The preview** draws real parts of the app (a list row, a pill, tags, a button, a field, a card) under the theme being edited, through the existing preview attribute of the page frame (`data-preview-look`) and a style sheet of the draft: the editor keeps its unsaved values as the theme's draft on the server and links the draft's sheet (WP-05: `/themes/{id}/draft.css`), never a sheet made in the browser, so the policy stays `style-src 'self'`. Nothing reaches other people until *Save*.
- **Readability** (*Easy*, *OK*, *Hard to read*) comes from `bb-core::contrast` through `bb-wasm`, with the same answers as the server; the line on top sums it up (*"42 pairs easy to read, 1 hard to read"*). A theme for everyone with a pair *Hard to read* cannot be saved, and the message names the pairs with *Show*; a personal theme asks *"2 pairs are hard to read. Save anyway?"*. The server checks again on save.
- **An edit lock** (ADR-0019): while somebody edits a theme, others see *"Ana is editing this theme"* with *Ask … to finish* and *Edit anyway* as on a long text.
- **The words** are the glossary's: *Own theme*, *For everyone*, *Personal*, *Theme editor*, *Readability*, *Save a copy*, *Download*, *Upload*, *Create a theme*. No *custom*, *clone* or *contrast ratio* on a screen.
- **Lazy:** the editor is a chunk of its own, loaded when first opened (PERF-04).

## Tasks

1. Write the acceptance tests below.
2. The editor, its groups and the preview; *Create a theme* (which built-in theme to start from), *Save a copy*, *Download*, *Upload* with the server's refusals shown at their values.
3. Readability through `bb-wasm`; saving with the rules above.
4. The lists of themes for everyone and of personal themes, with *Delete* and its sentence (*"12 people use this theme. They will see Dusk instead."*); the theme pickers list built-in themes, then themes for everyone, then the person's own.
5. The edit lock on a theme.
6. The editor on a phone: the groups as a list, the preview from a button.
7. User docs; pictures of every screen of the editor in the four themes, on computer and phone, looked at against UX §1.1 and §7 before the push; the report names them.

## Acceptance tests (write first)

- `thm_03_the_editor_rates_each_pair_like_the_server` (component, the cases of `bb-core`).
- `thm_03_a_theme_for_everyone_that_is_hard_to_read_cannot_be_saved_until_fixed` (browser; the exit demo's step).
- `thm_03_a_personal_theme_that_is_hard_to_read_is_saved_after_a_warning` (browser).
- `thm_03_the_preview_changes_as_you_type_and_nobody_else_sees_it_before_save` (browser, two people).
- `thm_04_an_administrator_creates_a_theme_for_everyone_and_another_person_chooses_it` (browser).
- `thm_04_a_downloaded_theme_uploads_again_unchanged` (browser).
- `thm_04_a_second_person_sees_who_is_editing_the_theme` (browser).
- TEST-A11Y: the editor, keyboard only (every color can be typed; no value needs a drag).

## Checks

THEME-ONLY, STY-PARTS, STY-TOKENS, STY-BOLD, STY-CONTRAST, SEC-HEADERS, TXT-LITERAL, TXT-CATALOG, TXT-TERMS, ARCH-WEB-API, ARCH-WEB-FEATURES, TEST-A11Y, TEST-EDIT-LOCKS, PERF-BUNDLE, LINT-TS.

## Review

Code review with the SSOT/DRY/ODR gate: readability only from `bb-core` (no contrast maths in TypeScript), the list of what a theme may set only from the generated list, the lock through the existing lock of long texts. A **security review**: no sheet or style is made in the browser, uploads are judged by the server, a personal theme never reaches another person. A **UX review** from the pictures in the four themes, on computer and phone, against UX design §5.6, §1.1 and §7.

## Depends on

Wave 1 merged (WP-05: stored themes, the sheet, the contrast in `bb-core` and `bb-wasm`).
