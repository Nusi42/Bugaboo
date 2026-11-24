# WP-12 — The relation map on screen

**Wave 3 · Folder / branch:** `bugaboo.win4` / `WIN4` · **Model:** Opus (our own layout, nothing moves under the cursor, a picture with a full list form for screen readers)
**Features:** ISS-16 (owner); PRJ-06 shown. UX design §4.15, §3.3; ADR-0009 amendment (WP-01); answers M10-Q10, Q11.

## Goal

*Show relation map* on an issue, and *Show as › Map* for a search, draw the map of UX design §4.15: the starting issue on the left, the others in columns by steps away ([ADR-0009](../../adr/0009-frontend-stack.md), amendment 2026-11-24), each kind of relation with its own line and word, the chain of a pointed issue highlighted, *Start from here* to walk along a chain, loops flagged, and the list form that phones and screen readers get. The picture shows exactly what the route answers and nothing more.

## Where

- **Owns:** `web/app/src/features/relation-map/**` (new, a lazy chunk: the layout, the cards, the lines and their labels, *Steps away*, *Kinds shown*, zoom and *Fit*, the side panel's chain, the list form), the entry *Map* of *Show as* in `web/app/src/features/search/`, *Show relation map* on the issue page (`web/app/src/features/issues/`, the menu entry), the address `/i/WEB-142/map` in `paths.ts`, the parts `map-card`, `map-line` and the kinds' line styles in `design/theme.kdl`, `userDocs/issues/relation-map.md` (new).
- **Reuses, does not copy:** the row state of cards (R4, R9), the side panel, the words of each kind of relation from the API (never a list in TypeScript).
- **Shared files, one entry each:** the *Show as* switch, `paths.ts`, `messages/en.json`, `design/theme.kdl` (its own parts).

## What is decided

- **Our own layout** (M10-Q10): as [ADR-0009](../../adr/0009-frontend-stack.md), amendment 2026-11-24, item 2 decides it (the one home of the layout); the same answer gives the same picture. Positions only through `variableOf` on tokens marked `run-time="true"` (coding-standards §4, THEME-ONLY), as the timeline's bars.
- **Nothing moves under the cursor** (UX design §6.4): the picture is laid out once per answer; pointing only highlights; *Start from here* reads a new answer and draws it with the new start on the left.
- **Lines** (UX design §4.15): one style per kind from the theme file (*blocks* orange with an arrow, *causes* red with an arrow, *duplicate* dashed grey, *related* grey without an arrow, *part of* violet towards the parent); each line's word in a label that never touches a line (R3); color is never the only signal.
- **Toolbar:** *Steps away* 1 · 2 · 3 · *Everything* (5, M10-Q11), *Kinds shown* (chips; never *Follow*), *Picture · List*, zoom with a percentage and *Fit*, and the note "Only issues you may see are shown." Over 200 issues the map says so and suggests fewer steps or fewer kinds.
- **Walking:** pointing at (or tapping) an issue highlights its chain back to the start for a moment; clicking fades everything not on its chain, and the side panel shows the chain as numbered sentences ("WEB-142 blocks WEB-147, which is blocked by WEB-148") with *Start from here* and *Open issue*.
- **Loops** from the route are flagged with a warning sign and the sentence "WEB-160 blocks WEB-161, which blocks WEB-160. Neither can start until one of these relations is removed."
- **The list form:** the same chains as sentences, grouped by steps away, each issue with its status icon and priority edge. It is what phones show first (with *Show as picture*), and what screen readers get: the picture is one image with the label "Relation map of WEB-142, 8 issues, see the list", and the list is the reading order.

## Tasks

1. Write the acceptance tests below.
2. The layout and its unit tests (same answer, same picture; the fixed order in a column).
3. Cards, lines, labels, the legend; zoom, *Fit*, moving the background.
4. The toolbar, pointing, the chain in the side panel, *Start from here*; loops.
5. The list form, the phone, the screen-reader texts; *Show as › Map*.
6. User docs; pictures in the four themes on computer and phone, looked at against UX §1.1, §7 and §6.4 before the push; the report names them.

## Acceptance tests (write first)

- `iss_16_show_relation_map_draws_the_start_on_the_left_and_columns_by_steps_away` (browser).
- `iss_16_the_same_answer_gives_the_same_picture` (unit).
- `iss_16_pointing_highlights_the_chain_and_moves_nothing` (browser, `expectStill`).
- `iss_16_clicking_shows_the_chain_as_sentences_and_start_from_here_walks_on` (browser).
- `iss_16_kinds_shown_and_steps_away_narrow_the_map` (browser).
- `iss_16_a_loop_is_flagged_with_its_sentence` (browser).
- `iss_16_the_list_form_holds_the_same_chains_as_the_picture` (browser).
- `iss_16_the_phone_starts_in_the_list_form` (browser, phone).
- `iss_16_over_200_issues_the_map_says_so` (browser).
- `iss_16_show_as_map_draws_a_search_and_the_relations_between_its_results` (browser).
- `iss_16_the_screen_draws_nothing_the_answer_does_not_hold` (browser: a reader with a hidden issue between two of theirs sees no line and no gap).
- TEST-A11Y: the map's picture and list form in the four themes; the keyboard reaches every card and *Start from here*.

## Checks

TEST-A11Y, THEME-ONLY, STY-PARTS, STY-TOKENS, STY-BOLD, STY-CONTRAST, TXT-LITERAL, TXT-CATALOG, TXT-TERMS, ARCH-WEB-API, ARCH-WEB-FEATURES, PERF-BUNDLE (the map is a lazy chunk), DEP-UNUSED (d3-force not added), LINT-TS.

## Docs to update

`userDocs/issues/relation-map.md` (new), `userDocs/issues/relations.md` (a link), the README of the issues docs; the ADR-0009 amendment's *Enforced by* line.

## Review

Code review with the SSOT/DRY/ODR gate: positions only through tokens, the kinds' words and styles from their one source. Security review: the screen draws only the answer. A UX review of the pictures in the four themes on computer and phone against UX design §4.15.

## Personal data

None: the map carries no people.

## Depends on

WP-05 merged (the map route).
