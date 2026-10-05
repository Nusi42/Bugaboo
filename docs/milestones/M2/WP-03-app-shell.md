# WP-03 — App shell: sidebar, top bar, phone bar, panels, keyboard, optimistic helpers, themes

**Wave 1 · Folder / branch:** `bugaboo.win3` / `WIN3`
**Features:** UX-01, UX-04, UX-05, UX-06, THM-01, THM-02, THM-06, CFG-04 (appearance in the browser); UX design §1, §2.1, §3, §6, §7, §8; ADR-0009, ADR-0010; follow-ups M0-F04, M0-F05, M1-F12.

## Goal

The frame every later screen lives in, the same on a computer, a tablet and a phone, in all four themes: sidebar, top bar, phone bottom bar, focus view and side panel, the list → editor pattern of the settings screens, a quick-search frame, one keyboard shortcut system and helpers that make changes feel instant.

## Owns

`web/app/src/features/shell/**`, `web/app/src/routes/{layouts,Root,router}*`, `web/app/src/shared/{paths,focus,use-page}*` and new `shared/{shortcuts,optimistic}*`, `web/app/index.html`, `web/packages/ui/**` (new components), `design/tokens.json` (density, motion and more-contrast tokens), the TypeScript part of `cargo xtask gen messages` (`xtask/src/gen/messages.rs`), checks `xtask/src/checks/{test_a11y,txt_literal,sty_tokens,arch_web_features}.rs` and their fixtures, `userDocs/basics/*` (new).

## Tasks

1. **Layout (UX-01, UX §3):** computer (1280 px and wider): sidebar with the main sections, a place for the project tree (filled by WP-08), *Settings* at the bottom, can be narrowed to icons; top bar with where you are (`ACME › Website`). Tablet: the sidebar slides in from a *Menu* button. Phone: one screen at a time with *Back*, a bottom bar with labels; only items whose screen exists are shown (*New* and *Notifications* come with M3 and M4), and *More* holds the rest. Bottom sheets replace small pop-ups on phones.
2. **Frames:** focus view and side panel (UX §3.4) as empty frames with their keyboard behavior; the **list → editor pattern** (UX §5.1): a searchable list, a simple item opens in the side panel, a big one gets a full page; the *Organization settings* frame (left menu by topic, only topics that exist) and the *My settings* frame (WP-07 and WP-09 add their pages). Move the M1 pages to the addresses of UX §2.1 (`/me/…`, `/settings/…`).
3. **Quick-search frame:** `Ctrl+K` (and the search box) opens a dialog with a result list that goes to the pages and, once WP-08 adds them, projects; sources are pluggable so CFG-08 (M5) adds issues and actions.
4. **Keyboard (UX-05):** one shortcut registry, defined once (ID, default keys, message key, where it works); sequences such as `g h`; never fires while typing in a field; `?` opens the list of all shortcuts; a person's own keys can replace the defaults (WP-07 stores them). Every shortcut also has a visible button or menu item (principle 9).
5. **Optimistic helpers (UX-06):** a mutation helper on TanStack Query that shows the change at once, rolls back with a plain message when the server refuses, and shows *Saving… / Saved / Not saved yet*; skeleton shapes that appear only after 0.3 s; *Undo* for 10 seconds for reversible actions (§6). Target: under 100 ms from the click to the visible change.
6. **Themes (THM-01, THM-02, THM-06):** the four themes and *Match my device*, with one light and one dark choice (THM-02); `prefers-reduced-motion` and the *Less motion* choice turn motion off; `prefers-contrast: more` gives stronger text and lines (new tokens, checked by STY-CONTRAST); *Larger text* and the spacing choice scale through tokens. Until WP-07 stores them for the person, the choices live in the browser (`useAppearance()` takes its values from one source, so WP-07 only swaps the source).
7. **Follow-ups:** the page title comes from the message catalog at build time (M0-F04); look again at the first frame in the device theme now that the choice may come from the server, and write the result into the follow-up (M0-F05); the generated TypeScript catalog holds only the areas the browser shows (errors, setting help, UI), not server-only ones such as `config.*` (M1-F12).
8. **New dependencies:** icons (Lucide, ISC, only the icons used, UX §7.5) and TanStack Virtual if the shell needs it; register entries.

## Acceptance tests (write first)

- Playwright on the e2e port: the shell on computer, tablet and phone sizes; *Back* and the phone's back gesture; the bottom bar shows only existing screens; `Ctrl+K` opens and closes quick search and `Esc` returns focus where it was.
- Keyboard: every shortcut of the registry is in the `?` list; a shortcut does nothing while a text field has focus; a changed key works and the old one does not (Vitest).
- Optimistic helper: with a slowed server, the change is visible at once and *Saved* follows; a refused change rolls back with the message (Vitest and one Playwright test with a delayed route).
- Themes: four themes × *Match my device* light and dark, *Less motion*, more contrast; axe finds no violation on every page in every theme, on computer and phone (TEST-A11Y).
- Screenshots of the shell for the M2 review page.

## Checks

- **TEST-A11Y** switched on: the Playwright suite visits every address in `paths.ts` in four themes on computer and phone with axe; the check fails when an address has no entry in that suite (bad fixture: a path without a test).
- **TXT-LITERAL**, **STY-TOKENS**, **ARCH-WEB-FEATURES** switched on (catalogue entries exist; implementation, fixtures). Any existing finding is fixed, not suppressed.

## Depends on

M1. WP-07 later connects the appearance choices and own shortcut keys to *My settings*; WP-08 fills the project tree.
